//! Engine resource tests use a trusted scripted issuer, not real filesystem truth.
use vellum_engine::coordinator::{
    CoordinatorConfig, CoordinatorLimits, DocumentEdits, InvocationLimits, SourceCoordinator,
};
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::{
    Acknowledgment, Application, ContentComparison, EvidenceDisposition, Protection,
    ProviderProfile, ResourceError, ResourceLimits, SaveRequirements, Synchronization,
};
use vellum_engine::source::{Limits, Replacement};
#[path = "save_behaviors/causal.rs"]
mod causal;
#[path = "save_behaviors/comparison.rs"]
mod comparison;
#[path = "save_behaviors/continuity.rs"]
mod continuity;
#[path = "save_behaviors/retention.rs"]
mod retention;
#[path = "save_behaviors/root_accounting.rs"]
mod root_accounting;
#[path = "save_behaviors/serialization.rs"]
mod serialization;
#[path = "save_behaviors/timing.rs"]
mod timing;

fn owner() -> SourceCoordinator {
    owner_with_annotations(None)
}
fn owner_with_annotations(
    annotations: Option<vellum_engine::coordinator::AnnotationLimits>,
) -> SourceCoordinator {
    owner_with_annotations_and_blob(annotations, 4096)
}
fn owner_with_annotations_and_blob(
    annotations: Option<vellum_engine::coordinator::AnnotationLimits>,
    max_blob_bytes: usize,
) -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        search: None,
        annotations,
        limits: CoordinatorLimits {
            max_document_incarnations: 16,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 2048,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 8,
            max_intent_bytes: 4096,
        }),
        history: Some(vellum_engine::coordinator::HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        }),
        resources: Some(ResourceLimits {
            max_providers: 4,
            max_live_tasks: 16,
            max_issued_tasks: 32,
            max_queued_per_destination: 4,
            max_outstanding: 4,
            max_blob_bytes,
            max_key_bytes: 64,
            max_validator_bytes: 64,
            max_encoders: 4,
            max_live_artifacts: 16,
            max_issued_artifacts: 64,
            max_held_artifact_bytes: 65536,
            max_future_baseline_bytes: 65536,
            max_live_reads: 8,
            max_issued_reads: 32,
            max_read_chunks: 16,
            max_held_read_bytes: 1048576,
            max_restorers: 4,
            max_restoration_bytes: 8192,
        }),
    })
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}

#[test]
fn ambiguous_leading_literal_bom_refuses_before_save_issuance_or_dispatch_without_changing_source()
{
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let source = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "\u{feff}".into(),
                }],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(edit, &mut telemetry).unwrap();
    let current = owner
        .read(&source.document, &mut telemetry)
        .unwrap()
        .version;
    let generation = owner.generation();
    let usage = owner.resource_usage();
    assert_eq!(
        owner.queue_pinned_save(&current, requirements(), &mut telemetry),
        Err(ResourceError::Source(
            vellum_engine::source::SourceError::AmbiguousBom
        ))
    );
    assert_eq!(owner.resource_usage(), usage);
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner
            .read(&source.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "\u{feff}A"
    );
}

#[test]
fn given_alias_reads_then_one_owner_is_joined_without_overwriting_dirty_source() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: document.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "dirty".into(),
                }],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(edit, &mut telemetry).unwrap();
    let alias = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            Some(&port.identity_domain()),
            &mut telemetry,
        )
        .unwrap();
    let joined = owner
        .load_resource(
            alias.read_result(b"key", b"other", b"external").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(joined.document, document.document);
    assert_eq!(
        owner
            .read(&joined.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "dirty"
    );
}

#[test]
fn given_a_queued_capture_when_source_changes_then_dispatched_payload_is_the_original_bytes() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"\xef\xbb\xbfA\r\n")
                .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let before = owner.generation();
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    assert_eq!(owner.generation(), before);
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: document.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "new".into(),
                }],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    let local_edit = owner.commit(edit, &mut telemetry).unwrap();
    let source_group = local_edit.history_group.unwrap();
    let history_before_save = owner.history_usage();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"\xef\xbb\xbfA\r\n")
                .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"\xef\xbb\xbfA\r\n");
    let result = port
        .result(
            &dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::Succeeded,
            Some(b"v1"),
        )
        .unwrap();
    owner.record_save_evidence(result, &mut telemetry).unwrap();
    owner.publish_save_baseline(&task, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .saved_state(&document.document, &mut telemetry)
            .unwrap()
            .content,
        ContentComparison::Modified
    );
    assert_eq!(
        owner
            .save_outcome(&task, &mut telemetry)
            .unwrap()
            .captured
            .revision,
        document.revision
    );
    assert_eq!(owner.history_usage(), history_before_save);
    let undo = owner
        .prepare_reversal(
            &source_group,
            vellum_engine::coordinator::HistoryDirection::Undo,
            &mut telemetry,
        )
        .unwrap();
    owner.commit(undo, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .saved_state(&document.document, &mut telemetry)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(
        owner
            .save_outcome(&task, &mut telemetry)
            .unwrap()
            .captured
            .revision,
        document.revision
    );
}

