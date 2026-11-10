use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::resource::*;
use vellum_engine::source::Replacement;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
use support::Fixture;
#[path = "save_operation_behaviors/annotations.rs"]
mod annotations;
#[path = "support/resource_catalog_fixture.rs"]
#[allow(dead_code)]
mod catalog_support;
#[path = "save_operation_behaviors/decoder.rs"]
mod decoder;
#[path = "support/discovered_context_fixture.rs"]
mod discovery_support;
#[path = "support/native_operation_discovery_fixture.rs"]
mod installed_observation;
#[path = "save_operation_behaviors/observation.rs"]
mod observation;
#[path = "save_operation_behaviors/read_observation.rs"]
mod read_observation;
#[path = "save_operation_behaviors/read_operations.rs"]
mod read_operations;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "authority".into(),
        engine: "engine".into(),
        recipient: "recipient".into(),
        purpose: "save".into(),
        destination: "destination".into(),
        profile: "native/1".into(),
        continuity: "epoch".into(),
    }
}
fn limits() -> SaveOperationLimitsV1 {
    SaveOperationLimitsV1 {
        max_issued: 8,
        max_label_bytes: 64,
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_store_bytes: 131072,
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedSaveOperationReleaseV1<'_> {
    TrustedSaveOperationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        write_allowed: Some(true),
        source_readable: Some(true),
        disposition_readable: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
    }
}
fn current() -> TrustedSourceExportFactsV1 {
    TrustedSourceExportFactsV1 {
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
        destination_allowed: None,
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: None,
    }
}
fn admission(f: &TrustedSourceExportFactsV1) -> TrustedOrdinarySourceSaveAdmissionV1<'_> {
    TrustedOrdinarySourceSaveAdmissionV1 {
        current: f,
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
    }
}
fn request<'a>(
    d: &'a DocumentVersion,
    target: &'a SourceBindingTargetV1,
    operation: &'a [u8],
) -> OwnedSourceSaveRequestV1<'a> {
    OwnedSourceSaveRequestV1 {
        origin: SaveOperationOriginV1 {
            operation,
            effect: b"write",
            attempt: b"one",
        },
        source: d,
        expected_serialization: 1,
        expected_binding: 0,
        target,
        requirements: SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        },
        condition: SaveOperationConditionV1::Pinned,
    }
}
#[test]
fn matching_save_operation_recovers_one_original_capture_without_dispatch_or_new_issuance() {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    let c = context();
    let g = release(&c);
    let common = current();
    let a = admission(&common);
    let target = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let r = request(&f.doc, &target, b"save");
    let generation = f.owner.generation();
    let first = f
        .owner
        .request_owned_source_save_v1(&r, &g, Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    assert_eq!(first.status, SaveOperationAdmissionStatusV1::Admitted);
    let before = f.owner.resource_usage();
    let again = f
        .owner
        .request_owned_source_save_v1(&r, &g, None, None, &mut f.recorder)
        .unwrap();
    assert_eq!(
        again.status,
        SaveOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(again.task, first.task);
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner
            .save_outcome(&first.task, &mut f.recorder)
            .unwrap()
            .phase,
        SavePhase::Queued
    );
}

#[test]
fn changed_complete_intent_or_original_scope_never_retargets_the_parent() {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    let c = context();
    let g = release(&c);
    let common = current();
    let a = admission(&common);
    let target = f.port.source_binding_target(b"source").unwrap();
    let foreign_target = f.port.source_binding_target(b"other").unwrap();
    let pair = f.pair();
    let d = f.doc.clone();
    let base = request(&d, &target, b"save");
    let admitted = f
        .owner
        .request_owned_source_save_v1(&base, &g, Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    for changed in 0..8 {
        let mut r = request(&d, &target, b"save");
        match changed {
            0 => r.origin.effect = b"different",
            1 => r.origin.attempt = b"two",
            2 => r.expected_serialization = 2,
            3 => r.expected_binding = 1,
            4 => r.target = &foreign_target,
            5 => r.requirements.acknowledgment = Acknowledgment::Synchronized,
            6 => r.condition = SaveOperationConditionV1::OwnChain,
            7 => r.requirements.protection = Protection::AtomicConditional,
            _ => unreachable!(),
        }
        assert_eq!(
            f.owner
                .request_owned_source_save_v1(&r, &g, Some(&a), pair.as_ref(), &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(f.owner.resource_usage(), before);
    }
    for field in 0..7 {
        let mut changed = c.clone();
        match field {
            0 => changed.authority.push('x'),
            1 => changed.engine.push('x'),
            2 => changed.recipient.push('x'),
            3 => changed.purpose.push('x'),
            4 => changed.destination.push('x'),
            5 => changed.profile.push('x'),
            6 => changed.continuity.push('x'),
            _ => unreachable!(),
        }
        assert_eq!(
            f.owner.request_owned_source_save_v1(
                &base,
                &release(&changed),
                None,
                None,
                &mut f.recorder
            ),
            Err(ResourceError::Unavailable)
        );
    }
    assert_eq!(
        f.owner
            .request_owned_source_save_v1(&base, &g, None, None, &mut f.recorder)
            .unwrap()
            .task,
        admitted.task
    );
}

#[test]
fn matching_lookup_survives_edits_and_admission_fences_but_never_recaptures_or_ignores_release_denial()
 {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    let c = context();
    let common = current();
    let a = admission(&common);
    let target = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let d = f.doc.clone();
    let req = request(&d, &target, b"save");
    let first = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    let p = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 4,
                    expected: "text".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(p, &mut f.recorder).unwrap();
    f.owner
        .set_read_only(&d.document, true, &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    for denied in [None, Some(false)] {
        for field in 0..12 {
            let mut g = release(&c);
            match field {
                0 => g.authorized = denied,
                1 => g.scope = denied,
                2 => g.continuous = denied,
                3 => g.live = denied,
                4 => g.not_stopped = denied,
                5 => g.write_allowed = denied,
                6 => g.source_readable = denied,
                7 => g.disposition_readable = denied,
                8 => g.retained = denied,
                9 => g.reply_ready = denied,
                10 => g.transfer_ready = denied,
                11 => g.resources_ready = denied,
                _ => unreachable!(),
            }
            assert_eq!(
                f.owner
                    .request_owned_source_save_v1(&req, &g, None, None, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
        }
    }
    let recovered = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), None, None, &mut f.recorder)
        .unwrap();
    assert_eq!(recovered.task, first.task);
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(
        f.owner
            .save_outcome(&first.task, &mut f.recorder)
            .unwrap()
            .captured,
        d
    );
    let current = f.owner.read(&d.document, &mut f.recorder).unwrap().version;
    let mut changed = req;
    changed.source = &current;
    assert_eq!(
        f.owner
            .request_owned_source_save_v1(&changed, &release(&c), None, None, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn stopped_retired_child_leaves_a_spent_parent_and_no_payload_or_reexecution() {
    let mut f = Fixture::new();
    let mut l = limits();
    l.max_issued = 1;
    f.owner
        .configure_save_operations_v1(l, &mut f.recorder)
        .unwrap();
    let c = context();
    let common = current();
    let a = admission(&common);
    let target = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let d = f.doc.clone();
    let req = request(&d, &target, b"save");
    let first = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.retire_save(&first.task, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let reserved = f.owner.save_operation_usage_v1().reserved_store_bytes;
    f.owner.stop_save(&first.task, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .request_owned_source_save_v1(&req, &release(&c), None, None, &mut f.recorder)
            .unwrap()
            .task,
        first.task
    );
    f.owner.retire_save(&first.task, &mut f.recorder).unwrap();
    let usage = f.owner.save_operation_usage_v1();
    assert_eq!((usage.issued, usage.owned, usage.retired), (1, 0, 1));
    assert_eq!(usage.reserved_store_bytes, reserved);
    assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
    assert_eq!(f.owner.resource_usage().issued_lifetime, 1);
    assert_eq!(
        f.owner.request_owned_source_save_v1(
            &req,
            &release(&c),
            Some(&a),
            pair.as_ref(),
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    let next = request(&d, &target, b"other");
    assert_eq!(
        f.owner.request_owned_source_save_v1(
            &next,
            &release(&c),
            Some(&a),
            pair.as_ref(),
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
}

#[test]
fn unknown_external_effect_cannot_be_retried_or_retired_by_recovery_stop_or_quiescence() {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    let c = context();
    let common = current();
    let a = admission(&common);
    let target = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let d = f.doc.clone();
    let req = request(&d, &target, b"save");
    let first = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    let pair = f.pair();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &first.task,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair.unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner.stop_save(&first.task, &mut f.recorder).unwrap();
    f.owner
        .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner
            .request_owned_source_save_v1(&req, &release(&c), None, None, &mut f.recorder)
            .unwrap()
            .task,
        first.task
    );
    assert_eq!(f.owner.resource_usage(), before);
    drop(dispatch);
    assert_eq!(
        f.owner.retire_save(&first.task, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(f.owner.save_operation_usage_v1().owned, 1);
}

#[test]
fn bounded_optional_operation_diagnostics_preserve_actual_admission_and_retirement_semantics() {
    use vellum_engine::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for cap in [None, Some(0), Some(1), Some(128)] {
            let mut f = Fixture::new();
            f.recorder = cap
                .map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                })
                .with_native_timing(timing);
            f.owner
                .configure_save_operations_v1(limits(), &mut f.recorder)
                .unwrap();
            let c = context();
            let common = current();
            let a = admission(&common);
            let target = f.port.source_binding_target(b"source").unwrap();
            let pair = f.pair();
            let d = f.doc.clone();
            let req = request(&d, &target, b"PRIVATE_ORIGIN");
            let first = f
                .owner
                .request_owned_source_save_v1(
                    &req,
                    &release(&c),
                    Some(&a),
                    pair.as_ref(),
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(
                f.owner
                    .request_owned_source_save_v1(&req, &release(&c), None, None, &mut f.recorder)
                    .unwrap()
                    .task,
                first.task
            );
            f.owner.stop_save(&first.task, &mut f.recorder).unwrap();
            f.owner.retire_save(&first.task, &mut f.recorder).unwrap();
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(!format!("{records:?}").contains("PRIVATE"));
            if cap == Some(128) {
                for boundary in [
                    Boundary::SaveOperationConfiguration,
                    Boundary::SaveOperationAdmission,
                    Boundary::SaveOperationRecovery,
                    Boundary::SaveOperationRetirement,
                ] {
                    assert!(records.iter().any(|r| r.boundary == boundary));
                }
                for boundary in [
                    Boundary::SaveAdmission,
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
            assert_eq!(f.owner.save_operation_usage_v1().retired, 1);
            assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
        }
    }
}

#[test]
fn each_fresh_native_admission_premise_is_exact_and_failure_spends_no_parent_or_child() {
    for denied in [None, Some(false)] {
        for field in 0..20 {
            let mut f = Fixture::new();
            f.owner
                .configure_save_operations_v1(limits(), &mut f.recorder)
                .unwrap();
            let c = context();
            let mut common = current();
            match field {
                0 => common.authorized = denied,
                1 => common.scope = denied,
                2 => common.continuous = denied,
                3 => common.live = denied,
                4 => common.not_stopped = denied,
                5 => common.source_readable = denied,
                6 => common.write_allowed = denied,
                7 => common.binding_coverage_complete = denied,
                8 => common.writer_coverage_complete = denied,
                9 => common.provider_available = denied,
                10 => common.resources_ready = denied,
                11 => common.launch_allowed = denied,
                12 => common.identity_allowed = denied,
                13 => common.companions_ready = denied,
                14 => common.versions_available = denied,
                15 => common.reply_ready = denied,
                _ => {}
            }
            let mut a = admission(&common);
            match field {
                16 => a.owner_control_allowed = denied,
                17 => a.preparation_reserved = denied,
                18 => a.ordinary_bindings_current = denied,
                19 => a.ordinary_identity_current = denied,
                _ => {}
            }
            let target = f.port.source_binding_target(b"source").unwrap();
            let pair = f.pair();
            let req = request(&f.doc, &target, b"save");
            let before = f.owner.resource_usage();
            let parent = f.owner.save_operation_usage_v1();
            assert_eq!(
                f.owner.request_owned_source_save_v1(
                    &req,
                    &release(&c),
                    Some(&a),
                    pair.as_ref(),
                    &mut f.recorder
                ),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.resource_usage(), before);
            assert_eq!(f.owner.save_operation_usage_v1(), parent);
        }
    }
}

#[test]
fn owned_own_chain_uses_the_same_actual_codec_and_credits_and_releases_only_on_retirement() {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    f.owner
        .configure_source_continuity_v1(
            SourceContinuityLimitsV1 {
                max_edges: 8,
                max_derivation_bytes: 8192,
                max_receipt_bytes: 16384,
                max_retained_bytes: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    let p = f
        .owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: true,
            },
            None,
            &mut f.recorder,
        )
        .unwrap();
    let d = f
        .owner
        .load_resource(
            p.read_result_causal(
                b"typed",
                ResourceVersionV1 {
                    epoch: b"epoch",
                    token: b"v0",
                },
                b"text",
            )
            .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let target = p.source_binding_target(b"typed").unwrap();
    let c = context();
    let common = current();
    let a = admission(&common);
    let mut req = request(&d, &target, b"save");
    req.condition = SaveOperationConditionV1::OwnChain;
    let first = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), Some(&a), None, &mut f.recorder)
        .unwrap();
    let charge = f.owner.resource_usage().continuity_retained_bytes;
    assert!(charge > 0);
    assert_eq!(
        f.owner
            .request_owned_source_save_v1(&req, &release(&c), None, None, &mut f.recorder)
            .unwrap()
            .task,
        first.task
    );
    assert!(
        f.owner
            .save_outcome(&first.task, &mut f.recorder)
            .unwrap()
            .own_chain
    );
    f.owner.stop_save(&first.task, &mut f.recorder).unwrap();
    assert_eq!(f.owner.resource_usage().continuity_retained_bytes, charge);
    f.owner.retire_save(&first.task, &mut f.recorder).unwrap();
    assert_eq!(f.owner.resource_usage().continuity_retained_bytes, 0);
    assert_eq!(f.owner.save_operation_usage_v1().retired, 1);
}

#[test]
fn rejected_child_admission_never_spends_or_retroactively_adopts_an_operation() {
    let mut f = Fixture::new();
    f.owner
        .configure_save_operations_v1(limits(), &mut f.recorder)
        .unwrap();
    let c = context();
    let common = current();
    let a = admission(&common);
    let target = f.port.source_binding_target(b"source").unwrap();
    let raw_pair = f.pair();
    let pair = f.pair();
    let d = f.doc.clone();
    let raw = f
        .owner
        .queue_pinned_save_with_pair(
            &d,
            request(&d, &target, b"unused").requirements,
            raw_pair.unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let before = f.owner.resource_usage();
    let parent = f.owner.save_operation_usage_v1();
    for invalid in 0..6 {
        let mut req = request(&d, &target, b"save");
        let expected = match invalid {
            0 => {
                req.expected_serialization = 2;
                ResourceError::Stale
            }
            1 => {
                req.expected_binding = 1;
                ResourceError::Stale
            }
            2 => {
                req.requirements.protection = Protection::AtomicConditional;
                ResourceError::Unsupported
            }
            3 => {
                req.requirements.acknowledgment = Acknowledgment::Synchronized;
                ResourceError::Unsupported
            }
            4 => {
                req.origin.operation = b"\xff";
                ResourceError::Malformed
            }
            5 => {
                req.origin.operation = b"";
                ResourceError::Limit
            }
            _ => unreachable!(),
        };
        assert_eq!(
            f.owner.request_owned_source_save_v1(
                &req,
                &release(&c),
                Some(&a),
                pair.as_ref(),
                &mut f.recorder
            ),
            Err(expected)
        );
        assert_eq!(f.owner.resource_usage(), before);
        assert_eq!(f.owner.save_operation_usage_v1(), parent);
    }
    let req = request(&d, &target, b"save");
    let admitted = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), Some(&a), pair.as_ref(), &mut f.recorder)
        .unwrap();
    assert_ne!(admitted.task, raw);
    assert_eq!(f.owner.resource_usage().issued_lifetime, 2);
    assert_eq!(f.owner.save_operation_usage_v1().issued, 1);
}
