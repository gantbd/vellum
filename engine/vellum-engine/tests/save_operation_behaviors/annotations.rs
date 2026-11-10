//! Scripted privileged codec issuer; real reference-JSON conformance is separate.
use super::*;
#[test]
fn document_resource_pins_keep_annotation_artifact_after_its_task_and_parent_retire() {
    let mut z = Setup::new();
    let initial =
        z.f.owner
            .document_resource_pins_v1(&z.d.document, &mut z.f.recorder)
            .unwrap();
    let task = z.admit();
    let artifact =
        z.f.owner
            .save_outcome(&task, &mut z.f.recorder)
            .unwrap()
            .artifact;
    let pins =
        z.f.owner
            .document_resource_pins_v1(&z.d.document, &mut z.f.recorder)
            .unwrap();
    assert_eq!(
        (pins.write_tasks, pins.write_parents, pins.artifacts),
        (1, 1, 1)
    );
    z.f.owner.stop_save(&task, &mut z.f.recorder).unwrap();
    assert_eq!(
        z.f.owner
            .document_resource_pins_v1(&z.d.document, &mut z.f.recorder)
            .unwrap(),
        pins
    );
    z.f.owner.retire_save(&task, &mut z.f.recorder).unwrap();
    let held =
        z.f.owner
            .document_resource_pins_v1(&z.d.document, &mut z.f.recorder)
            .unwrap();
    assert_eq!(
        (held.write_tasks, held.write_parents, held.artifacts),
        (0, 0, 1)
    );
    z.f.owner
        .retire_resource_artifact(&artifact, &mut z.f.recorder)
        .unwrap();
    assert_eq!(
        z.f.owner
            .document_resource_pins_v1(&z.d.document, &mut z.f.recorder)
            .unwrap(),
        initial
    );
}
#[path = "facade.rs"]
mod facade;
fn ann_release(c: &SettingsExecutionContextV1) -> TrustedAnnotationSaveOperationReleaseV1<'_> {
    TrustedAnnotationSaveOperationReleaseV1 {
        context: c,
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
    }
}
fn ann_admission() -> TrustedOrdinaryAnnotationSaveAdmissionV1 {
    TrustedOrdinaryAnnotationSaveAdmissionV1 {
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
    }
}
fn ann_request<'a>(
    d: &'a DocumentVersion,
    set: u64,
    e: &'a AnnotationEncodingContext,
    destination: &'a ResourceRead,
    source: &'a SourceBindingTargetV1,
    operation: &'a [u8],
) -> OwnedAnnotationSaveRequestV1<'a> {
    OwnedAnnotationSaveRequestV1 {
        origin: SaveOperationOriginV1 {
            operation,
            effect: b"annotation-write",
            attempt: b"one",
        },
        source: d,
        expected_set: set,
        expected_annotation_binding: 1,
        expected_source_binding: Some(ExpectedSaveBindingV1 {
            target: source,
            generation: 0,
        }),
        encoding: e,
        destination,
        requirements: SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        },
        condition: SaveOperationConditionV1::Pinned,
    }
}
fn candidate(
    f: &mut Fixture,
    e: &AnnotationEncoderPort,
    d: &DocumentVersion,
    set: u64,
) -> EncodedAnnotations {
    let snapshot = f
        .owner
        .capture_annotations(d, set, &mut f.recorder)
        .unwrap();
    e.seal(snapshot, b"ENCODED".to_vec()).unwrap()
}
struct Setup {
    f: Fixture,
    e: AnnotationEncoderPort,
    encoding: AnnotationEncodingContext,
    d: DocumentVersion,
    set: u64,
    source: SourceBindingTargetV1,
    destination: ResourceRead,
    c: SettingsExecutionContextV1,
}
pub(super) fn observation_fixture() -> (Fixture, SaveId, SettingsExecutionContextV1) {
    let mut z = Setup::new();
    let id = z.admit();
    (z.f, id, z.c)
}
#[test]
fn annotation_save_queue_timing_stops_once_and_retirement_keeps_the_observed_population() {
    for dispatched in [false, true] {
        let mut z = Setup::new();
        z.f.owner
            .configure_resource_queue_timing_v1(
                vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                &mut z.f.recorder,
            )
            .unwrap();
        let id = z.admit();
        let before = z
            .f
            .owner
            .resource_queue_attempt_timing_v1(ResourceQueueAttemptV1::Save(&id), &mut z.f.recorder)
            .unwrap();
        assert_eq!(
            (before.kind, before.phase),
            (
                ResourceQueueKindV1::AnnotationSave,
                ResourceQueuePhaseV1::Pending
            )
        );
        let dispatch = if dispatched {
            let pair = z.f.pair().unwrap();
            Some(
                z.f.owner
                    .dispatch_save_with_pair(
                        &id,
                        z.f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
                        pair,
                        &mut z.f.recorder,
                    )
                    .unwrap(),
            )
        } else {
            None
        };
        z.f.owner.stop_save(&id, &mut z.f.recorder).unwrap();
        let stopped = z
            .f
            .owner
            .resource_queue_attempt_timing_v1(ResourceQueueAttemptV1::Save(&id), &mut z.f.recorder)
            .unwrap();
        assert_eq!(
            stopped.phase,
            if dispatched {
                ResourceQueuePhaseV1::Dispatched
            } else {
                ResourceQueuePhaseV1::NotDispatched
            }
        );
        assert!(stopped.queue_duration.is_some());
        if let Some(dispatch) = dispatch {
            for e in [
                z.f.port
                    .result(
                        &dispatch,
                        Application::NotWritten,
                        0,
                        Synchronization::NotRequested,
                        None,
                    )
                    .unwrap(),
                z.f.port.no_future_effects(&dispatch).unwrap(),
                z.f.port.quiescence(&dispatch).unwrap(),
            ] {
                z.f.owner
                    .record_save_evidence(e, &mut z.f.recorder)
                    .unwrap();
            }
        }
        assert_eq!(
            z.f.owner
                .release_annotation_save_capacity(&id, &mut z.f.recorder)
                .unwrap(),
            BaselineCapacityState::NotWritten
        );
        z.f.owner.retire_save(&id, &mut z.f.recorder).unwrap();
        let ResourceQueueResponsivenessV1::Observed(snapshot) =
            z.f.owner
                .resource_queue_responsiveness_v1(&mut z.f.recorder)
        else {
            panic!("observed")
        };
        let p = snapshot.families[ResourceQueueKindV1::AnnotationSave.index()];
        assert_eq!(
            (p.observed_admissions, p.not_dispatched, p.retired_rows),
            (1, usize::from(!dispatched), 1)
        );
        assert_eq!(p.max_known_queue_duration, stopped.queue_duration);
    }
}
impl Setup {
    fn new() -> Self {
        let mut f = Fixture::new();
        f.owner
            .configure_save_operations_v1(limits(), &mut f.recorder)
            .unwrap();
        let e = f
            .owner
            .register_annotation_encoder(&mut f.recorder)
            .unwrap();
        let encoding = e.context();
        let d = f.doc.clone();
        let set = f
            .owner
            .annotation_set_version(&d.document, &mut f.recorder)
            .unwrap();
        let source = f.port.source_binding_target(b"source").unwrap();
        let destination = f.port.read_result(b"comments", b"a0", b"{}").unwrap();
        Self {
            f,
            e,
            encoding,
            d,
            set,
            source,
            destination,
            c: context(),
        }
    }
    fn admit(&mut self) -> SaveId {
        let encoded = candidate(&mut self.f, &self.e, &self.d, self.set);
        let pair = self.f.pair();
        let req = ann_request(
            &self.d,
            self.set,
            &self.encoding,
            &self.destination,
            &self.source,
            b"annotations",
        );
        self.f
            .owner
            .request_owned_annotation_save_v1(
                &req,
                &ann_release(&self.c),
                Some(&ann_admission()),
                Some(encoded),
                pair,
                &mut self.f.recorder,
            )
            .unwrap()
            .task
    }
}
#[test]
fn matching_annotation_operation_recovers_the_original_task_and_claim_without_source_disclosure_or_reencoding()
 {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    let e = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let encoding = e.context();
    let d = f.doc.clone();
    f.owner
        .set_read_only(&d.document, true, &mut f.recorder)
        .unwrap();
    let set = f
        .owner
        .annotation_set_version(&d.document, &mut f.recorder)
        .unwrap();
    let encoded = candidate(&mut f, &e, &d, set);
    let pair = f.pair();
    let source = f.port.source_binding_target(b"source").unwrap();
    let destination = f.port.read_result(b"comments", b"a0", b"{}").unwrap();
    let req = ann_request(&d, set, &encoding, &destination, &source, b"annotations");
    let c = context();
    let generation = f.owner.generation();
    let first = f
        .owner
        .request_owned_annotation_save_v1(
            &req,
            &ann_release(&c),
            Some(&ann_admission()),
            Some(encoded),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let before = f.owner.resource_usage();
    assert_eq!(before.pending_baseline_publications, 1);
    assert!(before.reserved_baseline_bytes > 0);
    f.owner.stop_annotation_save_admission(&mut f.recorder);
    let fenced = f.owner.resource_usage();
    let again = f
        .owner
        .request_owned_annotation_save_v1(&req, &ann_release(&c), None, None, None, &mut f.recorder)
        .unwrap();
    assert_eq!(first.status, SaveOperationAdmissionStatusV1::Admitted);
    assert_eq!(
        again.status,
        SaveOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(first.task, again.task);
    assert_eq!(f.owner.resource_usage(), fenced);
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.save_operation_usage_v1().owned, 1);
}

#[test]
fn complete_annotation_intent_context_and_pinned_condition_cannot_be_retargeted() {
    let mut z = Setup::new();
    let id = z.admit();
    let other_encoder =
        z.f.owner
            .register_annotation_encoder(&mut z.f.recorder)
            .unwrap()
            .context();
    let different_bytes = z.f.port.read_result(b"comments", b"a0", b"OTHER").unwrap();
    let different_token = z.f.port.read_result(b"comments", b"a1", b"{}").unwrap();
    let different_key = z.f.port.read_result(b"other", b"a0", b"{}").unwrap();
    let other_source = z.f.port.source_binding_target(b"other-source").unwrap();
    let replacement =
        z.f.owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut z.f.recorder,
            )
            .unwrap();
    let different_registration = replacement.read_result(b"comments", b"a0", b"{}").unwrap();
    let before = z.f.owner.resource_usage();
    for field in 0..13 {
        let mut req = ann_request(
            &z.d,
            z.set,
            &z.encoding,
            &z.destination,
            &z.source,
            b"annotations",
        );
        match field {
            0 => req.origin.effect = b"other-effect",
            1 => req.origin.attempt = b"two",
            2 => req.expected_set += 1,
            3 => req.expected_annotation_binding += 1,
            4 => req.expected_source_binding = None,
            5 => req.expected_source_binding.as_mut().unwrap().generation += 1,
            6 => req.expected_source_binding.as_mut().unwrap().target = &other_source,
            7 => req.encoding = &other_encoder,
            8 => req.destination = &different_bytes,
            9 => req.destination = &different_token,
            10 => req.destination = &different_key,
            11 => req.destination = &different_registration,
            12 => req.requirements.acknowledgment = Acknowledgment::Synchronized,
            _ => unreachable!(),
        }
        assert_eq!(
            z.f.owner.request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                None,
                None,
                None,
                &mut z.f.recorder
            ),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(z.f.owner.resource_usage(), before);
    }
    for field in 0..7 {
        let mut c = z.c.clone();
        match field {
            0 => c.authority.push('x'),
            1 => c.engine.push('x'),
            2 => c.recipient.push('x'),
            3 => c.purpose.push('x'),
            4 => c.destination.push('x'),
            5 => c.profile.push('x'),
            6 => c.continuity.push('x'),
            _ => unreachable!(),
        }
        let req = ann_request(
            &z.d,
            z.set,
            &z.encoding,
            &z.destination,
            &z.source,
            b"annotations",
        );
        assert_eq!(
            z.f.owner.request_owned_annotation_save_v1(
                &req,
                &ann_release(&c),
                None,
                None,
                None,
                &mut z.f.recorder
            ),
            Err(ResourceError::Unavailable)
        );
    }
    let req = ann_request(
        &z.d,
        z.set,
        &z.encoding,
        &z.destination,
        &z.source,
        b"annotations",
    );
    assert_eq!(
        z.f.owner
            .request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                None,
                None,
                None,
                &mut z.f.recorder
            )
            .unwrap()
            .task,
        id
    );
}