#[test]
fn given_sync_failure_after_full_write_then_the_write_is_retained_without_a_false_saved_baseline() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(
            &document,
            SaveRequirements {
                acknowledgment: Acknowledgment::Synchronized,
                ..requirements()
            },
            &mut telemetry,
        )
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::Failed,
                Some(b"v1"),
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        owner
            .save_outcome(&task, &mut telemetry)
            .unwrap()
            .application,
        Application::Written
    );
    assert_eq!(
        owner.publish_save_baseline(&task, &mut telemetry),
        Err(ResourceError::AcknowledgmentUnsatisfied)
    );
}

#[test]
fn ordinary_source_acknowledgment_does_not_require_an_available_synchronization_capability() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut recorder,
        )
        .unwrap();
    let source = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(&source, requirements(), &mut recorder)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut recorder,
        )
        .unwrap();
    assert!(!dispatch.synchronization_required());
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
            &mut recorder,
        )
        .unwrap();
    owner.publish_save_baseline(&task, &mut recorder).unwrap();
}

#[test]
fn given_unknown_write_and_quiescence_then_same_destination_stays_blocked_until_safe_disposition() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let first = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let second = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    assert_eq!(
        owner
            .dispatch_save(
                &second,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut telemetry
            )
            .err(),
        Some(ResourceError::Blocked)
    );
    let dispatch = owner
        .dispatch_save(
            &first,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    owner.stop_save(&first, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .dispatch_save(
                &second,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut telemetry
            )
            .err(),
        Some(ResourceError::Blocked)
    );
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    owner
        .dispatch_save(
            &second,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
}

#[test]
fn given_conflicting_receipts_then_witnesses_are_preserved_and_destination_is_contested() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let written = port
        .result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap();
    let absent = port
        .result(
            &dispatch,
            Application::NotWritten,
            0,
            Synchronization::NotRequested,
            None,
        )
        .unwrap();
    owner
        .record_save_evidence(written.clone(), &mut telemetry)
        .unwrap();
    assert_eq!(
        owner.record_save_evidence(absent.clone(), &mut telemetry),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert_eq!(
        owner.record_save_evidence(absent, &mut telemetry),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        owner.record_save_evidence(written, &mut telemetry),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        owner.publish_save_baseline(&task, &mut telemetry),
        Err(ResourceError::Contested)
    );
    owner
        .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    owner
        .record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    assert_eq!(
        owner.retire_save(&task, &mut telemetry),
        Err(ResourceError::Retained)
    );
}

#[test]
fn given_conflict_or_stronger_required_guarantee_then_no_external_dispatch_is_issued() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        owner.queue_pinned_save(
            &document,
            SaveRequirements {
                protection: Protection::AtomicConditional,
                acknowledgment: Acknowledgment::Ordinary
            },
            &mut telemetry
        ),
        Err(ResourceError::Unsupported)
    );
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    assert_eq!(
        owner
            .dispatch_save(
                &task,
                port.read_result(b"key", b"changed", b"external").unwrap(),
                &mut telemetry
            )
            .err(),
        Some(ResourceError::Conflict)
    );
    assert_eq!(
        owner
            .read(&document.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
    owner.stop_save(&task, &mut telemetry).unwrap();
    owner.retire_save(&task, &mut telemetry).unwrap();
    assert_eq!(
        owner.save_outcome(&task, &mut telemetry).err(),
        Some(ResourceError::Unavailable)
    );
}

#[test]
fn given_a_newer_qualified_write_then_retirement_cannot_erase_its_baseline_election_hold() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let first = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &first,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    owner
        .record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    owner.publish_save_baseline(&first, &mut telemetry).unwrap();
    let second = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch2 = owner
        .dispatch_save(
            &second,
            port.read_result(b"key", b"v1", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch2,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"v2"),
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(port.no_future_effects(&dispatch2).unwrap(), &mut telemetry)
        .unwrap();
    owner
        .record_save_evidence(port.quiescence(&dispatch2).unwrap(), &mut telemetry)
        .unwrap();
    assert_eq!(
        owner.retire_save(&second, &mut telemetry),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        owner.publish_save_baseline(&first, &mut telemetry),
        Err(ResourceError::Superseded)
    );
    owner
        .publish_save_baseline(&second, &mut telemetry)
        .unwrap();
    drop(dispatch);
    owner.retire_save(&first, &mut telemetry).unwrap();
    assert_eq!(
        owner.retire_save(&second, &mut telemetry),
        Err(ResourceError::Retained)
    );
}

#[test]
fn given_current_permission_loss_then_launch_refuses_without_recapturing_or_publishing_text() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    owner
        .set_read_only(&document.document, true, &mut telemetry)
        .unwrap();
    assert_eq!(
        owner
            .dispatch_save(
                &task,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut telemetry
            )
            .err(),
        Some(ResourceError::Permission)
    );
    assert_eq!(
        owner.dispose(&document.document, &mut telemetry),
        Err(vellum_engine::coordinator::CoordinatorError::Resource(
            ResourceError::Retained
        ))
    );
    owner.stop_save(&task, &mut telemetry).unwrap();
    owner.retire_save(&task, &mut telemetry).unwrap();
    owner.dispose(&document.document, &mut telemetry).unwrap();
}

