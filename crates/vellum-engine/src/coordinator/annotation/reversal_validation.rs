//! Exact immutable annotation reversal meaning. Existing staging separately
//! checks captured versions, current policy and source currency, without refresh.
use super::*;
enum Expected<'a> {
    Membership(&'a Option<Authored>),
    Body(&'a Arc<str>),
    Target(&'a TargetState),
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn validate_annotation_reversal_plan_set(
        &self,
        journals: &[AnnotationJournal],
        direction: HistoryDirection,
        prepared: &PreparedTransaction,
    ) -> Result<(), HistoryError> {
        if journals
            .len()
            .checked_mul(2)
            .is_none_or(|maximum| prepared.annotations.len() > maximum)
        {
            return Err(HistoryError::Faulted);
        }
        let mut expected_count = 0usize;
        for journal in journals {
            let linked = prepared
                .candidates
                .iter()
                .any(|c| c.expected.document == journal.id.document);
            let mut check = |expected: Expected<'_>| -> Result<(), HistoryError> {
                if prepared
                    .annotations
                    .iter()
                    .filter(|plan| {
                        plan.id == journal.id
                            && match (&plan.wish, &expected) {
                                (
                                    Wish::RestoreMembership {
                                        desired,
                                        source,
                                        linked: actual,
                                        ..
                                    },
                                    Expected::Membership(wanted),
                                ) => {
                                    desired == *wanted
                                        && source.document == journal.id.document
                                        && *actual == linked
                                }
                                (Wish::RestoreBody { body, .. }, Expected::Body(wanted)) => {
                                    body == *wanted
                                }
                                (
                                    Wish::RestoreTarget {
                                        state,
                                        source,
                                        linked: actual,
                                        ..
                                    },
                                    Expected::Target(wanted),
                                ) => {
                                    state == *wanted
                                        && source.document == journal.id.document
                                        && *actual == linked
                                }
                                _ => false,
                            }
                    })
                    .count()
                    != 1
                {
                    return Err(HistoryError::Faulted);
                }
                expected_count = expected_count.checked_add(1).ok_or(HistoryError::Faulted)?;
                Ok(())
            };
            match &journal.delta {
                Delta::Membership { before, after } => {
                    let desired = match direction {
                        HistoryDirection::Undo => before,
                        HistoryDirection::Redo => after,
                    };
                    check(Expected::Membership(desired))?;
                }
                Delta::Fields { body, target } => {
                    if let Some((before, after)) = body {
                        let desired = match direction {
                            HistoryDirection::Undo => before,
                            HistoryDirection::Redo => after,
                        };
                        check(Expected::Body(desired))?;
                    }
                    if let Some((before, after)) = target {
                        let desired = match direction {
                            HistoryDirection::Undo => before,
                            HistoryDirection::Redo => after,
                        };
                        check(Expected::Target(desired))?;
                    }
                }
            }
        }
        if expected_count != prepared.annotations.len() {
            return Err(HistoryError::Faulted);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_body_or_target_inverse_is_not_an_alternate_valid_reversal() {
        for target in [false, true] {
            let mut r = Recorder::disabled();
            let (mut o, source, id) = super::super::tests::setup(&mut r);
            let value = &o.annotations.records[id.slot].as_ref().unwrap().value;
            let mutation = if target {
                AnnotationMutation::Reattach {
                    annotation: id.clone(),
                    expected_target: value.target_generation,
                    target: SourceTarget {
                        source,
                        target: AttachmentTarget::ContentRange { start: 1, end: 2 },
                        basis: TargetBasis::Before,
                    },
                }
            } else {
                AnnotationMutation::UpdateBody {
                    annotation: id.clone(),
                    expected_content: value.content,
                    body: "PRIVATE_NEW".into(),
                }
            };
            let p = o.prepare_compound(&[], &[], &[mutation], &mut r).unwrap();
            let group = o.commit(p, &mut r).unwrap().history_group.unwrap();
            let mut p = o
                .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
                .unwrap();
            match &mut p.annotations[0].wish {
                Wish::RestoreBody { body, .. } => *body = Arc::from("PRIVATE_WRONG"),
                Wish::RestoreTarget { state, .. } => {
                    state.authored.target = AttachmentTarget::ContentRange { start: 0, end: 2 }
                }
                _ => panic!("wrong inverse"),
            }
            let generation = o.generation();
            assert_eq!(
                o.commit(p, &mut r).err(),
                Some(CoordinatorError::History(HistoryError::Faulted))
            );
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation);
        }
    }
    #[test]
    fn later_body_publication_retains_existing_stale_refusal_without_refresh_or_fault() {
        let mut r = Recorder::disabled();
        let (mut o, _, id) = super::super::tests::setup(&mut r);
        let mut group = None;
        let mut reversal = None;
        for text in ["PRIVATE_NEW", "PRIVATE_LATER"] {
            let version = o.annotations.records[id.slot]
                .as_ref()
                .unwrap()
                .value
                .content;
            let p = o
                .prepare_compound(
                    &[],
                    &[],
                    &[AnnotationMutation::UpdateBody {
                        annotation: id.clone(),
                        expected_content: version,
                        body: text.into(),
                    }],
                    &mut r,
                )
                .unwrap();
            let outcome = o.commit(p, &mut r).unwrap();
            if group.is_none() {
                group = outcome.history_group;
                reversal = Some(
                    o.prepare_reversal(group.as_ref().unwrap(), HistoryDirection::Undo, &mut r)
                        .unwrap(),
                );
            }
        }
        let generation = o.generation();
        assert_eq!(
            o.commit(reversal.unwrap(), &mut r).err(),
            Some(CoordinatorError::Stale)
        );
        assert!(!o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(
            &*o.annotations.records[id.slot].as_ref().unwrap().value.body,
            "PRIVATE_LATER"
        );
    }
    #[test]
    fn missing_duplicate_or_changed_annotation_inverse_meaning_faults_before_publication() {
        for change in 0..4 {
            let mut r = Recorder::disabled();
            let (mut o, _, id) = super::super::tests::setup(&mut r);
            let HistoryAvailability::Eligible(group) = o
                .annotation_history_status(&id, HistoryDirection::Undo, &mut r)
                .unwrap()
            else {
                panic!("missing group")
            };
            let mut p = o
                .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
                .unwrap();
            match change {
                0 => {
                    p.annotations.pop();
                }
                1 | 2 => {
                    p.annotations.push(p.annotations[0].clone());
                    if change == 2 {
                        p.annotations.push(p.annotations[0].clone());
                    }
                }
                _ => {
                    let Wish::RestoreMembership { desired, .. } = &mut p.annotations[0].wish else {
                        panic!("wrong plan")
                    };
                    *desired = Some(
                        o.annotations.records[id.slot]
                            .as_ref()
                            .unwrap()
                            .value
                            .authored(),
                    );
                }
            }
            let generation = o.generation();
            assert_eq!(
                o.commit(p, &mut r).err(),
                Some(CoordinatorError::History(HistoryError::Faulted))
            );
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation);
            let record = &o.annotations.records[id.slot].as_ref().unwrap().value;
            assert!(record.present);
            assert_eq!(&*record.body, "PRIVATE_OLD");
        }
    }
}
