use super::super::source_export::tests::{rebind_facts, rebind_options};
use super::*;
#[test]
fn owned_rebind_interruption_never_releases_a_staged_result_or_erases_unknown_execution() {
    use crate::instrumentation::NativeTimingProfileV1;
    for cut in 0..=2 {
        for capacity in [0, 1, 128] {
            let (mut o, mut setup, d, port) = super::super::serialization_tests::fixture();
            o.configure_retention_budget(262144, &mut setup).unwrap();
            let context = crate::coordinator::serialization::tests::context();
            let native = rebind_facts();
            let facts = TrustedSourceRebindInvocationFactsV1 {
                context: &context,
                native: &native,
            };
            let q = SourceRebindRequestV1 {
                document: d.document.clone(),
                target: port.source_binding_target(b"PRIVATE-TARGET").unwrap(),
                options: rebind_options(Some(0), 1),
            };
            let job = o.issue_invocation(&mut setup).unwrap();
            let before = o.retention_usage();
            o.submit_source_rebind_v1(
                &job,
                &q,
                &SourceRebindAdmissionV1 {
                    facts: &facts,
                    limits: SourceRebindLimitsV1 {
                        max_capture_bytes: 4096,
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
            let funded = o.retention_usage();
            crate::coordinator::shutdown::tests::invocation_funding(&o, &job, 1, 4096, &mut setup);
            let generation = o.generation();
            o.publication_fault_after = Some(cut);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.run_source_rebind_v1(&job, &facts, &mut r)
                ))
                .is_err()
            );
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation + u64::from(cut == 2));
            assert_eq!(
                o.invocation_state(&job, &mut setup),
                Err(InvocationError::Faulted)
            );
            assert_eq!(o.retention_usage(), funded);
            let release = TrustedSourceRebindResultV1 {
                context: &context,
                publication_epoch: 7,
                publication_allowed: Some(true),
                outcome_readable: Some(true),
                result_allowed: Some(true),
                result_context_current: Some(true),
                historical_document_readable: Some(true),
                reply_ready: Some(true),
                resources_ready: Some(true),
            };
            assert_eq!(
                o.source_rebind_result_v1(&job, &release, &mut setup),
                Err(InvocationError::Faulted)
            );
            assert_eq!(
                o.forget_invocation_outcome(&job, &mut setup),
                Err(InvocationError::Faulted)
            );
            let control = InvocationObservationPolicyControlV1 {
                context: &context,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            o.retire_source_rebind_v1(&job, &control, &mut setup)
                .unwrap();
            assert_eq!(o.retention_usage(), before);
            assert_eq!(
                o.invocation_state(&job, &mut setup),
                Err(InvocationError::Faulted)
            );
            assert!(o.is_faulted());
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == 128 {
                for boundary in [Boundary::SourceRebindCommit, Boundary::SourceRebindService] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.disposition == Disposition::Unknown
                        && e.service_time.is_some()));
                }
            }
        }
    }
}
#[test]
fn direct_rebind_populates_preparation_commit_and_service_timing_without_capture_dependencies() {
    use crate::instrumentation::NativeTimingProfileV1;
    for mode in 0..4 {
        let (mut o, _, d, port) = super::super::serialization_tests::fixture();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let generation = o.generation();
        assert_eq!(
            o.rebind_source_document(
                &d.document,
                port.source_binding_target(b"PRIVATE-TARGET").unwrap(),
                rebind_options(Some(0), 1),
                &rebind_facts(),
                &mut r
            ),
            Ok(true)
        );
        assert_eq!(o.generation(), generation + 1);
        assert_eq!(o.document(&d.document).unwrap().source.text(), "A");
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{events:?}").contains("PRIVATE"));
        if mode == 1 || mode == 2 {
            for boundary in [
                Boundary::SourceRebindPreparation,
                Boundary::SourceRebindCommit,
                Boundary::SourceRebindPublication,
            ] {
                let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
                assert_eq!(matching.len(), 1);
                assert_eq!(matching[0].disposition, Disposition::Completed);
                assert_eq!(matching[0].service_time.is_some(), mode == 2);
            }
        }
        if mode == 0 {
            assert!(events.is_empty());
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn rebind_commit_rechecks_nonpublication_writer_work_not_just_the_native_root() {
    let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
    let plan = o
        .prepare_source_rebind_native(
            &d.document,
            port.source_binding_target(b"out").unwrap(),
            rebind_options(Some(0), 1),
            &rebind_facts(),
            &mut r,
        )
        .unwrap();
    let root = o.native_root_identity().unwrap();
    let task = o
        .queue_pinned_save(
            &d,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(o.native_root_identity().unwrap(), root);
    let usage = o.resource_usage();
    assert_eq!(
        o.publish_source_rebind_native(plan, &rebind_facts(), &mut r),
        Err(ResourceError::Retained)
    );
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(o.resource_usage(), usage);
    assert_eq!(o.resources.bindings[0].identity.key.0.as_ref(), b"key");
    assert!(
        o.document(&d.document)
            .unwrap()
            .retained_bindings
            .source_history
            .is_empty()
    );
    o.stop_save(&task, &mut r).unwrap();
    // A fresh preparation can proceed; the old refused certificate was consumed.
    assert_eq!(
        o.rebind_source_document(
            &d.document,
            port.source_binding_target(b"out").unwrap(),
            rebind_options(Some(0), 1),
            &rebind_facts(),
            &mut r
        ),
        Ok(true)
    );
}
#[test]
fn rebind_installation_faults_fence_uncertain_roots_and_observe_unknown_without_private_labels() {
    use crate::instrumentation::NativeTimingProfileV1;
    for cut in 0..=2 {
        for mode in 0..4 {
            let (mut o, mut setup, d, port) = super::super::serialization_tests::fixture();
            let before = o.generation();
            o.publication_fault_after = Some(cut);
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
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
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o
                    .rebind_source_document(
                        &d.document,
                        port.source_binding_target(b"PRIVATE-DESTINATION").unwrap(),
                        rebind_options(Some(0), 1),
                        &rebind_facts(),
                        &mut r
                    )))
                .is_err()
            );
            assert!(o.is_faulted());
            assert_eq!(o.generation(), before + u64::from(cut == 2));
            let history = &o.documents[d.document.slot]
                .as_ref()
                .unwrap()
                .retained_bindings
                .source_history;
            assert_eq!(history.len(), usize::from(cut != 0));
            if cut != 0 {
                assert_eq!(history[0].baseline.as_ref().unwrap().bytes.0.as_ref(), b"A");
            }
            assert_eq!(
                o.read(&d.document, &mut setup).err(),
                Some(CoordinatorError::Faulted)
            );
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if mode == 1 || mode == 2 {
                for boundary in [
                    Boundary::SourceRebindPublication,
                    Boundary::SourceRebindCommit,
                ] {
                    let matches: Vec<_> =
                        events.iter().filter(|e| e.boundary == boundary).collect();
                    assert_eq!(matches.len(), 1);
                    assert_eq!(matches[0].disposition, Disposition::Unknown);
                    assert_eq!(matches[0].service_time.is_some(), mode == 2);
                }
            }
            if mode == 3 {
                assert!(r.dropped() > 0);
            }
        }
    }
}
#[test]
fn shared_rebind_preparer_is_read_only_and_preserves_exact_archived_provenance() {
    for scratch in [false, true] {
        let (mut o, mut r, loaded, port) = super::super::serialization_tests::fixture();
        let d = if scratch {
            o.create(b"scratch", &mut r).unwrap()
        } else {
            loaded
        };
        let root = o.native_root_identity().unwrap();
        let before = o.semantic_root_usage(&mut r).unwrap();
        let plan = o
            .prepare_source_rebind_native(
                &d.document,
                port.source_binding_target(b"out").unwrap(),
                rebind_options(if scratch { None } else { Some(0) }, 1),
                &rebind_facts(),
                &mut r,
            )
            .unwrap();
        assert_eq!(o.native_root_identity().unwrap(), root);
        assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
        assert_eq!(
            o.publish_source_rebind_native(plan, &rebind_facts(), &mut r),
            Ok(true)
        );
        let binding = o
            .resources
            .bindings
            .iter()
            .find(|b| b.document == d.document)
            .unwrap();
        assert_eq!(binding.identity.key.0.as_ref(), b"out");
        assert!(binding.baseline.is_none());
        let history = &o
            .document(&d.document)
            .unwrap()
            .retained_bindings
            .source_history;
        assert_eq!(history.len(), usize::from(!scratch));
        if !scratch {
            assert_eq!(history[0].baseline.as_ref().unwrap().bytes.0.as_ref(), b"A");
        }
        assert_eq!(
            o.read(&d.document, &mut r).unwrap().source.text(),
            if scratch { "scratch" } else { "A" }
        );
    }
}
#[test]
fn private_rebind_preparations_reject_foreign_changed_roots_and_current_permission_loss() {
    for change in ["foreign", "publication", "permission"] {
        let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
        let plan = o
            .prepare_source_rebind_native(
                &d.document,
                port.source_binding_target(b"out").unwrap(),
                rebind_options(Some(0), 1),
                &rebind_facts(),
                &mut r,
            )
            .unwrap();
        let mut current = rebind_facts();
        if change == "publication" {
            o.create(b"other", &mut r).unwrap();
        }
        if change == "permission" {
            current.rebind_allowed = None;
        }
        if change == "foreign" {
            let (mut other, mut r, _, _) = super::super::serialization_tests::fixture();
            assert_eq!(
                other.publish_source_rebind_native(plan, &current, &mut r),
                Err(ResourceError::Unavailable)
            );
        } else {
            let root = o.native_root_identity().unwrap();
            assert_eq!(
                o.publish_source_rebind_native(plan, &current, &mut r),
                Err(if change == "publication" {
                    ResourceError::Stale
                } else {
                    ResourceError::Unavailable
                })
            );
            assert_eq!(o.native_root_identity().unwrap(), root);
        }
        assert!(!o.is_faulted());
        assert!(
            o.document(&d.document)
                .unwrap()
                .retained_bindings
                .source_history
                .is_empty()
        );
    }
}
