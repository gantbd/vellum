//! BDD for retained source invocations in the trusted local owner profile.
use vellum_engine::coordinator::{
    CoordinatorError, CoordinatorLimits, DocumentEdits, DocumentVersion, InvocationError,
    InvocationLimits, InvocationOutcome, InvocationState, SourceCoordinator, SourceIntent,
};
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::source::{Limits, Replacement};
#[path = "invocation_behaviors/policy.rs"]
mod policy;

fn owner() -> SourceCoordinator {
    owner_with(InvocationLimits {
        max_lifetime_issued: 8,
        max_retained: 4,
        max_intent_bytes: 4096,
    })
}

fn owner_with(invocation_limits: InvocationLimits) -> SourceCoordinator {
    SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 16,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 1024,
            },
        },
        invocation_limits,
    )
}

#[test]
fn given_optional_native_service_timing_then_execution_replay_and_refusal_preserve_the_same_outcome()
 {
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1};
    for mode in 0..4 {
        let mut o = owner();
        let mut r = if mode == 2 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        }
        .with_native_timing(if mode == 0 {
            NativeTimingProfileV1::Off
        } else {
            NativeTimingProfileV1::ProcessMonotonicV1
        });
        let d = o.create(b"A", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        // Saturated capture deliberately stays full; other modes start clean.
        if mode != 3 {
            while r.pop_front().is_some() {}
        }
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::NotSubmitted));
        o.submit(&id, &intent(d.clone(), "A", "B"), &mut r).unwrap();
        assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
        let generation = o.generation();
        assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
        assert_eq!(o.generation(), generation);
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::Committed(_))
        ));
        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "B");
        assert!(!o.is_faulted());
        let mut measured = vec![];
        while let Some(v) = r.pop_front() {
            if v.service_time.is_some() {
                measured.push((v.boundary, v.disposition));
            }
        }
        assert_eq!(
            measured,
            if mode == 1 {
                vec![
                    (Boundary::InvocationExecution, Disposition::Refused),
                    (Boundary::InvocationExecution, Disposition::Completed),
                    (Boundary::InvocationReplay, Disposition::Completed),
                    (Boundary::Outcome, Disposition::Completed),
                    (Boundary::Query, Disposition::Completed),
                ]
            } else {
                vec![]
            }
        );
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        o.retire_invocation(&id, &mut r).unwrap();
    }
}

#[test]
fn given_capture_fills_during_service_then_timing_loss_does_not_lose_publication_or_outcome() {
    use vellum_engine::instrumentation::NativeTimingProfileV1;
    let mut baseline = None;
    for profile in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        let mut o = owner();
        let mut setup = Recorder::disabled();
        let d = o.create(b"A", &mut setup).unwrap();
        let id = o.issue_invocation(&mut setup).unwrap();
        o.submit(&id, &intent(d.clone(), "A", "B"), &mut setup)
            .unwrap();
        // Entry has a free slot, but an inner publication marker consumes it.
        let mut r = Recorder::try_with_capacity(1)
            .unwrap()
            .with_native_timing(profile);
        assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
        assert!(r.dropped() > 0);
        let captured = r.pop_front().unwrap();
        assert_eq!(captured.boundary, Boundary::Admission);
        assert!(captured.service_time.is_none());
        assert!(r.pop_front().is_none());
        let evidence = (o.generation(), r.dropped(), captured);
        if let Some(expected) = baseline {
            assert_eq!(evidence, expected);
        } else {
            baseline = Some(evidence);
        }
        assert_eq!(o.read(&d.document, &mut setup).unwrap().source.text(), "B");
        assert!(matches!(
            o.invocation_outcome(&id, &mut setup),
            Ok(InvocationOutcome::Committed(_))
        ));
        assert!(!o.is_faulted());
        o.forget_invocation_outcome(&id, &mut setup).unwrap();
        o.retire_invocation(&id, &mut setup).unwrap();
    }
}

