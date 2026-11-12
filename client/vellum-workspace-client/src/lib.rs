//! Presentation-neutral projection of one live Vellum workspace.
//!
//! This crate owns client-local view state only. It does not own source truth,
//! filesystem authority, cursor/selection semantics, transport retry policy,
//! or a UI event loop. Every service mutation remains in `vellum-client`;
//! uncertain requests remain fenced there under their exact identity.
#![forbid(unsafe_code)]

use vellum_client::Client;
use vellum_protocol::{
    OutcomeV1, RefusalV1, ReplyV1, ServiceAccessErrorV1, ServiceControlErrorV1, ServicePort,
    SnapshotV1, WorkspaceChangeKindV1, WorkspaceChangesV1, WorkspaceDocumentIdV1, WorkspaceIdV1,
    WorkspaceInventoryV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionFreshnessV1 {
    Absent,
    Current,
    Stale,
}

/// Content-safe client-local projection facts. Source bytes remain available
/// only through `snapshot()` and are redacted from `Debug`.
#[derive(Clone, Eq, PartialEq)]
pub struct WorkspaceProjectionV1 {
    view_open: bool,
    inventory: Option<WorkspaceInventoryV1>,
    inventory_freshness: ProjectionFreshnessV1,
    selected: Option<WorkspaceDocumentIdV1>,
    snapshot: Option<SnapshotV1>,
    snapshot_freshness: ProjectionFreshnessV1,
    change_cursor: Option<vellum_protocol::WorkspaceChangeCursorV1>,
}

impl Default for WorkspaceProjectionV1 {
    fn default() -> Self {
        Self {
            view_open: false,
            inventory: None,
            inventory_freshness: ProjectionFreshnessV1::Absent,
            selected: None,
            snapshot: None,
            snapshot_freshness: ProjectionFreshnessV1::Absent,
            change_cursor: None,
        }
    }
}

impl std::fmt::Debug for WorkspaceProjectionV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceProjectionV1")
            .field("view_open", &self.view_open)
            .field(
                "inventory",
                &self
                    .inventory
                    .as_ref()
                    .map(|value| (value.documents.len(), value.roots.len())),
            )
            .field("inventory_freshness", &self.inventory_freshness)
            .field("selected", &self.selected)
            .field("snapshot", &self.snapshot.as_ref().map(|_| "<redacted>"))
            .field("snapshot_freshness", &self.snapshot_freshness)
            .field("change_cursor", &self.change_cursor)
            .finish()
    }
}

impl WorkspaceProjectionV1 {
    pub const fn view_open(&self) -> bool {
        self.view_open
    }

    pub const fn inventory(&self) -> Option<&WorkspaceInventoryV1> {
        self.inventory.as_ref()
    }

    pub const fn inventory_freshness(&self) -> ProjectionFreshnessV1 {
        self.inventory_freshness
    }

    pub const fn selected(&self) -> Option<WorkspaceDocumentIdV1> {
        self.selected
    }

    pub const fn snapshot(&self) -> Option<&SnapshotV1> {
        self.snapshot.as_ref()
    }

