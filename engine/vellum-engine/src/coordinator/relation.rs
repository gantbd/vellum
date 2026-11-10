//! Headless container lifecycle meaning, shared staging and typed historical receipts.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationCommandV1 {
    CreateView,
    RetireView {
        view: LogicalViewId,
        expected: LogicalViewStamp,
    },
    CreateWorkspace,
    RetireWorkspace {
        expected: WorkspaceContext,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationReceiptV1 {
    ViewCreated(LogicalViewId),
    ViewRetired(LogicalViewId),
    WorkspaceCreated(WorkspaceId),
    WorkspaceRetired(WorkspaceId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationErrorV1 {
    View(crate::selection::SelectionError),
    Workspace(CoordinatorError),
}
impl RelationErrorV1 {
    pub(super) fn faulted(self) -> bool {
        matches!(
            self,
            Self::View(crate::selection::SelectionError::Faulted)
                | Self::Workspace(
                    CoordinatorError::Faulted
                        | CoordinatorError::History(HistoryError::Faulted)
                        | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
                )
        )
    }
    pub(super) fn matches_kind(self, command: &RelationCommandV1) -> bool {
        matches!(
            (self, command),
            (
                Self::View(_),
                RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. }
            ) | (
                Self::Workspace(_),
                RelationCommandV1::CreateWorkspace | RelationCommandV1::RetireWorkspace { .. }
            )
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RelationProfile {
    View(LogicalViewLimits),
    Workspace(WorkspaceLimits),
}
#[derive(Debug)]
pub(super) struct PreparedRelation {
    command: RelationCommandV1,
    profile: RelationProfile,
}
impl PreparedRelation {
    #[cfg(test)]
    pub(super) fn corrupt_for_test(&mut self, profile: bool) {
        if profile {
            self.profile = RelationProfile::Workspace(WorkspaceLimits {
                max_issued: 1,
                max_live: 1,
                max_memberships: 1,
                max_work: 1,
            });
        } else {
            self.command = RelationCommandV1::CreateWorkspace;
        }
    }
    pub(super) fn matches(
        &self,
        command: &RelationCommandV1,
        profile: Option<RelationProfile>,
    ) -> bool {
        self.command == *command && Some(self.profile) == profile
    }
}
pub(super) enum RelationStage {
    View(views::ViewRelationStage),
    Workspace(workspace::WorkspaceRelationStage),
}
impl RelationStage {
    pub(super) fn receipt(&self) -> RelationReceiptV1 {
        match self {
            Self::View(stage) => stage.receipt(),
            Self::Workspace(stage) => stage.receipt(),
        }
    }
    pub(super) fn install(
        self,
        owner: &mut SourceCoordinator,
        r: &mut Recorder,
    ) -> Result<(), RelationErrorV1> {
        match self {
            Self::View(stage) => stage.install(owner, r).map_err(RelationErrorV1::View),
            Self::Workspace(stage) => stage.install(owner, r).map_err(RelationErrorV1::Workspace),
        }
    }
}
impl SourceCoordinator {
    pub(super) fn relation_profile(&self, command: &RelationCommandV1) -> Option<RelationProfile> {
        match command {
            RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. } => {
                self.view_relation_profile().map(RelationProfile::View)
            }
            RelationCommandV1::CreateWorkspace | RelationCommandV1::RetireWorkspace { .. } => self
                .workspace_relation_profile()
                .map(RelationProfile::Workspace),
        }
    }
    pub(super) fn prepare_relation(
        &self,
        command: &RelationCommandV1,
        profile: Option<RelationProfile>,
        r: &mut Recorder,
    ) -> Result<PreparedRelation, RelationErrorV1> {
        let result = match command {
            RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. } => self
                .validate_view_relation(command, r)
                .map_err(RelationErrorV1::View),
            RelationCommandV1::CreateWorkspace | RelationCommandV1::RetireWorkspace { .. } => self
                .validate_workspace_relation(command, r)
                .map_err(RelationErrorV1::Workspace),
        }
        .and_then(|()| {
            if let Some(profile) =
                profile.filter(|profile| self.relation_profile(command) == Some(*profile))
            {
                Ok(PreparedRelation {
                    command: command.clone(),
                    profile,
                })
            } else {
                Err(match command {
                    RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. } => {
                        RelationErrorV1::View(crate::selection::SelectionError::Faulted)
                    }
                    RelationCommandV1::CreateWorkspace
                    | RelationCommandV1::RetireWorkspace { .. } => {
                        RelationErrorV1::Workspace(CoordinatorError::Faulted)
                    }
                })
            }
        });
        relation_observe(r, Boundary::RelationPreparation, &result);
        result
    }
    pub(super) fn stage_relation(
        &mut self,
        prepared: PreparedRelation,
        r: &mut Recorder,
    ) -> Result<RelationStage, RelationErrorV1> {
        let result = (|| {
            if self.relation_profile(&prepared.command) != Some(prepared.profile) {
                return Err(match prepared.command {
                    RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. } => {
                        RelationErrorV1::View(crate::selection::SelectionError::Stale)
                    }
                    RelationCommandV1::CreateWorkspace
                    | RelationCommandV1::RetireWorkspace { .. } => {
                        RelationErrorV1::Workspace(CoordinatorError::Stale)
                    }
                });
            }
            match prepared.command {
                command
                @ (RelationCommandV1::CreateView | RelationCommandV1::RetireView { .. }) => self
                    .stage_view_relation(&command, r)
                    .map(RelationStage::View)
                    .map_err(RelationErrorV1::View),
                command @ (RelationCommandV1::CreateWorkspace
                | RelationCommandV1::RetireWorkspace { .. }) => self
                    .stage_workspace_relation(&command, r)
                    .map(RelationStage::Workspace)
                    .map_err(RelationErrorV1::Workspace),
            }
        })();
        relation_observe(r, Boundary::RelationStaging, &result);
        result
    }
}
pub(super) fn relation_observe<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, RelationErrorV1>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(error) if error.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