#[test]
fn given_reordered_and_partitioned_records_then_retries_use_the_original_normalized_intent() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"abc", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let mut edits = Vec::new();
    for (start, from, to) in [(0, "a", "A"), (1, "b", "B"), (2, "c", "C")] {
        edits.push(Replacement {
            start,
            end: start + 1,
            expected: from.into(),
            inserted: to.into(),
        });
    }
    let first = SourceIntent {
        writes: vec![DocumentEdits {
            expected: document.clone(),
            edits: vec![edits[2].clone(), edits[0].clone(), edits[1].clone()],
        }],
        reads: vec![],
    };
    owner.submit(&invocation, &first, &mut telemetry).unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let partitioned = SourceIntent {
            writes: order
                .iter()
                .map(|index| DocumentEdits {
                    expected: document.clone(),
                    edits: vec![edits[*index].clone()],
                })
                .collect(),
            reads: vec![],
        };
        assert_eq!(
            owner.submit(&invocation, &partitioned, &mut telemetry),
            Ok(InvocationState::Finished)
        );
    }
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "ABC"
    );
}

#[test]
fn given_malformed_or_conflicting_records_then_refusal_does_not_consume_first_submission() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let valid = intent(document.clone(), "A", "B");
    let mut overlap = valid.clone();
    let duplicate = overlap.writes[0].edits[0].clone();
    overlap.writes[0].edits.push(duplicate);
    assert_eq!(
        owner.submit(&invocation, &overlap, &mut telemetry),
        Err(InvocationError::MalformedIntent)
    );
    let mut conflicting = valid.clone();
    let mut other = valid.writes[0].clone();
    other.expected.revision += 1;
    conflicting.writes.push(other);
    assert_eq!(
        owner.submit(&invocation, &conflicting, &mut telemetry),
        Err(InvocationError::ConflictingParticipant)
    );
    assert_eq!(
        owner.run(&invocation, &mut telemetry),
        Err(InvocationError::NotSubmitted)
    );
    owner.submit(&invocation, &valid, &mut telemetry).unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "B"
    );
}

#[test]
fn given_an_invalid_empty_record_then_equal_source_does_not_elide_its_precondition() {
    use vellum_engine::source::SourceError;
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let mut request = intent(document.clone(), "", "");
    request.writes[0].edits[0].expected = "not empty".into();
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .invocation_outcome(&invocation, &mut telemetry)
            .unwrap(),
        &InvocationOutcome::Rejected(CoordinatorError::Source(SourceError::ExpectedMismatch))
    );
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_capacity_pressure_then_retirement_releases_retention_but_not_lifetime_issuance() {
    let mut owner = owner_with(InvocationLimits {
        max_lifetime_issued: 2,
        max_retained: 1,
        max_intent_bytes: 4096,
    });
    let mut telemetry = Recorder::disabled();
    let first = owner.issue_invocation(&mut telemetry).unwrap();
    assert_eq!(
        owner.issue_invocation(&mut telemetry),
        Err(InvocationError::Limit)
    );
    owner.retire_invocation(&first, &mut telemetry).unwrap();
    let second = owner.issue_invocation(&mut telemetry).unwrap();
    owner.retire_invocation(&second, &mut telemetry).unwrap();
    assert_eq!(
        owner.issue_invocation(&mut telemetry),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        owner.run(&first, &mut telemetry),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        owner.run(&second, &mut telemetry),
        Err(InvocationError::Unavailable)
    );
}

