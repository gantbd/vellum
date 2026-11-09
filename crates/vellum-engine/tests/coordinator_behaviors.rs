//! BDD for the trusted local source owner, not the complete command facade.
use vellum_engine::coordinator::{
    CoordinatorError, CoordinatorLimits, DocumentEdits, SourceCoordinator,
};
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::source::{Limits, Replacement};

fn limits() -> CoordinatorLimits {
    CoordinatorLimits {
        max_document_incarnations: 16,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 16,
            max_edit_payload_bytes: 1024,
        },
    }
}

fn replacement(expected: &str, inserted: &str) -> Replacement {
    Replacement {
        start: 0,
        end: expected.len(),
        expected: expected.into(),
        inserted: inserted.into(),
    }
}

#[test]
fn given_two_documents_when_committed_then_both_change_at_one_generation() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"oldA", &mut telemetry).unwrap();
    let b = owner.create(b"oldB", &mut telemetry).unwrap();
    let before = owner.generation();
    let writes = [
        DocumentEdits {
            expected: a.clone(),
            edits: vec![replacement("oldA", "newA")],
        },
        DocumentEdits {
            expected: b.clone(),
            edits: vec![replacement("oldB", "newB")],
        },
    ];
    let prepared = owner.prepare(&writes, &[], &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "oldA"
    );
    let committed = owner.commit(prepared, &mut telemetry).unwrap();
    assert_eq!(committed.generation, before + 1);
    assert_eq!(committed.changes.len(), 2);
    for change in &committed.changes {
        assert_eq!(change.after.revision, change.before.revision + 1);
    }
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "newA"
    );
    assert_eq!(
        owner
            .read(&b.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "newB"
    );
}

