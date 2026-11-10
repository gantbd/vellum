use super::super::source_export::tests::{
    activation, ready_activation, ready_activation_with_quiescence,
};
use super::*;
use crate::instrumentation::NativeTimingProfileV1;
fn authority_facts() -> SourceSaveAsAuthorityFactsV1 {
    SourceSaveAsAuthorityFactsV1 {
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn native_preparation_is_read_only_and_repeats_preserve_existing_native_semantics() {
    let (mut o, mut r, _, p, id) = ready_activation();
    let before = o.native_root_identity().unwrap();
    let usage = o.resource_usage();
    let plan = o
        .prepare_source_save_as_native(
            &id,
            p.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            None,
            1,
            &activation(),
            &mut r,
        )
        .unwrap();
    assert_eq!(o.native_root_identity().unwrap(), before);
    assert_eq!(o.resource_usage(), usage);
    assert!(
        !o.resource_task(&id)
            .unwrap()
            .source_export
            .as_ref()
            .unwrap()
            .binding_applied
    );
    assert_eq!(
        o.commit_source_save_as_native(plan, &activation(), &mut r),
        Ok(true)
    );
    let generation = o.generation;
    // Existing native repeats do not re-probe or consume another history slot.
    let plan = o
        .prepare_source_save_as_native(
            &id,
            p.source_save_as_probe(b"ignored", None).unwrap(),
            None,
            0,
            &activation(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.commit_source_save_as_native(plan, &activation(), &mut r),
        Ok(false)
    );
    assert_eq!(o.generation, generation);
}
#[test]
fn prepared_activation_rechecks_stop_root_current_facts_funding_record_and_writer_work() {
    for change in ["stop", "root", "facts", "funding", "record", "writer"] {
        let (mut o, mut r, d, p, id) = ready_activation();
        let plan = o
            .prepare_source_save_as_native(
                &id,
                p.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
                None,
                1,
                &activation(),
                &mut r,
            )
            .unwrap();
        let mut current = activation();
        match change {
            "stop" => {
                o.stop_save(&id, &mut r).unwrap();
            }
            "root" => {
                o.create(b"unrelated", &mut r).unwrap();
            }
            "facts" => current.probe_attested = None,
            "funding" => {
                let bytes = o
                    .semantic_root_capacity_usage(&mut r)
                    .unwrap()
                    .admission_bytes;
                o.configure_semantic_root_budget(bytes, &mut r).unwrap();
            }
            "record" => {
                o.resources.tasks[id.slot]
                    .as_mut()
                    .unwrap()
                    .source_export
                    .as_mut()
                    .unwrap()
                    .marker = Arc::new(())
            }
            _ => {
                o.queue_pinned_save(
                    &d,
                    SaveRequirements {
                        protection: Protection::BestEffort,
                        acknowledgment: Acknowledgment::Ordinary,
                    },
                    &mut r,
                )
                .unwrap();
            }
        }
        let root = o.native_root_identity().unwrap();
        let usage = o.resource_usage();
        assert!(
            o.commit_source_save_as_native(plan, &current, &mut r)
                .is_err(),
            "{change}"
        );
        assert_eq!(o.native_root_identity().unwrap(), root);
        assert_eq!(o.resource_usage(), usage);
        assert!(!o.faulted);
        assert!(
            !o.resource_task(&id)
                .unwrap()
                .source_export
                .as_ref()
                .unwrap()
                .binding_applied
        );
    }
}
#[test]
fn every_native_activation_cut_preserves_a_fault_fence_and_an_unknown_timed_observation() {
    for cut in 0..=4 {
        for capacity in [0, 1, 128] {
            let (mut o, _, d, p, id) = ready_activation();
            let generation = o.generation;
            o.publication_fault_after = Some(cut);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o
                    .activate_source_save_as(
                        &id,
                        p.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
                        None,
                        1,
                        &activation(),
                        &mut r
                    )))
                .is_err()
            );
            assert!(o.faulted);
            assert_eq!(
                o.resources.source_export_reservations.len(),
                usize::from(cut < 2)
            );
            assert_eq!(
                o.documents[d.document.slot]
                    .as_ref()
                    .unwrap()
                    .retained_bindings
                    .source_history
                    .len(),
                usize::from(cut >= 1)
            );
            let state = o
                .resource_task(&id)
                .unwrap()
                .source_export
                .as_ref()
                .unwrap();
            assert_eq!(state.released, cut >= 3);
            assert_eq!(state.binding_applied, cut >= 3);
            assert_eq!(o.generation, generation + u64::from(cut >= 4));
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == 128 {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::SourceSaveAsActivation
                            && e.disposition == Disposition::Unknown
                            && e.service_time.is_some())
                );
            }
        }
    }
}
#[test]
fn weak_task_authority_is_stable_and_does_not_override_real_retirement_obligations() {
    let (mut o, mut r, _, _, id) = ready_activation_with_quiescence(true);
    let usage = o.resource_usage();
    let root = o.native_root_identity().unwrap();
    let a = o
        .borrow_source_save_as_authority_v1(&id, &authority_facts(), &mut r)
        .unwrap();
    let b = o
        .borrow_source_save_as_authority_v1(&id, &authority_facts(), &mut r)
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(
        o.source_save_as_authority_current_v1(&a, &authority_facts(), &mut r),
        Ok(())
    );
    assert_eq!(o.resource_usage(), usage);
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Retained));
    o.release_source_export_reservation(&id, &activation().control, &mut r)
        .unwrap();
    o.retire_save(&id, &mut r).unwrap();
    assert_eq!(
        o.source_save_as_authority_current_v1(&a, &authority_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(
        o.source_save_as_authority_current_v1(&b, &authority_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(!o.faulted);
}
#[test]
fn authority_requires_every_current_fact_exact_record_owner_and_native_saveas_kind() {
    let (mut o, mut r, d, port, id) = ready_activation();
    let a = o
        .borrow_source_save_as_authority_v1(&id, &authority_facts(), &mut r)
        .unwrap();
    for field in 0..3 {
        for value in [None, Some(false)] {
            let mut facts = authority_facts();
            *match field {
                0 => &mut facts.owner_control_allowed,
                1 => &mut facts.preparation_reserved,
                _ => &mut facts.resources_ready,
            } = value;
            assert_eq!(
                o.borrow_source_save_as_authority_v1(&id, &facts, &mut r)
                    .err(),
                Some(ResourceError::Unavailable)
            );
            assert_eq!(
                o.source_save_as_authority_current_v1(&a, &facts, &mut r),
                Err(ResourceError::Unavailable)
            );
        }
    }
    let (mut peer, mut recorder, _, _, _) = ready_activation();
    assert_eq!(
        peer.source_save_as_authority_current_v1(&a, &authority_facts(), &mut recorder),
        Err(ResourceError::Unavailable)
    );
    let ordinary = o
        .queue_pinned_save(
            &d,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.borrow_source_save_as_authority_v1(&ordinary, &authority_facts(), &mut r)
            .err(),
        Some(ResourceError::Unsupported)
    );
    let serialization = o.source_serialization(&d.document, &mut r).unwrap();
    let prepared = o
        .prepare_source_artifact(&d, serialization, &mut r)
        .unwrap();
    let copy = o
        .queue_source_export(
            &prepared,
            port.read_result(b"copy", b"v0", b"x")
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
            &super::super::source_export::tests::facts(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.borrow_source_save_as_authority_v1(&copy, &authority_facts(), &mut r)
            .err(),
        Some(ResourceError::Unsupported)
    );
    o.resources.tasks[id.slot]
        .as_mut()
        .unwrap()
        .source_export
        .as_mut()
        .unwrap()
        .marker = Arc::new(());
    assert_eq!(
        o.source_save_as_authority_current_v1(&a, &authority_facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(!o.faulted);
}
#[test]
fn owned_commit_cuts_never_release_a_staged_result_or_refund_the_write_producer() {
    for cut in 0..=5 {
        let (mut o, mut r, _, p, save) = ready_activation();
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
        let c = SettingsExecutionContextV1 {
            authority: "a".into(),
            engine: "e".into(),
            recipient: "r".into(),
            purpose: "activate".into(),
            destination: "local".into(),
            profile: "saveas/1".into(),
            continuity: "c".into(),
        };
        let native = activation();
        let grant = authority_facts();
        let current = TrustedSourceSaveAsInvocationFactsV1 {
            context: &c,
            native: &native,
            authority: &grant,
            interpretation_current: Some(true),
        };
        let auth = o
            .borrow_source_save_as_authority_v1(&save, &grant, &mut r)
            .unwrap();
        let request = SourceSaveAsRequestV1 {
            probe: p.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            pair: None,
            max_binding_history: 1,
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_source_save_as_v1(
            &id,
            &auth,
            &request,
            &SourceSaveAsAdmissionV1 {
                facts: &current,
                limits: SourceSaveAsLimitsV1 {
                    max_capture_bytes: 8192,
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
        crate::coordinator::shutdown::tests::invocation_funding(&o, &id, 1, 8192, &mut r);
        o.publication_fault_after = Some(cut);
        let mut r = Recorder::try_with_capacity(128)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || o.run_source_save_as_v1(&id, &current, &mut r)
            ))
            .is_err()
        );
        assert!(o.faulted);
        assert_eq!(
            o.invocation_outcome(&id, &mut r).err(),
            Some(InvocationError::Faulted)
        );
        assert_eq!(
            o.source_save_as_result_v1(
                &id,
                &TrustedSourceSaveAsResultV1 {
                    context: &c,
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
        o.retire_source_save_as_v1(
            &id,
            &InvocationObservationPolicyControlV1 {
                context: &c,
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
        assert_eq!(
            o.save_outcome(&save, &mut r).unwrap().application,
            Application::Written
        );
        assert!(
            std::iter::from_fn(|| r.pop_front())
                .any(|e| e.boundary == Boundary::SourceSaveAsService
                    && e.disposition == Disposition::Unknown
                    && e.service_time.is_some())
        );
    }
}
#[test]
fn activation_and_authority_timing_are_optional_bounded_and_do_not_change_native_results() {
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 128] {
            let (mut o, _, _, p, id) = ready_activation();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let a = o
                .borrow_source_save_as_authority_v1(&id, &authority_facts(), &mut r)
                .unwrap();
            o.source_save_as_authority_current_v1(&a, &authority_facts(), &mut r)
                .unwrap();
            assert_eq!(
                o.activate_source_save_as(
                    &id,
                    p.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
                    None,
                    1,
                    &activation(),
                    &mut r
                ),
                Ok(true)
            );
            assert_eq!(
                o.activate_source_save_as(
                    &id,
                    p.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
                    None,
                    1,
                    &activation(),
                    &mut r
                ),
                Ok(false)
            );
            assert!(!o.faulted);
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(events.len() <= capacity);
            if capacity == 128 {
                for boundary in [
                    Boundary::SourceSaveAsAuthorityBorrow,
                    Boundary::SourceSaveAsAuthorityValidation,
                    Boundary::SourceSaveAsActivation,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
            assert!(!format!("{a:?} {events:?}").contains("out"));
        }
    }
}