#[test]
fn annotation_disclosure_and_each_fresh_host_premise_are_exact_independent_conditions() {
    for denied in [None, Some(false)] {
        let mut z = Setup::new();
        z.admit();
        for field in 0..12 {
            let mut g = ann_release(&z.c);
            match field {
                0 => g.authorized = denied,
                1 => g.scope = denied,
                2 => g.continuous = denied,
                3 => g.live = denied,
                4 => g.not_stopped = denied,
                5 => g.write_allowed = denied,
                6 => g.annotation_readable = denied,
                7 => g.disposition_readable = denied,
                8 => g.retained = denied,
                9 => g.reply_ready = denied,
                10 => g.transfer_ready = denied,
                11 => g.resources_ready = denied,
                _ => unreachable!(),
            }
            let before = z.f.owner.resource_usage();
            let req = ann_request(
                &z.d,
                z.set,
                &z.encoding,
                &z.destination,
                &z.source,
                b"annotations",
            );
            assert_eq!(
                z.f.owner.request_owned_annotation_save_v1(
                    &req,
                    &g,
                    None,
                    None,
                    None,
                    &mut z.f.recorder
                ),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(z.f.owner.resource_usage(), before);
        }
        for field in 0..29 {
            let mut z = Setup::new();
            let mut g = ann_admission();
            match field {
                0 => g.authorized = denied,
                1 => g.scope = denied,
                2 => g.continuous = denied,
                3 => g.live = denied,
                4 => g.not_stopped = denied,
                5 => g.annotation_readable = denied,
                6 => g.complete_disclosure = denied,
                7 => g.write_allowed = denied,
                8 => g.binding_coverage_complete = denied,
                9 => g.writer_coverage_complete = denied,
                10 => g.provider_available = denied,
                11 => g.resources_ready = denied,
                12 => g.launch_allowed = denied,
                13 => g.destination_allowed = denied,
                14 => g.destination_context_attested = denied,
                15 => g.identity_allowed = denied,
                16 => g.companions_ready = denied,
                17 => g.versions_available = denied,
                18 => g.reply_ready = denied,
                19 => g.aliases_current = denied,
                20 => g.source_evidence_ready = denied,
                21 => g.retained = denied,
                22 => g.transfer_ready = denied,
                23 => g.preparation_reserved = denied,
                24 => g.owner_control_allowed = denied,
                25 => g.ordinary_bindings_current = denied,
                26 => g.ordinary_identity_current = denied,
                27 => g.historical_artifact_readable = denied,
                28 => g.overwrite_destination_allowed = denied,
                _ => unreachable!(),
            }
            let before = z.f.owner.resource_usage();
            let parent = z.f.owner.save_operation_usage_v1();
            let encoded = candidate(&mut z.f, &z.e, &z.d, z.set);
            let pair = z.f.pair();
            let req = ann_request(
                &z.d,
                z.set,
                &z.encoding,
                &z.destination,
                &z.source,
                b"annotations",
            );
            assert_eq!(
                z.f.owner.request_owned_annotation_save_v1(
                    &req,
                    &ann_release(&z.c),
                    Some(&g),
                    Some(encoded),
                    pair,
                    &mut z.f.recorder
                ),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(z.f.owner.resource_usage(), before);
            assert_eq!(z.f.owner.save_operation_usage_v1(), parent);
        }
    }
}

#[test]
fn unknown_effect_and_baseline_claim_remain_independent_of_parent_recovery_and_retirement() {
    let mut z = Setup::new();
    let id = z.admit();
    let pair = z.f.pair().unwrap();
    let dispatch =
        z.f.owner
            .dispatch_save_with_pair(
                &id,
                z.f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
                pair,
                &mut z.f.recorder,
            )
            .unwrap();
    assert_eq!(dispatch.bytes(), b"ENCODED");
    let req = ann_request(
        &z.d,
        z.set,
        &z.encoding,
        &z.destination,
        &z.source,
        b"annotations",
    );
    assert_eq!(
        z.f.owner
            .request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                None,
                None,
                None,
                &mut z.f.recorder
            )
            .unwrap()
            .task,
        id
    );
    z.f.owner.stop_save(&id, &mut z.f.recorder).unwrap();
    z.f.owner
        .record_save_evidence(z.f.port.quiescence(&dispatch).unwrap(), &mut z.f.recorder)
        .unwrap();
    assert_eq!(
        z.f.owner
            .release_annotation_save_capacity(&id, &mut z.f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        z.f.owner.retire_save(&id, &mut z.f.recorder),
        Err(ResourceError::Retained)
    );
    for fact in [
        z.f.port
            .result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
        z.f.port.no_future_effects(&dispatch).unwrap(),
    ] {
        z.f.owner
            .record_save_evidence(fact, &mut z.f.recorder)
            .unwrap();
    }
    assert_eq!(
        z.f.owner.retire_save(&id, &mut z.f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        z.f.owner
            .release_annotation_save_capacity(&id, &mut z.f.recorder),
        Ok(BaselineCapacityState::NotWritten)
    );
    // A resolved claim does not waive a provider's actual dispatch/evidence drain.
    assert_eq!(
        z.f.owner.retire_save(&id, &mut z.f.recorder),
        Err(ResourceError::Retained)
    );
    let artifact =
        z.f.owner
            .save_outcome(&id, &mut z.f.recorder)
            .unwrap()
            .artifact;
    let reserved = z.f.owner.save_operation_usage_v1().reserved_store_bytes;
    drop(dispatch);
    z.f.owner.retire_save(&id, &mut z.f.recorder).unwrap();
    assert_eq!(z.f.owner.save_operation_usage_v1().retired, 1);
    assert_eq!(
        z.f.owner.save_operation_usage_v1().reserved_store_bytes,
        reserved
    );
    assert_eq!(
        z.f.owner.request_owned_annotation_save_v1(
            &req,
            &ann_release(&z.c),
            None,
            None,
            None,
            &mut z.f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    // Ordinary annotation artifact ownership has its existing separate discharge.
    assert!(z.f.owner.resource_usage().held_artifact_bytes > 0);
    z.f.owner
        .retire_resource_artifact(&artifact, &mut z.f.recorder)
        .unwrap();
    assert_eq!(z.f.owner.resource_usage().held_artifact_bytes, 0);
}

#[test]
fn later_authored_edits_and_replacement_candidates_do_not_change_the_original_parent() {
    let mut z = Setup::new();
    let id = z.admit();
    let p =
        z.f.owner
            .prepare(
                &[DocumentEdits {
                    expected: z.d.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 4,
                        expected: "text".into(),
                        inserted: "later".into(),
                    }],
                }],
                &[],
                &mut z.f.recorder,
            )
            .unwrap();
    z.f.owner.commit(p, &mut z.f.recorder).unwrap();
    let now =
        z.f.owner
            .read(&z.d.document, &mut z.f.recorder)
            .unwrap()
            .version;
    let set =
        z.f.owner
            .annotation_set_version(&now.document, &mut z.f.recorder)
            .unwrap();
    let replacement =
        z.f.owner
            .register_annotation_encoder(&mut z.f.recorder)
            .unwrap();
    let different = candidate(&mut z.f, &replacement, &now, set);
    z.f.owner.stop_annotation_save_admission(&mut z.f.recorder);
    let before = z.f.owner.resource_usage();
    let req = ann_request(
        &z.d,
        z.set,
        &z.encoding,
        &z.destination,
        &z.source,
        b"annotations",
    );
    assert_eq!(
        z.f.owner
            .request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                None,
                Some(different),
                None,
                &mut z.f.recorder
            )
            .unwrap()
            .task,
        id
    );
    assert_eq!(z.f.owner.resource_usage(), before);
    assert_eq!(
        z.f.owner
            .save_outcome(&id, &mut z.f.recorder)
            .unwrap()
            .captured,
        z.d
    );
    let changed = ann_request(
        &now,
        set,
        &z.encoding,
        &z.destination,
        &z.source,
        b"annotations",
    );
    assert_eq!(
        z.f.owner.request_owned_annotation_save_v1(
            &changed,
            &ann_release(&z.c),
            None,
            None,
            None,
            &mut z.f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn failed_annotation_child_and_unsupported_profiles_do_not_spend_an_operation_or_claim() {
    let mut z = Setup::new();
    let req = ann_request(
        &z.d,
        z.set,
        &z.encoding,
        &z.destination,
        &z.source,
        b"annotations",
    );
    let before = z.f.owner.resource_usage();
    assert_eq!(
        z.f.owner.request_owned_annotation_save_v1(
            &req,
            &ann_release(&z.c),
            Some(&ann_admission()),
            None,
            None,
            &mut z.f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    );
    assert_eq!(z.f.owner.resource_usage(), before);
    let causal =
        z.f.owner
            .register_provider_with_causal_capabilities(
                ProviderProfile {
                    synchronization_supported: false,
                },
                ProviderDestinationCapabilitiesV1::default(),
                ProviderCausalCapabilitiesV1 {
                    causal_receipts: true,
                    non_reuse_tokens: false,
                },
                None,
                &mut z.f.recorder,
            )
            .unwrap();
    let typed = causal
        .read_result_causal(
            b"comments",
            ResourceVersionV1 {
                epoch: b"epoch",
                token: b"v0",
            },
            b"{}",
        )
        .unwrap();
    let req = ann_request(&z.d, z.set, &z.encoding, &typed, &z.source, b"annotations");
    let before = z.f.owner.resource_usage();
    assert_eq!(
        z.f.owner.request_owned_annotation_save_v1(
            &req,
            &ann_release(&z.c),
            Some(&ann_admission()),
            None,
            None,
            &mut z.f.recorder
        ),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(z.f.owner.resource_usage(), before);
    let replacement =
        z.f.owner
            .register_annotation_encoder(&mut z.f.recorder)
            .unwrap();
    let replacement_context = replacement.context();
    for kind in 0..5 {
        let encoded = candidate(&mut z.f, &z.e, &z.d, z.set);
        let pair = z.f.pair();
        let mut req = ann_request(
            &z.d,
            z.set,
            &z.encoding,
            &z.destination,
            &z.source,
            b"annotations",
        );
        let expected = match kind {
            0 => {
                req.expected_set += 1;
                ResourceError::Stale
            }
            1 => {
                req.encoding = &replacement_context;
                ResourceError::Stale
            }
            2 => {
                req.expected_annotation_binding += 1;
                ResourceError::Stale
            }
            3 => {
                req.condition = SaveOperationConditionV1::OwnChain;
                ResourceError::Unsupported
            }
            4 => {
                req.requirements.protection = Protection::AtomicConditional;
                ResourceError::Unsupported
            }
            _ => unreachable!(),
        };
        let before = z.f.owner.resource_usage();
        let parent = z.f.owner.save_operation_usage_v1();
        assert_eq!(
            z.f.owner.request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                Some(&ann_admission()),
                Some(encoded),
                pair,
                &mut z.f.recorder
            ),
            Err(expected)
        );
        assert_eq!(z.f.owner.resource_usage(), before);
        assert_eq!(z.f.owner.save_operation_usage_v1(), parent);
    }
    assert_eq!(z.f.owner.save_operation_usage_v1().issued, 0);
    z.admit();
    let mut z = Setup::new();
    let root =
        z.f.owner
            .semantic_root_usage(&mut z.f.recorder)
            .unwrap()
            .total_bytes;
    z.f.owner
        .configure_semantic_root_budget(root, &mut z.f.recorder)
        .unwrap();
    let encoded = candidate(&mut z.f, &z.e, &z.d, z.set);
    let pair = z.f.pair();
    let req = ann_request(
        &z.d,
        z.set,
        &z.encoding,
        &z.destination,
        &z.source,
        b"annotations",
    );
    let before = z.f.owner.resource_usage();
    let parent = z.f.owner.save_operation_usage_v1();
    assert_eq!(
        z.f.owner.request_owned_annotation_save_v1(
            &req,
            &ann_release(&z.c),
            Some(&ann_admission()),
            Some(encoded),
            pair,
            &mut z.f.recorder
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(z.f.owner.resource_usage(), before);
    assert_eq!(z.f.owner.save_operation_usage_v1(), parent);
    assert_eq!(z.f.owner.save_operation_usage_v1().issued, 0);
}

#[test]
fn source_and_annotation_operations_share_one_namespace_and_lifetime_parent_envelope() {
    let mut f = Fixture::new();
    let mut l = limits();
    l.max_issued = 1;
    f.owner
        .configure_save_operations_v1(l, &mut f.recorder)
        .unwrap();
    let e = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let encoding = e.context();
    let d = f.doc.clone();
    let set = f
        .owner
        .annotation_set_version(&d.document, &mut f.recorder)
        .unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let c = context();
    let common = current();
    let a = admission(&common);
    let raw = request(&d, &source, b"same");
    f.owner
        .request_owned_source_save_v1(&raw, &release(&c), Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    let destination = f.port.read_result(b"comments", b"a0", b"{}").unwrap();
    for (operation, error) in [
        (b"same".as_slice(), ResourceError::Unavailable),
        (b"different".as_slice(), ResourceError::Limit),
    ] {
        let req = ann_request(&d, set, &encoding, &destination, &source, operation);
        let before = f.owner.resource_usage();
        assert_eq!(
            f.owner.request_owned_annotation_save_v1(
                &req,
                &ann_release(&c),
                Some(&ann_admission()),
                None,
                None,
                &mut f.recorder
            ),
            Err(error)
        );
        assert_eq!(f.owner.resource_usage(), before);
    }
    assert_eq!(f.owner.save_operation_usage_v1().issued, 1);
}

#[test]
fn annotation_parent_diagnostics_are_optional_bounded_and_private_through_tombstoning() {
    use vellum_engine::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for cap in [None, Some(0), Some(1), Some(128)] {
            let mut z = Setup::new();
            z.f.recorder = cap
                .map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                })
                .with_native_timing(timing);
            let encoded = candidate(&mut z.f, &z.e, &z.d, z.set);
            let pair = z.f.pair();
            let req = ann_request(
                &z.d,
                z.set,
                &z.encoding,
                &z.destination,
                &z.source,
                b"PRIVATE_ANNOTATION",
            );
            let id =
                z.f.owner
                    .request_owned_annotation_save_v1(
                        &req,
                        &ann_release(&z.c),
                        Some(&ann_admission()),
                        Some(encoded),
                        pair,
                        &mut z.f.recorder,
                    )
                    .unwrap()
                    .task;
            assert_eq!(
                z.f.owner
                    .request_owned_annotation_save_v1(
                        &req,
                        &ann_release(&z.c),
                        None,
                        None,
                        None,
                        &mut z.f.recorder
                    )
                    .unwrap()
                    .task,
                id
            );
            let artifact =
                z.f.owner
                    .save_outcome(&id, &mut z.f.recorder)
                    .unwrap()
                    .artifact;
            z.f.owner.stop_save(&id, &mut z.f.recorder).unwrap();
            z.f.owner.retire_save(&id, &mut z.f.recorder).unwrap();
            z.f.owner
                .retire_resource_artifact(&artifact, &mut z.f.recorder)
                .unwrap();
            let records: Vec<_> = std::iter::from_fn(|| z.f.recorder.pop_front()).collect();
            assert!(!format!("{records:?}").contains("PRIVATE"));
            if cap == Some(128) {
                for b in [
                    Boundary::SaveOperationAdmission,
                    Boundary::SaveOperationRecovery,
                    Boundary::SaveOperationRetirement,
                    Boundary::AnnotationSaveAdmission,
                ] {
                    assert!(records.iter().any(|r| r.boundary == b));
                }
                for boundary in [
                    Boundary::AnnotationSaveAdmission,
                    Boundary::SaveOperationAdmission,
                    Boundary::SaveOperationRecovery,
                ] {
                    assert!(records.iter().any(|r| r.boundary == boundary
                        && r.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            } else if cap == Some(1) {
                assert!(records.len() <= 1);
            } else {
                assert!(records.is_empty());
            }
            assert_eq!(z.f.owner.save_operation_usage_v1().retired, 1);
            assert_eq!(z.f.owner.resource_usage().held_artifact_bytes, 0);
        }
    }
}

#[test]
fn unbound_source_has_no_fabricated_source_association_in_an_owned_annotation_save() {
    let mut z = Setup::new();
    let d = z.f.owner.create(b"notes", &mut z.f.recorder).unwrap();
    let plan =
        z.f.owner
            .prepare_annotation_binding(
                &d.document,
                z.f.port
                    .read_result(b"notes-comments", b"v0", b"{}")
                    .unwrap(),
                &mut z.f.recorder,
            )
            .unwrap();
    z.f.owner
        .activate_annotation_binding(plan, None, &mut z.f.recorder)
        .unwrap();
    let set =
        z.f.owner
            .annotation_set_version(&d.document, &mut z.f.recorder)
            .unwrap();
    let encoded = candidate(&mut z.f, &z.e, &d, set);
    let destination =
        z.f.port
            .read_result(b"notes-comments", b"v0", b"{}")
            .unwrap();
    let mut req = ann_request(&d, set, &z.encoding, &destination, &z.source, b"notes");
    req.expected_source_binding = None;
    let id =
        z.f.owner
            .request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                Some(&ann_admission()),
                Some(encoded),
                None,
                &mut z.f.recorder,
            )
            .unwrap()
            .task;
    assert_eq!(
        z.f.owner
            .request_owned_annotation_save_v1(
                &req,
                &ann_release(&z.c),
                None,
                None,
                None,
                &mut z.f.recorder
            )
            .unwrap()
            .task,
        id
    );
    assert_eq!(
        z.f.owner
            .saved_state(&d.document, &mut z.f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unbound
    );
}
