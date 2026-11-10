use super::*;
fn ready(key: &[u8]) -> (SourceCoordinator, Recorder, SourceReadIdV1) {
    let (mut o, mut r, _, port) = super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_source_reader_v1(super::tests::limits(), &mut r)
        .unwrap();
    let id = o
        .admit_source_read_v1(
            port.source_read_target_v1(key).unwrap(),
            ReadStrength::CapturedRead,
            &mut r,
        )
        .unwrap();
    let dispatch = o.submit_source_read_v1(&id, &mut r).unwrap();
    let version = SourceReadVersionV1::Opaque(b"v1");
    let bytes = b"\xef\xbb\xbfEnglish\r\nDeutsch\n";
    for evidence in [
        port.source_read_start_v1(&dispatch, version).unwrap(),
        port.source_read_chunk_v1(&dispatch, 0, 0, bytes).unwrap(),
        port.source_read_end_v1(&dispatch, version, 1, bytes.len())
            .unwrap(),
    ] {
        o.record_source_read_v1(evidence, &mut r).unwrap();
    }
    (o, r, id)
}
#[test]
fn activation_preparation_is_read_only_and_create_join_repeat_preserve_native_meaning() {
    for joined in [false, true] {
        let (mut o, mut r, id) = ready(if joined { b"key" } else { b"new" });
        let before = o.native_root_identity().unwrap();
        let docs = o.documents.len();
        let plan = o
            .prepare_source_read_activation_native(&id, &mut r)
            .unwrap();
        assert_eq!(o.native_root_identity().unwrap(), before);
        assert_eq!(o.documents.len(), docs);
        assert!(o.source_read_task(&id).unwrap().activated.is_none());
        let generation = o.generation();
        let result = o
            .commit_source_read_activation_native(plan, &mut r)
            .unwrap();
        assert_eq!(
            result.disposition,
            if joined {
                SourceReadActivationDispositionV1::Joined
            } else {
                SourceReadActivationDispositionV1::Created
            }
        );
        assert_eq!(o.generation(), generation + u64::from(!joined));
        assert_eq!(o.documents.len(), docs + usize::from(!joined));
        let expected = if joined { "A" } else { "English\r\nDeutsch\n" };
        assert_eq!(
            o.document(&result.document.document).unwrap().source.text(),
            expected
        );
        let first_marker = o.source_read_status_v1(&id, &mut r).unwrap().activated;
        let again = o.activate_source_read_v1(&id, &mut r).unwrap();
        assert_eq!(
            again.disposition,
            SourceReadActivationDispositionV1::AlreadyActivated
        );
        assert_eq!(
            o.source_read_status_v1(&id, &mut r).unwrap().activated,
            first_marker
        );
        assert_eq!(o.generation(), generation + u64::from(!joined));
    }
}
#[test]
fn prepared_activation_rechecks_task_stop_and_exact_root_without_partial_birth() {
    for change in ["stop", "root"] {
        let (mut o, mut r, id) = ready(b"new");
        let plan = o
            .prepare_source_read_activation_native(&id, &mut r)
            .unwrap();
        if change == "stop" {
            o.stop_source_read_v1(&id, &mut r).unwrap();
        } else {
            o.create(b"other", &mut r).unwrap();
        }
        let docs = o.documents.len();
        let generation = o.generation();
        assert!(
            o.commit_source_read_activation_native(plan, &mut r)
                .is_err()
        );
        assert_eq!(o.documents.len(), docs);
        assert_eq!(o.generation(), generation);
        assert!(o.source_read_task(&id).unwrap().activated.is_none());
        assert!(!o.is_faulted());
    }
}
#[test]
fn prepared_join_cannot_replace_an_activation_marker_even_without_root_publication() {
    let (mut o, mut r, id) = ready(b"key");
    let first = o
        .prepare_source_read_activation_native(&id, &mut r)
        .unwrap();
    let second = o
        .prepare_source_read_activation_native(&id, &mut r)
        .unwrap();
    let root = o.native_root_identity().unwrap();
    o.commit_source_read_activation_native(first, &mut r)
        .unwrap();
    assert_eq!(o.native_root_identity().unwrap(), root);
    let marker = o.source_read_task(&id).unwrap().activated.clone();
    assert_eq!(
        o.commit_source_read_activation_native(second, &mut r),
        Err(ResourceError::Stale)
    );
    assert_eq!(o.source_read_task(&id).unwrap().activated, marker);
    assert!(!o.is_faulted());
}
#[test]
fn prepared_birth_rechecks_current_root_funding_before_any_publication() {
    let (mut o, mut r, id) = ready(b"new");
    let plan = o
        .prepare_source_read_activation_native(&id, &mut r)
        .unwrap();
    let root = o.native_root_identity().unwrap();
    let bytes = o
        .semantic_root_capacity_usage(&mut r)
        .unwrap()
        .admission_bytes;
    o.configure_semantic_root_budget(bytes, &mut r).unwrap();
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(
        o.commit_source_read_activation_native(plan, &mut r),
        Err(ResourceError::Limit)
    );
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert!(o.source_read_task(&id).unwrap().activated.is_none());
    assert!(!o.is_faulted());
}
#[test]
fn activation_timing_is_optional_bounded_and_does_not_change_success_or_refusal() {
    use crate::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 128] {
            for stop in [false, true] {
                let (mut o, _, id) = ready(b"PRIVATE-NEW");
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(timing);
                if stop {
                    o.stop_source_read_v1(&id, &mut r).unwrap();
                }
                let generation = o.generation();
                let result = o.activate_source_read_v1(&id, &mut r);
                if stop {
                    assert_eq!(result, Err(ResourceError::Unavailable));
                } else {
                    assert_eq!(
                        result.unwrap().disposition,
                        SourceReadActivationDispositionV1::Created
                    );
                }
                assert_eq!(o.generation(), generation + u64::from(!stop));
                assert!(!o.is_faulted());
                let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(records.len() <= capacity);
                assert!(!format!("{records:?}").contains("PRIVATE"));
                if capacity == 128 {
                    let event = records
                        .iter()
                        .find(|e| e.boundary == Boundary::SourceReadActivation)
                        .unwrap();
                    assert_eq!(
                        event.disposition,
                        if stop {
                            Disposition::Refused
                        } else {
                            Disposition::Completed
                        }
                    );
                    assert_eq!(
                        event.service_time.is_some(),
                        timing == NativeTimingProfileV1::ProcessMonotonicV1
                    );
                }
            }
        }
    }
}
#[test]
fn owned_activation_fault_cuts_never_expose_a_staged_result_or_known_rejection() {
    for cut in 0..=5 {
        let (mut o, mut r, task) = ready(b"new");
        o.invocations = SourceCoordinator::with_invocation_limits(
            o.limits,
            InvocationLimits {
                max_lifetime_issued: 4,
                max_retained: 4,
                max_intent_bytes: 16384,
            },
        )
        .invocations;
        let context = SettingsExecutionContextV1 {
            authority: "a".into(),
            engine: "e".into(),
            recipient: "r".into(),
            purpose: "load".into(),
            destination: "local".into(),
            profile: "load/1".into(),
            continuity: "c".into(),
        };
        let facts = TrustedSourceLoadFactsV1 {
            context: &context,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            interpretation_current: Some(true),
            identity_allowed: Some(true),
            binding_coverage_complete: Some(true),
            activation_allowed: Some(true),
            companions_ready: Some(true),
            versions_available: Some(true),
            reply_ready: Some(true),
            resources_ready: Some(true),
            owner_control_allowed: Some(true),
            preparation_reserved: Some(true),
            writer_coverage_complete: Some(true),
            disposition_readable: Some(true),
            creation_allowed: Some(true),
            owner_access_allowed: Some(true),
        };
        let auth = o
            .borrow_source_load_authority_v1(
                &task,
                &SourceLoadAuthorityFactsV1 {
                    owner_control_allowed: Some(true),
                    preparation_reserved: Some(true),
                    resources_ready: Some(true),
                },
                &mut r,
            )
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_source_load_v1(
            &id,
            &auth,
            &SourceLoadAdmissionV1 {
                facts: &facts,
                limits: SourceLoadLimitsV1 {
                    max_capture_bytes: 4096,
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
        crate::coordinator::shutdown::tests::invocation_funding(&o, &id, 1, 4096, &mut r);
        o.publication_fault_after = Some(cut);
        let mut r = Recorder::try_with_capacity(128)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                || o.run_source_load_v1(&id, &facts, &mut r)
            ))
            .is_err()
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.invocation_outcome(&id, &mut r).err(),
            Some(InvocationError::Faulted)
        );
        let release = TrustedSourceLoadResultV1 {
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
            o.source_load_result_v1(&id, &release, &mut r),
            Err(InvocationError::Faulted)
        );
        assert!(
            std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary == Boundary::SourceLoadService
                && e.disposition == Disposition::Unknown
                && e.service_time.is_some())
        );
        let reader_usage = o.source_reader_usage_v1().unwrap();
        let funded = o.retention_usage();
        let control = InvocationObservationPolicyControlV1 {
            context: &context,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        o.retire_source_load_v1(&id, &control, &mut r).unwrap();
        assert!(o.retention_usage().reserved < funded.reserved);
        assert_eq!(o.source_reader_usage_v1().unwrap(), reader_usage);
        assert!(o.is_faulted());
    }
}
#[test]
fn each_activation_installation_cut_preserves_an_unknown_fence_until_full_completion() {
    use crate::instrumentation::NativeTimingProfileV1;
    for cut in 0..=4 {
        for capacity in [0, 1, 128] {
            let (mut o, _, id) = ready(b"PRIVATE-NEW");
            let docs = o.documents.len();
            let generation = o.generation();
            o.publication_fault_after = Some(cut);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.activate_source_read_v1(&id, &mut r)
                ))
                .is_err()
            );
            assert!(o.is_faulted());
            assert_eq!(o.documents.len(), docs + usize::from(cut >= 1));
            assert_eq!(o.generation(), generation + u64::from(cut >= 2));
            assert_eq!(
                o.resources
                    .bindings
                    .iter()
                    .any(|b| b.identity.key.0.as_ref() == b"PRIVATE-NEW"),
                cut >= 3
            );
            assert_eq!(
                o.source_read_task(&id).unwrap().activated.is_some(),
                cut >= 4
            );
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(!format!("{records:?}").contains("PRIVATE"));
            if capacity == 128 {
                assert!(
                    records
                        .iter()
                        .any(|r| r.boundary == Boundary::SourceReadActivation
                            && r.disposition == Disposition::Unknown
                            && r.service_time.is_some())
                );
            }
        }
    }
}
