use super::*;
use crate::attachment::{AttachmentTarget, LocationEvidence, Placement};
use crate::instrumentation::NativeTimingProfileV1;
pub(in crate::coordinator::resource) fn ready(
    count: usize,
) -> (
    SourceCoordinator,
    AnnotationReadId,
    AnnotationRestoration,
    Recorder,
) {
    let (mut o, port, id, mut r) = super::super::read::tests::fixture();
    let dispatch = o.submit_annotation_read(&id, None, &mut r).unwrap();
    for e in [
        port.annotation_read_start(&dispatch, b"v0").unwrap(),
        port.annotation_read_end(&dispatch, b"v0", 0, 0).unwrap(),
        port.annotation_read_quiescence(&dispatch).unwrap(),
    ] {
        o.record_annotation_read(e, &mut r).unwrap();
    }
    let capture = o.annotation_read_capture(&id, None, &mut r).unwrap();
    let source = capture.source().clone();
    let snapshot = o.capture_annotations(&source, 0, &mut r).unwrap();
    let encoder = o.register_annotation_encoder(&mut r).unwrap();
    let restorer = o
        .register_annotation_restorer_for_encoding(&encoder.context(), &mut r)
        .unwrap();
    let records = (0..count)
        .map(|_| RestoredAnnotation {
            identity: restorer.identity(b"PRIVATE-ID", 1).unwrap(),
            body: "PRIVATE-BODY".into(),
            content_version: 1,
            target_generation: 1,
            authored: SourceTarget {
                source: source.clone(),
                basis: TargetBasis::Before,
                target: AttachmentTarget::ContentRange { start: 0, end: 1 },
            },
            placement: Placement::Located {
                target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                evidence: LocationEvidence::Affected,
            },
        })
        .collect();
    let package = restorer.seal(capture, vec![snapshot], records).unwrap();
    (o, id, package, r)
}
#[test]
fn native_preparations_do_not_publish_and_empty_restoration_still_consumes_the_read() {
    for count in [0, 1] {
        let (mut o, id, package, mut r) = ready(count);
        let before = o.native_root_identity().unwrap();
        let generation = o.generation;
        let plan = o
            .prepare_annotation_restoration_native(package, None, &mut r)
            .unwrap();
        assert_eq!(o.native_root_identity().unwrap(), before);
        assert!(
            o.resources.reads[id.slot]
                .as_ref()
                .unwrap()
                .restoration
                .is_none()
        );
        let restored = o
            .commit_annotation_restoration_native(plan, &mut r)
            .unwrap();
        assert_eq!(restored.changed, count != 0);
        assert_eq!(restored.annotations.len(), count);
        assert_eq!(o.generation, generation + u64::from(count != 0));
        assert!(
            o.resources.reads[id.slot]
                .as_ref()
                .unwrap()
                .restoration
                .is_some()
        );
        o.attach_annotation_read_baselines(&mut r).unwrap();
        let before = o.native_root_identity().unwrap();
        let generation = o.generation;
        let plan = o
            .prepare_loaded_baseline_native(&id, None, 16384, &mut r)
            .unwrap();
        assert_eq!(o.native_root_identity().unwrap(), before);
        // Cancellation after consumption does not destroy historical provenance.
        o.stop_annotation_read(&id, &mut r).unwrap();
        assert_eq!(o.commit_loaded_baseline_native(plan, &mut r), Ok(true));
        assert_eq!(o.generation, generation + 1);
        assert_eq!(
            o.adopt_loaded_annotation_baseline(&id, None, 16384, &mut r),
            Ok(false)
        );
        assert_eq!(o.generation, generation + 1);
    }
}
#[test]
fn restoration_staging_prepays_its_new_document_handle_vector_at_the_exact_boundary() {
    for fits in [false, true] {
        let (mut o, id, package, mut r) = ready(1);
        let required = package.charge
            + std::mem::size_of::<crate::coordinator::annotation::Record>()
            + std::mem::size_of::<AnnotationFact>()
            + std::mem::size_of::<DocumentId>();
        o.limits.max_staged_bytes = required - usize::from(!fits);
        let result = o.restore_annotations(package, None, &mut r);
        if fits {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(ResourceError::Limit));
        }
        assert_eq!(
            o.resources.reads[id.slot]
                .as_ref()
                .unwrap()
                .restoration
                .is_some(),
            fits
        );
        assert!(!o.faulted);
    }
}
#[test]
fn prepared_restoration_rechecks_stop_root_and_current_funding_without_consuming() {
    for change in ["stop", "root", "funding"] {
        let (mut o, id, package, mut r) = ready(1);
        let plan = o
            .prepare_annotation_restoration_native(package, None, &mut r)
            .unwrap();
        match change {
            "stop" => o.stop_annotation_read(&id, &mut r).unwrap(),
            "root" => {
                o.create(b"unrelated", &mut r).unwrap();
            }
            _ => {
                let bytes = o
                    .semantic_root_capacity_usage(&mut r)
                    .unwrap()
                    .admission_bytes;
                o.configure_semantic_root_budget(bytes, &mut r).unwrap();
            }
        }
        let root = o.native_root_identity().unwrap();
        assert!(
            o.commit_annotation_restoration_native(plan, &mut r)
                .is_err()
        );
        assert_eq!(o.native_root_identity().unwrap(), root);
        assert!(
            o.resources.reads[id.slot]
                .as_ref()
                .unwrap()
                .restoration
                .is_none()
        );
        assert!(!o.faulted);
    }
}
#[test]
fn prepared_adoption_rechecks_current_funding_and_late_conflict_without_partial_installation() {
    for conflict in [false, true] {
        let (mut o, id, package, mut r) = ready(1);
        o.restore_annotations(package, None, &mut r).unwrap();
        o.attach_annotation_read_baselines(&mut r).unwrap();
        let plan = o
            .prepare_loaded_baseline_native(&id, None, 16384, &mut r)
            .unwrap();
        if conflict {
            o.resources.reads[id.slot].as_mut().unwrap().contested = true;
        } else {
            let bytes = o
                .semantic_root_capacity_usage(&mut r)
                .unwrap()
                .admission_bytes;
            o.configure_semantic_root_budget(bytes, &mut r).unwrap();
        }
        let generation = o.generation;
        assert_eq!(
            o.commit_loaded_baseline_native(plan, &mut r),
            Err(if conflict {
                ResourceError::Contested
            } else {
                ResourceError::Limit
            })
        );
        assert_eq!(o.generation, generation);
        assert!(o.resources.annotation_bindings[0].baseline.is_none());
        assert!(
            !o.resources.reads[id.slot]
                .as_ref()
                .unwrap()
                .restoration
                .as_ref()
                .unwrap()
                .outcome
                .loaded_baseline_adopted
        );
        assert!(!o.faulted);
    }
}
#[test]
fn optional_timing_and_saturated_capture_preserve_restoration_and_adoption_results() {
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 128] {
            let (mut o, id, package, _) = ready(1);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let restored = o.restore_annotations(package, None, &mut r).unwrap();
            assert!(restored.changed);
            assert_eq!(restored.annotations.len(), 1);
            o.attach_annotation_read_baselines(&mut r).unwrap();
            assert_eq!(
                o.adopt_loaded_annotation_baseline(&id, None, 16384, &mut r),
                Ok(true)
            );
            assert_eq!(
                o.adopt_loaded_annotation_baseline(&id, None, 16384, &mut r),
                Ok(false)
            );
            assert!(!o.faulted);
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(events.len() <= capacity);
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == 128 {
                for boundary in [
                    Boundary::AnnotationRestorationPublication,
                    Boundary::LoadedBaselineAdoption,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.disposition == Disposition::Completed
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}
#[test]
fn every_restoration_installation_cut_is_unknown_and_keeps_the_fence() {
    for count in [0, 1] {
        for cut in 0..=4 {
            for capacity in [0, 1, 128] {
                let (mut o, id, package, _) = ready(count);
                let generation = o.generation;
                let records = o.annotations.records.len();
                o.publication_fault_after = Some(cut);
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || o.restore_annotations(package, None, &mut r)
                    ))
                    .is_err()
                );
                assert!(o.faulted);
                assert_eq!(
                    o.annotations.records.len(),
                    records + if cut >= 1 { count } else { 0 }
                );
                assert_eq!(o.generation, generation + u64::from(count != 0 && cut >= 3));
                assert_eq!(
                    o.resources.reads[id.slot]
                        .as_ref()
                        .unwrap()
                        .restoration
                        .is_some(),
                    cut >= 4
                );
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(!format!("{events:?}").contains("PRIVATE"));
                if capacity == 128 {
                    assert!(
                        events
                            .iter()
                            .any(|e| e.boundary == Boundary::AnnotationRestorationPublication
                                && e.disposition == Disposition::Unknown
                                && e.service_time.is_some())
                    );
                }
            }
        }
    }
}
#[test]
fn owned_annotation_commit_cuts_withhold_staged_results_and_protected_cleanup_preserves_producer_state()
 {
    for baseline in [false, true] {
        for cut in 0..=if baseline { 4 } else { 5 } {
            let (mut o, read, package, mut r) = ready(1);
            o.invocations = SourceCoordinator::with_invocation_limits(
                o.limits,
                InvocationLimits {
                    max_lifetime_issued: 4,
                    max_retained: 4,
                    max_intent_bytes: 32768,
                },
            )
            .invocations;
            o.configure_retention_budget(1048576, &mut r).unwrap();
            let request = if baseline {
                o.restore_annotations(package, None, &mut r).unwrap();
                o.attach_annotation_read_baselines(&mut r).unwrap();
                AnnotationLoadRequestV1::AdoptBaseline {
                    read: read.clone(),
                    pair: None,
                    max_baseline_bytes: 16384,
                }
            } else {
                AnnotationLoadRequestV1::Restore {
                    package: Arc::new(package),
                    pair: None,
                }
            };
            let context = SettingsExecutionContextV1 {
                authority: "a".into(),
                engine: "e".into(),
                recipient: "r".into(),
                purpose: "load".into(),
                destination: "local".into(),
                profile: "annotation-load/1".into(),
                continuity: "c".into(),
            };
            let facts = TrustedAnnotationLoadFactsV1 {
                context: &context,
                authorized: Some(true),
                scope: Some(true),
                continuous: Some(true),
                live: Some(true),
                not_stopped: Some(true),
                interpretation_current: Some(true),
                resources_ready: Some(true),
                preparation_reserved: Some(true),
                owner_control_allowed: Some(true),
                companions_ready: Some(true),
                reply_ready: Some(true),
                disposition_readable: Some(true),
                annotation_writable: Some(true),
                restoration_allowed: Some(true),
                identity_restorable: Some(true),
                source_evidence_ready: Some(true),
                aliases_current: Some(true),
                target_evidence_ready: Some(true),
                history_restoration_ready: Some(true),
                annotation_readable: Some(true),
                binding_coverage_complete: Some(true),
                writer_coverage_complete: Some(true),
                baseline_publication_allowed: Some(true),
                historical_artifact_readable: Some(true),
                baseline_coverage_complete: Some(true),
            };
            let auth = o
                .borrow_annotation_load_authority_v1(
                    &read,
                    &AnnotationLoadAuthorityFactsV1 {
                        owner_control_allowed: Some(true),
                        preparation_reserved: Some(true),
                        resources_ready: Some(true),
                    },
                    &mut r,
                )
                .unwrap();
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_annotation_load_v1(
                &id,
                &auth,
                &request,
                &AnnotationLoadAdmissionV1 {
                    facts: &facts,
                    limits: AnnotationLoadLimitsV1 {
                        max_capture_bytes: 16384,
                        max_result_bytes: 256,
                    },
                    publication_epoch: 7,
                    publication_allowed: Some(true),
                    result_admission_allowed: Some(true),
                    result_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
            crate::coordinator::shutdown::tests::invocation_funding(&o, &id, 1, 16384, &mut r);
            o.publication_fault_after = Some(cut);
            let mut r = Recorder::try_with_capacity(128)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.run_annotation_load_v1(&id, &facts, &mut r)
                ))
                .is_err()
            );
            assert!(o.faulted);
            assert_eq!(
                o.invocation_outcome(&id, &mut r).err(),
                Some(InvocationError::Faulted)
            );
            assert_eq!(
                o.annotation_load_result_v1(
                    &id,
                    &TrustedAnnotationLoadResultV1 {
                        context: &context,
                        publication_epoch: 7,
                        publication_allowed: Some(true),
                        outcome_readable: Some(true),
                        result_allowed: Some(true),
                        result_context_current: Some(true),
                        historical_document_readable: Some(true),
                        reply_ready: Some(true),
                        resources_ready: Some(true)
                    },
                    &mut r
                ),
                Err(InvocationError::Faulted)
            );
            let producer = o.resource_usage();
            let funding = o.retention_usage();
            o.retire_annotation_load_v1(
                &id,
                &InvocationObservationPolicyControlV1 {
                    context: &context,
                    owner_control_allowed: Some(true),
                    retention_released: Some(true),
                    delivery_drained: Some(true),
                },
                &mut r,
            )
            .unwrap();
            assert_eq!(o.resource_usage(), producer);
            assert!(o.retention_usage().reserved < funding.reserved);
            assert!(o.faulted);
            assert!(
                std::iter::from_fn(|| r.pop_front())
                    .any(|e| e.boundary == Boundary::AnnotationLoadService
                        && e.disposition == Disposition::Unknown
                        && e.service_time.is_some())
            );
        }
    }
}
#[test]
fn every_adoption_installation_cut_is_unknown_and_keeps_the_fence() {
    for cut in 0..=3 {
        for capacity in [0, 1, 128] {
            let (mut o, id, package, mut r) = ready(0);
            o.restore_annotations(package, None, &mut r).unwrap();
            o.attach_annotation_read_baselines(&mut r).unwrap();
            let generation = o.generation;
            o.publication_fault_after = Some(cut);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.adopt_loaded_annotation_baseline(&id, None, 16384, &mut r)
                ))
                .is_err()
            );
            assert!(o.faulted);
            assert_eq!(
                o.resources.annotation_bindings[0].baseline.is_some(),
                cut >= 1
            );
            assert_eq!(
                o.resources.reads[id.slot]
                    .as_ref()
                    .unwrap()
                    .restoration
                    .as_ref()
                    .unwrap()
                    .outcome
                    .loaded_baseline_adopted,
                cut >= 2
            );
            assert_eq!(o.generation, generation + u64::from(cut >= 3));
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == 128 {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::LoadedBaselineAdoption
                            && e.disposition == Disposition::Unknown
                            && e.service_time.is_some())
                );
            }
        }
    }
}