#[test]
fn given_a_stale_second_target_when_committed_then_the_first_is_not_published() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let b = owner.create(b"B", &mut telemetry).unwrap();
    let writes = [
        DocumentEdits {
            expected: a.clone(),
            edits: vec![replacement("A", "a")],
        },
        DocumentEdits {
            expected: b.clone(),
            edits: vec![replacement("B", "b")],
        },
    ];
    let prepared = owner.prepare(&writes, &[], &mut telemetry).unwrap();
    let intervening = owner
        .prepare(
            &[DocumentEdits {
                expected: b.clone(),
                edits: vec![replacement("B", "new")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(intervening, &mut telemetry).unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.commit(prepared, &mut telemetry),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
    assert_eq!(
        owner
            .read(&b.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "new"
    );
}

#[test]
fn given_a_changed_read_dependency_when_committed_then_the_write_refuses() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let context = owner.create(b"context", &mut telemetry).unwrap();
    let prepared = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "a")],
            }],
            std::slice::from_ref(&context),
            &mut telemetry,
        )
        .unwrap();
    let other = owner
        .prepare(
            &[DocumentEdits {
                expected: context,
                edits: vec![replacement("context", "changed")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(other, &mut telemetry).unwrap();
    assert_eq!(
        owner.commit(prepared, &mut telemetry),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_read_only_policy_changed_then_prepared_equal_text_does_not_bypass_validation() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let writes = [DocumentEdits {
        expected: a.clone(),
        edits: vec![replacement("A", "A")],
    }];
    let prepared = owner.prepare(&writes, &[], &mut telemetry).unwrap();
    owner
        .set_read_only(&a.document, true, &mut telemetry)
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.commit(prepared, &mut telemetry),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner.prepare(&writes, &[], &mut telemetry).err(),
        Some(CoordinatorError::Unavailable)
    );
}

#[test]
fn given_equal_source_when_validated_then_no_revision_or_publication_is_fabricated() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::try_with_capacity(16).unwrap();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let generation = owner.generation();
    while telemetry.pop_front().is_some() {}
    let prepared = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "A")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    let outcome = owner.commit(prepared, &mut telemetry).unwrap();
    assert!(outcome.changes.is_empty());
    assert_eq!(outcome.generation, generation);
    assert_eq!(owner.read(&a.document, &mut telemetry).unwrap().version, a);
    while let Some(record) = telemetry.pop_front() {
        assert_ne!(record.boundary, Boundary::Publication);
    }
}

#[test]
fn given_foreign_handles_and_preparations_when_used_then_they_are_unavailable() {
    let mut first = SourceCoordinator::new(limits());
    let mut second = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = first.create(b"A", &mut telemetry).unwrap();
    second.create(b"other", &mut telemetry).unwrap();
    assert_eq!(
        second.read(&a.document, &mut telemetry).err(),
        Some(CoordinatorError::Unavailable)
    );
    let prepared = first
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "a")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        second.commit(prepared, &mut telemetry),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        first
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_disposal_and_reopen_when_old_work_arrives_then_it_cannot_match_the_new_incarnation() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let old = owner.create(b"same", &mut telemetry).unwrap();
    let prepared = owner
        .prepare(
            &[DocumentEdits {
                expected: old.clone(),
                edits: vec![replacement("same", "late")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.dispose(&old.document, &mut telemetry).unwrap();
    let new = owner.create(b"same", &mut telemetry).unwrap();
    assert_ne!(old.document, new.document);
    assert_eq!(
        owner.commit(prepared, &mut telemetry),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        owner
            .read(&new.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "same"
    );
}

#[test]
fn given_bounds_or_duplicate_participants_when_prepared_then_nothing_changes() {
    let mut configuration = limits();
    configuration.max_document_incarnations = 1;
    configuration.max_staged_bytes = 0;
    let mut owner = SourceCoordinator::new(configuration);
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    assert_eq!(
        owner.create(b"B", &mut telemetry),
        Err(CoordinatorError::Limit)
    );
    let write = DocumentEdits {
        expected: a.clone(),
        edits: vec![replacement("A", "a")],
    };
    assert_eq!(
        owner
            .prepare(std::slice::from_ref(&write), &[], &mut telemetry)
            .err(),
        Some(CoordinatorError::Limit)
    );
    assert_eq!(
        owner
            .prepare(&[write.clone(), write], &[], &mut telemetry)
            .err(),
        Some(CoordinatorError::DuplicateParticipant)
    );
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_write_policy_toggles_back_when_committed_then_old_preparation_is_still_stale() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let prepared = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "a")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner
        .set_read_only(&a.document, true, &mut telemetry)
        .unwrap();
    owner
        .set_read_only(&a.document, false, &mut telemetry)
        .unwrap();
    assert_eq!(
        owner.commit(prepared, &mut telemetry),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_an_unrelated_document_changes_when_committed_then_exact_dependencies_still_hold() {
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let prepared = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "a")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.create(b"unrelated", &mut telemetry).unwrap();
    owner.commit(prepared, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "a"
    );
}

#[test]
fn given_invalid_decode_or_bad_edit_when_prepared_then_generation_and_sources_survive() {
    use vellum_engine::source::SourceError;
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let b = owner.create(b"B", &mut telemetry).unwrap();
    let before = owner.generation();
    assert_eq!(
        owner.create(b"\xff", &mut telemetry),
        Err(CoordinatorError::Source(SourceError::InvalidUtf8))
    );
    let writes = [
        DocumentEdits {
            expected: a.clone(),
            edits: vec![replacement("A", "a")],
        },
        DocumentEdits {
            expected: b.clone(),
            edits: vec![replacement("wrong", "b")],
        },
    ];
    assert!(owner.prepare(&writes, &[], &mut telemetry).is_err());
    assert_eq!(owner.generation(), before);
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
    assert_eq!(
        owner
            .read(&b.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "B"
    );
}

#[test]
fn given_disabled_available_or_full_telemetry_when_committed_then_authoritative_results_match() {
    use vellum_engine::instrumentation::{Disposition, Observation};
    let mut outcomes = Vec::new();
    for capacity in [0, 1, 32] {
        let mut owner = SourceCoordinator::new(limits());
        let mut telemetry = Recorder::try_with_capacity(capacity).unwrap();
        // Saturate the capacity-one case before any engine behavior.
        telemetry.record(Observation {
            boundary: Boundary::Query,
            disposition: Disposition::Completed,
            service_time: None,
        });
        let a = owner.create(b"CONFIDENTIAL_A", &mut telemetry).unwrap();
        let b = owner.create(b"CONFIDENTIAL_B", &mut telemetry).unwrap();
        let writes = [
            DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("CONFIDENTIAL_A", "a")],
            },
            DocumentEdits {
                expected: b.clone(),
                edits: vec![replacement("CONFIDENTIAL_B", "b")],
            },
        ];
        let prepared = owner.prepare(&writes, &[], &mut telemetry).unwrap();
        let commit = owner.commit(prepared, &mut telemetry).unwrap();
        outcomes.push((
            commit.generation,
            commit
                .changes
                .iter()
                .map(|c| (c.before.revision, c.after.revision))
                .collect::<Vec<_>>(),
            owner
                .read(&a.document, &mut telemetry)
                .unwrap()
                .source
                .text()
                .to_owned(),
            owner
                .read(&b.document, &mut telemetry)
                .unwrap()
                .source
                .text()
                .to_owned(),
        ));
        if capacity == 1 {
            assert!(telemetry.dropped() > 0);
        }
        let records: Vec<_> = std::iter::from_fn(|| telemetry.pop_front()).collect();
        if capacity == 32 {
            assert_eq!(
                records
                    .iter()
                    .filter(|r| r.boundary == Boundary::Publication)
                    .count(),
                1
            );
            assert!(records.iter().all(|r| r.service_time.is_none()));
        }
        assert!(!format!("{records:?}").contains("CONFIDENTIAL"));
    }
    assert!(outcomes.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn given_an_exhausted_incarnation_budget_when_disposed_then_the_slot_is_not_reused() {
    let mut configuration = limits();
    configuration.max_document_incarnations = 1;
    let mut owner = SourceCoordinator::new(configuration);
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    owner.dispose(&a.document, &mut telemetry).unwrap();
    assert_eq!(
        owner.create(b"A", &mut telemetry),
        Err(CoordinatorError::Limit)
    );
    assert_eq!(
        owner.read(&a.document, &mut telemetry).err(),
        Some(CoordinatorError::Unavailable)
    );
}

#[test]
fn given_staging_bounds_when_a_large_source_shrinks_then_only_candidate_bytes_count() {
    let mut configuration = limits();
    configuration.max_staged_bytes = 1;
    let mut owner = SourceCoordinator::new(configuration);
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"large source", &mut telemetry).unwrap();
    let candidate = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("large source", "x")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(candidate, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "x"
    );
}

#[test]
fn given_a_stale_request_when_rejected_then_diagnostics_report_no_publication() {
    use vellum_engine::instrumentation::Disposition;
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::try_with_capacity(32).unwrap();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let first = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "a")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    let second = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "b")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(first, &mut telemetry).unwrap();
    while telemetry.pop_front().is_some() {}
    assert_eq!(
        owner.commit(second, &mut telemetry),
        Err(CoordinatorError::Stale)
    );
    let record = telemetry.pop_front().unwrap();
    assert_eq!(record.boundary, Boundary::Admission);
    assert_eq!(record.disposition, Disposition::Refused);
    assert_eq!(telemetry.pop_front(), None);
}

#[test]
fn given_two_threaded_competitors_when_host_serialized_then_only_one_revision_wins() {
    use std::sync::{Arc, Mutex};
    let mut owner = SourceCoordinator::new(limits());
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap();
    let first = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "first")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    let second = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![replacement("A", "second")],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    let owner = Arc::new(Mutex::new(owner));
    let threads = [first, second].map(|prepared| {
        let owner = owner.clone();
        std::thread::spawn(move || {
            owner
                .lock()
                .unwrap()
                .commit(prepared, &mut Recorder::disabled())
        })
    });
    let outcomes = threads.map(|thread| thread.join().unwrap());
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == Err(CoordinatorError::Stale))
            .count(),
        1
    );
    let owner = owner.lock().unwrap();
    let read = owner.read(&a.document, &mut telemetry).unwrap();
    assert_eq!(read.version.revision, a.revision + 1);
    assert!(matches!(read.source.text(), "first" | "second"));
}
