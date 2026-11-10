//! Original history intent must survive preparation and service unchanged.
use super::*;
pub(in crate::coordinator) mod results;
pub use results::{
    ReversalDecisionV1, ReversalResultV1, TrustedReversalResultAdmissionV1,
    TrustedReversalResultGrantV1, TrustedReversalResultReleaseV1,
};

/// An explicit group or an original expected frontier. This is a native owner
/// API, not a recipient authority grant or a wire-level scope reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryReversalTargetV1 {
    Group(HistoryGroupId),
    SourceFrontier {
        document: DocumentId,
        expected_group: HistoryGroupId,
    },
    AnnotationFrontier {
        annotation: AnnotationId,
        expected_group: HistoryGroupId,
    },
}
impl HistoryReversalTargetV1 {
    pub(super) fn group(&self) -> &HistoryGroupId {
        match self {
            Self::Group(group) => group,
            Self::SourceFrontier { expected_group, .. }
            | Self::AnnotationFrontier { expected_group, .. } => expected_group,
        }
    }
}

#[derive(Debug)]
pub(super) struct ReversalReadIntent {
    pub(super) target: HistoryReversalTargetV1,
    pub(super) direction: HistoryDirection,
    pub(super) reads: read_dependency::ReadCapture,
    pub(super) result: Option<Box<results::State>>,
}

fn read_error(error: ReadDependencyError) -> InvocationError {
    match error {
        ReadDependencyError::Capacity => InvocationError::Capacity,
        ReadDependencyError::Limit => InvocationError::Limit,
        ReadDependencyError::InvalidVersion | ReadDependencyError::DuplicateSubject => {
            InvocationError::MalformedIntent
        }
    }
}

impl SourceCoordinator {
    pub(super) fn reversal_intent_matches(
        plan: &PreparedTransaction,
        group: &HistoryGroupId,
        direction: HistoryDirection,
    ) -> bool {
        !plan.scoped
            && plan.scopes.is_none()
            && plan.derived_kind.is_none()
            && plan.derived_edit.is_empty()
            && plan.read_dependencies.is_empty()
            && matches!(&plan.history, history::PreparedHistory::Reversal { group: actual, direction: actual_direction, .. }
                if actual == group && *actual_direction == direction)
    }

    /// Retains the complete declared read conjunction in the same invocation
    /// ledger as the original group and direction. Final validation and atomic
    /// publication use the existing source/annotation history publisher.
    pub fn submit_reversal_with_reads_v3(
        &mut self,
        id: &InvocationId,
        group: &HistoryGroupId,
        direction: HistoryDirection,
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_reversal_target_with_reads_v3(
            id,
            &HistoryReversalTargetV1::Group(group.clone()),
            direction,
            request,
            r,
        )
    }

