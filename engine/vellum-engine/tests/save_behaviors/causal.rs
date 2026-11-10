use super::*;
use vellum_engine::resource::*;
fn version(token: &[u8]) -> ResourceVersionV1<'_> {
    ResourceVersionV1 {
        epoch: b"epoch",
        token,
    }
}
#[test]
fn causal_source_receipt_is_distinct_from_result_disposition_and_quiescence() {
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
                non_reuse_tokens: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let d = o
        .load_resource(
            p.read_result_causal(b"key", version(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    let id = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = o
        .dispatch_save(
            &id,
            p.read_result_causal(b"key", version(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.record_save_evidence(
            p.causal_edge(&dispatch, version(b"v0"), version(b"v1"))
                .unwrap(),
            &mut r
        ),
        Ok(EvidenceDisposition::Accepted)
    );
    let out = o.save_outcome(&id, &mut r).unwrap();
    assert_eq!(out.application, Application::Unknown);
    assert!(!out.no_future_effects && !out.quiescent);
    o.record_save_evidence(
        p.result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(version(b"v1")),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.record_save_evidence(p.no_future_effects(&dispatch).unwrap(), &mut r)
        .unwrap();
    o.publish_save_baseline(&id, &mut r).unwrap();
    assert_eq!(
        o.saved_state(&d.document, &mut r).unwrap().content,
        ContentComparison::Unchanged
    );
}
fn fixture(
    non_reuse_tokens: bool,
) -> (
    SourceCoordinator,
    Recorder,
    ProviderPort,
    vellum_engine::coordinator::DocumentVersion,
    SaveId,
    SaveDispatch,
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
                non_reuse_tokens,
            },
            None,
            &mut r,
        )
        .unwrap();
    let d = o
        .load_resource(
            p.read_result_causal(b"key", version(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    let id = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = o
        .dispatch_save(
            &id,
            p.read_result_causal(b"key", version(b"v0"), b"A").unwrap(),
            &mut r,
        )
        .unwrap();
    (o, r, p, d, id, dispatch)
}
#[test]
fn all_24_receipt_orders_preserve_independent_causal_result_and_release_facts() {
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    if [a, b, c, d]
                        .iter()
                        .enumerate()
                        .any(|(i, x)| [a, b, c, d][..i].contains(x))
                    {
                        continue;
                    }
                    let (mut o, mut r, p, doc, id, dispatch) = fixture(true);
                    let receipts = [
                        p.causal_edge(&dispatch, version(b"v0"), version(b"v1"))
                            .unwrap(),
                        p.result_causal(
                            &dispatch,
                            Application::Written,
                            1,
                            Synchronization::NotRequested,
                            Some(version(b"v1")),
                        )
                        .unwrap(),
                        p.no_future_effects(&dispatch).unwrap(),
                        p.quiescence(&dispatch).unwrap(),
                    ];
                    for index in [a, b, c, d] {
                        assert_eq!(
                            o.record_save_evidence(receipts[index].clone(), &mut r),
                            Ok(EvidenceDisposition::Accepted)
                        );
                        assert_eq!(
                            o.record_save_evidence(receipts[index].clone(), &mut r),
                            Ok(EvidenceDisposition::Duplicate)
                        );
                    }
                    let out = o.save_outcome(&id, &mut r).unwrap();
                    assert!(
                        out.causal_edge_available
                            && out.no_future_effects
                            && out.quiescent
                            && !out.contested
                    );
                    o.publish_save_baseline(&id, &mut r).unwrap();
                    let next = o.queue_pinned_save(&doc, requirements(), &mut r).unwrap();
                    let next = o
                        .dispatch_save(
                            &next,
                            p.read_result_causal(b"key", version(b"v1"), b"A").unwrap(),
                            &mut r,
                        )
                        .unwrap();
                    assert_eq!(next.expected_causal_version(), Some(version(b"v1")));
                    assert_eq!(next.condition_v1(), Err(ResourceError::Unsupported));
                    assert_eq!(next.expected_validator(), None);
                    assert_eq!(
                        next.condition_v2(),
                        SaveWriteConditionV2::Present {
                            bytes: b"A",
                            version: SaveVersionV1::Causal(version(b"v1"))
                        }
                    );
                }
            }
        }
    }
}
#[test]
fn causal_result_edge_disagreement_faults_in_both_orders_without_rewriting_the_first_facts() {
    for edge_first in [false, true] {
        for mode in [
            "token",
            "epoch",
            "not-written",
            "before-token",
            "before-epoch",
        ] {
            let (mut o, mut r, p, _, id, dispatch) = fixture(false);
            let before = ResourceVersionV1 {
                epoch: if mode == "before-epoch" {
                    b"other"
                } else {
                    b"epoch"
                },
                token: if mode == "before-token" {
                    b"other"
                } else {
                    b"v0"
                },
            };
            let after = ResourceVersionV1 {
                epoch: if mode == "epoch" { b"other" } else { b"epoch" },
                token: if mode == "token" { b"other" } else { b"v1" },
            };
            let edge = p.causal_edge(&dispatch, before, after).unwrap();
            let result = p
                .result_causal(
                    &dispatch,
                    if mode == "not-written" {
                        Application::NotWritten
                    } else {
                        Application::Written
                    },
                    if mode == "not-written" { 0 } else { 1 },
                    Synchronization::NotRequested,
                    Some(version(b"v1")),
                )
                .unwrap();
            let events = if edge_first {
                [edge, result]
            } else {
                [result, edge]
            };
            let dispositions: Vec<_> = events
                .into_iter()
                .map(|e| o.record_save_evidence(e, &mut r).unwrap())
                .collect();
            assert!(
                dispositions.contains(&EvidenceDisposition::Contradiction),
                "{edge_first} {mode}"
            );
            let out = o.save_outcome(&id, &mut r).unwrap();
            assert!(out.contested);
            assert_eq!(o.resource_usage().contested_tasks, 1);
            o.record_save_evidence(p.no_future_effects(&dispatch).unwrap(), &mut r)
                .unwrap();
            o.record_save_evidence(p.quiescence(&dispatch).unwrap(), &mut r)
                .unwrap();
            o.stop_save(&id, &mut r).unwrap();
            assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Retained));
            assert_eq!(
                o.publish_save_baseline(&id, &mut r),
                Err(ResourceError::Contested)
            );
        }
    }
}
#[test]
fn causal_conflict_witness_capacity_is_fixed_and_equal_facts_remain_duplicates_after_fault() {
    let (mut o, mut r, p, _, id, dispatch) = fixture(true);
    let good = p
        .causal_edge(&dispatch, version(b"v0"), version(b"v1"))
        .unwrap();
    o.record_save_evidence(good.clone(), &mut r).unwrap();
    let result = p
        .result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(version(b"v1")),
        )
        .unwrap();
    o.record_save_evidence(result.clone(), &mut r).unwrap();
    for token in [b"bad1".as_slice(), b"bad2".as_slice()] {
        let bad = p
            .causal_edge(&dispatch, version(b"v0"), version(token))
            .unwrap();
        assert_eq!(
            o.record_save_evidence(bad.clone(), &mut r),
            Ok(EvidenceDisposition::Contradiction)
        );
        assert_eq!(
            o.record_save_evidence(bad, &mut r),
            Ok(EvidenceDisposition::Duplicate)
        );
    }
    let before = o.save_outcome(&id, &mut r).unwrap();
    assert_eq!(before.causal_conflict_witnesses, 2);
    assert_eq!(
        o.record_save_evidence(
            p.causal_edge(&dispatch, version(b"v0"), version(b"bad3"))
                .unwrap(),
            &mut r
        ),
        Err(ResourceError::EvidenceCapacity)
    );
    assert_eq!(o.save_outcome(&id, &mut r).unwrap(), before);
    assert_eq!(
        o.record_save_evidence(good, &mut r),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        o.record_save_evidence(result, &mut r),
        Ok(EvidenceDisposition::Duplicate)
    );
}
#[test]
fn same_token_in_a_different_result_epoch_is_a_conflict_even_without_a_causal_edge() {
    let (mut o, mut r, p, _, id, dispatch) = fixture(true);
    let first = p
        .result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(version(b"v1")),
        )
        .unwrap();
    assert_eq!(
        o.record_save_evidence(first.clone(), &mut r),
        Ok(EvidenceDisposition::Accepted)
    );
    let other = p
        .result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(ResourceVersionV1 {
                epoch: b"other",
                token: b"v1",
            }),
        )
        .unwrap();
    assert_eq!(
        o.record_save_evidence(other.clone(), &mut r),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert_eq!(
        o.record_save_evidence(other, &mut r),
        Ok(EvidenceDisposition::Duplicate)
    );
    assert_eq!(
        o.record_save_evidence(first, &mut r),
        Ok(EvidenceDisposition::Duplicate)
    );
    let out = o.save_outcome(&id, &mut r).unwrap();
    assert!(out.contested && !out.causal_edge_available);
    assert_eq!(out.causal_conflict_witnesses, 1); // shared typed result/edge cap
    let edge = p
        .causal_edge(&dispatch, version(b"v0"), version(b"different"))
        .unwrap();
    assert_eq!(
        o.record_save_evidence(edge, &mut r),
        Ok(EvidenceDisposition::Contradiction)
    );
    let full = o.save_outcome(&id, &mut r).unwrap();
    assert_eq!(full.causal_conflict_witnesses, 2);
    assert_eq!(
        o.record_save_evidence(
            p.causal_edge(&dispatch, version(b"v0"), version(b"third"))
                .unwrap(),
            &mut r
        ),
        Err(ResourceError::EvidenceCapacity)
    );
    assert_eq!(o.save_outcome(&id, &mut r).unwrap(), full);
}
#[test]
fn epoch_changes_and_equal_tokens_are_facts_not_implicit_permission_to_advance_a_chain() {
    for after in [
        version(b"v0"),
        ResourceVersionV1 {
            epoch: b"new-epoch",
            token: b"v1",
        },
    ] {
        let (mut o, mut r, p, _, id, dispatch) = fixture(false);
        o.record_save_evidence(
            p.causal_edge(&dispatch, version(b"v0"), after).unwrap(),
            &mut r,
        )
        .unwrap();
        o.record_save_evidence(
            p.result_causal(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(after),
            )
            .unwrap(),
            &mut r,
        )
        .unwrap();
        let out = o.save_outcome(&id, &mut r).unwrap();
        assert!(out.causal_edge_available && !out.contested);
        assert_eq!(dispatch.expected_causal_version(), Some(version(b"v0")));
        assert_eq!(o.resource_usage().issued_lifetime, 1);
    }
}
#[test]
fn independent_capabilities_and_opaque_compatibility_cannot_invent_an_epoch_or_downgrade_dispatch()
{
    let mut o = owner();
    let mut r = Recorder::disabled();
    let plain = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: true,
            },
            None,
            &mut r,
        )
        .unwrap();
    assert_eq!(
        plain
            .read_result_causal(b"key", version(b"v0"), b"A")
            .unwrap_err(),
        ResourceError::Unsupported
    );
    let nonreuse_only = o
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1 {
                exclusive_create: true,
                atomic_conditional_write: true,
            },
            ProviderCausalCapabilitiesV1 {
                causal_receipts: false,
                non_reuse_tokens: true,
            },
            None,
            &mut r,
        )
        .unwrap();
    assert_eq!(
        nonreuse_only
            .read_result_causal(b"key", version(b"v0"), b"A")
            .unwrap_err(),
        ResourceError::Unsupported
    );
    let (mut o, mut r, p, _, id, dispatch) = fixture(false);
    assert_eq!(
        p.result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"v1")
        )
        .unwrap_err(),
        ResourceError::Unsupported
    );
    assert_eq!(
        plain
            .causal_edge(&dispatch, version(b"v0"), version(b"v1"))
            .unwrap_err(),
        ResourceError::Unavailable
    );
    assert!(!o.save_outcome(&id, &mut r).unwrap().version_available);
    let queued = o
        .queue_pinned_save(
            &o.read(
                &o.save_outcome(&id, &mut r).unwrap().captured.document,
                &mut r,
            )
            .unwrap()
            .version,
            requirements(),
            &mut r,
        )
        .unwrap();
    o.record_save_evidence(
        p.result_causal(
            &dispatch,
            Application::NotWritten,
            0,
            Synchronization::NotRequested,
            None,
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.record_save_evidence(p.no_future_effects(&dispatch).unwrap(), &mut r)
        .unwrap();
    assert_eq!(
        o.dispatch_save(&queued, p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
            .unwrap_err(),
        ResourceError::Conflict
    );
    assert!(
        o.dispatch_save(
            &queued,
            p.read_result_causal(b"key", version(b"v0"), b"A").unwrap(),
            &mut r
        )
        .is_ok()
    );
}
#[test]
fn typed_epoch_and_token_fields_have_separate_limits_and_partial_results_do_not_guess_versions() {
    let (mut o, mut r, p, _, id, dispatch) = fixture(true);
    for v in [
        ResourceVersionV1 {
            epoch: b"",
            token: b"v0",
        },
        ResourceVersionV1 {
            epoch: b"epoch",
            token: b"",
        },
        ResourceVersionV1 {
            epoch: &[0; 65],
            token: b"v0",
        },
        ResourceVersionV1 {
            epoch: b"epoch",
            token: &[0; 65],
        },
    ] {
        assert!(p.read_result_causal(b"key", v, b"A").is_err());
        assert!(p.causal_edge(&dispatch, version(b"v0"), v).is_err());
    }
    o.record_save_evidence(
        p.result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            None,
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.record_save_evidence(
        p.causal_edge(&dispatch, version(b"v0"), version(b"v1"))
            .unwrap(),
        &mut r,
    )
    .unwrap();
    o.publish_save_baseline(&id, &mut r).unwrap();
    assert!(!o.save_outcome(&id, &mut r).unwrap().version_available);
    o.record_save_evidence(
        p.result_causal(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(version(b"v1")),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.publish_save_baseline(&id, &mut r).unwrap();
    assert!(o.save_outcome(&id, &mut r).unwrap().version_available);
}
#[test]
fn causal_intake_remains_protected_and_capture_is_bounded_optional_and_private() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, mut r, p, d, id, dispatch) = fixture(true);
        o.set_read_only(&d.document, true, &mut r).unwrap();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        o.record_save_evidence(
            p.causal_edge(&dispatch, version(b"v0"), version(b"v1"))
                .unwrap(),
            &mut r,
        )
        .unwrap();
        o.record_save_evidence(
            p.result_causal(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(version(b"v1")),
            )
            .unwrap(),
            &mut r,
        )
        .unwrap();
        assert!(o.save_outcome(&id, &mut r).unwrap().causal_edge_available);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{records:?}").contains("epoch"));
        if capacity == Some(128) {
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::CausalSaveEvidence)
            );
            assert!(records.iter().any(|v| v.boundary == Boundary::SaveEvidence));
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        assert!(!format!("{:?}", dispatch.condition_v2()).contains("epoch"));
    }
}
