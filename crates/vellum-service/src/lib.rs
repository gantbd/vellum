//! Bounded in-process Vellum service interpreter.
#![forbid(unsafe_code)]

use std::sync::Arc;
use vellum_editor::{EditorView, Workspace, WorkspaceError};
use vellum_engine::{coordinator::DocumentVersion, source::Replacement};
use vellum_protocol::{
    ClientId, CommandV1, OutcomeV1, RefusalV1, ReplyV1, RequestV1, SnapshotV1, VersionWitness,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceLimits {
    pub max_clients: usize,
    pub max_issued_clients: usize,
    pub max_requests: usize,
    pub max_witnesses: usize,
    pub max_snapshot_bytes: usize,
    pub max_insert_bytes: usize,
}

impl vellum_protocol::ServicePort for Service {
    fn connect(&mut self) -> Result<ClientId, vellum_protocol::ServiceControlErrorV1> {
        self.connect()
            .map_err(vellum_protocol::ServiceControlErrorV1::Refused)
    }

    fn execute(
        &mut self,
        request: RequestV1,
    ) -> Result<ReplyV1, vellum_protocol::ServiceAccessErrorV1> {
        Ok(self.execute(request))
    }

    fn disconnect(
        &mut self,
        client: ClientId,
    ) -> Result<(), vellum_protocol::ServiceControlErrorV1> {
        self.disconnect(client)
            .map_err(vellum_protocol::ServiceControlErrorV1::Refused)
    }
}

impl ServiceLimits {
    pub const fn reference() -> Self {
        Self {
            max_clients: 8,
            max_issued_clients: 64,
            max_requests: 256,
            max_witnesses: 256,
            max_snapshot_bytes: 65_536,
            max_insert_bytes: 65_536,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceBuildError {
    InvalidLimits,
    Capacity,
}

/// Saturating, non-authoritative operational evidence.
///
/// Counters never participate in admission or semantic decisions. Retention
/// gauges are sampled when [`Service::usage`] is called. `counter_saturated`
/// is sticky so an observer can detect loss of numeric fidelity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServiceUsage {
    pub connect_attempts: u64,
    pub connected: u64,
    pub connection_refused: u64,
    pub disconnect_attempts: u64,
    pub disconnected: u64,
    pub disconnect_replayed: u64,
    pub disconnect_refused: u64,
    pub requests: u64,
    pub accepted: u64,
    pub refused: u64,
    pub replayed: u64,
    pub conflicts: u64,
    pub stale: u64,
    pub invalid: u64,
    pub limited: u64,
    pub unavailable: u64,
    pub counter_saturated: bool,
    pub active_clients: usize,
    pub client_capacity: usize,
    pub issued_clients: usize,
    pub issued_client_capacity: usize,
    pub retired_clients: usize,
    pub retained_requests: usize,
    pub request_capacity: usize,
    pub retained_witnesses: usize,
    pub witness_capacity: usize,
}

struct WitnessRecord {
    client: ClientId,
    id: VersionWitness,
    version: DocumentVersion,
    text: Arc<str>,
}

struct RequestRecord {
    request: RequestV1,
    reply: ReplyV1,
}

struct ClientRecord {
    id: ClientId,
    view: EditorView,
}

/// One serialized owner for a shared workspace and bounded client views.
///
/// Semantic replacements are version-bound and never borrow another client's
/// cursor, selection, active document, request namespace, or witness namespace.
pub struct Service {
    workspace: Workspace,
    limits: ServiceLimits,
    clients: Vec<ClientRecord>,
    retired_clients: Vec<ClientId>,
    requests: Vec<RequestRecord>,
    witnesses: Vec<WitnessRecord>,
    next_client: u64,
    issued_clients: usize,
    next_witness: u64,
    usage: ServiceUsage,
}

impl Service {
    pub fn new(workspace: Workspace, limits: ServiceLimits) -> Result<Self, ServiceBuildError> {
        if limits.max_clients == 0
            || limits.max_issued_clients < limits.max_clients
            || limits.max_requests == 0
            || limits.max_witnesses == 0
            || limits.max_snapshot_bytes == 0
            || limits.max_insert_bytes == 0
        {
            return Err(ServiceBuildError::InvalidLimits);
        }
        let mut clients = Vec::new();
        clients
            .try_reserve_exact(limits.max_clients)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut retired_clients = Vec::new();
        retired_clients
            .try_reserve_exact(limits.max_issued_clients)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut requests = Vec::new();
        requests
            .try_reserve_exact(limits.max_requests)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut witnesses = Vec::new();
        witnesses
            .try_reserve_exact(limits.max_witnesses)
            .map_err(|_| ServiceBuildError::Capacity)?;
        Ok(Self {
            workspace,
            limits,
            clients,
            retired_clients,
            requests,
            witnesses,
            next_client: 1,
            issued_clients: 0,
            next_witness: 1,
            usage: ServiceUsage::default(),
        })
    }

    pub fn connect(&mut self) -> Result<ClientId, RefusalV1> {
        bump(
            &mut self.usage.connect_attempts,
            &mut self.usage.counter_saturated,
        );
        if self.clients.len() == self.limits.max_clients {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::ClientCapacity);
        }
        if self.issued_clients == self.limits.max_issued_clients {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::ClientLifetimeCapacity);
        }
        let Some(client) = ClientId::new(self.next_client) else {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::CounterExhausted);
        };
        let view = self.workspace.open_view().map_err(classify_workspace)?;
        self.next_client = self.next_client.checked_add(1).unwrap_or(0);
        self.issued_clients += 1;
        self.clients.push(ClientRecord { id: client, view });
        bump(&mut self.usage.connected, &mut self.usage.counter_saturated);
        Ok(client)
    }

    pub fn disconnect(&mut self, client: ClientId) -> Result<(), RefusalV1> {
        bump(
            &mut self.usage.disconnect_attempts,
            &mut self.usage.counter_saturated,
        );
        if self.retired_clients.contains(&client) {
            bump(
                &mut self.usage.disconnect_replayed,
                &mut self.usage.counter_saturated,
            );
            return Ok(());
        }
        let Some(index) = self.clients.iter().position(|record| record.id == client) else {
            bump(
                &mut self.usage.disconnect_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::UnknownClient);
        };
        self.clients.swap_remove(index);
        self.requests
            .retain(|record| record.request.client != client);
        self.witnesses.retain(|record| record.client != client);
        self.retired_clients.push(client);
        bump(
            &mut self.usage.disconnected,
            &mut self.usage.counter_saturated,
        );
        Ok(())
    }

    pub fn execute(&mut self, request: RequestV1) -> ReplyV1 {
        bump(&mut self.usage.requests, &mut self.usage.counter_saturated);
        if !self
            .clients
            .iter()
            .any(|client| client.id == request.client)
        {
            return self.observed(ReplyV1::Refused(RefusalV1::UnknownClient));
        }
        if let Some((same_request, retained_reply)) = self
            .requests
            .iter()
            .find(|retained| {
                retained.request.client == request.client
                    && retained.request.request == request.request
            })
            .map(|retained| (retained.request == request, retained.reply.clone()))
        {
            let reply = if same_request {
                bump(&mut self.usage.replayed, &mut self.usage.counter_saturated);
                retained_reply
            } else {
                ReplyV1::Refused(RefusalV1::RequestConflict)
            };
            return self.observed(reply);
        }
        if self.requests.len() == self.limits.max_requests {
            return self.observed(ReplyV1::Refused(RefusalV1::RequestCapacity));
        }
        let reply = self.execute_fresh(request.client, &request.command);
        self.requests.push(RequestRecord {
            request,
            reply: reply.clone(),
        });
        self.observed(reply)
    }

    pub fn usage(&self) -> ServiceUsage {
        ServiceUsage {
            active_clients: self.clients.len(),
            client_capacity: self.limits.max_clients,
            issued_clients: self.issued_clients,
            issued_client_capacity: self.limits.max_issued_clients,
            retired_clients: self.retired_clients.len(),
            retained_requests: self.requests.len(),
            request_capacity: self.limits.max_requests,
            retained_witnesses: self.witnesses.len(),
            witness_capacity: self.limits.max_witnesses,
            ..self.usage
        }
    }

    fn observed(&mut self, reply: ReplyV1) -> ReplyV1 {
        match &reply {
            ReplyV1::Accepted(_) => {
                bump(&mut self.usage.accepted, &mut self.usage.counter_saturated)
            }
            ReplyV1::Refused(refusal) => {
                bump(&mut self.usage.refused, &mut self.usage.counter_saturated);
                let category = match refusal {
                    RefusalV1::RequestConflict | RefusalV1::EditorConflict => {
                        &mut self.usage.conflicts
                    }
                    RefusalV1::Stale => &mut self.usage.stale,
                    RefusalV1::UnknownClient
                    | RefusalV1::UnknownWitness
                    | RefusalV1::InvalidRange => &mut self.usage.invalid,
                    RefusalV1::ClientCapacity
                    | RefusalV1::ClientLifetimeCapacity
                    | RefusalV1::RequestCapacity
                    | RefusalV1::WitnessCapacity
                    | RefusalV1::InputLimit
                    | RefusalV1::SnapshotLimit
                    | RefusalV1::EditorCapacity
                    | RefusalV1::CounterExhausted => &mut self.usage.limited,
                    RefusalV1::EditorClosed
                    | RefusalV1::EditorIncomplete
                    | RefusalV1::EditorUnavailable => &mut self.usage.unavailable,
                };
                bump(category, &mut self.usage.counter_saturated);
            }
        }
        reply
    }

    fn execute_fresh(&mut self, client: ClientId, command: &CommandV1) -> ReplyV1 {
        let result = match command {
            CommandV1::Snapshot => self.snapshot(client).map(OutcomeV1::Snapshot),
            CommandV1::Replace {
                expected,
                start,
                end,
                inserted,
            } => self
                .replace(client, *expected, *start, *end, inserted)
                .map(OutcomeV1::Replaced),
        };
        match result {
            Ok(outcome) => ReplyV1::Accepted(outcome),
            Err(refusal) => ReplyV1::Refused(refusal),
        }
    }

    fn snapshot(&mut self, client: ClientId) -> Result<SnapshotV1, RefusalV1> {
        self.preflight_witness()?;
        let view = self.view(client)?;
        let captured = self.workspace.snapshot(&view).map_err(classify_workspace)?;
        if captured.text.len() > self.limits.max_snapshot_bytes {
            return Err(RefusalV1::SnapshotLimit);
        }
        let text: Arc<str> = Arc::from(captured.text);
        Ok(self.install_witness(client, captured.version, text))
    }

    fn replace(
        &mut self,
        client: ClientId,
        expected: VersionWitness,
        start: u64,
        end: u64,
        inserted: &str,
    ) -> Result<SnapshotV1, RefusalV1> {
        if inserted.len() > self.limits.max_insert_bytes {
            return Err(RefusalV1::InputLimit);
        }
        let retained = self
            .witnesses
            .iter()
            .find(|entry| entry.client == client && entry.id == expected)
            .ok_or(RefusalV1::UnknownWitness)?;
        let expected_version = retained.version.clone();
        let original = Arc::clone(&retained.text);
        let view = self.view(client)?;
        if self.workspace.version(&view).map_err(classify_workspace)? != expected_version {
            return Err(RefusalV1::Stale);
        }
        let start = usize::try_from(start).map_err(|_| RefusalV1::InvalidRange)?;
        let end = usize::try_from(end).map_err(|_| RefusalV1::InvalidRange)?;
        if start > end
            || end > original.len()
            || !original.is_char_boundary(start)
            || !original.is_char_boundary(end)
        {
            return Err(RefusalV1::InvalidRange);
        }
        let new_len = original
            .len()
            .checked_sub(end - start)
            .and_then(|length| length.checked_add(inserted.len()))
            .ok_or(RefusalV1::InputLimit)?;
        if new_len > self.limits.max_snapshot_bytes {
            return Err(RefusalV1::SnapshotLimit);
        }
        self.preflight_witness()?;

        let expected_fragment = copy_string(&original[start..end])?;
        let inserted_owned = copy_string(inserted)?;
        let mut projected = String::new();
        projected
            .try_reserve_exact(new_len)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        projected.push_str(&original[..start]);
        projected.push_str(inserted);
        projected.push_str(&original[end..]);
        let projected: Arc<str> = Arc::from(projected);
        let mut replacements = Vec::new();
        replacements
            .try_reserve_exact(1)
            .map_err(|_| RefusalV1::EditorCapacity)?;
        replacements.push(Replacement {
            start,
            end,
            expected: expected_fragment,
            inserted: inserted_owned,
        });
        self.workspace
            .edit_snapshot(&view, &expected_version, replacements)
            .map_err(classify_workspace)?;
        let version = self.workspace.version(&view).map_err(classify_workspace)?;
        Ok(self.install_witness(client, version, projected))
    }

    fn preflight_witness(&self) -> Result<(), RefusalV1> {
        if self.witnesses.len() == self.limits.max_witnesses {
            return Err(RefusalV1::WitnessCapacity);
        }
        if self.next_witness == 0 {
            return Err(RefusalV1::CounterExhausted);
        }
        Ok(())
    }

    fn install_witness(
        &mut self,
        client: ClientId,
        version: DocumentVersion,
        text: Arc<str>,
    ) -> SnapshotV1 {
        let witness = VersionWitness::new(self.next_witness).expect("preflight rejects zero");
        self.next_witness = self.next_witness.checked_add(1).unwrap_or(0);
        let snapshot = SnapshotV1 {
            witness,
            revision: version.revision,
            text: Arc::clone(&text),
        };
        self.witnesses.push(WitnessRecord {
            client,
            id: witness,
            version,
            text,
        });
        snapshot
    }

    fn view(&self, client: ClientId) -> Result<EditorView, RefusalV1> {
        self.clients
            .iter()
            .find(|record| record.id == client)
            .map(|record| record.view.clone())
            .ok_or(RefusalV1::UnknownClient)
    }
}

fn copy_string(value: &str) -> Result<String, RefusalV1> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| RefusalV1::EditorCapacity)?;
    copy.push_str(value);
    Ok(copy)
}

fn classify_workspace(error: WorkspaceError) -> RefusalV1 {
    match error {
        WorkspaceError::Capacity => RefusalV1::EditorCapacity,
        WorkspaceError::Empty | WorkspaceError::InvalidView | WorkspaceError::Session(_) => {
            RefusalV1::EditorUnavailable
        }
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    if next == *value {
        *saturated = true;
    }
    *value = next;
}
