//! Container jobs use the existing one-shot ledger; no lifecycle callback executor.
use super::*;
mod arguments;
pub use arguments::*;
pub(in crate::coordinator::invocation) mod results;
pub use results::{
    RelationResultV1, TrustedRelationResultAdmissionV1, TrustedRelationResultReleaseV1,
};
impl SourceCoordinator {
    pub fn submit_relation(
        &mut self,
        id: &InvocationId,
        command: &RelationCommandV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            let bytes = results::base_bytes();
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            if let Some(original) = &slot.intent {
                if !matches!(original,BoundIntent::Relation {command:old,..} if old==command) {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let profile = self.relation_profile(command);
            let prepared = self.prepare_relation(command, profile, r);
            self.install_invocation_admission(
                id,
                BoundIntent::Relation {
                    command: command.clone(),
                    profile,
                    result: None,
                },
                Phase::RelationPending(prepared),
            );
            Ok(InvocationState::Pending)
        })();
        capture(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }
    pub(super) fn run_relation(
        &mut self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated pending slot");
        let Phase::RelationPending(prepared) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("validated relation job")
        };
        let intact = match (&slot.intent, &prepared) {
            (
                Some(BoundIntent::Relation {
                    command, profile, ..
                }),
                Ok(prepared),
            ) => prepared.matches(command, *profile),
            (Some(BoundIntent::Relation { command, .. }), Err(error)) => {
                error.matches_kind(command)
            }
            _ => false,
        };
        self.faulted = true;
        if !intact {
            r.record(Observation {
                boundary: Boundary::RelationPublication,
                disposition: Disposition::Unknown,
                service_time: None,
            });
            return Err(InvocationError::Faulted);
        }
        let decision = prepared
            .and_then(|prepared| self.stage_relation(prepared, r))
            .and_then(|stage| {
                let receipt = stage.receipt();
                let result = self.stage_relation_result_v1(id, &receipt)?;
                let outcome = InvocationOutcome::RelationCommitted(receipt);
                stage.install(self, r)?;
                #[cfg(test)]
                if self.relation_publication_fault {
                    panic!("injected relation publication interruption");
                }
                Ok((outcome, result))
            });
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            super::super::relation::relation_observe(r, Boundary::RelationPublication, &decision);
            return Err(InvocationError::Faulted);
        }
        let published = decision.is_ok();
        super::super::relation::relation_observe(r, Boundary::RelationPublication, &decision);
        let (outcome, result) =
            decision.unwrap_or_else(|e| (InvocationOutcome::RelationRejected(e), None));
        if let Some(BoundIntent::Relation {
            result: Some(capture),
            ..
        }) = self.invocations.slots[id.slot]
            .as_mut()
            .and_then(|slot| slot.intent.as_mut())
        {
            capture.receipt = result;
        } else if result.is_some() {
            return Err(InvocationError::Faulted);
        }
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        if published {
            r.record(Observation {
                boundary: Boundary::Publication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        Ok(InvocationState::Finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::SelectionBudget;
    pub(super) fn setup(workspace: bool) -> (SourceCoordinator, RelationCommandV1, Recorder) {
        let mut o = SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 1024,
                source: Limits {
                    max_source_bytes: 1024,
                    max_edits: 16,
                    max_edit_payload_bytes: 4096,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 8,
                max_retained: 8,
                max_intent_bytes: 16384,
            },
        );
        let mut r = Recorder::disabled();
        o.configure_logical_views(
            LogicalViewLimits {
                max_live_views: 8,
                max_issued_views: 8,
                max_total_selections: 16,
                normalization: SelectionBudget {
                    max_input_selections: 8,
                    max_output_selections: 8,
                    max_work: 1024,
                },
                max_mapping_steps: 1024,
            },
            &mut r,
        )
        .unwrap();
        o.configure_workspaces(
            WorkspaceLimits {
                max_issued: 8,
                max_live: 8,
                max_memberships: 8,
                max_work: 1024,
            },
            &mut r,
        )
        .unwrap();
        (
            o,
            if workspace {
                RelationCommandV1::CreateWorkspace
            } else {
                RelationCommandV1::CreateView
            },
            r,
        )
    }
    #[test]
    fn descriptor_threshold_is_checked_before_binding_any_relation_preparation() {
        for workspace in [false, true] {
            let (mut o, command, mut r) = setup(workspace);
            let id = o.issue_invocation(&mut r).unwrap();
            let bytes = results::base_bytes();
            o.invocations.limits.max_intent_bytes = bytes - 1;
            assert_eq!(
                o.submit_relation(&id, &command, &mut r),
                Err(InvocationError::Limit)
            );
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
            o.invocations.limits.max_intent_bytes = bytes;
            o.submit_relation(&id, &command, &mut r).unwrap();
            o.run(&id, &mut r).unwrap();
            assert!(matches!(
                o.invocation_outcome(&id, &mut r),
                Ok(InvocationOutcome::RelationCommitted(_))
            ));
        }
    }
    #[test]
    fn altered_capture_wrong_error_family_or_job_kind_fences_unknown_without_a_prefix() {
        for change in 0..4 {
            let (mut o, command, mut r) = setup(false);
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_relation(&id, &command, &mut r).unwrap();
            let slot = o.invocations.slots[id.slot].as_mut().unwrap();
            match change {
                0 | 1 => {
                    let Phase::RelationPending(Ok(prepared)) = &mut slot.phase else {
                        panic!("not prepared")
                    };
                    prepared.corrupt_for_test(change == 1);
                }
                2 => {
                    slot.phase = Phase::RelationPending(Err(RelationErrorV1::Workspace(
                        CoordinatorError::Unavailable,
                    )))
                }
                _ => {
                    slot.intent = Some(BoundIntent::Source(SourceIntent {
                        writes: vec![],
                        reads: vec![],
                    }))
                }
            }
            let g = o.generation();
            assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
            assert!(o.is_faulted());
            assert_eq!(o.generation(), g);
            assert_eq!(o.logical_view_usage().issued_lifetime, 0);
            assert_eq!(o.workspace_usage().issued_lifetime, 0);
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Err(InvocationError::Faulted)
            );
        }
    }
    #[test]
    fn service_does_not_adopt_changed_native_profile_even_if_the_action_still_fits() {
        for workspace in [false, true] {
            let (mut o, command, mut r) = setup(workspace);
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_relation(&id, &command, &mut r).unwrap();
            if workspace {
                o.change_workspace_relation_profile_for_test();
            } else {
                o.change_view_relation_profile_for_test();
            }
            let g = o.generation();
            o.run(&id, &mut r).unwrap();
            assert!(!o.is_faulted());
            assert_eq!(o.generation(), g);
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::RelationRejected(if workspace {
                    RelationErrorV1::Workspace(CoordinatorError::Stale)
                } else {
                    RelationErrorV1::View(crate::selection::SelectionError::Stale)
                }))
            );
        }
    }
    #[test]
    fn interruption_after_root_install_never_publishes_a_known_rejected_result_or_identity_receipt()
    {
        for workspace in [false, true] {
            let (mut o, command, mut r) = setup(workspace);
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_relation(&id, &command, &mut r).unwrap();
            let g = o.generation();
            o.relation_publication_fault = true;
            let panic =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r)));
            assert!(panic.is_err());
            assert!(o.is_faulted());
            assert_eq!(o.generation(), g + 1);
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(o.invocation_usage().faulted_executions, 1);
            assert_eq!(
                if workspace {
                    o.workspace_usage().issued_lifetime
                } else {
                    o.logical_view_usage().issued_lifetime
                },
                1
            );
        }
    }
}