#[test]
fn given_intent_storage_limits_then_oversized_submission_keeps_the_slot_unsubmitted() {
    let mut owner = owner_with(InvocationLimits {
        max_lifetime_issued: 2,
        max_retained: 1,
        max_intent_bytes: 1,
    });
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    assert_eq!(
        owner.submit(
            &invocation,
            &intent(document.clone(), "A", "B"),
            &mut telemetry
        ),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        owner.run(&invocation, &mut telemetry),
        Err(InvocationError::NotSubmitted)
    );
    owner
        .retire_invocation(&invocation, &mut telemetry)
        .unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_changes_after_admission_then_final_validation_rejects_without_refreshing_intent() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let request = intent(document.clone(), "A", "B");
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    owner
        .set_read_only(&document.document, true, &mut telemetry)
        .unwrap();
    owner
        .set_read_only(&document.document, false, &mut telemetry)
        .unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .invocation_outcome(&invocation, &mut telemetry)
            .unwrap(),
        &InvocationOutcome::Rejected(CoordinatorError::Stale)
    );
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_applied_facts_when_late_stop_arrives_then_historical_result_is_not_rewritten() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .submit(
            &invocation,
            &intent(document.clone(), "A", "B"),
            &mut telemetry,
        )
        .unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    owner.stop_invocation(&invocation, &mut telemetry).unwrap();
    assert!(matches!(
        owner
            .invocation_outcome(&invocation, &mut telemetry)
            .unwrap(),
        InvocationOutcome::Committed(_)
    ));
    owner
        .forget_invocation_outcome(&invocation, &mut telemetry)
        .unwrap();
    owner.stop_invocation(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner.invocation_outcome(&invocation, &mut telemetry),
        Err(InvocationError::OutcomeUnavailable)
    );
}

#[test]
fn given_capture_disabled_available_or_full_then_replay_and_retirement_semantics_match() {
    use vellum_engine::instrumentation::{Disposition, Observation};
    let mut results = Vec::new();
    for capacity in [0, 1, 64] {
        let mut owner = owner();
        let mut telemetry = Recorder::try_with_capacity(capacity).unwrap();
        telemetry.record(Observation {
            boundary: Boundary::Query,
            disposition: Disposition::Completed,
            service_time: None,
        });
        let document = owner.create(b"PRIVATE_TEXT", &mut telemetry).unwrap();
        let invocation = owner.issue_invocation(&mut telemetry).unwrap();
        let request = intent(document.clone(), "PRIVATE_TEXT", "new");
        owner.submit(&invocation, &request, &mut telemetry).unwrap();
        owner.submit(&invocation, &request, &mut telemetry).unwrap();
        owner.run(&invocation, &mut telemetry).unwrap();
        owner.run(&invocation, &mut telemetry).unwrap();
        let generation = owner.generation();
        let revision = owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .version
            .revision;
        let outcome_applied = matches!(
            owner
                .invocation_outcome(&invocation, &mut telemetry)
                .unwrap(),
            InvocationOutcome::Committed(_)
        );
        owner
            .forget_invocation_outcome(&invocation, &mut telemetry)
            .unwrap();
        owner
            .retire_invocation(&invocation, &mut telemetry)
            .unwrap();
        results.push((
            generation,
            revision,
            outcome_applied,
            owner.run(&invocation, &mut telemetry),
        ));
        let records: Vec<_> = std::iter::from_fn(|| telemetry.pop_front()).collect();
        assert!(!format!("{records:?}").contains("PRIVATE_TEXT"));
        if capacity == 64 {
            assert_eq!(
                records
                    .iter()
                    .filter(|record| record.boundary == Boundary::Publication)
                    .count(),
                1
            );
        }
        if capacity == 1 {
            assert!(telemetry.dropped() > 0);
        }
    }
    assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
}

fn intent(expected: DocumentVersion, from: &str, to: &str) -> SourceIntent {
    SourceIntent {
        writes: vec![DocumentEdits {
            expected,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: from.into(),
                inserted: to.into(),
            }],
        }],
        reads: vec![],
    }
}