    pub const fn snapshot_freshness(&self) -> ProjectionFreshnessV1 {
        self.snapshot_freshness
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceProjectionRefusalV1 {
    InventoryRequired,
    UnknownDocument,
    SelectionRequired,
    ForeignWorkspace,
    UnexpectedReply,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceClientErrorV1 {
    Access(ServiceAccessErrorV1),
    Refused(RefusalV1),
    Projection(WorkspaceProjectionRefusalV1),
}

impl From<ServiceAccessErrorV1> for WorkspaceClientErrorV1 {
    fn from(value: ServiceAccessErrorV1) -> Self {
        Self::Access(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceChangePollV1 {
    Current {
        changes: usize,
        observed_head: u64,
    },
    Gap {
        requested_after: u64,
        observed_head: u64,
    },
}

/// One reusable human/agent workspace controller. It may be embedded behind a
/// Ratatui, Iced, MCP, or orchestration projection without importing any of
/// those application shells.
#[derive(Debug)]
pub struct WorkspaceClientV1<T> {
    client: Client<T>,
    projection: WorkspaceProjectionV1,
}

/// Failed disconnect retains the complete controller without inflating the
/// ordinary success result or discarding an uncertain client boundary.
pub struct WorkspaceDisconnectFailureV1<T> {
    controller: Box<WorkspaceClientV1<T>>,
    error: ServiceControlErrorV1,
}

impl<T> WorkspaceDisconnectFailureV1<T> {
    pub fn into_parts(self) -> (WorkspaceClientV1<T>, ServiceControlErrorV1) {
        (*self.controller, self.error)
    }

    pub const fn error(&self) -> ServiceControlErrorV1 {
        self.error
    }
}

impl<T> std::fmt::Debug for WorkspaceDisconnectFailureV1<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkspaceDisconnectFailureV1")
            .field("controller", &"<retained>")
            .field("error", &self.error)
            .finish()
    }
}

impl<T: ServicePort> WorkspaceClientV1<T> {
    pub fn connect(transport: T) -> Result<Self, ServiceControlErrorV1> {
        Client::connect(transport).map(Self::from_client)
    }

    pub fn from_client(client: Client<T>) -> Self {
        Self {
            client,
            projection: WorkspaceProjectionV1::default(),
        }
    }

    pub const fn projection(&self) -> &WorkspaceProjectionV1 {
        &self.projection
    }

    pub fn open_view(&mut self) -> Result<(), WorkspaceClientErrorV1> {
        match self.client.open_view()? {
            ReplyV1::Accepted(OutcomeV1::ViewOpened(_)) => {
                self.projection.view_open = true;
                Ok(())
            }
            ReplyV1::Refused(reason) => Err(WorkspaceClientErrorV1::Refused(reason)),
            ReplyV1::Accepted(_) => Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::UnexpectedReply,
            )),
        }
    }

    pub fn refresh_inventory(&mut self) -> Result<(), WorkspaceClientErrorV1> {
        match self.client.workspace_inventory()? {
            ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) => {
                self.install_inventory(inventory);
                Ok(())
            }
            ReplyV1::Refused(reason) => Err(WorkspaceClientErrorV1::Refused(reason)),
            ReplyV1::Accepted(_) => Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::UnexpectedReply,
            )),
        }
    }

    pub fn select_document(
        &mut self,
        document: WorkspaceDocumentIdV1,
    ) -> Result<(), WorkspaceClientErrorV1> {
        let Some(inventory) = self.projection.inventory.as_ref() else {
            return Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::InventoryRequired,
            ));
        };
        if !inventory.documents.contains(&document) {
            return Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::UnknownDocument,
            ));
        }
        match self.client.select_document(document)? {
            ReplyV1::Accepted(OutcomeV1::DocumentSelected(selected)) if selected == document => {
                self.projection.selected = Some(document);
                self.projection.snapshot = None;
                self.projection.snapshot_freshness = ProjectionFreshnessV1::Absent;
                Ok(())
            }
            ReplyV1::Refused(reason) => Err(WorkspaceClientErrorV1::Refused(reason)),
            ReplyV1::Accepted(_) => Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::UnexpectedReply,
            )),
        }
    }

    pub fn refresh_snapshot(&mut self) -> Result<(), WorkspaceClientErrorV1> {
        if self.projection.selected.is_none() {
            return Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::SelectionRequired,
            ));
        }
        match self.client.snapshot()? {
            ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) => {
                self.projection.snapshot = Some(snapshot);
                self.projection.snapshot_freshness = ProjectionFreshnessV1::Current;
                Ok(())
            }
            ReplyV1::Refused(reason) => Err(WorkspaceClientErrorV1::Refused(reason)),
            ReplyV1::Accepted(_) => Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::UnexpectedReply,
            )),
        }
    }

    pub fn poll_changes(&mut self) -> Result<WorkspaceChangePollV1, WorkspaceClientErrorV1> {
        let after = self.projection.change_cursor;
        match self.client.workspace_changes(after, 1)? {
            ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(page))) => {
                self.require_workspace(page.workspace)?;
                for change in &page.changes {
                    self.apply_change(change.kind);
                }
                self.projection.change_cursor = Some(page.next);
                Ok(WorkspaceChangePollV1::Current {
                    changes: page.changes.len(),
                    observed_head: page.observed_head,
                })
            }
            ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Gap(gap))) => {
                self.require_workspace(gap.workspace)?;
                self.projection.change_cursor = None;
                self.projection.inventory_freshness = ProjectionFreshnessV1::Stale;
                if self.projection.snapshot.is_some() {
                    self.projection.snapshot_freshness = ProjectionFreshnessV1::Stale;
                }
                Ok(WorkspaceChangePollV1::Gap {
                    requested_after: gap.requested_after,
                    observed_head: gap.observed_head,
                })
            }
            ReplyV1::Refused(reason) => Err(WorkspaceClientErrorV1::Refused(reason)),
            ReplyV1::Accepted(_) => Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::UnexpectedReply,
            )),
        }
    }

    pub fn disconnect(self) -> Result<T, WorkspaceDisconnectFailureV1<T>> {
        let Self { client, projection } = self;
        client
            .disconnect()
            .map_err(|(client, error)| WorkspaceDisconnectFailureV1 {
                controller: Box::new(Self { client, projection }),
                error,
            })
    }

    fn require_workspace(&self, workspace: WorkspaceIdV1) -> Result<(), WorkspaceClientErrorV1> {
        if self
            .projection
            .inventory
            .as_ref()
            .is_some_and(|inventory| inventory.workspace != workspace)
        {
            Err(WorkspaceClientErrorV1::Projection(
                WorkspaceProjectionRefusalV1::ForeignWorkspace,
            ))
        } else {
            Ok(())
        }
    }

    fn install_inventory(&mut self, inventory: WorkspaceInventoryV1) {
        let workspace_changed = self
            .projection
            .inventory
            .as_ref()
            .is_some_and(|current| current.workspace != inventory.workspace);
        if workspace_changed {
            self.projection.selected = None;
            self.projection.snapshot = None;
            self.projection.snapshot_freshness = ProjectionFreshnessV1::Absent;
            self.projection.change_cursor = None;
        } else if self
            .projection
            .selected
            .is_some_and(|selected| !inventory.documents.contains(&selected))
        {
            self.projection.selected = None;
            self.projection.snapshot = None;
            self.projection.snapshot_freshness = ProjectionFreshnessV1::Absent;
        }
        self.projection.inventory = Some(inventory);
        self.projection.inventory_freshness = ProjectionFreshnessV1::Current;
    }

    fn apply_change(&mut self, change: WorkspaceChangeKindV1) {
        match change {
            WorkspaceChangeKindV1::DocumentChanged { document, .. } => {
                if self.projection.selected == Some(document) && self.projection.snapshot.is_some()
                {
                    self.projection.snapshot_freshness = ProjectionFreshnessV1::Stale;
                }
            }
            WorkspaceChangeKindV1::DocumentRemoved(document) => {
                self.projection.inventory_freshness = ProjectionFreshnessV1::Stale;
                if self.projection.selected == Some(document) {
                    self.projection.selected = None;
                    self.projection.snapshot = None;
                    self.projection.snapshot_freshness = ProjectionFreshnessV1::Absent;
                }
            }
            WorkspaceChangeKindV1::DocumentAdded(_)
            | WorkspaceChangeKindV1::RootAdded(_)
            | WorkspaceChangeKindV1::RootRemoved(_) => {
                self.projection.inventory_freshness = ProjectionFreshnessV1::Stale;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::VecDeque, sync::Arc};
    use vellum_protocol::{
        ClientId, RequestV1, VersionWitness, WorkspaceChangeCursorV1, WorkspaceChangePageV1,
        WorkspaceChangeV1,
    };

    #[derive(Debug)]
    struct ScriptedPort {
        replies: VecDeque<Result<ReplyV1, ServiceAccessErrorV1>>,
        commands: Vec<vellum_protocol::CommandV1>,
    }

    impl ServicePort for ScriptedPort {
        fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
            Ok(ClientId::new(1).unwrap())
        }

        fn execute(&mut self, request: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
            self.commands.push(request.command);
            self.replies.pop_front().unwrap()
        }

        fn disconnect(&mut self, _: ClientId) -> Result<(), ServiceControlErrorV1> {
            Ok(())
        }
    }

    fn workspace() -> WorkspaceIdV1 {
        WorkspaceIdV1::new(7).unwrap()
    }

    fn document() -> WorkspaceDocumentIdV1 {
        WorkspaceDocumentIdV1::new(workspace(), 9).unwrap()
    }

    fn inventory(documents: Vec<WorkspaceDocumentIdV1>) -> ReplyV1 {
        ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(WorkspaceInventoryV1 {
            workspace: workspace(),
            documents,
            roots: Vec::new(),
        }))
    }

    fn controller(
        replies: impl IntoIterator<Item = Result<ReplyV1, ServiceAccessErrorV1>>,
    ) -> WorkspaceClientV1<ScriptedPort> {
        WorkspaceClientV1::connect(ScriptedPort {
            replies: replies.into_iter().collect(),
            commands: Vec::new(),
        })
        .unwrap()
    }

    #[test]
    fn empty_workspace_is_a_current_projection_not_a_synthetic_document() {
        let mut controller = controller([Ok(inventory(Vec::new()))]);
        controller.refresh_inventory().unwrap();
        assert!(!controller.projection().view_open());
        assert_eq!(
            controller.projection().inventory_freshness(),
            ProjectionFreshnessV1::Current
        );
        assert!(
            controller
                .projection()
                .inventory()
                .unwrap()
                .documents
                .is_empty()
        );
        assert_eq!(controller.projection().selected(), None);
        assert_eq!(controller.projection().snapshot(), None);
    }

    #[test]
    fn selection_and_snapshot_are_explicit_and_source_is_redacted_from_debug() {
        let snapshot = SnapshotV1 {
            witness: VersionWitness::new(3).unwrap(),
            revision: 4,
            text: Arc::from("private source"),
        };
        let mut controller = controller([
            Ok(inventory(vec![document()])),
            Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
            Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot.clone()))),
        ]);
        controller.refresh_inventory().unwrap();
        controller.select_document(document()).unwrap();
        controller.refresh_snapshot().unwrap();
        assert_eq!(controller.projection().snapshot(), Some(&snapshot));
        let debug = format!("{:?}", controller.projection());
        assert!(!debug.contains("private source"));
    }

    #[test]
    fn selected_document_change_marks_only_the_snapshot_stale() {
        let snapshot = SnapshotV1 {
            witness: VersionWitness::new(3).unwrap(),
            revision: 4,
            text: Arc::from("source"),
        };
        let cursor = WorkspaceChangeCursorV1::new(workspace(), 2, 1).unwrap();
        let change = WorkspaceChangeV1::new(
            1,
            WorkspaceChangeKindV1::DocumentChanged {
                document: document(),
                source_revision: 5,
            },
        )
        .unwrap();
        let mut controller = controller([
            Ok(inventory(vec![document()])),
            Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
            Ok(ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot))),
            Ok(ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(
                WorkspaceChangesV1::Page(WorkspaceChangePageV1 {
                    workspace: workspace(),
                    observed_head: 1,
                    changes: vec![change],
                    next: cursor,
                }),
            ))),
        ]);
        controller.refresh_inventory().unwrap();
        controller.select_document(document()).unwrap();
        controller.refresh_snapshot().unwrap();
        assert_eq!(
            controller.poll_changes().unwrap(),
            WorkspaceChangePollV1::Current {
                changes: 1,
                observed_head: 1
            }
        );
        assert_eq!(
            controller.projection().inventory_freshness(),
            ProjectionFreshnessV1::Current
        );
        assert_eq!(
            controller.projection().snapshot_freshness(),
            ProjectionFreshnessV1::Stale
        );
    }

    #[test]
    fn transport_uncertainty_preserves_projection_and_fences_other_commands() {
        let mut controller = controller([
            Ok(inventory(vec![document()])),
            Err(ServiceAccessErrorV1::OutcomeUnknown),
            Ok(ReplyV1::Accepted(OutcomeV1::DocumentSelected(document()))),
        ]);
        controller.refresh_inventory().unwrap();
        assert_eq!(
            controller.select_document(document()),
            Err(WorkspaceClientErrorV1::Access(
                ServiceAccessErrorV1::OutcomeUnknown
            ))
        );
        assert_eq!(controller.projection().selected(), None);
        assert_eq!(
            controller.refresh_inventory(),
            Err(WorkspaceClientErrorV1::Access(
                ServiceAccessErrorV1::OutcomeUnknown
            ))
        );
        controller.select_document(document()).unwrap();
        assert_eq!(controller.projection().selected(), Some(document()));
    }
}
