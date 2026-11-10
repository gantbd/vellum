use super::*;
#[test]
fn retained_release_invocations_keep_every_binding_and_provenance_interruption_unknown() {
    for kind in 0..3 {
        for cut in 0..if kind == 2 { 3 } else { 4 } {
            for mode in 0..4 {
                let (mut o, d, mut setup) = if kind == 0 {
                    let (mut o, port, _, mut r) = super::super::read::tests::fixture();
                    let source = o
                        .load_resource(
                            port.read_result(b"source", b"v0", b"source").unwrap(),
                            &mut r,
                        )
                        .unwrap();
                    let p = o.prepare_retained_disposal(&source, 0, &mut r).unwrap();
                    o.commit_retained_disposal(p, &mut r).unwrap();
                    (o, source.document, r)
                } else {
                    disposed()
                };
                if kind == 2 {
                    let expected = o.retained_document(&d).unwrap().lifecycle;
                    o.release_disposed_annotation_binding(&d, expected, &mut setup)
                        .unwrap();
                }
                o.invocations = SourceCoordinator::with_invocation_limits(
                    o.limits,
                    InvocationLimits {
                        max_lifetime_issued: 4,
                        max_retained: 4,
                        max_intent_bytes: 8192,
                    },
                )
                .invocations;
                o.configure_retention_budget(8192, &mut setup).unwrap();
                let context = SettingsExecutionContextV1 {
                    authority: "private authority".into(),
                    engine: "e".into(),
                    recipient: "r".into(),
                    purpose: "p".into(),
                    destination: "d".into(),
                    profile: "native".into(),
                    continuity: "c".into(),
                };
                let facts = TrustedRetainedReleaseFactsV1 {
                    context: &context,
                    authorized: Some(true),
                    scope: Some(true),
                    continuous: Some(true),
                    live: Some(true),
                    release_allowed: Some(true),
                    resources_ready: Some(true),
                    preparation_reserved: Some(true),
                };
                let request = RetainedReleaseRequestV1 {
                    document: d.clone(),
                    expected: o.retained_document(&d).unwrap().lifecycle,
                    operation: match kind {
                        0 => RetainedReleaseOperationV1::SourceBinding,
                        1 => RetainedReleaseOperationV1::AnnotationBinding,
                        _ => RetainedReleaseOperationV1::Provenance,
                    },
                };
                let id = o.issue_invocation(&mut setup).unwrap();
                o.submit_retained_release_v1(
                    &id,
                    &request,
                    &RetainedReleaseAdmissionV1 {
                        facts: &facts,
                        limits: RetainedReleaseLimitsV1 {
                            max_capture_bytes: 2048,
                            max_result_bytes: 512,
                        },
                        publication_epoch: 7,
                        publication_allowed: Some(true),
                        result_admission_allowed: Some(true),
                        result_allowed: Some(true),
                    },
                    &mut setup,
                )
                .unwrap();
                let unrelated = o.retention.reserve(71).unwrap();
                let before = o.retention_usage();
                let resources = o.resource_usage();
                o.publication_fault_after = Some(cut);
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(
                        crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                    );
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Control,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || o.run_retained_release_v1(&id, &facts, &mut r)
                    ))
                    .is_err()
                );
                assert!(o.faulted);
                assert_eq!(
                    o.invocation_state(&id, &mut setup),
                    Err(InvocationError::Faulted)
                );
                assert_eq!(
                    o.retained_release_result_v1(
                        &id,
                        &TrustedRetainedReleaseResultV1 {
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
                        &mut setup
                    ),
                    Err(InvocationError::Faulted)
                );
                assert_eq!(o.retention_usage(), before);
                assert_eq!(o.resource_usage().retained, resources.retained);
                assert_eq!(o.resource_usage().outstanding, resources.outstanding);
                o.retire_retained_release_v1(
                    &id,
                    &InvocationObservationPolicyControlV1 {
                        context: &context,
                        owner_control_allowed: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    },
                    &mut setup,
                )
                .unwrap();
                assert_eq!(o.retention_usage().reserved, 71);
                assert!(o.retention.validate_exact(&unrelated, 71).is_ok());
                assert_eq!(
                    o.invocation_state(&id, &mut setup),
                    Err(InvocationError::Faulted)
                );
                assert_eq!(o.invocation_usage().issued_lifetime, 1);
                if mode == 1 || mode == 2 {
                    let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
                        .filter(|e| {
                            matches!(
                                e.boundary,
                                Boundary::RetainedReleaseService
                                    | Boundary::DisposedBindingRelease
                                    | Boundary::DisposedProvenanceRelease
                            )
                        })
                        .collect();
                    assert_eq!(events.len(), 2);
                    for e in events {
                        assert_eq!(e.disposition, Disposition::Unknown);
                        assert_eq!(e.service_time.is_some(), mode == 2);
                    }
                }
            }
        }
    }
}
#[test]
fn interrupted_binding_transfer_and_provenance_publication_are_fenced_without_false_completion() {
    use crate::instrumentation::NativeTimingProfileV1;
    for kind in 0..3 {
        for point in 0..=if kind == 2 { 2 } else { 3 } {
            for mode in 0..4 {
                let (mut o, d, mut setup) = if kind == 0 {
                    let (mut o, port, _, mut r) = super::super::read::tests::fixture();
                    let source = o
                        .load_resource(
                            port.read_result(b"source", b"v0", b"source").unwrap(),
                            &mut r,
                        )
                        .unwrap();
                    let plan = o.prepare_retained_disposal(&source, 0, &mut r).unwrap();
                    o.commit_retained_disposal(plan, &mut r).unwrap();
                    (o, source.document, r)
                } else {
                    disposed()
                };
                if kind == 2 {
                    let expected = o.retained_document(&d).unwrap().lifecycle;
                    o.release_disposed_annotation_binding(&d, expected, &mut setup)
                        .unwrap();
                    let expected = o.retained_document(&d).unwrap().lifecycle;
                    o.release_disposed_source_binding(&d, expected, &mut setup)
                        .unwrap();
                }
                let expected = o.retained_document(&d).unwrap().lifecycle;
                let before = o.generation();
                let usage = o.resource_usage();
                let held = o.retention_usage();
                o.publication_fault_after = Some(point);
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Control,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                let boundary = if kind == 2 {
                    Boundary::DisposedProvenanceRelease
                } else {
                    Boundary::DisposedBindingRelease
                };
                let attempt =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match kind {
                        0 => o.release_disposed_source_binding(&d, expected, &mut r),
                        1 => o.release_disposed_annotation_binding(&d, expected, &mut r),
                        _ => o.release_disposed_provenance(&d, expected, &mut r),
                    }));
                assert!(attempt.is_err());
                assert!(o.is_faulted());
                assert_eq!(
                    o.generation(),
                    before + u64::from(point == if kind == 2 { 2 } else { 3 })
                );
                let actual = o.retained_document(&d).unwrap().lifecycle;
                if kind == 2 {
                    assert_eq!(
                        actual.phase,
                        if point == 0 {
                            LifecyclePhase::DisposedRetained
                        } else {
                            LifecyclePhase::ProvenanceReleased
                        }
                    );
                } else {
                    assert_eq!(
                        actual.generation,
                        expected.generation + u64::from(point >= 2)
                    );
                    assert_eq!(
                        actual.pair_generation,
                        expected.pair_generation + u64::from(point >= 2)
                    );
                    let bindings = if kind == 0 {
                        o.resources
                            .bindings
                            .iter()
                            .filter(|b| b.document == d)
                            .count()
                    } else {
                        o.resources
                            .annotation_bindings
                            .iter()
                            .filter(|b| b.document == d)
                            .count()
                    };
                    assert_eq!(bindings, usize::from(point == 0));
                }
                let after = o.resource_usage();
                assert_eq!(
                    (
                        after.retained,
                        after.queued,
                        after.outstanding,
                        after.unresolved,
                        after.contested_tasks
                    ),
                    (
                        usage.retained,
                        usage.queued,
                        usage.outstanding,
                        usage.unresolved,
                        usage.contested_tasks
                    )
                );
                assert_eq!(o.retention_usage(), held);
                assert_eq!(
                    match kind {
                        0 => o.release_disposed_source_binding(&d, expected, &mut setup),
                        1 => o.release_disposed_annotation_binding(&d, expected, &mut setup),
                        _ => o.release_disposed_provenance(&d, expected, &mut setup),
                    },
                    Err(ResourceError::Faulted)
                );
                assert_eq!(
                    o.create(b"other", &mut setup),
                    Err(CoordinatorError::Faulted)
                );
                let mut count = 0;
                while let Some(e) = r.pop_front() {
                    if e.boundary == boundary {
                        count += 1;
                        assert_eq!(e.disposition, Disposition::Unknown);
                        assert_eq!(e.service_time.is_some(), mode == 2);
                    }
                    assert!(!format!("{e:?}").contains("private"));
                }
                assert_eq!(count, usize::from(mode == 1 || mode == 2));
                if mode == 3 {
                    assert!(r.dropped() > 0);
                }
            }
        }
    }
}
#[test]
fn lifecycle_resource_integrity_faults_are_unknown_timed_and_do_not_publish() {
    use crate::instrumentation::NativeTimingProfileV1;
    for point in 0..4 {
        let (mut owner, _, _, mut setup) = super::super::read::tests::fixture();
        let document = owner.resources.annotation_bindings[0].document.clone();
        let source = DocumentVersion {
            document: document.clone(),
            revision: 0,
        };
        let plan = owner
            .prepare_retained_disposal(&source, 0, &mut setup)
            .unwrap();
        let mut plan = Some(plan);
        if point >= 2 {
            owner
                .commit_retained_disposal(plan.take().unwrap(), &mut setup)
                .unwrap();
        }
        if point == 3 {
            let current = owner.retained_document(&document).unwrap().lifecycle;
            owner
                .release_disposed_annotation_binding(&document, current, &mut setup)
                .unwrap();
            let current = owner.retained_document(&document).unwrap().lifecycle;
            owner
                .release_disposed_source_binding(&document, current, &mut setup)
                .unwrap();
            let state = &mut owner.documents[document.slot].as_mut().unwrap().lifecycle;
            state.phase = LifecyclePhase::ProvenanceReleased;
            state.generation = 0;
        } else {
            let key = owner.resources.annotation_bindings[0].key.clone();
            owner.resources.annotation_bindings.push(AnnotationBinding {
                document: document.clone(),
                key,
                baseline: None,
            });
        }
        let expected = owner.retained_document(&document).unwrap().lifecycle;
        let generation = owner.generation();
        let usage = owner.resource_usage();
        let mut r = Recorder::try_with_capacity(32)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let boundary = match point {
            0 => {
                assert_eq!(
                    owner.prepare_retained_disposal(&source, 0, &mut r).err(),
                    Some(CoordinatorError::Resource(ResourceError::Faulted))
                );
                Boundary::DisposalPreparation
            }
            1 => {
                assert_eq!(
                    owner.commit_retained_disposal(plan.take().unwrap(), &mut r),
                    Err(CoordinatorError::Resource(ResourceError::Faulted))
                );
                assert!(owner.is_faulted());
                Boundary::RetainedDisposal
            }
            2 => {
                assert_eq!(
                    owner.release_disposed_annotation_binding(&document, expected, &mut r),
                    Err(ResourceError::Faulted)
                );
                assert!(owner.is_faulted());
                Boundary::DisposedBindingRelease
            }
            _ => {
                assert_eq!(
                    owner.release_disposed_provenance(&document, expected, &mut r),
                    Err(ResourceError::Faulted)
                );
                assert!(owner.is_faulted());
                Boundary::DisposedProvenanceRelease
            }
        };
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.resource_usage(), usage);
        assert_eq!(
            owner.retained_document(&document).unwrap().lifecycle,
            expected
        );
        let event = r.pop_front().unwrap();
        assert_eq!(event.boundary, boundary);
        assert_eq!(event.disposition, Disposition::Unknown);
        assert!(event.service_time.is_some());
        assert!(r.pop_front().is_none());
    }
}
#[test]
fn provenance_release_counter_refusal_is_atomic_and_current_repetition_spends_no_counter() {
    for lifecycle_counter in [false, true] {
        let (mut owner, document, mut recorder) = disposed();
        let state = owner.retained_document(&document).unwrap().lifecycle;
        owner
            .release_disposed_annotation_binding(&document, state, &mut recorder)
            .unwrap();
        if lifecycle_counter {
            owner.documents[document.slot]
                .as_mut()
                .unwrap()
                .lifecycle
                .generation = u64::MAX;
        } else {
            owner.generation = u64::MAX;
        }
        let state = owner.retained_document(&document).unwrap().lifecycle;
        let before = owner.resource_usage();
        let generation = owner.generation;
        assert_eq!(
            owner.release_disposed_provenance(&document, state, &mut recorder),
            Err(ResourceError::VersionExhausted)
        );
        assert_eq!(owner.resource_usage(), before);
        assert_eq!(owner.generation, generation);
        assert_eq!(owner.retained_document(&document).unwrap().lifecycle, state);
        owner.generation = u64::MAX - 1;
        owner.documents[document.slot]
            .as_mut()
            .unwrap()
            .lifecycle
            .generation = u64::MAX - 1;
        let state = owner.retained_document(&document).unwrap().lifecycle;
        assert!(
            owner
                .release_disposed_provenance(&document, state, &mut recorder)
                .unwrap()
        );
        let state = owner.retained_document(&document).unwrap().lifecycle;
        assert!(
            !owner
                .release_disposed_provenance(&document, state, &mut recorder)
                .unwrap()
        );
        assert_eq!(owner.generation, u64::MAX);
        assert_eq!(state.generation, u64::MAX);
    }
}
fn disposed() -> (SourceCoordinator, DocumentId, Recorder) {
    let (mut owner, _, _, mut recorder) = super::super::read::tests::fixture();
    let document = owner.resources.annotation_bindings[0].document.clone();
    let source = DocumentVersion {
        document: document.clone(),
        revision: 0,
    };
    let plan = owner
        .prepare_retained_disposal(&source, 0, &mut recorder)
        .unwrap();
    owner.commit_retained_disposal(plan, &mut recorder).unwrap();
    (owner, document, recorder)
}
#[test]
fn binding_release_counter_exhaustion_preserves_owner_archive_and_lifecycle() {
    for counter in 0..3 {
        let (mut owner, document, mut recorder) = disposed();
        match counter {
            0 => {
                owner.documents[document.slot]
                    .as_mut()
                    .unwrap()
                    .lifecycle
                    .generation = u64::MAX
            }
            1 => {
                owner.documents[document.slot]
                    .as_mut()
                    .unwrap()
                    .lifecycle
                    .pair_generation = u64::MAX
            }
            _ => owner.generation = u64::MAX,
        }
        let expected = owner.retained_document(&document).unwrap().lifecycle;
        let usage = owner.resource_usage();
        let generation = owner.generation;
        assert_eq!(
            owner.release_disposed_annotation_binding(&document, expected, &mut recorder),
            Err(ResourceError::VersionExhausted)
        );
        assert_eq!(owner.resource_usage(), usage);
        assert_eq!(owner.generation, generation);
        assert_eq!(
            owner.retained_document(&document).unwrap().lifecycle,
            expected
        );
        assert!(
            owner
                .retained_document(&document)
                .unwrap()
                .retained_bindings
                .annotations
                .is_none()
        );
    }
}
#[test]
fn duplicate_owner_inventory_is_fenced_not_repaired_by_key_release() {
    let (mut owner, document, mut recorder) = disposed();
    let key = owner.resources.annotation_bindings[0].key.clone();
    owner.resources.annotation_bindings.push(AnnotationBinding {
        document: document.clone(),
        key,
        baseline: None,
    });
    let expected = owner.retained_document(&document).unwrap().lifecycle;
    let usage = owner.resource_usage();
    assert_eq!(
        owner.release_disposed_annotation_binding(&document, expected, &mut recorder),
        Err(ResourceError::Faulted)
    );
    assert!(owner.faulted);
    assert_eq!(owner.resource_usage(), usage);
    assert_eq!(
        owner.retained_document(&document).unwrap().lifecycle,
        expected
    );
}