#[test]
fn given_undelivered_receipt_capabilities_then_quiescence_does_not_establish_delivery_drain() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let late = port
        .result(
            &dispatch,
            Application::Unknown,
            0,
            Synchronization::Unknown,
            None,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    owner
        .record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut telemetry)
        .unwrap();
    assert_eq!(
        owner.retire_save(&task, &mut telemetry),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    assert_eq!(
        owner.retire_save(&task, &mut telemetry),
        Err(ResourceError::Retained)
    );
    owner.record_save_evidence(late, &mut telemetry).unwrap();
    owner.retire_save(&task, &mut telemetry).unwrap();
}

#[test]
fn given_malformed_or_third_conflicting_results_then_the_original_evidence_is_preserved() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        port.result(
            &dispatch,
            Application::Written,
            0,
            Synchronization::Unknown,
            None
        )
        .err(),
        Some(ResourceError::Malformed)
    );
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        owner.record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"different")
            )
            .unwrap(),
            &mut telemetry
        ),
        Err(ResourceError::EvidenceCapacity)
    );
    let outcome = owner.save_outcome(&task, &mut telemetry).unwrap();
    assert_eq!(outcome.application, Application::Written);
    assert!(outcome.contested);
}

#[test]
fn given_tokenless_acknowledgment_then_historical_bytes_do_not_authorize_a_fresh_pinned_write() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = owner
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch = owner
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner.publish_save_baseline(&task, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .saved_state(&document.document, &mut telemetry)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(
        owner.queue_pinned_save(&document, requirements(), &mut telemetry),
        Err(ResourceError::MissingEvidence)
    );
    owner
        .record_save_evidence(
            port.result(
                &dispatch,
                Application::Unknown,
                0,
                Synchronization::Unknown,
                Some(b"v1"),
            )
            .unwrap(),
            &mut telemetry,
        )
        .unwrap();
    owner.publish_save_baseline(&task, &mut telemetry).unwrap();
    assert!(
        owner
            .queue_pinned_save(&document, requirements(), &mut telemetry)
            .is_ok()
    );
}