#[test]
fn given_matching_pending_and_terminal_retries_then_source_publishes_only_once() {
    let mut owner = owner();
    let mut telemetry = Recorder::try_with_capacity(64).unwrap();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let request = intent(document.clone(), "A", "B");
    assert_eq!(
        owner.submit(&invocation, &request, &mut telemetry),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        owner.submit(&invocation, &request, &mut telemetry),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        owner.run(&invocation, &mut telemetry),
        Ok(InvocationState::Finished)
    );
    let generation = owner.generation();
    for _ in 0..3 {
        assert_eq!(
            owner.submit(&invocation, &request, &mut telemetry),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            owner.run(&invocation, &mut telemetry),
            Ok(InvocationState::Finished)
        );
    }
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "B"
    );
    match owner
        .invocation_outcome(&invocation, &mut telemetry)
        .unwrap()
    {
        InvocationOutcome::Committed(outcome) => assert_eq!(outcome.changes.len(), 1),
        other => panic!("unexpected {other:?}"),
    }
    let records: Vec<_> = std::iter::from_fn(|| telemetry.pop_front()).collect();
    assert_eq!(
        records
            .iter()
            .filter(|r| r.boundary == Boundary::Publication)
            .count(),
        1
    );
}

#[test]
fn given_changed_intent_when_retried_then_original_pending_intent_survives() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let request = intent(document.clone(), "A", "B");
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    assert_eq!(
        owner.submit(
            &invocation,
            &intent(document.clone(), "A", "C"),
            &mut telemetry
        ),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "B"
    );
}

#[test]
fn given_caller_mutation_after_submit_then_owned_intent_and_candidate_do_not_change() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let mut request = intent(document.clone(), "A", "B");
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    request.writes[0].edits[0].inserted = "C".into();
    assert_eq!(
        owner.submit(&invocation, &request, &mut telemetry),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "B"
    );
}

#[test]
fn given_forgotten_applied_status_then_a_cache_miss_never_makes_execution_fresh() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let request = intent(document.clone(), "A", "B");
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    owner
        .forget_invocation_outcome(&invocation, &mut telemetry)
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.invocation_outcome(&invocation, &mut telemetry),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(
        owner.submit(&invocation, &request, &mut telemetry),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        owner.run(&invocation, &mut telemetry),
        Ok(InvocationState::Finished)
    );
    assert_eq!(owner.generation(), generation);
}

#[test]
fn given_retirement_then_old_and_foreign_identities_never_address_a_new_slot() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let old = owner.issue_invocation(&mut telemetry).unwrap();
    owner.retire_invocation(&old, &mut telemetry).unwrap();
    let new = owner.issue_invocation(&mut telemetry).unwrap();
    assert_ne!(old, new);
    assert_eq!(
        owner.submit(&old, &intent(document, "A", "B"), &mut telemetry),
        Err(InvocationError::Unavailable)
    );
    let mut other = self::owner();
    assert_eq!(
        other.run(&new, &mut telemetry),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        owner.run(&new, &mut telemetry),
        Err(InvocationError::NotSubmitted)
    );
}

#[test]
fn given_a_spent_noop_when_source_changes_then_retry_does_not_reclassify_it() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let request = intent(document.clone(), "A", "A");
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    let other = owner
        .prepare(
            &intent(document.clone(), "A", "B").writes,
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(other, &mut telemetry).unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.submit(&invocation, &request, &mut telemetry),
        Ok(InvocationState::Finished)
    );
    owner.run(&invocation, &mut telemetry).unwrap();
    match owner
        .invocation_outcome(&invocation, &mut telemetry)
        .unwrap()
    {
        InvocationOutcome::Committed(outcome) => assert!(outcome.changes.is_empty()),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "B"
    );
}

#[test]
fn given_pending_work_when_retired_then_it_is_preserved_until_stopped_or_finished() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .submit(
            &invocation,
            &intent(document.clone(), "A", "B"),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        owner.retire_invocation(&invocation, &mut telemetry),
        Err(InvocationError::Pending)
    );
    owner.stop_invocation(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner.run(&invocation, &mut telemetry),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        owner
            .invocation_outcome(&invocation, &mut telemetry)
            .unwrap(),
        &InvocationOutcome::StoppedBeforePublication
    );
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
    owner
        .retire_invocation(&invocation, &mut telemetry)
        .unwrap();
}