    /// A frontier is resolved only on first admission and must match the
    /// supplied expected group. Replay never re-reads or widens that meaning.
    pub fn submit_reversal_target_with_reads_v3(
        &mut self,
        id: &InvocationId,
        target: &HistoryReversalTargetV1,
        direction: HistoryDirection,
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            let input = read_dependency::ReadInput::V3(request);
            let bytes = input
                .retained_bytes(&[])
                .and_then(|n| {
                    n.checked_add(
                        std::mem::size_of::<ReversalReadIntent>()
                            - std::mem::size_of::<read_dependency::ReadCapture>(),
                    )
                })
                .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            let reads = input.copy(&[]).map_err(read_error)?;
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::DependentReversal(old) if old.target == *target && old.direction == direction && old.reads == reads)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let prepared = self
                .check_owned_read_capture(&reads, r)
                .and_then(|()| {
                    self.check_reversal_target(target, direction, r)
                        .map_err(CoordinatorError::History)
                })
                .and_then(|()| {
                    self.prepare_reversal(target.group(), direction, r)
                        .map_err(CoordinatorError::History)
                })
                .and_then(|mut plan| {
                    plan.reads_required = true;
                    plan.declared_reads = Some(reads.copy_owned()?);
                    Ok(plan)
                });
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slot is issued")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::DependentReversal(ReversalReadIntent {
                    target: target.clone(),
                    direction,
                    reads,
                    result: None,
                }),
                Phase::Pending {
                    prepared: prepared.map_err(NativePreparationError::Coordinator),
                    reserved,
                },
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

    fn check_reversal_target(
        &self,
        target: &HistoryReversalTargetV1,
        direction: HistoryDirection,
        r: &mut Recorder,
    ) -> Result<(), HistoryError> {
        let status = match target {
            HistoryReversalTargetV1::Group(_) => return Ok(()),
            HistoryReversalTargetV1::SourceFrontier { document, .. } => {
                self.history_status(document, direction, r)?
            }
            HistoryReversalTargetV1::AnnotationFrontier { annotation, .. } => {
                self.annotation_history_status(annotation, direction, r)?
            }
        };
        match status {
            HistoryAvailability::Eligible(group) if &group == target.group() => Ok(()),
            HistoryAvailability::Blocked(group) if &group == target.group() => {
                Err(HistoryError::Blocked)
            }
            HistoryAvailability::Eligible(_)
            | HistoryAvailability::Blocked(_)
            | HistoryAvailability::Empty => Err(HistoryError::Stale),
            HistoryAvailability::Unavailable => Err(HistoryError::Unavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn fixture() -> (SourceCoordinator, DocumentId, HistoryGroupId, Recorder) {
        let mut o = SourceCoordinator::with_invocation_and_history_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 4096,
                source: Limits {
                    max_source_bytes: 1024,
                    max_edits: 16,
                    max_edit_payload_bytes: 2048,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 16,
                max_retained: 16,
                max_intent_bytes: 4096,
            },
            HistoryLimits {
                max_retained_groups: 16,
                max_journal_bytes: 65536,
                max_pins: 8,
            },
        );
        let mut r = Recorder::disabled();
        let a = o.create(b"A", &mut r).unwrap();
        let p = o
            .prepare(
                &[DocumentEdits {
                    expected: a.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "A".into(),
                        inserted: "a".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        let g = o.commit(p, &mut r).unwrap().history_group.unwrap();
        (o, a.document, g, r)
    }

    #[test]
    fn reversal_plan_cannot_be_reclassified_as_an_ordinary_edit() {
        let (mut o, a, g, mut r) = fixture();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal(&id, &g, HistoryDirection::Undo, &mut r)
            .unwrap();
        let Phase::Pending {
            prepared: Ok(plan), ..
        } = &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
        else {
            unreachable!()
        };
        plan.history = history::PreparedHistory::Ordinary;
        let generation = o.generation();
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.documents[a.slot].as_ref().unwrap().source.text(), "a");
        assert!(matches!(
            o.invocations.slots[id.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
    }

    fn reads(version: DocumentVersion) -> ReadDependencyRequestV3 {
        ReadDependencyRequestV3 {
            predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(version),
            ))],
            budget: ReadDependencyBudget {
                max_predicates: 4,
                max_work: 128,
            },
        }
    }

    #[test]
    fn dependent_reversal_publishes_once_and_replay_preserves_original_frontier_meaning() {
        let (mut o, a, g, mut r) = fixture();
        let dependency = o.create(b"dependency", &mut r).unwrap();
        let request = reads(dependency);
        let target = HistoryReversalTargetV1::SourceFrontier {
            document: a.clone(),
            expected_group: g.clone(),
        };
        let id = o.issue_invocation(&mut r).unwrap();
        assert_eq!(
            o.submit_reversal_target_with_reads_v3(
                &id,
                &target,
                HistoryDirection::Undo,
                &request,
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
        assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "A");
        let generation = o.generation();
        assert_eq!(
            o.submit_reversal_target_with_reads_v3(
                &id,
                &target,
                HistoryDirection::Undo,
                &request,
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.submit_reversal(&id, &g, HistoryDirection::Undo, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        let mut changed = request.clone();
        changed.budget.max_work += 1;
        assert_eq!(
            o.submit_reversal_target_with_reads_v3(
                &id,
                &target,
                HistoryDirection::Undo,
                &changed,
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.submit_reversal_target_with_reads_v3(
                &id,
                &target,
                HistoryDirection::Redo,
                &request,
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
    }

    #[test]
    fn dependency_loss_after_admission_rejects_without_a_reversal_prefix() {
        let (mut o, a, g, mut r) = fixture();
        let dependency = o.create(b"dependency", &mut r).unwrap();
        let request = reads(dependency.clone());
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r)
            .unwrap();
        let p = o
            .prepare(
                &[DocumentEdits {
                    expected: dependency,
                    edits: vec![Replacement {
                        start: 0,
                        end: 10,
                        expected: "dependency".into(),
                        inserted: "changed".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(p, &mut r).unwrap();
        let generation = o.generation();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::Rejected(_))
        ));
        assert!(!o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "a");
        assert_eq!(
            o.history_status(&a, HistoryDirection::Undo, &mut r)
                .unwrap(),
            HistoryAvailability::Eligible(g.clone())
        );
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r),
            Ok(InvocationState::Finished)
        );
    }

    #[test]
    fn wrong_frontier_is_retained_as_a_refusal_not_silently_resolved_to_another_group() {
        let (mut o, a, g, mut r) = fixture();
        let b = o.create(b"B", &mut r).unwrap();
        let request = reads(b.clone());
        let target = HistoryReversalTargetV1::SourceFrontier {
            document: b.document,
            expected_group: g,
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_target_with_reads_v3(
            &id,
            &target,
            HistoryDirection::Undo,
            &request,
            &mut r,
        )
        .unwrap();
        let generation = o.generation();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::Rejected(CoordinatorError::History(
                HistoryError::Stale
            )))
        ));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "a");
    }

    #[test]
    fn dependent_reversal_budget_is_checked_before_binding_and_malformed_reads_do_not_submit() {
        let (mut o, _, g, mut r) = fixture();
        let dependency = o.create(b"dependency", &mut r).unwrap();
        let mut request = reads(dependency);
        let id = o.issue_invocation(&mut r).unwrap();
        let bytes = read_dependency::ReadInput::V3(&request)
            .retained_bytes(&[])
            .unwrap()
            + std::mem::size_of::<ReversalReadIntent>()
            - std::mem::size_of::<read_dependency::ReadCapture>();
        o.invocations.limits.max_intent_bytes = bytes - 1;
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        o.invocations.limits.max_intent_bytes = 4096;
        request.predicates.push(request.predicates[0].clone());
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r),
            Err(InvocationError::MalformedIntent)
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        request.predicates.pop();
        o.invocations.limits.max_intent_bytes = bytes;
        o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::Committed(_))
        ));
    }

    #[test]
    fn altered_dependency_capture_or_reversal_direction_fences_before_publication() {
        for drift in 0..4 {
            let (mut o, a, g, mut r) = fixture();
            let dependency = o.create(b"dependency", &mut r).unwrap();
            let request = reads(dependency);
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r)
                .unwrap();
            let Phase::Pending {
                prepared: Ok(plan), ..
            } = &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
            else {
                unreachable!()
            };
            match drift {
                0 => plan.declared_reads.as_mut().unwrap().budget.max_work += 1,
                1 => plan.declared_reads = None,
                2 => plan.reads_required = false,
                _ => {
                    let history::PreparedHistory::Reversal { direction, .. } = &mut plan.history
                    else {
                        unreachable!()
                    };
                    *direction = HistoryDirection::Redo;
                }
            }
            let generation = o.generation();
            assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
            assert_eq!(o.generation(), generation);
            assert_eq!(o.documents[a.slot].as_ref().unwrap().source.text(), "a");
        }
    }

    #[test]
    fn dependent_reversal_can_stop_without_publication_and_does_not_restart_on_replay() {
        let (mut o, a, g, mut r) = fixture();
        let dependency = o.create(b"dependency", &mut r).unwrap();
        let request = reads(dependency);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r)
            .unwrap();
        let generation = o.generation();
        o.stop_invocation(&id, &mut r).unwrap();
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r),
            Ok(InvocationState::Finished)
        );
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::StoppedBeforePublication)
        ));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "a");
    }

    #[test]
    fn dependency_order_is_canonical_but_refused_original_reads_cannot_be_refreshed() {
        let (mut o, a, g, mut r) = fixture();
        let d = o.create(b"dependency", &mut r).unwrap();
        let mut request = reads(d.clone());
        let second = o.create(b"other", &mut r).unwrap();
        request.predicates.extend(reads(second).predicates);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r)
            .unwrap();
        request.predicates.reverse();
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &request, &mut r),
            Ok(InvocationState::Pending)
        );
        o.stop_invocation(&id, &mut r).unwrap();
        let p = o
            .prepare(
                &[DocumentEdits {
                    expected: d.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 10,
                        expected: "dependency".into(),
                        inserted: "changed".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(p, &mut r).unwrap();
        let stale = reads(d);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &stale, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::Rejected(_))
        ));
        let ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(original))) =
            &stale.predicates[0]
        else {
            unreachable!()
        };
        let fresh = reads(o.read(&original.document, &mut r).unwrap().version);
        assert_eq!(
            o.submit_reversal_with_reads_v3(&id, &g, HistoryDirection::Undo, &fresh, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "a");
    }
}
