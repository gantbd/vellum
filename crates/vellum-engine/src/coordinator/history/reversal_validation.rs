//! Bounded prepublication checks over the original group and complete staged
//! frontier movement. No result issuance, naming or permission grant.
use super::*;
impl SourceCoordinator {
    pub(in crate::coordinator) fn validate_reversal_plan(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), HistoryError> {
        let PreparedHistory::Reversal {
            group,
            direction,
            stamps,
            annotation_stamps,
        } = &prepared.history
        else {
            return Ok(());
        };
        let timing = r.begin_service_timing();
        let result = (|| {
            // Only immutable meaning is checked here. Existing annotation and
            // history staging retain current permission/version refusal precedence.
            let original = self.group(group)?;
            let count = original
                .members
                .len()
                .checked_add(original.annotations.len())
                .ok_or(HistoryError::Faulted)?;
            if count == 0
                || count > self.limits.max_participants
                || prepared.candidates.len() != original.members.len()
                || stamps.len() != original.members.len()
                || annotation_stamps.len() != original.annotations.len()
            {
                return Err(HistoryError::Faulted);
            }
            for member in &original.members {
                if stamps
                    .iter()
                    .filter(|(id, _)| *id == member.document)
                    .count()
                    != 1
                    || prepared
                        .candidates
                        .iter()
                        .filter(|c| c.expected.document == member.document)
                        .count()
                        != 1
                {
                    return Err(HistoryError::Faulted);
                }
                let candidate = prepared
                    .candidates
                    .iter()
                    .find(|c| c.expected.document == member.document)
                    .ok_or(HistoryError::Faulted)?;
                let journal = match direction {
                    HistoryDirection::Undo => &member.inverse,
                    HistoryDirection::Redo => &member.forward,
                };
                if candidate.edits != *journal {
                    return Err(HistoryError::Faulted);
                }
            }
            for journal in &original.annotations {
                if annotation_stamps
                    .iter()
                    .filter(|(id, _)| *id == journal.id)
                    .count()
                    != 1
                {
                    return Err(HistoryError::Faulted);
                }
            }
            self.validate_annotation_reversal_plan_set(&original.annotations, *direction, prepared)
        })();
        r.record(Observation {
            boundary: Boundary::HistoryReversalPlanValidation,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(HistoryError::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
    pub(in crate::coordinator) fn validate_reversal_decision(
        &self,
        prepared: &PreparedTransaction,
        decision: &HistoryDecision,
        r: &mut Recorder,
    ) -> Result<(), HistoryError> {
        let PreparedHistory::Reversal {
            group,
            direction,
            stamps,
            annotation_stamps,
        } = &prepared.history
        else {
            return Ok(());
        };
        let timing = r.begin_service_timing();
        let result = (|| {
            let original = self.group(group).map_err(|_| HistoryError::Faulted)?;
            let count = original
                .members
                .len()
                .checked_add(original.annotations.len())
                .ok_or(HistoryError::Faulted)?;
            let (before, after) = match direction {
                HistoryDirection::Undo => (GroupState::Applied, GroupState::Undone),
                HistoryDirection::Redo => (GroupState::Undone, GroupState::Applied),
            };
            if original.state != before
                || count == 0
                || count > self.limits.max_participants
                || decision.group.is_some()
                || !decision.invalidated.is_empty()
                || decision.reversal.as_ref() != Some(&(group.clone(), after))
                || decision.outcome_group.as_ref() != Some(group)
                || decision.updates.len() != count
                || prepared.candidates.len() != original.members.len()
                || stamps.len() != original.members.len()
                || annotation_stamps.len() != original.annotations.len()
            {
                return Err(HistoryError::Faulted);
            }
            for (index, update) in decision.updates.iter().enumerate() {
                if !original.contains(&update.scope)
                    || decision.updates[..index]
                        .iter()
                        .any(|old| old.scope == update.scope)
                {
                    return Err(HistoryError::Faulted);
                }
                let current = match &update.scope {
                    Scope::Source(id) => {
                        &self
                            .retained_document(id)
                            .map_err(|_| HistoryError::Faulted)?
                            .history
                    }
                    Scope::Annotation(id) => {
                        &self
                            .annotations
                            .records
                            .get(id.slot)
                            .and_then(Option::as_ref)
                            .ok_or(HistoryError::Faulted)?
                            .history
                    }
                };
                let (from, to, next_from, next_to) = match direction {
                    HistoryDirection::Undo => (
                        &current.undo,
                        &current.redo,
                        &update.frontier.undo,
                        &update.frontier.redo,
                    ),
                    HistoryDirection::Redo => (
                        &current.redo,
                        &current.undo,
                        &update.frontier.redo,
                        &update.frontier.undo,
                    ),
                };
                if from.last() != Some(group)
                    || from.iter().filter(|id| *id == group).count() != 1
                    || to.contains(group)
                    || next_from.as_slice()
                        != &from[..from.len().checked_sub(1).ok_or(HistoryError::Faulted)?]
                    || next_to.len() != to.len().checked_add(1).ok_or(HistoryError::Faulted)?
                    || next_to.last() != Some(group)
                    || next_to[..to.len()] != **to
                    || Some(update.frontier.version) != current.version.checked_add(1)
                    || update.frontier.undo_lost != current.undo_lost
                    || update.frontier.redo_lost != current.redo_lost
                {
                    return Err(HistoryError::Faulted);
                }
            }
            Ok(())
        })();
        r.record(Observation {
            boundary: Boundary::HistoryReversalCandidateValidation,
            disposition: if result.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Unknown
            },
            service_time: timing.finish(),
        });
        result
    }
}
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(in crate::coordinator) enum Fault {
    MissingScope,
    DuplicateScope,
    WrongState,
    WrongGroup,
    WrongVersion,
    MissingOpposite,
    UnchangedFrom,
    LostFlag,
    Invalidated,
    UnexpectedGroup,
    MissingOutcome,
}
#[cfg(test)]
pub(in crate::coordinator) fn corrupt(decision: &mut HistoryDecision, fault: Option<Fault>) {
    let Some(fault) = fault else { return };
    match fault {
        Fault::MissingScope => {
            decision.updates.pop();
        }
        Fault::DuplicateScope => {
            decision.updates[1].scope = decision.updates[0].scope.clone();
        }
        Fault::WrongState => {
            decision.reversal.as_mut().unwrap().1 = GroupState::Invalidated;
        }
        Fault::WrongGroup => {
            decision.reversal.as_mut().unwrap().0.original.generation += 1;
        }
        Fault::WrongVersion => {
            decision.updates[0].frontier.version += 1;
        }
        Fault::MissingOpposite => {
            decision.updates[0].frontier.redo.pop();
        }
        Fault::UnchangedFrom => {
            let group = decision.outcome_group.clone().unwrap();
            decision.updates[0].frontier.undo.push(group);
        }
        Fault::LostFlag => {
            decision.updates[0].frontier.redo_lost = !decision.updates[0].frontier.redo_lost;
        }
        Fault::Invalidated => {
            decision
                .invalidated
                .push(decision.outcome_group.clone().unwrap());
        }
        Fault::UnexpectedGroup => {
            decision.group = Some(Group {
                id: decision.outcome_group.clone().unwrap(),
                members: vec![],
                annotations: vec![],
                state: GroupState::Applied,
                bytes: 0,
            });
        }
        Fault::MissingOutcome => {
            decision.outcome_group = None;
        }
    }
}
