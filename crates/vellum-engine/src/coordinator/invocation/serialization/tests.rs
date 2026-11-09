use super::*;
use crate::coordinator::serialization::tests::*;
#[test]
fn shutdown_preserves_original_serialization_replay_and_protected_stop_for_both_forms() {
    for scoped in [false, true] {
        let (mut o, mut r, d, b) = scoped_fixture_with_budget(1048576);
        let c = context();
        let f = facts(&c, &d.document);
        let reads = reads();
        let raw = request(&d.document, &reads);
        let ticket = o.issue_invocation(&mut r).unwrap();
        let submit = |o: &mut SourceCoordinator, r: &mut Recorder| {
            if scoped {
                o.submit_scoped_source_serialization(
                    &ticket,
                    &ScopedSourceSerializationRequestV1 {
                        basis: &b,
                        serialization: raw.serialization,
                        dependencies: &reads,
                    },
                    &c,
                    &gates(&f),
                    r,
                )
            } else {
                o.submit_source_serialization(&ticket, &raw, &c, &f, r)
            }
        };
        assert_eq!(submit(&mut o, &mut r), Ok(InvocationState::Pending));
        crate::coordinator::shutdown::tests::install(&mut o, &mut r);
        let generation = o.generation();
        let funding = o.retention_usage();
        let bases = o.serialization_basis_usage();
        assert_eq!(submit(&mut o, &mut r), Ok(InvocationState::Pending));
        o.request_invocation_stop(
            &ticket,
            &TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
        let settled = if scoped {
            o.run_scoped_source_serialization(&ticket, &c, &gates(&f), &mut r)
        } else {
            o.run_source_serialization(&ticket, &c, &f, &mut r)
        };
        assert_eq!(settled, Ok(InvocationState::Finished));
        assert!(matches!(
            o.invocation_outcome(&ticket, &mut r),
            Ok(InvocationOutcome::StoppedBeforePublication)
        ));
        assert_eq!(submit(&mut o, &mut r), Ok(InvocationState::Finished));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), funding);
        assert_eq!(o.serialization_basis_usage(), bases);
        assert!(!o.is_faulted());
    }
}
#[test]
fn both_serialization_forms_settle_accepted_stop_without_metadata_publication_or_basis_retirement()
{
    for scoped in [false, true] {
        let (mut o, _, d, b) = scoped_fixture();
        let mut r = timed_service_recorder();
        let c = context();
        let f = facts(&c, &d.document);
        let reads = reads();
        let raw = request(&d.document, &reads);
        let ticket = o.issue_invocation(&mut r).unwrap();
        if scoped {
            o.submit_scoped_source_serialization(
                &ticket,
                &ScopedSourceSerializationRequestV1 {
                    basis: &b,
                    serialization: raw.serialization,
                    dependencies: &reads,
                },
                &c,
                &gates(&f),
                &mut r,
            )
            .unwrap();
        } else {
            o.submit_source_serialization(&ticket, &raw, &c, &f, &mut r)
                .unwrap();
        }
        let held = o.serialization_basis_usage();
        let generation = o.generation();
        o.request_invocation_stop(
            &ticket,
            &TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.serialization_basis_usage(), held);
        let mut denied = facts(&c, &d.document);
        denied.authorized = Some(false);
        denied.write_allowed = None;
        if scoped {
            o.run_scoped_source_serialization(&ticket, &c, &gates(&denied), &mut r)
                .unwrap();
        } else {
            o.run_source_serialization(&ticket, &c, &denied, &mut r)
                .unwrap();
        }
        assert_eq!(o.generation(), generation);
        assert_service_markers(
            &mut r,
            &[(Boundary::InvocationExecution, Disposition::Completed)],
        );
        assert_eq!(o.serialization_basis_usage(), held);
        assert!(matches!(
            o.invocation_outcome(&ticket, &mut r),
            Ok(InvocationOutcome::StoppedBeforePublication)
        ));
        assert_eq!(
            o.inspect_serialization_basis(&b, &c, &gates(&f), &mut r),
            SerializationBasisAvailabilityV1::Current
        );
    }
}
fn scoped_fixture() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SerializationBasisId,
) {
    scoped_fixture_with_budget(16384)
}
fn scoped_fixture_with_budget(
    budget: usize,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SerializationBasisId,
) {
    let (mut o, mut r, d) = setup();
    let c = context();
    let f = facts(&c, &d.document);
    o.configure_retention_budget(budget, &mut r).unwrap();
    o.configure_serialization_bases(
        SerializationBasisLimitsV1 {
            serialization: limits(),
            max_contexts: 4,
            max_issued_contexts: 8,
            max_context_bytes: 1024,
            max_held_bytes: 16384,
        },
        &pool_control(),
        &mut r,
    )
    .unwrap();
    let b = o
        .capture_serialization_basis(&d.document, &c, &gates(&f), &mut r)
        .unwrap();
    (o, r, d, b)
}
fn pool_control() -> SerializationPoolControlV1 {
    SerializationPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn gates<'a>(f: &'a TrustedSerializationFactsV1<'a>) -> TrustedSerializationBasisFactsV1<'a> {
    TrustedSerializationBasisFactsV1 {
        native: f,
        owner_control_allowed: Some(true),
        basis_allowed: Some(true),
        host_basis_allowed: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn invalid_configuration_read() -> ReadDependencyV3 {
    let (o, _, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    ReadDependencyV3::ConfigurationState {
        registry: o.settings_registry(&mut r).unwrap(),
        expected: 0,
        interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
    }
}
#[test]
fn changed_serialization_mandatory_count_refuses_before_copy_without_skipping_equal_count_validation()
 {
    for scoped in [false, true] {
        for same_count in [false, true] {
            let (mut o, mut r, d, b) = scoped_fixture();
            let c = context();
            let mut f = facts(&c, &d.document);
            let reads = reads();
            let valid = [ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(d.clone()),
            ))];
            if same_count {
                f.required_reads = &valid;
            }
            let raw = request(&d.document, &reads);
            let scoped_request = ScopedSourceSerializationRequestV1 {
                basis: &b,
                serialization: raw.serialization,
                dependencies: &reads,
            };
            let p = if scoped {
                o.prepare_scoped_source_serialization(&scoped_request, &c, &gates(&f), &mut r)
            } else {
                o.prepare_source_serialization(&raw, &c, &f, &mut r)
            }
            .unwrap();
            let invalid = [invalid_configuration_read()];
            f.required_reads = &invalid;
            let before = o.generation();
            let result = if scoped {
                o.commit_scoped_source_serialization(p, &c, &gates(&f), &mut r)
            } else {
                o.commit_source_serialization(p, &c, &f, &mut r)
            };
            let expected = if same_count {
                SourceSerializationErrorV1::Dependency(CoordinatorError::ReadDependency(
                    ReadDependencyError::InvalidVersion,
                ))
            } else {
                SourceSerializationErrorV1::Stale
            };
            assert_eq!(result, Err(expected));
            assert_eq!(o.generation(), before);
            assert!(!o.is_faulted());
            assert_eq!(
                o.inspect_serialization_basis(&b, &c, &gates(&f), &mut r),
                SerializationBasisAvailabilityV1::Current
            );
        }
    }
}
#[test]
fn changed_serialization_retry_read_count_is_intent_mismatch_without_reinterpreting_new_descriptors()
 {
    for scoped in [false, true] {
        let (mut o, mut r, d, b) = scoped_fixture();
        let c = context();
        let f = facts(&c, &d.document);
        let reads = reads();
        let raw = request(&d.document, &reads);
        let scoped_request = ScopedSourceSerializationRequestV1 {
            basis: &b,
            serialization: raw.serialization,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        if scoped {
            o.submit_scoped_source_serialization(&job, &scoped_request, &c, &gates(&f), &mut r)
        } else {
            o.submit_source_serialization(&job, &raw, &c, &f, &mut r)
        }
        .unwrap();
        let mut changed = super::super::super::serialization::tests::reads();
        changed.predicates.push(invalid_configuration_read());
        let raw_retry = SourceSerializationRequestV1 {
            dependencies: &changed,
            ..raw
        };
        let scoped_retry = ScopedSourceSerializationRequestV1 {
            dependencies: &changed,
            ..scoped_request
        };
        let result = if scoped {
            o.submit_scoped_source_serialization(&job, &scoped_retry, &c, &gates(&f), &mut r)
        } else {
            o.submit_source_serialization(&job, &raw_retry, &c, &f, &mut r)
        };
        assert_eq!(result, Err(InvocationError::IntentMismatch));
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Ok(InvocationState::Pending)
        );
        if scoped {
            o.run_scoped_source_serialization(&job, &c, &gates(&f), &mut r)
        } else {
            o.run_source_serialization(&job, &c, &f, &mut r)
        }
        .unwrap();
        assert_eq!(
            o.source_serialization(&d.document, &mut r)
                .unwrap()
                .generation,
            2
        );
    }
}
#[test]
fn scoped_complete_parent_bound_precedes_loss_and_exact_ceiling_serves_same_original_basis() {
    let (mut o, mut r, d, b) = scoped_fixture();
    let c = context();
    let f = facts(&c, &d.document);
    let mut g = gates(&f);
    let reads = reads();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: request(&d.document, &reads).serialization,
        dependencies: &reads,
    };
    let p = o
        .prepare_scoped_source_serialization(&req, &c, &g, &mut r)
        .unwrap();
    let bytes = p.retained_bytes().unwrap();
    drop(p);
    let job = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    g.basis_allowed = Some(false);
    assert!(matches!(
        o.prepare_scoped_source_serialization(&req, &c, &g, &mut r),
        Err(SourceSerializationErrorV1::Limit)
    ));
    assert_eq!(
        o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r),
        Err(InvocationError::Limit)
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
    o.invocations.limits.max_intent_bytes = bytes;
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    o.run_scoped_source_serialization(&job, &c, &g, &mut r)
        .unwrap();
}
#[test]
fn scoped_noop_at_exhausted_counters_needs_no_new_version_or_publication() {
    let (mut o, mut r, d, _) = scoped_fixture();
    let c = context();
    let f = facts(&c, &d.document);
    let g = gates(&f);
    let reads = reads();
    o.generation = u64::MAX;
    o.documents[d.document.slot]
        .as_mut()
        .unwrap()
        .serialization
        .generation = 9_007_199_254_740_991;
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let mut req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: SourceSerializationV1 {
            profile: SourceSerializationProfileV1::StrictUtf8OneBom,
            bom: false,
        },
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    assert_eq!(
        o.run_scoped_source_serialization(&job, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation, u64::MAX);
    req.serialization.bom = true;
    assert!(matches!(
        o.prepare_scoped_source_serialization(&req, &c, &g, &mut r),
        Err(SourceSerializationErrorV1::VersionExhausted)
    ));
    assert_eq!(
        o.inspect_serialization_basis(&b, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Current
    );
    assert!(!o.is_faulted());
}
#[test]
fn scoped_valid_plan_substitution_with_equal_conditions_faults_and_preserves_protected_discharge() {
    let (mut o, mut r, d, a) = scoped_fixture();
    let c = context();
    let f = facts(&c, &d.document);
    let g = gates(&f);
    let reads = reads();
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &a,
        serialization: request(&d.document, &reads).serialization,
        dependencies: &reads,
    };
    let other = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: req.serialization,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    let p = o
        .prepare_scoped_source_serialization(&other, &c, &g, &mut r)
        .unwrap();
    o.invocations.slots[job.slot].as_mut().unwrap().phase = Phase::SerializationPending(p);
    let publication = o.generation;
    assert_eq!(
        o.run_scoped_source_serialization(&job, &c, &g, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(o.generation, publication);
    for b in [&a, &b] {
        assert!(o.retire_serialization_basis(b, &c, &release(&c), &mut r));
    }
    o.retire_serialization_bases(&pool_control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.is_faulted());
}
#[test]
fn scoped_interrupted_actual_install_is_unknown_not_a_rejection_and_does_not_block_cleanup() {
    let (mut o, mut r, d, b) = scoped_fixture();
    let c = context();
    let f = facts(&c, &d.document);
    let g = gates(&f);
    let reads = reads();
    let req = ScopedSourceSerializationRequestV1 {
        basis: &b,
        serialization: request(&d.document, &reads).serialization,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source_serialization(&job, &req, &c, &g, &mut r)
        .unwrap();
    let publication = o.generation;
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_scoped_source_serialization(&job, &c, &g, &mut r)
        ))
        .is_err()
    );
    assert_eq!(o.generation, publication + 1);
    assert!(o.is_faulted());
    assert!(
        o.documents[d.document.slot]
            .as_ref()
            .unwrap()
            .serialization
            .serialization
            .bom
    );
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(
        o.invocation_outcome(&job, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.retire_serialization_basis(&b, &c, &release(&c), &mut r));
    o.retire_serialization_bases(&pool_control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn whole_parent_wrapper_refuses_before_binding_and_exact_ceiling_succeeds() {
    let (mut o, mut r, d) = setup();
    let c = context();
    let reads = reads();
    let req = request(&d.document, &reads);
    let f = facts(&c, &d.document);
    let p = o
        .prepare_source_serialization(&req, &c, &f, &mut r)
        .unwrap();
    let full = p.retained_bytes().unwrap();
    drop(p);
    let job = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = full - 1;
    assert!(matches!(
        o.prepare_source_serialization(&req, &c, &f, &mut r),
        Err(SourceSerializationErrorV1::Limit)
    ));
    assert_eq!(
        o.submit_source_serialization(&job, &req, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert!(
        o.invocations.slots[job.slot]
            .as_ref()
            .unwrap()
            .intent
            .is_none()
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    o.invocations.limits.max_intent_bytes = full;
    assert_eq!(
        o.submit_source_serialization(&job, &req, &c, &f, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.run_source_serialization(&job, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
}
#[test]
fn invalid_versions_do_not_bind_and_authority_precedes_metadata_queries() {
    let (mut o, mut r, d) = setup();
    let c = context();
    let reads = reads();
    let mut f = facts(&c, &d.document);
    let job = o.issue_invocation(&mut r).unwrap();
    for version in [0, 9_007_199_254_740_992] {
        let mut req = request(&d.document, &reads);
        req.expected = version;
        assert_eq!(
            o.submit_source_serialization(&job, &req, &c, &f, &mut r),
            Err(InvocationError::MalformedIntent)
        );
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Ok(InvocationState::Issued)
        );
    }
    o.documents[d.document.slot]
        .as_mut()
        .unwrap()
        .serialization
        .generation = 0;
    f.write_allowed = None;
    assert!(matches!(
        o.prepare_source_serialization(&request(&d.document, &reads), &c, &f, &mut r),
        Err(SourceSerializationErrorV1::Unavailable)
    ));
    assert!(!o.is_faulted());
    f.write_allowed = Some(true);
    assert_eq!(
        o.submit_source_serialization(&job, &request(&d.document, &reads), &c, &f, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Issued(_)
    ));
}
#[test]
fn valid_prepared_job_substitution_stays_running_unknown_without_publication() {
    let (mut o, mut r, d) = setup();
    let c = context();
    let reads = reads();
    let req = request(&d.document, &reads);
    let f = facts(&c, &d.document);
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_source_serialization(&job, &req, &c, &f, &mut r)
        .unwrap();
    let other = o
        .prepare_source_serialization(&req, &c, &f, &mut r)
        .unwrap();
    o.invocations.slots[job.slot].as_mut().unwrap().phase = Phase::SerializationPending(other);
    let g = o.generation;
    assert_eq!(
        o.run_source_serialization(&job, &c, &f, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation, g);
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
}
#[test]
fn interruption_after_actual_metadata_install_never_fabricates_receipt_or_retry() {
    let (mut o, mut r, d) = setup();
    let c = context();
    let reads = reads();
    let f = facts(&c, &d.document);
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_source_serialization(&job, &request(&d.document, &reads), &c, &f, &mut r)
        .unwrap();
    let g = o.generation;
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_source_serialization(&job, &c, &f, &mut r)
        ))
        .is_err()
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation, g + 1);
    let state = o.documents[d.document.slot].as_ref().unwrap().serialization;
    assert_eq!(state.generation, 2);
    assert!(state.serialization.bom);
    assert!(matches!(
        o.invocations.slots[job.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(
        o.invocation_outcome(&job, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.run_source_serialization(&job, &c, &f, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.generation, g + 1);
}
