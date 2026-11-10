use super::*;
use vellum_engine::coordinator::{SourceContinuityLimitsV1, TrustedSourceContinuityLaunchV1};
use vellum_engine::resource::*;
fn v(token: &[u8]) -> ResourceVersionV1<'_> {
    ResourceVersionV1 {
        epoch: b"epoch",
        token,
    }
}
fn limits() -> SourceContinuityLimitsV1 {
    SourceContinuityLimitsV1 {
        max_edges: 8,
        max_derivation_bytes: 8192,
        max_receipt_bytes: 16384,
        max_retained_bytes: 65536,
    }
}
#[test]
fn own_chain_advances_only_the_launched_condition_and_keeps_the_original_source_capture() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let p = o
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
            &mut r,
        )
        .unwrap();
    o.configure_source_continuity_v1(limits(), &mut r).unwrap();
    let d = o
        .load_resource(
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    let first = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let second = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let dispatch = o
        .dispatch_save(
            &first,
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    for e in [
        p.causal_edge(&dispatch, v(b"v0"), v(b"v1")).unwrap(),
        p.result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(v(b"v1")),
        )
        .unwrap(),
        p.no_future_effects(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, &mut r).unwrap();
    }
    let dispatch = o
        .dispatch_own_chain_save_v1(
            &second,
            p.read_result_causal(b"key", v(b"v1"), b"A").unwrap(),
            None,
            &TrustedSourceContinuityLaunchV1 {
                causal_progression_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(dispatch.expected_causal_version(), Some(v(b"v1")));
    assert_eq!(dispatch.bytes(), b"A");
    assert_eq!(o.save_outcome(&second, &mut r).unwrap().captured, d);
    let proof = dispatch.continuity_receipt().unwrap();
    assert_eq!(proof.original(), v(b"v0"));
    assert_eq!(proof.derived(), v(b"v1"));
    assert_eq!(proof.source(), &d);
    assert_eq!(proof.edges().len(), 1);
    assert_eq!(proof.edges()[0].producer(), &first);
    assert_eq!(proof.edges()[0].before(), v(b"v0"));
    assert_eq!(proof.edges()[0].after(), v(b"v1"));
    assert!(proof.edges()[0].dispatch_serial() < proof.dispatch_serial());
    // Actual causal evidence correlates with the derived, NOT original, head.
    assert_eq!(
        o.record_save_evidence(
            p.causal_edge(&dispatch, v(b"v1"), v(b"v2")).unwrap(),
            &mut r
        ),
        Ok(EvidenceDisposition::Accepted)
    );
}
fn fixture(
    l: SourceContinuityLimitsV1,
    reuse: bool,
) -> (
    SourceCoordinator,
    Recorder,
    ProviderPort,
    vellum_engine::coordinator::DocumentVersion,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let p = o
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: reuse,
            },
            None,
            &mut r,
        )
        .unwrap();
    o.configure_source_continuity_v1(l, &mut r).unwrap();
    let d = o
        .load_resource(
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    (o, r, p, d)
}
fn launch(
    o: &mut SourceCoordinator,
    p: &ProviderPort,
    id: &SaveId,
    token: &[u8],
    r: &mut Recorder,
) -> Result<SaveDispatch, ResourceError> {
    o.dispatch_own_chain_save_v1(
        id,
        p.read_result_causal(b"key", v(token), b"A").unwrap(),
        None,
        &TrustedSourceContinuityLaunchV1 {
            causal_progression_allowed: Some(true),
        },
        r,
    )
}
fn done(
    o: &mut SourceCoordinator,
    p: &ProviderPort,
    dispatch: &SaveDispatch,
    before: &[u8],
    after: &[u8],
    r: &mut Recorder,
) {
    for e in [
        p.causal_edge(dispatch, v(before), v(after)).unwrap(),
        p.result_causal(
            dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::NotRequested,
            Some(v(after)),
        )
        .unwrap(),
        p.no_future_effects(dispatch).unwrap(),
        p.quiescence(dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, r).unwrap();
    }
}
#[test]
fn chained_own_writes_keep_later_source_capture_and_original_anchor_with_ordered_receipts() {
    let (mut o, mut r, p, d) = fixture(limits(), true);
    let first = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let second = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let current = o.read(&d.document, &mut r).unwrap().version;
    let third = o
        .queue_own_chain_save_v1(&current, requirements(), None, &mut r)
        .unwrap();
    let a = o
        .dispatch_save(
            &first,
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    done(&mut o, &p, &a, b"v0", b"v1", &mut r);
    drop(a);
    assert_eq!(o.retire_save(&first, &mut r), Err(ResourceError::Retained));
    let b = launch(&mut o, &p, &second, b"v1", &mut r).unwrap();
    done(&mut o, &p, &b, b"v1", b"v2", &mut r);
    o.publish_save_baseline(&second, &mut r).unwrap();
    drop(b);
    let c = launch(&mut o, &p, &third, b"v2", &mut r).unwrap();
    assert_eq!(c.bytes(), b"B");
    assert_eq!(c.expected_bytes(), Some(b"A".as_slice()));
    let proof = c.continuity_receipt().unwrap();
    assert_eq!(proof.original(), v(b"v0"));
    assert_eq!(proof.derived(), v(b"v2"));
    assert_eq!(proof.source(), &current);
    assert_eq!(proof.edges().len(), 2);
    assert_eq!(proof.edges()[0].producer(), &first);
    assert_eq!(proof.edges()[1].producer(), &second);
    o.retire_save(&first, &mut r).unwrap(); // proof values no longer need the producer
    assert_eq!(c.continuity_receipt().unwrap().edges().len(), 2);
}
#[test]
fn known_nonapplication_is_skipped_but_quiescence_unknown_or_missing_causal_edges_cannot_advance() {
    for mode in ["skip", "unknown", "no-future", "no-edge"] {
        let (mut o, mut r, p, d) = fixture(limits(), true);
        let a = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let b = o
            .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
            .unwrap();
        let a = o
            .dispatch_save(
                &a,
                p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        o.record_save_evidence(p.quiescence(&a).unwrap(), &mut r)
            .unwrap();
        if mode != "unknown" {
            o.record_save_evidence(
                p.result_causal(
                    &a,
                    if mode == "skip" {
                        Application::NotWritten
                    } else {
                        Application::Written
                    },
                    if mode == "skip" { 0 } else { 1 },
                    Synchronization::NotRequested,
                    if mode == "skip" { None } else { Some(v(b"v1")) },
                )
                .unwrap(),
                &mut r,
            )
            .unwrap();
        }
        if mode != "no-future" {
            o.record_save_evidence(p.no_future_effects(&a).unwrap(), &mut r)
                .unwrap();
        }
        let before = o.resource_usage();
        let out = launch(
            &mut o,
            &p,
            &b,
            if mode == "skip" { b"v0" } else { b"v1" },
            &mut r,
        );
        if mode == "skip" {
            assert_eq!(out.unwrap().continuity_receipt().unwrap().edges().len(), 0);
        } else {
            assert_eq!(
                out.unwrap_err(),
                if mode == "no-edge" {
                    ResourceError::MissingEvidence
                } else {
                    ResourceError::Blocked
                }
            );
            assert_eq!(o.resource_usage(), before);
            assert_eq!(o.save_outcome(&b, &mut r).unwrap().dispatch_serial, None);
        }
    }
}
#[test]
fn advancement_requires_both_capabilities_current_permission_and_matching_final_probe_without_legacy_bypass()
 {
    for mode in ["nonreuse", "permission", "probe", "legacy"] {
        let (mut o, mut r, p, d) = fixture(limits(), mode != "nonreuse");
        let a = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let b = o
            .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
            .unwrap();
        let a = o
            .dispatch_save(
                &a,
                p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        done(&mut o, &p, &a, b"v0", b"v1", &mut r);
        let before = o.resource_usage();
        let result = if mode == "legacy" {
            o.dispatch_save(
                &b,
                p.read_result_causal(b"key", v(b"v1"), b"A").unwrap(),
                &mut r,
            )
        } else {
            o.dispatch_own_chain_save_v1(
                &b,
                p.read_result_causal(b"key", v(if mode == "probe" { b"v9" } else { b"v1" }), b"A")
                    .unwrap(),
                None,
                &TrustedSourceContinuityLaunchV1 {
                    causal_progression_allowed: if mode == "permission" {
                        Some(false)
                    } else {
                        Some(true)
                    },
                },
                &mut r,
            )
        };
        assert_eq!(
            result.unwrap_err(),
            match mode {
                "nonreuse" => ResourceError::Unsupported,
                "permission" => ResourceError::Permission,
                "probe" => ResourceError::Conflict,
                _ => ResourceError::Unavailable,
            }
        );
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.save_outcome(&b, &mut r).unwrap().dispatch_serial, None);
    }
}
#[test]
fn epoch_changes_equal_token_edges_and_observed_token_cycles_refuse_without_refreshing_intent() {
    for mode in ["epoch", "same", "cycle"] {
        let (mut o, mut r, p, d) = fixture(limits(), true);
        let a = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let mid = if mode == "cycle" {
            Some(
                o.queue_own_chain_save_v1(&d, requirements(), None, &mut r)
                    .unwrap(),
            )
        } else {
            None
        };
        let b = o
            .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
            .unwrap();
        let a = o
            .dispatch_save(
                &a,
                p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        let after = match mode {
            "epoch" => ResourceVersionV1 {
                epoch: b"new-epoch",
                token: b"v1",
            },
            "same" => v(b"v0"),
            _ => v(b"v1"),
        };
        for e in [
            p.causal_edge(&a, v(b"v0"), after).unwrap(),
            p.result_causal(
                &a,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(after),
            )
            .unwrap(),
            p.no_future_effects(&a).unwrap(),
        ] {
            o.record_save_evidence(e, &mut r).unwrap();
        }
        if let Some(mid) = mid {
            let mid = launch(&mut o, &p, &mid, b"v1", &mut r).unwrap();
            done(&mut o, &p, &mid, b"v1", b"v0", &mut r);
        }
        let before = o.resource_usage();
        assert_eq!(
            o.dispatch_own_chain_save_v1(
                &b,
                p.read_result_causal(b"key", after, b"A").unwrap(),
                None,
                &TrustedSourceContinuityLaunchV1 {
                    causal_progression_allowed: Some(true)
                },
                &mut r
            )
            .unwrap_err(),
            ResourceError::Conflict
        );
        assert_eq!(o.resource_usage(), before);
    }
}
#[test]
fn minimum_budget_is_reserved_before_artifact_issuance_and_edge_expansion_is_checked_before_launch()
{
    for mode in ["derivation", "receipt", "aggregate", "edges"] {
        let mut l = limits();
        match mode {
            "derivation" => l.max_derivation_bytes = 0,
            "receipt" => l.max_receipt_bytes = 0,
            "aggregate" => l.max_retained_bytes = 0,
            _ => l.max_edges = 0,
        };
        let (mut o, mut r, p, d) = fixture(l, true);
        let predecessor = if mode == "edges" {
            Some(o.queue_pinned_save(&d, requirements(), &mut r).unwrap())
        } else {
            None
        };
        let before = o.resource_usage();
        let b = o.queue_own_chain_save_v1(&d, requirements(), None, &mut r);
        if mode != "edges" {
            assert_eq!(b, Err(ResourceError::Limit));
            assert_eq!(o.resource_usage(), before);
        } else {
            let b = b.unwrap();
            let a = o
                .dispatch_save(
                    &predecessor.unwrap(),
                    p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
                    &mut r,
                )
                .unwrap();
            done(&mut o, &p, &a, b"v0", b"v1", &mut r);
            let before = o.resource_usage();
            assert_eq!(
                launch(&mut o, &p, &b, b"v1", &mut r).unwrap_err(),
                ResourceError::Limit
            );
            assert_eq!(o.resource_usage(), before);
            assert_eq!(o.save_outcome(&b, &mut r).unwrap().dispatch_serial, None);
        }
    }
}
#[test]
fn shared_credit_is_not_refunded_by_stop_and_expansion_waits_for_actual_retirement() {
    let (mut trial, mut tr, _, td) = fixture(limits(), true);
    trial
        .queue_own_chain_save_v1(&td, requirements(), None, &mut tr)
        .unwrap();
    let minimum = trial.resource_usage().continuity_retained_bytes;
    let mut l = limits();
    l.max_retained_bytes = minimum * 2;
    let (mut o, mut r, p, d) = fixture(l, true);
    let a = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let b = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let spare = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let a = o
        .dispatch_save(
            &a,
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    done(&mut o, &p, &a, b"v0", b"v1", &mut r);
    assert_eq!(
        launch(&mut o, &p, &b, b"v1", &mut r).unwrap_err(),
        ResourceError::Limit
    );
    o.stop_save(&spare, &mut r).unwrap();
    assert_eq!(o.resource_usage().continuity_retained_bytes, minimum * 2);
    assert_eq!(
        launch(&mut o, &p, &b, b"v1", &mut r).unwrap_err(),
        ResourceError::Limit
    );
    o.retire_save(&spare, &mut r).unwrap();
    assert_eq!(o.resource_usage().continuity_retained_bytes, minimum);
    let b = launch(&mut o, &p, &b, b"v1", &mut r).unwrap();
    assert_eq!(b.continuity_receipt().unwrap().edges().len(), 1);
    assert!(o.resource_usage().continuity_retained_bytes > minimum);
    assert!(o.resource_usage().continuity_retained_bytes <= l.max_retained_bytes);
}
#[test]
fn zero_edge_base_case_and_protected_retirement_preserve_credit_until_delivery_drain() {
    let (mut o, mut r, p, d) = fixture(limits(), false);
    let id = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let credit = o.resource_usage().continuity_retained_bytes;
    let dispatch = o
        .dispatch_own_chain_save_v1(
            &id,
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            None,
            &TrustedSourceContinuityLaunchV1 {
                causal_progression_allowed: None,
            },
            &mut r,
        )
        .unwrap();
    assert!(dispatch.continuity_receipt().unwrap().edges().is_empty());
    for e in [
        p.result_causal(
            &dispatch,
            Application::NotWritten,
            0,
            Synchronization::NotRequested,
            None,
        )
        .unwrap(),
        p.no_future_effects(&dispatch).unwrap(),
        p.quiescence(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, &mut r).unwrap();
    }
    assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Retained));
    assert_eq!(o.resource_usage().continuity_retained_bytes, credit);
    drop(dispatch);
    o.retire_save(&id, &mut r).unwrap();
    assert_eq!(o.resource_usage().continuity_retained_bytes, 0);
    assert_eq!(o.resource_usage().issued_lifetime, 1);
    assert_eq!(o.configure_source_continuity_v1(limits(), &mut r), Ok(()));
    let mut changed = limits();
    changed.max_edges += 1;
    assert_eq!(
        o.configure_source_continuity_v1(changed, &mut r),
        Err(ResourceError::Unsupported)
    );
}
#[test]
fn opaque_versions_do_not_acquire_own_chain_semantics_and_diagnostics_remain_private() {
    use vellum_engine::instrumentation::Boundary;
    let mut o = owner();
    let mut r = Recorder::disabled();
    let p = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    o.configure_source_continuity_v1(limits(), &mut r).unwrap();
    let d = o
        .load_resource(p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    let before = o.resource_usage();
    assert_eq!(
        o.queue_own_chain_save_v1(&d, requirements(), None, &mut r),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(o.resource_usage(), before);
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _, p, d) = fixture(limits(), true);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = o
            .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
            .unwrap();
        let dispatch = launch(&mut o, &p, &id, b"v0", &mut r).unwrap();
        assert!(dispatch.continuity_receipt().unwrap().edges().is_empty());
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{events:?}").contains("epoch"));
        if capacity == Some(128) {
            for b in [
                Boundary::SourceContinuityAdmission,
                Boundary::SourceContinuityDerivation,
                Boundary::SaveDispatch,
            ] {
                assert!(events.iter().any(|e| e.boundary == b));
            }
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn a_newer_unpublished_acknowledgment_cannot_erase_a_typed_roots_required_write_prefix() {
    let (mut o, mut r, p, d) = fixture(limits(), true);
    let a = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let ad = o
        .dispatch_save(
            &a,
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    done(&mut o, &p, &ad, b"v0", b"v1", &mut r);
    drop(ad);
    let b = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    // Both captured the original loaded condition; the scripted probe explicitly
    // attests it, rather than a claim of stronger external isolation.
    let bd = o
        .dispatch_save(
            &b,
            p.read_result_causal(b"key", v(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    done(&mut o, &p, &bd, b"v0", b"v2", &mut r);
    drop(bd);
    assert_eq!(o.retire_save(&a, &mut r), Err(ResourceError::Retained));
    o.publish_save_baseline(&b, &mut r).unwrap();
    o.retire_save(&a, &mut r).unwrap();
    let own = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let own = launch(&mut o, &p, &own, b"v2", &mut r).unwrap();
    assert_eq!(own.continuity_receipt().unwrap().anchor_serial(), 2);
    assert_eq!(own.continuity_receipt().unwrap().original(), v(b"v2"));
    assert!(own.continuity_receipt().unwrap().edges().is_empty());
}
#[test]
fn a_known_alias_write_from_another_registration_cannot_be_adopted_as_own_continuity() {
    let (mut o, mut r, p, d) = fixture(limits(), true);
    let alias = o
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: true,
            },
            Some(&p.identity_domain()),
            &mut r,
        )
        .unwrap();
    let g = vellum_engine::coordinator::TrustedSourceExportFactsV1 {
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
        create_allowed: None,
        overwrite_allowed: Some(true),
    };
    let format = o.source_serialization(&d.document, &mut r).unwrap();
    let prepared = o.prepare_source_artifact(&d, format, &mut r).unwrap();
    let copy = o
        .queue_source_export(
            &prepared,
            alias
                .read_result_causal(b"key", v(b"v0"), b"A")
                .unwrap()
                .into_source_export_destination(),
            SourceExportOptionsV1 {
                purpose: SourceExportPurposeV1::CopyExport,
                requirements: requirements(),
                pair: None,
            },
            &g,
            &mut r,
        )
        .unwrap();
    let own = o
        .queue_own_chain_save_v1(&d, requirements(), None, &mut r)
        .unwrap();
    let copy = o
        .dispatch_source_export(
            &copy,
            alias
                .read_result_causal(b"key", v(b"v0"), b"A")
                .unwrap()
                .into_source_export_destination(),
            None,
            &g,
            &mut r,
        )
        .unwrap();
    done(&mut o, &alias, &copy, b"v0", b"v1", &mut r);
    let before = o.resource_usage();
    assert_eq!(
        launch(&mut o, &p, &own, b"v1", &mut r).unwrap_err(),
        ResourceError::Conflict
    );
    assert_eq!(o.resource_usage(), before);
    assert_eq!(o.save_outcome(&own, &mut r).unwrap().dispatch_serial, None);
}
