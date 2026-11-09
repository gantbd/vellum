use super::*;
#[test]
fn facade_propagates_actual_publisher_panic_without_terminal_success_or_refund() {
    for cut in 0..3 {
        let mut o = SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 4,
                max_participants: 4,
                max_staged_bytes: 4096,
                source: crate::source::Limits {
                    max_source_bytes: 4096,
                    max_edits: 8,
                    max_edit_payload_bytes: 4096,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 4,
                max_retained: 4,
                max_intent_bytes: 32768,
            },
        );
        let mut r = Recorder::try_with_capacity(128).unwrap();
        o.configure_semantic_root_budget(1048576, &mut r).unwrap();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = SettingsExecutionContextV1 {
            authority: "a".into(),
            engine: "e".into(),
            recipient: "r".into(),
            purpose: "p".into(),
            destination: "d".into(),
            profile: "native/1".into(),
            continuity: "c".into(),
        };
        let f = TrustedScratchFactsV1 {
            context: &c,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            interpretation_current: Some(true),
            resources_ready: Some(true),
            companions_ready: Some(true),
            versions_available: Some(true),
            reply_ready: Some(true),
            lifecycle_allowed: Some(true),
            coverage_complete: Some(true),
            creation_allowed: Some(true),
        };
        let a = ScratchInvocationAdmissionV1 {
            facts: &f,
            limits: ScratchInvocationLimitsV1 {
                max_capture_bytes: 8192,
                max_result_bytes: 512,
            },
            publication_epoch: 7,
            publication_allowed: Some(true),
            result_admission_allowed: Some(true),
            result_identifiers_allowed: Some(true),
        };
        let q = ScratchDocumentRequestV1 {
            source: b"PRIVATE",
            serialization: SourceSerializationV1 {
                profile: SourceSerializationProfileV1::StrictUtf8OneBom,
                bom: false,
            },
            insertion_policy: GeneratedNewlineV1::Lf,
        };
        let id = o.issue_invocation(&mut r).unwrap();
        let _other = o.retention.reserve(71).unwrap();
        o.submit_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleCommandV1::Scratch(&q),
            NativeDocumentLifecycleAdmissionV1::Scratch(&a),
            &mut r,
        )
        .unwrap();
        let held = o.retention_usage();
        let generation = o.generation();
        o.publication_fault_after = Some(cut);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o
                .run_native_document_lifecycle_v1(
                    &id,
                    NativeDocumentLifecycleServiceV1::Scratch(&f),
                    &mut r
                )))
            .is_err()
        );
        assert!(o.is_faulted());
        assert!(matches!(
            o.invocations.slots[id.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.documents.len(), usize::from(cut > 0));
        assert_eq!(o.generation(), generation + u64::from(cut == 2));
        assert_eq!(
            o.run_native_document_lifecycle_v1(
                &id,
                NativeDocumentLifecycleServiceV1::Scratch(&f),
                &mut r
            ),
            Err(InvocationError::Faulted)
        );
        let ctl = InvocationObservationPolicyControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        o.retire_scratch_v1(&id, &ctl, &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 71);
        let mut unknown = false;
        while let Some(e) = r.pop_front() {
            if e.boundary == Boundary::DocumentLifecycleFacadeService {
                assert_eq!(e.disposition, Disposition::Unknown);
                unknown = true;
            }
        }
        assert!(unknown);
    }
}
