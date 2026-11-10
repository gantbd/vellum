//! Bounded in-process Vellum service interpreter.
#![forbid(unsafe_code)]

use std::sync::Arc;
use vellum_editor::{Editor, EditorError};
use vellum_engine::{coordinator::DocumentVersion, source::Replacement};
use vellum_protocol::{
    ClientId, CommandV1, OutcomeV1, RefusalV1, ReplyV1, RequestV1, SnapshotV1, VersionWitness,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceLimits {
    pub max_clients: usize,
    pub max_requests: usize,
    pub max_witnesses: usize,
    pub max_snapshot_bytes: usize,
    pub max_insert_bytes: usize,
}

impl vellum_protocol::ServicePort for Service {
    fn connect(&mut self) -> Result<ClientId, RefusalV1> {
        self.connect()
    }

    fn execute(&mut self, request: RequestV1) -> ReplyV1 {
        self.execute(request)
    }
}

impl ServiceLimits {
    pub const fn reference() -> Self {
        Self {
            max_clients: 1,
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
    pub retained_requests: usize,
    pub request_capacity: usize,
    pub retained_witnesses: usize,
    pub witness_capacity: usize,
}

struct WitnessRecord {
    id: VersionWitness,
    version: DocumentVersion,
    text: Arc<str>,
}

struct RequestRecord {
    request: RequestV1,
    reply: ReplyV1,
}

/// One serialized owner for the first native service profile.
///
/// The editor currently contains presentation state, so this profile admits
/// exactly one client. That is an explicit limit, not a multi-client claim.
/// Semantic replacements are version-bound and do not use the editor cursor.
pub struct Service {
    editor: Editor,
    limits: ServiceLimits,
    client: Option<ClientId>,
    requests: Vec<RequestRecord>,
    witnesses: Vec<WitnessRecord>,
    next_witness: u64,
    usage: ServiceUsage,
}

impl Service {
    pub fn new(editor: Editor, limits: ServiceLimits) -> Result<Self, ServiceBuildError> {
        if limits.max_clients != 1
            || limits.max_requests == 0
            || limits.max_witnesses == 0
            || limits.max_snapshot_bytes == 0
            || limits.max_insert_bytes == 0
        {
            return Err(ServiceBuildError::InvalidLimits);
        }
        let mut requests = Vec::new();
        requests
            .try_reserve_exact(limits.max_requests)
            .map_err(|_| ServiceBuildError::Capacity)?;
        let mut witnesses = Vec::new();
        witnesses
            .try_reserve_exact(limits.max_witnesses)
            .map_err(|_| ServiceBuildError::Capacity)?;
        Ok(Self {
            editor,
            limits,
            client: None,
            requests,
            witnesses,
            next_witness: 1,
            usage: ServiceUsage::default(),
        })
    }

    pub fn connect(&mut self) -> Result<ClientId, RefusalV1> {
        bump(
            &mut self.usage.connect_attempts,
            &mut self.usage.counter_saturated,
        );
        if self.client.is_some() {
            bump(
                &mut self.usage.connection_refused,
                &mut self.usage.counter_saturated,
            );
            return Err(RefusalV1::ClientCapacity);
        }
        let client = ClientId::new(1).expect("one is nonzero");
        self.client = Some(client);
        bump(&mut self.usage.connected, &mut self.usage.counter_saturated);
        Ok(client)
    }

    pub fn execute(&mut self, request: RequestV1) -> ReplyV1 {
        bump(&mut self.usage.requests, &mut self.usage.counter_saturated);
        if self.client != Some(request.client) {
            return self.observed(ReplyV1::Refused(RefusalV1::UnknownClient));
        }
        if let Some((same_request, retained_reply)) = self
            .requests
            .iter()
            .find(|retained| retained.request.request == request.request)
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
        let reply = self.execute_fresh(&request.command);
        self.requests.push(RequestRecord {
            request,
            reply: reply.clone(),
        });
        self.observed(reply)
    }

    pub fn usage(&self) -> ServiceUsage {
        ServiceUsage {
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

    fn execute_fresh(&mut self, command: &CommandV1) -> ReplyV1 {
        let result = match command {
            CommandV1::Snapshot => self.snapshot().map(OutcomeV1::Snapshot),
            CommandV1::Replace {
                expected,
                start,
                end,
                inserted,
            } => self
                .replace(*expected, *start, *end, inserted)
                .map(OutcomeV1::Replaced),
        };
        match result {
            Ok(outcome) => ReplyV1::Accepted(outcome),
            Err(refusal) => ReplyV1::Refused(refusal),
        }
    }

    fn snapshot(&mut self) -> Result<SnapshotV1, RefusalV1> {
        self.preflight_witness()?;
        let captured = self.editor.snapshot().map_err(classify_editor)?;
        if captured.text.len() > self.limits.max_snapshot_bytes {
            return Err(RefusalV1::SnapshotLimit);
        }
        let text: Arc<str> = Arc::from(captured.text);
        Ok(self.install_witness(captured.version, text))
    }

    fn replace(
        &mut self,
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
            .find(|entry| entry.id == expected)
            .ok_or(RefusalV1::UnknownWitness)?;
        let expected_version = retained.version.clone();
        let original = Arc::clone(&retained.text);
        if self.editor.version().map_err(classify_editor)? != expected_version {
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
        let caret = start
            .checked_add(inserted.len())
            .ok_or(RefusalV1::InputLimit)?;
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
        self.editor
            .edit_snapshot(&expected_version, replacements, caret)
            .map_err(classify_editor)?;
        let version = self.editor.version().map_err(classify_editor)?;
        Ok(self.install_witness(version, projected))
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

    fn install_witness(&mut self, version: DocumentVersion, text: Arc<str>) -> SnapshotV1 {
        let witness = VersionWitness::new(self.next_witness).expect("preflight rejects zero");
        self.next_witness = self.next_witness.checked_add(1).unwrap_or(0);
        let snapshot = SnapshotV1 {
            witness,
            revision: version.revision,
            text: Arc::clone(&text),
        };
        self.witnesses.push(WitnessRecord {
            id: witness,
            version,
            text,
        });
        snapshot
    }
}

fn copy_string(value: &str) -> Result<String, RefusalV1> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| RefusalV1::EditorCapacity)?;
    copy.push_str(value);
    Ok(copy)
}

fn classify_editor(error: EditorError) -> RefusalV1 {
    match error {
        EditorError::Closed => RefusalV1::EditorClosed,
        EditorError::Capacity => RefusalV1::EditorCapacity,
        EditorError::Unsaved
        | EditorError::IdentityConflict
        | EditorError::ClipboardPending
        | EditorError::CommentsIncomplete => RefusalV1::EditorConflict,
        EditorError::SaveIncomplete => RefusalV1::EditorIncomplete,
        _ => RefusalV1::EditorUnavailable,
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    if next == *value {
        *saturated = true;
    }
    *value = next;
}
