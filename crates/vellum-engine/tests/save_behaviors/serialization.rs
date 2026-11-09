use super::*;
use vellum_engine::coordinator::*;
#[path = "serialization_arguments.rs"]
mod arguments;
#[path = "codec.rs"]
mod codec;
#[path = "scoped_serialization.rs"]
mod scoped;
fn limits() -> SourceSerializationLimitsV1 {
    SourceSerializationLimitsV1 {
        max_dependencies: 8,
        max_label_bytes: 128,
        max_request_bytes: 4096,
        max_plan_bytes: 4096,
        max_work: 4096,
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "author".into(),
        purpose: "edit".into(),
        destination: "client".into(),
        profile: "v1".into(),
        continuity: "session".into(),
    }
}
fn facts<'a>(
    c: &'a SettingsExecutionContextV1,
    document: &'a DocumentId,
) -> TrustedSerializationFactsV1<'a> {
    TrustedSerializationFactsV1 {
        context: c,
        document,
        profile: limits(),
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        companions_ready: Some(true),
        settings_writable: Some(true),
        resources_ready: Some(true),
        write_allowed: Some(true),
        required_reads: &[],
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn intent(bom: bool) -> SourceSerializationV1 {
    SourceSerializationV1 {
        profile: SourceSerializationProfileV1::StrictUtf8OneBom,
        bom,
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_source_serialization("engine", "local", limits(), &mut r)
        .unwrap();
    (o, r)
}
fn apply(
    o: &mut SourceCoordinator,
    document: &DocumentId,
    bom: bool,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> SourceSerializationUpdateV1 {
    let expected = o.source_serialization(document, r).unwrap().generation;
    let reads = reads();
    let p = o
        .prepare_source_serialization(
            &SourceSerializationRequestV1 {
                document,
                expected,
                serialization: intent(bom),
                dependencies: &reads,
            },
            c,
            &facts(c, document),
            r,
        )
        .unwrap();
    o.commit_source_serialization(p, c, &facts(c, document), r)
        .unwrap()
}
#[test]
fn metadata_intent_is_versioned_without_source_edits_or_configuration_registry() {
    let (mut o, mut r) = ready();
    let c = context();
    let doc = o.create(b"A\r\n", &mut r).unwrap();
    let initial = o.source_serialization(&doc.document, &mut r).unwrap();
    assert_eq!(initial.generation, 1);
    assert!(!initial.serialization.bom);
    let history = o.history_usage();
    let before = o.generation();
    assert!(apply(&mut o, &doc.document, true, &c, &mut r).changed);
    assert_eq!(o.generation(), before + 1);
    assert_eq!(o.history_usage(), history);
    assert_eq!(o.read(&doc.document, &mut r).unwrap().version, doc);
    assert_eq!(
        o.read(&doc.document, &mut r).unwrap().source.text(),
        "A\r\n"
    );
    assert_eq!(
        o.source_serialization(&doc.document, &mut r)
            .unwrap()
            .generation,
        2
    );
}
#[test]
fn metadata_preparation_survives_source_journal_progress_and_undo_does_not_revert_format_intent() {
    let (mut o, mut r) = ready();
    let c = context();
    let doc = o.create(b"A", &mut r).unwrap();
    let reads = reads();
    let p = o
        .prepare_source_serialization(
            &SourceSerializationRequestV1 {
                document: &doc.document,
                expected: 1,
                serialization: intent(true),
                dependencies: &reads,
            },
            &c,
            &facts(&c, &doc.document),
            &mut r,
        )
        .unwrap();
    let text = o
        .prepare(
            &[DocumentEdits {
                expected: doc.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let edited = o.commit(text, &mut r).unwrap();
    let history = o.history_usage();
    o.commit_source_serialization(p, &c, &facts(&c, &doc.document), &mut r)
        .unwrap();
    assert_eq!(o.history_usage(), history);
    assert_eq!(o.read(&doc.document, &mut r).unwrap().source.text(), "B");
    let undo = o
        .prepare_reversal(
            edited.history_group.as_ref().unwrap(),
            HistoryDirection::Undo,
            &mut r,
        )
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    assert_eq!(o.read(&doc.document, &mut r).unwrap().source.text(), "A");
    assert!(
        o.source_serialization(&doc.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
}
#[test]
fn old_save_capture_keeps_original_format_and_late_acknowledgment_cannot_refresh_current_output() {
    let (mut o, mut r) = ready();
    let c = context();
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let doc = o
        .load_resource(port.read_result(b"key", b"v0", b"A\r\n").unwrap(), &mut r)
        .unwrap();
    let old = o.queue_pinned_save(&doc, requirements(), &mut r).unwrap();
    let held = o.resource_usage();
    apply(&mut o, &doc.document, true, &c, &mut r);
    assert_eq!(o.resource_usage(), held);
    assert_eq!(
        o.saved_state(&doc.document, &mut r).unwrap().content,
        ContentComparison::Modified
    );
    let old_outcome = o.save_outcome(&old, &mut r).unwrap();
    assert_eq!(old_outcome.serialization_generation, Some(1));
    assert_eq!(
        old_outcome.source_serialization.unwrap().serialization,
        intent(false)
    );
    let dispatch = o
        .dispatch_save(
            &old,
            port.read_result(b"key", b"v0", b"A\r\n").unwrap(),
            &mut r,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"A\r\n");
    o.record_save_evidence(
        port.result(
            &dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.publish_save_baseline(&old, &mut r).unwrap();
    assert_eq!(
        o.saved_state(&doc.document, &mut r).unwrap().content,
        ContentComparison::Modified
    );
    assert_eq!(
        o.source_serialization(&doc.document, &mut r)
            .unwrap()
            .generation,
        2
    );
    let new = o.queue_pinned_save(&doc, requirements(), &mut r).unwrap();
    assert_eq!(
        o.save_outcome(&new, &mut r)
            .unwrap()
            .serialization_generation,
        Some(2)
    );
    assert_eq!(
        o.dispatch_save(
            &new,
            port.read_result(b"key", b"v1", b"A\r\n").unwrap(),
            &mut r
        )
        .err(),
        Some(ResourceError::Blocked)
    );
    o.record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut r)
        .unwrap();
    let next = o
        .dispatch_save(
            &new,
            port.read_result(b"key", b"v1", b"A\r\n").unwrap(),
            &mut r,
        )
        .unwrap();
    assert_eq!(next.bytes(), b"\xef\xbb\xbfA\r\n");
    o.record_save_evidence(
        port.result(
            &next,
            Application::Written,
            next.bytes().len(),
            Synchronization::NotRequested,
            Some(b"v2"),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.publish_save_baseline(&new, &mut r).unwrap();
    assert_eq!(
        o.saved_state(&doc.document, &mut r).unwrap().content,
        ContentComparison::Unchanged
    );
    assert_eq!(o.read(&doc.document, &mut r).unwrap().version, doc);
    assert_eq!(
        o.save_outcome(&old, &mut r)
            .unwrap()
            .serialization_generation,
        Some(1)
    );
}
#[test]
fn format_intent_can_be_unencodable_until_capture_without_source_loss_or_save_issuance() {
    let (mut o, mut r) = ready();
    let c = context();
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let doc = o
        .load_resource(
            port.read_result(b"key", b"v0", b"\xef\xbb\xbf\xef\xbb\xbfA")
                .unwrap(),
            &mut r,
        )
        .unwrap();
    assert!(
        o.source_serialization(&doc.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
    assert!(apply(&mut o, &doc.document, false, &c, &mut r).changed);
    let usage = o.resource_usage();
    let generation = o.generation();
    assert_eq!(
        o.queue_pinned_save(&doc, requirements(), &mut r),
        Err(ResourceError::Source(
            vellum_engine::source::SourceError::AmbiguousBom
        ))
    );
    assert_eq!(o.resource_usage(), usage);
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.read(&doc.document, &mut r).unwrap().source.text(),
        "\u{feff}A"
    );
    apply(&mut o, &doc.document, true, &c, &mut r);
    let task = o.queue_pinned_save(&doc, requirements(), &mut r).unwrap();
    assert_eq!(
        o.save_outcome(&task, &mut r)
            .unwrap()
            .serialization_generation,
        Some(3)
    );
    let dispatch = o
        .dispatch_save(
            &task,
            port.read_result(b"key", b"v0", b"\xef\xbb\xbf\xef\xbb\xbfA")
                .unwrap(),
            &mut r,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"\xef\xbb\xbf\xef\xbb\xbfA");
}
#[test]
fn current_target_context_profile_and_every_exact_true_guard_remain_required() {
    for field in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = ready();
            let c = context();
            let doc = o.create(b"A", &mut r).unwrap();
            let mut f = facts(&c, &doc.document);
            let reads = reads();
            let p = o
                .prepare_source_serialization(
                    &SourceSerializationRequestV1 {
                        document: &doc.document,
                        expected: 1,
                        serialization: intent(true),
                        dependencies: &reads,
                    },
                    &c,
                    &f,
                    &mut r,
                )
                .unwrap();
            match field {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.not_stopped = value,
                5 => f.interpretation_current = value,
                6 => f.companions_ready = value,
                7 => f.settings_writable = value,
                8 => f.resources_ready = value,
                _ => f.write_allowed = value,
            }
            let before = o.generation();
            assert!(o.commit_source_serialization(p, &c, &f, &mut r).is_err());
            assert_eq!(o.generation(), before);
            assert_eq!(
                o.source_serialization(&doc.document, &mut r)
                    .unwrap()
                    .generation,
                1
            );
            assert!(!o.is_faulted());
        }
    }
    for field in 0..9 {
        let (mut o, mut r) = ready();
        let c = context();
        let doc = o.create(b"A", &mut r).unwrap();
        let other = o.create(b"B", &mut r).unwrap();
        let reads = reads();
        let p = o
            .prepare_source_serialization(
                &SourceSerializationRequestV1 {
                    document: &doc.document,
                    expected: 1,
                    serialization: intent(true),
                    dependencies: &reads,
                },
                &c,
                &facts(&c, &doc.document),
                &mut r,
            )
            .unwrap();
        let mut foreign = c.clone();
        match field {
            0 => foreign.authority.push('x'),
            1 => foreign.engine.push('x'),
            2 => foreign.recipient.push('x'),
            3 => foreign.purpose.push('x'),
            4 => foreign.destination.push('x'),
            5 => foreign.profile.push('x'),
            6 => foreign.continuity.push('x'),
            _ => {}
        }
        let mut f = facts(&foreign, &doc.document);
        if field == 7 {
            f.document = &other.document;
        }
        if field == 8 {
            f.profile.max_work -= 1;
        }
        assert!(o.commit_source_serialization(p, &c, &f, &mut r).is_err());
        assert_eq!(
            o.source_serialization(&doc.document, &mut r)
                .unwrap()
                .generation,
            1
        );
    }
}
#[test]
fn declared_and_mandatory_reads_are_current_conjunctive_and_required_sets_cannot_disappear() {
    for changed in ["none", "source", "required"] {
        let (mut o, mut r) = ready();
        let c = context();
        let doc = o.create(b"A", &mut r).unwrap();
        let predicate = ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(
            doc.clone(),
        )));
        let required = [predicate.clone()];
        let mut reads = reads();
        reads.predicates.push(predicate);
        reads.budget.max_predicates = 1;
        let mut f = facts(&c, &doc.document);
        f.required_reads = &required;
        let p = o
            .prepare_source_serialization(
                &SourceSerializationRequestV1 {
                    document: &doc.document,
                    expected: 1,
                    serialization: intent(true),
                    dependencies: &reads,
                },
                &c,
                &f,
                &mut r,
            )
            .unwrap();
        if changed == "source" {
            let edit = o
                .prepare(
                    &[DocumentEdits {
                        expected: doc.clone(),
                        edits: vec![Replacement {
                            start: 0,
                            end: 1,
                            expected: "A".into(),
                            inserted: "B".into(),
                        }],
                    }],
                    &[],
                    &mut r,
                )
                .unwrap();
            o.commit(edit, &mut r).unwrap();
        }
        if changed == "required" {
            f.required_reads = &[];
        }
        let before = o.generation();
        let result = o.commit_source_serialization(p, &c, &f, &mut r);
        if changed == "none" {
            assert!(result.unwrap().changed);
        } else {
            assert!(result.is_err());
            assert_eq!(o.generation(), before);
        }
    }
}
#[test]
fn stale_and_aba_metadata_preparations_do_not_revert_newer_intent_even_for_noops() {
    for noop in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let doc = o.create(b"A", &mut r).unwrap();
        let reads = reads();
        let p = o
            .prepare_source_serialization(
                &SourceSerializationRequestV1 {
                    document: &doc.document,
                    expected: 1,
                    serialization: intent(!noop),
                    dependencies: &reads,
                },
                &c,
                &facts(&c, &doc.document),
                &mut r,
            )
            .unwrap();
        apply(&mut o, &doc.document, true, &c, &mut r);
        apply(&mut o, &doc.document, false, &c, &mut r);
        let before = o.generation();
        assert_eq!(
            o.commit_source_serialization(p, &c, &facts(&c, &doc.document), &mut r),
            Err(SourceSerializationErrorV1::Stale)
        );
        assert_eq!(o.generation(), before);
        assert_eq!(
            o.source_serialization(&doc.document, &mut r)
                .unwrap()
                .generation,
            3
        );
    }
}
#[test]
fn retained_serialization_jobs_require_fresh_facts_replay_historically_and_do_not_rearm_after_stop()
{
    for stop in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let doc = o.create(b"A", &mut r).unwrap();
        let reads = reads();
        let request = SourceSerializationRequestV1 {
            document: &doc.document,
            expected: 1,
            serialization: intent(true),
            dependencies: &reads,
        };
        let f = facts(&c, &doc.document);
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_source_serialization(&job, &request, &c, &f, &mut r)
            .unwrap();
        assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
        if stop {
            o.stop_invocation(&job, &mut r).unwrap();
        } else {
            o.run_source_serialization(&job, &c, &f, &mut r).unwrap();
        }
        let before = o.generation();
        assert_eq!(
            o.submit_source_serialization(&job, &request, &c, &f, &mut r),
            Ok(InvocationState::Finished)
        );
        o.run_source_serialization(&job, &c, &f, &mut r).unwrap();
        assert_eq!(o.generation(), before);
        o.forget_invocation_outcome(&job, &mut r).unwrap();
        o.run_source_serialization(&job, &c, &f, &mut r).unwrap();
        assert_eq!(
            o.source_serialization(&doc.document, &mut r)
                .unwrap()
                .generation,
            if stop { 1 } else { 2 }
        );
        o.retire_invocation(&job, &mut r).unwrap();
        assert_eq!(
            o.submit_source_serialization(&job, &request, &c, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
    }
}
#[test]
fn serialization_authority_does_not_relax_source_read_only_or_native_save_permissions() {
    let (mut o, mut r) = ready();
    let c = context();
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let doc = o
        .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    o.set_read_only(&doc.document, true, &mut r).unwrap();
    assert!(apply(&mut o, &doc.document, true, &c, &mut r).changed);
    assert_eq!(
        o.queue_pinned_save(&doc, requirements(), &mut r),
        Err(ResourceError::Permission)
    );
    assert!(
        o.prepare(
            &[DocumentEdits {
                expected: doc.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into()
                }]
            }],
            &[],
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.read(&doc.document, &mut r).unwrap().source.text(), "A");
}
#[test]
fn serialization_signals_are_bounded_optional_and_do_not_change_semantics() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let mut o = owner();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        o.configure_source_serialization("engine", "local", limits(), &mut r)
            .unwrap();
        let doc = o.create(b"A", &mut r).unwrap();
        assert!(apply(&mut o, &doc.document, true, &c, &mut r).changed);
        assert!(!apply(&mut o, &doc.document, true, &c, &mut r).changed);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SerializationConfiguration,
                Boundary::SerializationQuery,
                Boundary::SerializationPreparation,
                Boundary::SerializationValidation,
                Boundary::SerializationPublication,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary));
            }
        }
        if capacity.is_none() {
            assert!(events.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        assert_eq!(o.read(&doc.document, &mut r).unwrap().version, doc);
    }
}
