use vellum_engine::coordinator::*;
use vellum_engine::resource::ResourceError;
#[path = "support/retained_release_fixture.rs"]
mod support;
use support::*;
#[test]
fn source_annotation_and_provenance_releases_retain_original_applied_and_unchanged_results() {
    let mut f = disposed();
    let c = context();
    let current = facts(&c);
    let mut results = Vec::new();
    for operation in [
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::Provenance,
        RetainedReleaseOperationV1::Provenance,
    ] {
        let expected = f
            .owner
            .lifecycle_state(&f.doc.document, &mut f.recorder)
            .unwrap();
        let request = RetainedReleaseRequestV1 {
            document: f.doc.document.clone(),
            expected,
            operation,
        };
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        let generation = f.owner.generation();
        assert_eq!(
            f.owner.submit_retained_release_v1(
                &id,
                &request,
                &admission(&current),
                &mut f.recorder
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(
            f.owner.run(&id, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        let result = f
            .owner
            .retained_release_result_v1(&id, &release(&c), &mut f.recorder)
            .unwrap();
        let changed = results.len() % 2 == 0;
        assert_eq!(result.changed, changed);
        assert_eq!(result.operation, operation);
        assert_eq!(result.document, f.doc.document);
        assert_eq!(f.owner.generation(), generation + u64::from(changed));
        assert_eq!(
            f.owner.submit_retained_release_v1(
                &id,
                &request,
                &admission(&current),
                &mut f.recorder
            ),
            Ok(InvocationState::Finished)
        );
        results.push((id, result));
    }
    for (id, result) in results {
        assert_eq!(
            f.owner
                .retained_release_result_v1(&id, &release(&c), &mut f.recorder)
                .unwrap(),
            result
        );
        assert_eq!(
            f.owner.retire_invocation(&id, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
        f.owner
            .retire_retained_release_v1(
                &id,
                &InvocationObservationPolicyControlV1 {
                    context: &c,
                    owner_control_allowed: Some(true),
                    retention_released: Some(true),
                    delivery_drained: Some(true),
                },
                &mut f.recorder,
            )
            .unwrap();
        f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
    }
    assert_eq!(f.owner.invocation_usage().issued_lifetime, 6);
    assert_eq!(f.owner.retention_usage().reserved, 0);
}

#[test]
fn unknown_write_application_remains_retained_even_after_stop_and_quiescence_and_retry() {
    use vellum_engine::resource::*;
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(131072, &mut f.recorder)
        .unwrap();
    // An ordinary save now retains a baseline-capacity promise and correctly
    // blocks disposal while unresolved. Use an explicitly authorized copy export
    // to the same resource to test the distinct unknown-effect binding hold.
    let serialization = f
        .owner
        .source_serialization(&f.doc.document, &mut f.recorder)
        .unwrap();
    let prepared = f
        .owner
        .prepare_source_artifact(&f.doc, serialization, &mut f.recorder)
        .unwrap();
    let export_facts = TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: Some(true),
        overwrite_allowed: Some(true),
    };
    let save = f
        .owner
        .queue_source_export(
            &prepared,
            f.port
                .read_result(b"source", b"s0", b"text")
                .unwrap()
                .into_source_export_destination(),
            SourceExportOptionsV1 {
                purpose: SourceExportPurposeV1::CopyExport,
                requirements: SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Ordinary,
                },
                pair: None,
            },
            &export_facts,
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f
        .owner
        .dispatch_source_export(
            &save,
            f.port
                .read_result(b"source", b"s0", b"text")
                .unwrap()
                .into_source_export_destination(),
            None,
            &export_facts,
            &mut f.recorder,
        )
        .unwrap();
    let p = f
        .owner
        .prepare_retained_disposal(&f.doc, 0, &mut f.recorder)
        .unwrap();
    f.owner
        .commit_retained_disposal(p, &mut f.recorder)
        .unwrap();
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    for e in [
        f.port.no_future_effects(&dispatch).unwrap(),
        f.port.quiescence(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    let c = context();
    let current = facts(&c);
    let q = request(&mut f, RetainedReleaseOperationV1::SourceBinding);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_retained_release_v1(&id, &q, &admission(&current), &mut f.recorder)
        .unwrap();
    let resources = f.owner.resource_usage();
    let generation = f.owner.generation();
    f.owner
        .run_retained_release_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.invocation_outcome(&id, &mut f.recorder),
        Ok(&InvocationOutcome::RetainedReleaseRejected(
            ResourceError::Retained
        ))
    );
    assert_eq!(
        f.owner
            .retained_release_result_v1(&id, &release(&c), &mut f.recorder),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(f.owner.resource_usage(), resources);
    assert_eq!(f.owner.generation(), generation);
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &q, &admission(&current), &mut f.recorder),
        Ok(InvocationState::Finished)
    );
    f.owner
        .run_retained_release_v1(&id, &current, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.generation(), generation);
    let fresh = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_retained_release_v1(&fresh, &q, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_retained_release_v1(&fresh, &current, &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .retained_release_result_v1(&fresh, &release(&c), &mut f.recorder)
            .unwrap()
            .changed
    );
}
#[test]
fn release_revalidates_targeted_lifecycle_but_not_unrelated_publication() {
    for stale in [false, true] {
        let mut f = disposed();
        let c = context();
        let current = facts(&c);
        let q = request(&mut f, RetainedReleaseOperationV1::SourceBinding);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_retained_release_v1(&id, &q, &admission(&current), &mut f.recorder)
            .unwrap();
        if stale {
            f.owner
                .release_disposed_annotation_binding(&q.document, q.expected, &mut f.recorder)
                .unwrap();
        } else {
            f.owner.create(b"unrelated", &mut f.recorder).unwrap();
        }
        let generation = f.owner.generation();
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        if stale {
            assert_eq!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(&InvocationOutcome::RetainedReleaseRejected(
                    ResourceError::Stale
                ))
            );
            assert_eq!(f.owner.generation(), generation);
        } else {
            assert!(
                f.owner
                    .retained_release_result_v1(&id, &release(&c), &mut f.recorder)
                    .unwrap()
                    .changed
            );
            assert_eq!(f.owner.generation(), generation + 1);
        }
    }
}
#[test]
fn every_admission_and_fresh_service_gate_is_exact_but_protected_stop_needs_no_release_right() {
    for field in 0..7 {
        for value in [None, Some(false)] {
            let mut f = disposed();
            let c = context();
            let mut denied = facts(&c);
            match field {
                0 => denied.authorized = value,
                1 => denied.scope = value,
                2 => denied.continuous = value,
                3 => denied.live = value,
                4 => denied.release_allowed = value,
                5 => denied.resources_ready = value,
                _ => denied.preparation_reserved = value,
            }
            let q = request(&mut f, RetainedReleaseOperationV1::SourceBinding);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            assert_eq!(
                f.owner
                    .submit_retained_release_v1(&id, &q, &admission(&denied), &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(f.owner.retention_usage().reserved, 0);
            f.owner
                .submit_retained_release_v1(&id, &q, &admission(&facts(&c)), &mut f.recorder)
                .unwrap();
            let generation = f.owner.generation();
            assert_eq!(
                f.owner
                    .run_retained_release_v1(&id, &denied, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            f.owner
                .request_invocation_stop(
                    &id,
                    &TrustedInvocationControlV1 {
                        context: &c,
                        owner_control_allowed: Some(true),
                        control_context_current: Some(true),
                        control_scope_valid: Some(true),
                        protected_control_ready: Some(true),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .run_retained_release_v1(&id, &denied, &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner.invocation_outcome(&id, &mut f.recorder),
                Ok(&InvocationOutcome::StoppedBeforePublication)
            );
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(
                f.owner
                    .retained_release_result_v1(&id, &release(&c), &mut f.recorder),
                Err(InvocationError::OutcomeUnavailable)
            );
        }
    }
}
#[test]
fn captured_release_ceilings_and_current_result_checks_never_widen_or_reexecute() {
    for bits in 0..8 {
        let mut f = disposed();
        let c = context();
        let current = facts(&c);
        let mut a = admission(&current);
        a.publication_allowed = Some(bits & 1 != 0);
        a.result_admission_allowed = Some(bits & 2 != 0);
        a.result_allowed = Some(bits & 4 != 0);
        let q = request(&mut f, RetainedReleaseOperationV1::SourceBinding);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        f.owner
            .submit_retained_release_v1(&id, &q, &a, &mut f.recorder)
            .unwrap();
        f.owner
            .run_retained_release_v1(&id, &current, &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner
                .retained_release_result_v1(&id, &release(&c), &mut f.recorder)
                .is_ok(),
            bits == 7
        );
        if bits != 7 {
            assert_eq!(
                f.owner
                    .submit_retained_release_v1(&id, &q, &admission(&current), &mut f.recorder),
                Err(InvocationError::IntentMismatch)
            );
        } else {
            for field in 0..7 {
                for value in [None, Some(false)] {
                    let mut denied = release(&c);
                    match field {
                        0 => denied.publication_allowed = value,
                        1 => denied.outcome_readable = value,
                        2 => denied.result_allowed = value,
                        3 => denied.result_context_current = value,
                        4 => denied.historical_document_readable = value,
                        5 => denied.reply_ready = value,
                        _ => denied.resources_ready = value,
                    }
                    assert_eq!(
                        f.owner
                            .retained_release_result_v1(&id, &denied, &mut f.recorder),
                        Err(InvocationError::Unavailable)
                    );
                }
            }
        }
    }
}
#[test]
fn protected_release_retirement_and_forgetting_do_not_refresh_or_refund_spent_work() {
    let mut f = disposed();
    let c = context();
    let current = facts(&c);
    let q = request(&mut f, RetainedReleaseOperationV1::SourceBinding);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    f.owner
        .submit_retained_release_v1(&id, &q, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_retained_release_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let held = f.owner.retention_usage();
    let generation = f.owner.generation();
    for field in 0..3 {
        for value in [None, Some(false)] {
            let mut control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            match field {
                0 => control.owner_control_allowed = value,
                1 => control.retention_released = value,
                _ => control.delivery_drained = value,
            }
            assert_eq!(
                f.owner
                    .retire_retained_release_v1(&id, &control, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(f.owner.retention_usage(), held);
        }
    }
    f.owner
        .forget_invocation_outcome(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .retained_release_result_v1(&id, &release(&c), &mut f.recorder),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(f.owner.retention_usage(), held);
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &q, &admission(&current), &mut f.recorder),
        Ok(InvocationState::Finished)
    );
    assert_eq!(f.owner.generation(), generation);
    f.owner
        .retire_retained_release_v1(
            &id,
            &InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
    assert_eq!(f.owner.invocation_usage().issued_lifetime, 1);
}

#[test]
fn native_release_admission_checks_complete_bounds_foreign_targets_and_unknown_ceilings_before_charge()
 {
    let mut f = disposed();
    let c = context();
    let current = facts(&c);
    let q = request(&mut f, RetainedReleaseOperationV1::SourceBinding);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let mut a = admission(&current);
    let minimum = RetainedReleaseLimitsV1::required_capture_bytes(&c).unwrap();
    a.limits.max_capture_bytes = minimum - 1;
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &q, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    a = admission(&current);
    a.limits.max_result_bytes = std::mem::size_of::<RetainedReleaseResultV1>() - 1;
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &q, &a, &mut f.recorder),
        Err(InvocationError::Limit)
    );
    for field in 0..3 {
        a = admission(&current);
        match field {
            0 => a.publication_allowed = None,
            1 => a.result_admission_allowed = None,
            _ => a.result_allowed = None,
        }
        assert_eq!(
            f.owner
                .submit_retained_release_v1(&id, &q, &a, &mut f.recorder),
            Err(InvocationError::Unavailable)
        );
    }
    let mut foreign = disposed();
    let foreign_q = request(&mut foreign, RetainedReleaseOperationV1::SourceBinding);
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &foreign_q, &admission(&current), &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(f.owner.retention_usage().reserved, 0);
    assert_eq!(
        f.owner.invocation_state(&id, &mut f.recorder),
        Ok(InvocationState::Issued)
    );
    a = admission(&current);
    a.limits.max_capture_bytes = minimum;
    a.limits.max_result_bytes = std::mem::size_of::<RetainedReleaseResultV1>();
    f.owner
        .submit_retained_release_v1(&id, &q, &a, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, minimum);
    let mut changed = q.clone();
    changed.operation = RetainedReleaseOperationV1::AnnotationBinding;
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &changed, &a, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    let mut epoch = a;
    epoch.publication_epoch += 1;
    assert_eq!(
        f.owner
            .submit_retained_release_v1(&id, &q, &epoch, &mut f.recorder),
        Err(InvocationError::IntentMismatch)
    );
    f.owner
        .run_retained_release_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut old_epoch = release(&c);
    old_epoch.publication_epoch += 1;
    assert_eq!(
        f.owner
            .retained_release_result_v1(&id, &old_epoch, &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    let mut wrong_context = c.clone();
    wrong_context.continuity = "foreign".into();
    assert_eq!(
        f.owner
            .retained_release_result_v1(&id, &release(&wrong_context), &mut f.recorder),
        Err(InvocationError::Unavailable)
    );
    assert!(
        f.owner
            .retained_release_result_v1(&id, &release(&c), &mut f.recorder)
            .is_ok()
    );
}
