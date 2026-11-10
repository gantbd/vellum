use super::*;
#[test]
fn annotation_admission_interruption_preserves_child_claim_and_fences_incomplete_parent_installation()
 {
    use crate::instrumentation::{Disposition, NativeTimingProfileV1};
    for cut in 0..5 {
        let mut z = Setup::new(8192);
        z.r = Recorder::try_with_capacity(128)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let tasks = z.o.resources.tasks.len();
        let artifacts = z.o.resources.artifacts.len();
        let funding = z.o.save_operation_usage_v1().reserved_store_bytes;
        let pending = z.o.pending_baseline_units(None);
        z.o.publication_fault_after = Some(cut);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| z.admit())).is_err());
        assert!(z.o.is_faulted());
        assert_eq!(
            z.o.resources.artifacts.len(),
            artifacts + usize::from(cut >= 1)
        );
        assert_eq!(z.o.resources.tasks.len(), tasks + usize::from(cut >= 2));
        assert_eq!(
            z.o.pending_baseline_units(None),
            pending + usize::from(cut >= 2)
        );
        assert_eq!(z.o.save_operation_usage_v1().issued, usize::from(cut >= 4));
        assert_eq!(z.o.save_operation_usage_v1().reserved_store_bytes, funding);
        if cut >= 2 {
            let task = z.o.resources.tasks[tasks].as_ref().unwrap();
            assert_eq!(task.operation, Some(0));
            assert_eq!(
                task.baseline_claim.as_ref().unwrap().state,
                BaselineCapacityState::Pending
            );
            assert_eq!(task.phase, SavePhase::Queued);
            assert!(!task.no_future);
        }
        let events: Vec<_> = std::iter::from_fn(|| z.r.pop_front()).collect();
        for boundary in [
            Boundary::AnnotationSaveAdmission,
            Boundary::SaveOperationAdmission,
        ] {
            assert!(events.iter().any(|e| e.boundary == boundary
                && e.disposition == Disposition::Unknown
                && e.service_time.is_some()));
            assert!(
                !events
                    .iter()
                    .any(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
            );
        }
        assert_eq!(
            z.o.create(b"must refuse", &mut z.r),
            Err(CoordinatorError::Faulted)
        );
    }
}
struct Setup {
    o: SourceCoordinator,
    r: Recorder,
    d: DocumentVersion,
    p: ProviderPort,
    encoder: AnnotationEncoderPort,
    encoding: AnnotationEncodingContext,
    set: u64,
    context: SettingsExecutionContextV1,
}
impl Setup {
    fn new(capture_bytes: usize) -> Self {
        let (mut o, mut r, old, p) =
            super::super::super::serialization_tests::pending_root_fixture();
        let task = o.resource_task(&old).unwrap();
        let d = task.captured.clone();
        let set = task.captured_annotation_set.unwrap();
        let encoding = task.encoding_context.as_ref().unwrap().clone();
        let encoder = AnnotationEncoderPort {
            registration: o
                .resources
                .encoders
                .iter()
                .find(|e| e.context == encoding)
                .unwrap()
                .clone(),
        };
        o.configure_save_operations_v1(
            SaveOperationLimitsV1 {
                max_issued: 8,
                max_label_bytes: 64,
                max_capture_bytes: capture_bytes,
                max_reply_bytes: 1024,
                max_store_bytes: 131072,
            },
            &mut r,
        )
        .unwrap();
        Self {
            o,
            r,
            d,
            p,
            encoder,
            encoding,
            set,
            context: crate::coordinator::serialization::tests::context(),
        }
    }
    fn admit(&mut self) -> Result<SaveOperationAdmissionV1, ResourceError> {
        let snapshot = self
            .o
            .capture_annotations(&self.d, self.set, &mut self.r)
            .unwrap();
        let encoded = self.encoder.seal(snapshot, b"new".to_vec()).unwrap();
        let destination = self.p.read_result(b"comments", b"v0", b"{}").unwrap();
        let source = self.p.source_binding_target(b"key").unwrap();
        let check = self
            .o
            .ordinary_pair_check(&self.d.document, &mut self.r)
            .unwrap()
            .unwrap();
        let pair = self
            .p
            .attest_ordinary_pair(&check, ResourceRelation::Distinct)
            .unwrap();
        let req = OwnedAnnotationSaveRequestV1 {
            origin: SaveOperationOriginV1 {
                operation: b"op",
                effect: b"e",
                attempt: b"a",
            },
            source: &self.d,
            expected_set: self.set,
            expected_annotation_binding: 1,
            expected_source_binding: Some(ExpectedSaveBindingV1 {
                target: &source,
                generation: 0,
            }),
            encoding: &self.encoding,
            destination: &destination,
            requirements: SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            condition: SaveOperationConditionV1::Pinned,
        };
        let g = TrustedAnnotationSaveOperationReleaseV1 {
            context: &self.context,
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            write_allowed: Some(true),
            annotation_readable: Some(true),
            disposition_readable: Some(true),
            retained: Some(true),
            reply_ready: Some(true),
            transfer_ready: Some(true),
            resources_ready: Some(true),
        };
        let a = TrustedOrdinaryAnnotationSaveAdmissionV1 {
            authorized: Some(true),
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            annotation_readable: Some(true),
            complete_disclosure: Some(true),
            write_allowed: Some(true),
            binding_coverage_complete: Some(true),
            writer_coverage_complete: Some(true),
            provider_available: Some(true),
            resources_ready: Some(true),
            launch_allowed: Some(true),
            destination_allowed: Some(true),
            destination_context_attested: Some(true),
            identity_allowed: Some(true),
            companions_ready: Some(true),
            versions_available: Some(true),
            reply_ready: Some(true),
            aliases_current: Some(true),
            source_evidence_ready: Some(true),
            retained: Some(true),
            transfer_ready: Some(true),
            preparation_reserved: Some(true),
            owner_control_allowed: Some(true),
            ordinary_bindings_current: Some(true),
            ordinary_identity_current: Some(true),
            historical_artifact_readable: Some(true),
            overwrite_destination_allowed: Some(true),
        };
        self.o.request_owned_annotation_save_v1(
            &req,
            &g,
            Some(&a),
            Some(encoded),
            Some(pair),
            &mut self.r,
        )
    }
}
#[test]
fn full_annotation_capsule_counts_conditions_and_both_association_key_incidences_before_issuance() {
    let c = crate::coordinator::serialization::tests::context();
    let exact = capsule_bytes(2, 1, 1, b"comments".len(), &c).unwrap()
        + b"{}".len()
        + b"v0".len()
        + b"comments".len()
        + b"key".len();
    for delta in [-1, 0, 1] {
        let mut z = Setup::new(exact.checked_add_signed(delta).unwrap());
        let before = z.o.resource_usage();
        let parent = z.o.save_operation_usage_v1();
        let result = z.admit();
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(z.o.resource_usage(), before);
            assert_eq!(z.o.save_operation_usage_v1(), parent);
        } else {
            result.unwrap();
            z.o.validate_save_operations().unwrap();
            assert_eq!(
                z.o.resource_usage().pending_baseline_publications,
                before.pending_baseline_publications + 1
            );
        }
    }
}
#[test]
fn annotation_parent_role_context_or_condition_drift_cannot_erase_child_responsibility() {
    for mode in 0..4 {
        let mut z = Setup::new(8192);
        let id = z.admit().unwrap().task;
        z.o.stop_save(&id, &mut z.r).unwrap();
        let alternate = z.o.register_annotation_encoder(&mut z.r).unwrap().context();
        let c = z.o.resources.operations.rows[0].capture.as_mut().unwrap();
        if mode == 3 {
            c.flavor = Flavor::Source {
                serialization: 1,
                condition: SaveOperationConditionV1::Pinned,
            };
        } else {
            let Flavor::Annotations {
                set,
                encoding,
                before_validator,
                ..
            } = &mut c.flavor
            else {
                panic!("annotation role")
            };
            match mode {
                0 => *set += 1,
                1 => *encoding = alternate,
                _ => *before_validator = blob(b"different", 64, true).unwrap(),
            }
        }
        let before = z.o.resource_usage();
        let parent = z.o.save_operation_usage_v1();
        assert_eq!(z.o.retire_save(&id, &mut z.r), Err(ResourceError::Faulted));
        assert!(z.o.is_faulted());
        assert_eq!(z.o.resource_usage(), before);
        assert_eq!(z.o.save_operation_usage_v1(), parent);
    }
}