#[test]
fn given_stale_or_denied_first_execution_then_matching_retry_retains_rejection() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .set_read_only(&document.document, true, &mut telemetry)
        .unwrap();
    let request = intent(document.clone(), "A", "B");
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    owner
        .set_read_only(&document.document, false, &mut telemetry)
        .unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .invocation_outcome(&invocation, &mut telemetry)
            .unwrap(),
        &InvocationOutcome::Rejected(CoordinatorError::Unavailable)
    );
    owner.submit(&invocation, &request, &mut telemetry).unwrap();
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_status_queries_then_they_never_execute_or_discard_pending_work() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    assert_eq!(
        owner.invocation_state(&invocation, &mut telemetry),
        Ok(InvocationState::Issued)
    );
    owner
        .submit(
            &invocation,
            &intent(document.clone(), "A", "B"),
            &mut telemetry,
        )
        .unwrap();
    let generation = owner.generation();
    for _ in 0..3 {
        assert_eq!(
            owner.invocation_state(&invocation, &mut telemetry),
            Ok(InvocationState::Pending)
        );
        assert_eq!(
            owner.invocation_outcome(&invocation, &mut telemetry),
            Err(InvocationError::OutcomeUnavailable)
        );
        assert_eq!(
            owner.forget_invocation_outcome(&invocation, &mut telemetry),
            Err(InvocationError::Pending)
        );
    }
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(
        owner.invocation_state(&invocation, &mut telemetry),
        Ok(InvocationState::Finished)
    );
}

#[test]
fn given_concurrent_matching_submissions_then_one_owner_keeps_one_execution() {
    use std::sync::{Arc, Mutex};
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    let request = intent(document.clone(), "A", "B");
    let shared = Arc::new(Mutex::new(owner));
    let threads = [0, 1].map(|_| {
        let shared = shared.clone();
        let invocation = invocation.clone();
        let request = request.clone();
        std::thread::spawn(move || {
            let mut owner = shared.lock().unwrap();
            let mut telemetry = Recorder::disabled();
            owner.submit(&invocation, &request, &mut telemetry).unwrap();
            owner.run(&invocation, &mut telemetry).unwrap()
        })
    });
    for thread in threads {
        assert_eq!(thread.join().unwrap(), InvocationState::Finished);
    }
    let owner = shared.lock().unwrap();
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .version
            .revision,
        document.revision + 1
    );
    assert_eq!(owner.generation(), 2);
}

#[test]
fn given_retention_transitions_then_usage_distinguishes_status_loss_from_spent_identity() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let document = owner.create(b"A", &mut telemetry).unwrap();
    let invocation = owner.issue_invocation(&mut telemetry).unwrap();
    assert_eq!(owner.invocation_usage().issued_unsubmitted, 1);
    owner
        .submit(&invocation, &intent(document, "A", "B"), &mut telemetry)
        .unwrap();
    assert_eq!(owner.invocation_usage().pending, 1);
    owner.run(&invocation, &mut telemetry).unwrap();
    assert_eq!(owner.invocation_usage().finished, 1);
    assert_eq!(owner.invocation_usage().outcomes_available, 1);
    owner
        .forget_invocation_outcome(&invocation, &mut telemetry)
        .unwrap();
    let forgotten = owner.invocation_usage();
    assert_eq!(forgotten.retained, 1);
    assert_eq!(forgotten.finished, 1);
    assert_eq!(forgotten.outcomes_available, 0);
    owner
        .retire_invocation(&invocation, &mut telemetry)
        .unwrap();
    let retired = owner.invocation_usage();
    assert_eq!(retired.retained, 0);
    assert_eq!(retired.issued_lifetime, 1);
    assert_eq!(retired.pending, 0);
    assert_eq!(retired.faulted_executions, 0);
    assert_eq!(retired.max_retained, 4);
}
#[path = "invocation_behaviors/observation.rs"]
mod observation;