#[test]
fn given_independent_destinations_then_an_unknown_write_does_not_freeze_other_writes_or_local_editing()
 {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let a = owner
        .load_resource(port.read_result(b"A", b"v0", b"A").unwrap(), &mut telemetry)
        .unwrap();
    let b = owner
        .load_resource(port.read_result(b"B", b"v0", b"B").unwrap(), &mut telemetry)
        .unwrap();
    let first = owner
        .queue_pinned_save(&a, requirements(), &mut telemetry)
        .unwrap();
    let _first_dispatch = owner
        .dispatch_save(
            &first,
            port.read_result(b"A", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let second = owner
        .queue_pinned_save(&b, requirements(), &mut telemetry)
        .unwrap();
    let _second_dispatch = owner
        .dispatch_save(
            &second,
            port.read_result(b"B", b"v0", b"B").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: a.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "edited".into(),
                }],
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    owner.commit(edit, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&a.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "edited"
    );
    assert_eq!(owner.resource_usage().unresolved, 2);
}

#[test]
fn given_foreign_ports_and_evidence_then_another_owner_cannot_adopt_them() {
    let mut first = owner();
    let mut second = owner();
    let mut telemetry = Recorder::disabled();
    let port = first
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    assert_eq!(
        second.load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry
        ),
        Err(ResourceError::Unavailable)
    );
    let document = first
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let task = first
        .queue_pinned_save(&document, requirements(), &mut telemetry)
        .unwrap();
    let dispatch = first
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    let receipt = port.quiescence(&dispatch).unwrap();
    assert_eq!(
        second.record_save_evidence(receipt, &mut telemetry),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(second.resource_usage().bindings, 0);
}

#[test]
fn given_invalid_utf8_then_resource_load_does_not_publish_a_document_or_binding() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.load_resource(
            port.read_result(b"key", b"v0", b"\xff").unwrap(),
            &mut telemetry
        ),
        Err(ResourceError::Source(
            vellum_engine::source::SourceError::InvalidUtf8
        ))
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.resource_usage().bindings, 0);
}

#[test]
fn given_retired_tasks_then_lifetime_issuance_never_recycles_old_origins() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let port = owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut telemetry,
        )
        .unwrap();
    let document = owner
        .load_resource(
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut telemetry,
        )
        .unwrap();
    for _ in 0..32 {
        let task = owner
            .queue_pinned_save(&document, requirements(), &mut telemetry)
            .unwrap();
        owner.stop_save(&task, &mut telemetry).unwrap();
        owner.retire_save(&task, &mut telemetry).unwrap();
        assert_eq!(
            owner.save_outcome(&task, &mut telemetry).err(),
            Some(ResourceError::Unavailable)
        );
    }
    assert_eq!(owner.resource_usage().retained, 0);
    assert_eq!(
        owner.queue_pinned_save(&document, requirements(), &mut telemetry),
        Err(ResourceError::Limit)
    );
}

#[test]
fn given_capture_disabled_available_or_full_then_save_semantics_and_safe_diagnostics_match() {
    use vellum_engine::instrumentation::{Boundary, Disposition, Observation};
    let mut results = Vec::new();
    for capacity in [0, 1, 64] {
        let mut owner = owner();
        let mut telemetry = Recorder::try_with_capacity(capacity).unwrap();
        telemetry.record(Observation {
            boundary: Boundary::Query,
            disposition: Disposition::Completed,
            service_time: None,
        });
        let port = owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: true,
                },
                None,
                &mut telemetry,
            )
            .unwrap();
        let document = owner
            .load_resource(
                port.read_result(b"PRIVATE_KEY", b"PRIVATE_VALIDATOR", b"PRIVATE_SOURCE")
                    .unwrap(),
                &mut telemetry,
            )
            .unwrap();
        let task = owner
            .queue_pinned_save(&document, requirements(), &mut telemetry)
            .unwrap();
        let dispatch = owner
            .dispatch_save(
                &task,
                port.read_result(b"PRIVATE_KEY", b"PRIVATE_VALIDATOR", b"PRIVATE_SOURCE")
                    .unwrap(),
                &mut telemetry,
            )
            .unwrap();
        assert!(!format!("{dispatch:?}{port:?}").contains("PRIVATE_"));
        owner
            .record_save_evidence(
                port.result(
                    &dispatch,
                    Application::Written,
                    dispatch.bytes().len(),
                    Synchronization::Succeeded,
                    Some(b"new"),
                )
                .unwrap(),
                &mut telemetry,
            )
            .unwrap();
        owner
            .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut telemetry)
            .unwrap();
        owner
            .record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut telemetry)
            .unwrap();
        owner.publish_save_baseline(&task, &mut telemetry).unwrap();
        let outcome = owner.save_outcome(&task, &mut telemetry).unwrap();
        results.push((
            owner.generation(),
            outcome.application,
            outcome.synchronization,
            owner
                .saved_state(&document.document, &mut telemetry)
                .unwrap(),
            owner.resource_usage(),
        ));
        let records: Vec<_> = std::iter::from_fn(|| telemetry.pop_front()).collect();
        assert!(!format!("{records:?}").contains("PRIVATE_"));
        if capacity == 1 {
            assert!(telemetry.dropped() > 0);
        }
    }
    assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
}
