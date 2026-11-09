use super::*;
fn pool() -> SerializationBasisLimitsV1 {
    SerializationBasisLimitsV1 {
        serialization: limits(),
        max_contexts: 4,
        max_issued_contexts: 16,
        max_context_bytes: 1024,
        max_held_bytes: 32768,
    }
}
fn pool_control() -> SerializationPoolControlV1 {
    SerializationPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn basis_facts<'a>(
    native: &'a TrustedSerializationFactsV1<'a>,
) -> TrustedSerializationBasisFactsV1<'a> {
    TrustedSerializationBasisFactsV1 {
        native,
        owner_control_allowed: Some(true),
        basis_allowed: Some(true),
        host_basis_allowed: Some(true),
    }
}
fn control(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn ready_scoped() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = ready();
    o.configure_retention_budget(32768, &mut r).unwrap();
    o.configure_serialization_bases(pool(), &pool_control(), &mut r)
        .unwrap();
    (o, r)
}
#[test]
fn scoped_retained_change_commutes_with_source_journals_and_keeps_old_save_capture() {
    let (mut o, mut r) = ready_scoped();
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
    let d = o
        .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    let save = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let f = facts(&c, &d.document);
    let g = basis_facts(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let reads = reads();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: intent(true),
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
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
    let history = o.commit(edit, &mut r).unwrap().history_group.unwrap();
    let before = o.read(&d.document, &mut r).unwrap().version;
    assert_eq!(
        o.run_scoped_source_serialization(&job, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.read(&d.document, &mut r).unwrap().version, before);
    assert_eq!(
        o.source_serialization(&d.document, &mut r)
            .unwrap()
            .generation,
        2
    );
    assert_eq!(
        o.save_outcome(&save, &mut r)
            .unwrap()
            .serialization_generation,
        Some(1)
    );
    let dispatch = o
        .dispatch_save(
            &save,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"A");
    let undo = o
        .prepare_reversal(&history, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "A");
    assert!(
        o.source_serialization(&d.document, &mut r)
            .unwrap()
            .serialization
            .bom
    );
}
#[test]
fn first_party_permission_is_distinct_and_cannot_be_bypassed_through_raw_service_or_commit() {
    let (mut o, mut r) = ready_scoped();
    let c = context();
    let d = o.create(b"A", &mut r).unwrap();
    let f = facts(&c, &d.document);
    let mut g = basis_facts(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let reads = reads();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: intent(true),
        dependencies: &reads,
    };
    for permission in [None, Some(false)] {
        g.host_basis_allowed = permission;
        assert!(matches!(
            o.prepare_scoped_source_serialization(&req, &c, &g, &mut r),
            Err(SourceSerializationErrorV1::Unavailable)
        ));
        assert_eq!(
            o.inspect_serialization_basis(&b, &c, &g, &mut r),
            SerializationBasisAvailabilityV1::Current
        );
    }
    g.host_basis_allowed = Some(true);
    let p = o
        .prepare_scoped_source_serialization(&req, &c, &g, &mut r)
        .unwrap();
    assert_eq!(
        format!("{p:?}"),
        "PreparedSourceSerializationV1(<redacted>)"
    );
    assert_eq!(
        o.commit_source_serialization(p, &c, &f, &mut r),
        Err(SourceSerializationErrorV1::Unavailable)
    );
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
    assert_eq!(
        o.run_source_serialization(&job, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.run_scoped_source_serialization(&job, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
}
#[test]
fn complete_semantic_preflight_precedes_basis_loss_observation_and_job_binding() {
    let (mut o, mut r) = ready_scoped();
    let c = context();
    let d = o.create(b"A", &mut r).unwrap();
    let f = facts(&c, &d.document);
    let mut g = basis_facts(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let mut reads = reads();
    reads
        .predicates
        .push(ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(DocumentVersion {
                document: d.document.clone(),
                revision: 99,
            }),
        )));
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: intent(true),
        dependencies: &reads,
    };
    g.basis_allowed = Some(false);
    let job = o.issue_invocation(&mut r).unwrap();
    assert!(
        o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
            .is_err()
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    g.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_serialization_basis(&b, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Current
    );
    let reads = super::reads();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: intent(true),
        dependencies: &reads,
    };
    g.basis_allowed = Some(false);
    assert!(
        o.prepare_scoped_source_serialization(&req, &c, &g, &mut r)
            .is_err()
    );
    g.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_serialization_basis(&b, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Unavailable
    );
}
#[test]
fn original_basis_identity_and_raw_scoped_exclusivity_survive_terminal_replay_and_release() {
    let (mut o, mut r) = ready_scoped();
    let c = context();
    let d = o.create(b"A", &mut r).unwrap();
    let f = facts(&c, &d.document);
    let g = basis_facts(&f);
    let reads = reads();
    let a = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &a,
        serialization: intent(true),
        dependencies: &reads,
    };
    let other = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: intent(true),
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_scoped_source_serialization(&job, &other, &c, &g, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let raw = SourceSerializationRequestV1 {
        document: &d.document,
        expected: 1,
        serialization: intent(true),
        dependencies: &reads,
    };
    assert_eq!(
        o.submit_source_serialization(&job, &raw, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_scoped_source_serialization(&job, &c, &g, &mut r)
        .unwrap();
    let publication = o.generation();
    assert!(o.retire_serialization_basis(&a, &c, &control(&c), &mut r));
    assert_eq!(
        o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.invocation_outcome(&job, &mut r).unwrap(),
        &InvocationOutcome::SerializationCommitted(SourceSerializationUpdateV1 { changed: true })
    );
    assert_eq!(
        o.run_scoped_source_serialization(&job, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), publication);
}
#[test]
fn fresh_service_permissions_and_original_basis_are_required_even_for_noops() {
    for loss in ["host", "basis", "write", "retire", "stop", "metadata"] {
        let (mut o, mut r) = ready_scoped();
        let c = context();
        let d = o.create(b"A", &mut r).unwrap();
        let mut f = facts(&c, &d.document);
        let g = basis_facts(&f);
        let b = o
            .capture_serialization_basis(&d.document, &c, &g, &mut r)
            .unwrap();
        let reads = reads();
        let req = ScopedSourceSerializationRequestV1 {
            basis: &b,
            serialization: intent(false),
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
            .unwrap();
        if loss == "retire" {
            assert!(o.retire_serialization_basis(&b, &c, &control(&c), &mut r));
        }
        if loss == "stop" {
            assert!(o.stop_serialization_basis(&b, &c, &control(&c), &mut r));
        }
        if loss == "metadata" {
            apply(&mut o, &d.document, true, &c, &mut r);
        }
        if loss == "write" {
            f.write_allowed = None;
        }
        let mut g = basis_facts(&f);
        if loss == "host" {
            g.host_basis_allowed = None;
        }
        if loss == "basis" {
            g.basis_allowed = Some(false);
        }
        let before = o.generation();
        assert_eq!(
            o.run_scoped_source_serialization(&job, &c, &g, &mut r),
            Ok(InvocationState::Finished)
        );
        assert!(matches!(
            o.invocation_outcome(&job, &mut r).unwrap(),
            InvocationOutcome::SerializationRejected(_)
        ));
        assert_eq!(o.generation(), before);
        assert!(!o.is_faulted());
    }
}
#[test]
fn cancellation_or_forgetting_does_not_retire_basis_or_reopen_execution() {
    let (mut o, mut r) = ready_scoped();
    let c = context();
    let d = o.create(b"A", &mut r).unwrap();
    let f = facts(&c, &d.document);
    let g = basis_facts(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let reads = reads();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: intent(true),
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    o.stop_invocation(&job, &mut r).unwrap();
    o.forget_invocation_outcome(&job, &mut r).unwrap();
    assert_eq!(
        o.run_scoped_source_serialization(&job, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.inspect_serialization_basis(&b, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Current
    );
    assert_eq!(o.serialization_basis_usage().retained, 1);
    assert_eq!(
        o.source_serialization(&d.document, &mut r)
            .unwrap()
            .generation,
        1
    );
}
#[test]
fn bounded_optional_basis_and_scoped_signals_preserve_engine_behavior() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        o.configure_retention_budget(32768, &mut r).unwrap();
        o.configure_serialization_bases(pool(), &pool_control(), &mut r)
            .unwrap();
        let c = context();
        let d = o.create(b"A", &mut r).unwrap();
        let f = facts(&c, &d.document);
        let g = basis_facts(&f);
        let b = o
            .capture_serialization_basis(&d.document, &c, &g, &mut r)
            .unwrap();
        let reads = reads();
        assert_eq!(
            o.inspect_serialization_basis(&b, &c, &g, &mut r),
            SerializationBasisAvailabilityV1::Current
        );
        let p = o
            .prepare_scoped_source_serialization(
                &ScopedSourceSerializationRequestV1 {
                    basis: &b,
                    serialization: intent(true),
                    dependencies: &reads,
                },
                &c,
                &g,
                &mut r,
            )
            .unwrap();
        assert!(
            o.commit_scoped_source_serialization(p, &c, &g, &mut r)
                .unwrap()
                .changed
        );
        assert!(o.stop_serialization_basis(&b, &c, &control(&c), &mut r));
        assert!(o.retire_serialization_basis(&b, &c, &control(&c), &mut r));
        o.retire_serialization_bases(&pool_control(), &mut r)
            .unwrap();
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SerializationBasisConfiguration,
                Boundary::SerializationBasisCapture,
                Boundary::SerializationBasisInspection,
                Boundary::SerializationBasisStop,
                Boundary::SerializationBasisRetirement,
                Boundary::SerializationPoolRetirement,
                Boundary::ScopedSerializationPreparation,
                Boundary::ScopedSerializationValidation,
                Boundary::ScopedSerializationPublication,
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
        assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn scoped_service_rechecks_declared_and_mandatory_reads_without_dropping_overlap() {
    for change in ["none", "source", "required"] {
        let (mut o, mut r) = ready_scoped();
        let c = context();
        let d = o.create(b"A", &mut r).unwrap();
        let mut f = facts(&c, &d.document);
        let predicate =
            ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(d.clone())));
        let required = [predicate.clone()];
        f.required_reads = &required;
        let b = o
            .capture_serialization_basis(&d.document, &c, &basis_facts(&f), &mut r)
            .unwrap();
        let mut reads = reads();
        reads.predicates.push(predicate);
        reads.budget.max_predicates = 1;
        let req = ScopedSourceSerializationRequestV1 {
            basis: &b,
            serialization: intent(true),
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_source_serialization(&job, &req, &c, &basis_facts(&f), &mut r)
            .unwrap();
        if change == "source" {
            let p = o
                .prepare(
                    &[DocumentEdits {
                        expected: d.clone(),
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
            o.commit(p, &mut r).unwrap();
        }
        if change == "required" {
            f.required_reads = &[];
        }
        let before = o.generation();
        o.run_scoped_source_serialization(&job, &c, &basis_facts(&f), &mut r)
            .unwrap();
        let committed = matches!(
            o.invocation_outcome(&job, &mut r).unwrap(),
            InvocationOutcome::SerializationCommitted(_)
        );
        assert_eq!(committed, change == "none");
        assert_eq!(o.generation() - before, u64::from(change == "none"));
    }
}
