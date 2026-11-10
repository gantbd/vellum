use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use fixture::Fixture;
#[path = "source_export_behaviors/creation.rs"]
mod creation;
#[path = "support/discovered_context_fixture.rs"]
mod discovery_support;
#[path = "source_export_behaviors/invocations.rs"]
mod invocations;
#[path = "support/invocation_observation_fixture.rs"]
#[allow(dead_code)]
mod observation_support;
#[path = "source_export_behaviors/rebind_facade.rs"]
mod rebind_facade;
#[path = "source_export_behaviors/saveas_facade.rs"]
mod saveas_facade;
#[path = "source_export_behaviors/saveas_invocations.rs"]
mod saveas_invocations;
fn facts() -> TrustedSourceExportFactsV1 {
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
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: Some(true),
        overwrite_allowed: Some(true),
    }
}
fn control() -> TrustedSourceExportControlV1 {
    TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    }
}
fn prepare(f: &mut Fixture, d: &DocumentVersion) -> PreparedSourceArtifactIdV1 {
    let s = f
        .owner
        .source_serialization(&d.document, &mut f.recorder)
        .unwrap();
    f.owner
        .prepare_source_artifact(d, s, &mut f.recorder)
        .unwrap()
}
fn requirements() -> SaveRequirements {
    SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    }
}
fn options(purpose: SourceExportPurposeV1) -> SourceExportOptionsV1 {
    SourceExportOptionsV1 {
        purpose,
        requirements: requirements(),
        pair: None,
    }
}
fn present(port: &ProviderPort, key: &[u8]) -> SourceExportDestinationV1 {
    port.read_result(key, b"v0", b"OLD")
        .unwrap()
        .into_source_export_destination()
}
fn queue(f: &mut Fixture, d: &DocumentVersion, key: &[u8]) -> SaveId {
    let p = prepare(f, d);
    f.owner
        .queue_source_export(
            &p,
            present(&f.port, key),
            options(SourceExportPurposeV1::CopyExport),
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
}

fn activation_facts() -> TrustedSourceSaveAsFactsV1 {
    TrustedSourceSaveAsFactsV1 {
        control: control(),
        probe_attested: Some(true),
        resource_continuity_attested: Some(true),
        create_allowed: Some(true),
        overwrite_allowed: Some(true),
    }
}
#[test]
fn absent_create_typed_after_version_establishes_a_baseline_without_a_fabricated_before_version() {
    let mut f = Fixture::new();
    let port = f
        .owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1 {
                exclusive_create: true,
                atomic_conditional_write: false,
            },
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: true,
            },
            None,
            &mut f.recorder,
        )
        .unwrap();
    let d = f.owner.create(b"new", &mut f.recorder).unwrap();
    let p = prepare(&mut f, &d);
    let id = f
        .owner
        .queue_source_export_v2(
            &p,
            port.absent_source_export_destination(b"new").unwrap(),
            SourceExportOptionsV2 {
                intent: options(SourceExportPurposeV1::SaveAs),
                result_version_profile: SaveResultVersionProfileV1::EpochToken,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            port.absent_source_export_destination(b"new").unwrap(),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(dispatch.condition_v2(), SaveWriteConditionV2::Absent);
    assert_eq!(dispatch.expected_causal_version(), None);
    assert_eq!(
        dispatch.result_version_profile(),
        SaveResultVersionProfileV1::EpochToken
    );
    assert_eq!(dispatch.condition_v1(), Err(ResourceError::Unsupported));
    let after = ResourceVersionV1 {
        epoch: b"epoch",
        token: b"v1",
    };
    f.owner
        .record_save_evidence(
            port.result_causal(
                &dispatch,
                Application::Written,
                3,
                Synchronization::NotRequested,
                Some(after),
            )
            .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                port.source_save_as_probe_causal(b"new", Some(after))
                    .unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
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
    let next = f
        .owner
        .queue_own_chain_save_v1(&d, requirements(), None, &mut f.recorder)
        .unwrap();
    let next = f
        .owner
        .dispatch_own_chain_save_v1(
            &next,
            port.read_result_causal(b"new", after, b"new").unwrap(),
            None,
            &TrustedSourceContinuityLaunchV1 {
                causal_progression_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    let proof = next.continuity_receipt().unwrap();
    assert_eq!(proof.original(), after);
    assert_eq!(proof.derived(), after);
    assert!(proof.edges().is_empty());
}
#[test]
fn typed_source_save_as_keeps_the_result_epoch_and_refuses_opaque_or_wrong_epoch_postwrite_probes()
{
    let mut f = Fixture::new();
    let port = f
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
    let before = ResourceVersionV1 {
        epoch: b"old-epoch",
        token: b"v0",
    };
    let after = ResourceVersionV1 {
        epoch: b"new-epoch",
        token: b"v1",
    };
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let p = prepare(&mut f, &d);
    let id = f
        .owner
        .queue_source_export(
            &p,
            port.read_result_causal(b"target", before, b"OLD")
                .unwrap()
                .into_source_export_destination(),
            options(SourceExportPurposeV1::SaveAs),
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            port.read_result_causal(b"target", before, b"OLD")
                .unwrap()
                .into_source_export_destination(),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    for e in [
        port.causal_edge(&dispatch, before, after).unwrap(),
        port.result_causal(
            &dispatch,
            Application::Written,
            4,
            Synchronization::NotRequested,
            Some(after),
        )
        .unwrap(),
        port.no_future_effects(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    assert_eq!(
        f.owner.activate_source_save_as(
            &id,
            port.source_save_as_probe(b"target", Some(b"v1")).unwrap(),
            None,
            0,
            &activation_facts(),
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    let wrong = ResourceVersionV1 {
        epoch: b"old-epoch",
        token: b"v1",
    };
    assert_eq!(
        f.owner.activate_source_save_as(
            &id,
            port.source_save_as_probe_causal(b"target", Some(wrong))
                .unwrap(),
            None,
            0,
            &activation_facts(),
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                port.source_save_as_probe_causal(b"target", Some(after))
                    .unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
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
    let prepared = prepare(&mut f, &d);
    let copy = f
        .owner
        .queue_source_export(
            &prepared,
            port.read_result_causal(b"target", after, b"data")
                .unwrap()
                .into_source_export_destination(),
            options(SourceExportPurposeV1::CopyExport),
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let next = f
        .owner
        .queue_own_chain_save_v1(&d, requirements(), None, &mut f.recorder)
        .unwrap();
    let copied = f
        .owner
        .dispatch_source_export(
            &copy,
            port.read_result_causal(b"target", after, b"data")
                .unwrap()
                .into_source_export_destination(),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let head = ResourceVersionV1 {
        epoch: after.epoch,
        token: b"v2",
    };
    for e in [
        port.causal_edge(&copied, after, head).unwrap(),
        port.result_causal(
            &copied,
            Application::Written,
            4,
            Synchronization::NotRequested,
            Some(head),
        )
        .unwrap(),
        port.no_future_effects(&copied).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    // Same-context copy keeps the real nonzero binding generation for continuity,
    // but never becomes an ordinary baseline producer or binding activation.
    assert_eq!(
        f.owner.publish_save_baseline(&copy, &mut f.recorder),
        Err(ResourceError::Unsupported)
    );
    let next = f
        .owner
        .dispatch_own_chain_save_v1(
            &next,
            port.read_result_causal(b"target", head, b"data").unwrap(),
            None,
            &TrustedSourceContinuityLaunchV1 {
                causal_progression_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(next.expected_causal_version(), Some(head));
    assert_eq!(next.continuity_receipt().unwrap().original(), after);
    assert_eq!(
        next.continuity_receipt().unwrap().edges()[0].producer(),
        &copy
    );
}
fn history_release() -> TrustedSourceBindingHistoryReleaseV1 {
    TrustedSourceBindingHistoryReleaseV1 {
        control: control(),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
        binding_history_release_allowed: Some(true),
        history_evidence_released: Some(true),
        snapshot_evidence_released: Some(true),
        read_evidence_released: Some(true),
        write_evidence_released: Some(true),
        provider_evidence_released: Some(true),
    }
}
fn rebind_facts() -> TrustedSourceRebindFactsV1 {
    TrustedSourceRebindFactsV1 {
        control: control(),
        destination_allowed: Some(true),
        rebind_allowed: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
    }
}
#[test]
fn pure_scratch_rebind_creates_an_association_but_no_baseline_writer_or_source_edit() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"prose and code", &mut f.recorder).unwrap();
    let generation = f.owner.generation();
    let mut before = f.owner.resource_usage();
    before.bindings += 1;
    assert!(
        f.owner
            .rebind_source_document(
                &d.document,
                f.port.source_binding_target(b"new").unwrap(),
                SourceRebindOptionsV1 {
                    expected_binding: None,
                    max_binding_history: 0,
                    pair: None
                },
                &rebind_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(f.owner.generation(), generation + 1);
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(
        f.owner.read(&d.document, &mut f.recorder).unwrap().version,
        d
    );
    assert_eq!(
        f.owner.saved_state(&d.document, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
    assert_eq!(
        f.owner
            .queue_pinned_save(&d, requirements(), &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
}
fn rebind(
    f: &mut Fixture,
    d: &DocumentVersion,
    key: &[u8],
    expected: Option<u64>,
    max: usize,
) -> Result<bool, ResourceError> {
    let target = f.port.source_binding_target(key).unwrap();
    let pair = f
        .owner
        .source_rebind_pair_check(&d.document, &target, &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        });
    f.owner.rebind_source_document(
        &d.document,
        target,
        SourceRebindOptionsV1 {
            expected_binding: expected,
            max_binding_history: max,
            pair,
        },
        &rebind_facts(),
        &mut f.recorder,
    )
}
#[test]
fn pure_rebind_archives_the_saved_producer_preserves_edit_history_and_does_not_mark_content_saved()
{
    let mut f = Fixture::new();
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let id = save_as(&mut f, &d, b"first");
    let dispatch = written(&mut f, &id, b"first", Some(b"v1"));
    f.owner
        .activate_source_save_as(
            &id,
            f.port.source_save_as_probe(b"first", Some(b"v1")).unwrap(),
            None,
            0,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 4,
                    expected: "data".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    let group = f
        .owner
        .commit(edit, &mut f.recorder)
        .unwrap()
        .history_group
        .unwrap();
    let current = f.owner.read(&d.document, &mut f.recorder).unwrap().version;
    let before = f.owner.resource_usage();
    assert_eq!(rebind(&mut f, &current, b"second", Some(1), 1), Ok(true));
    assert_eq!(
        f.owner.read(&d.document, &mut f.recorder).unwrap().version,
        current
    );
    assert_eq!(
        f.owner
            .read(&d.document, &mut f.recorder)
            .unwrap()
            .source
            .text(),
        "later"
    );
    let mut expected = before;
    expected.source_binding_history_entries += 1;
    assert_eq!(f.owner.resource_usage(), expected);
    assert_eq!(
        f.owner.retire_save(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner.saved_state(&d.document, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
    let reversal = f
        .owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut f.recorder)
        .unwrap();
    f.owner.commit(reversal, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .read(&d.document, &mut f.recorder)
            .unwrap()
            .source
            .text(),
        "data"
    );
    assert_eq!(
        f.owner.saved_state(&d.document, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
    // An explicit Save As into the same association establishes the new baseline
    // without another archive or invented destination read baseline.
    let current = f.owner.read(&d.document, &mut f.recorder).unwrap().version;
    let new = save_as(&mut f, &current, b"second");
    let _new_dispatch = written(&mut f, &new, b"second", Some(b"v2"));
    f.owner
        .activate_source_save_as(
            &new,
            f.port.source_save_as_probe(b"second", Some(b"v2")).unwrap(),
            None,
            1,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.owner.resource_usage().source_binding_history_entries, 1);
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    drop(dispatch);
}
#[test]
fn same_context_rebind_is_unchanged_and_stale_expectations_cannot_survive_a_binding_cycle() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let initial = f.owner.generation();
    assert_eq!(rebind(&mut f, &d, b"source", Some(0), 0), Ok(false));
    assert_eq!(f.owner.generation(), initial);
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(rebind(&mut f, &d, b"other", Some(0), 1), Ok(true));
    assert_eq!(rebind(&mut f, &d, b"source", Some(1), 2), Ok(true));
    assert_eq!(
        rebind(&mut f, &d, b"source", Some(0), 2),
        Err(ResourceError::Stale)
    );
    assert_eq!(f.owner.resource_usage().source_binding_history_entries, 2);
    let pair = f.pair();
    assert!(
        f.owner
            .release_source_binding_history(
                &d.document,
                Some(2),
                pair,
                2,
                &history_release(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(
        f.owner.saved_state(&d.document, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
}
#[test]
fn every_rebind_current_fact_is_exact_and_refuses_before_any_root_or_producer_change() {
    for index in 0..20 {
        for denied in [None, Some(false)] {
            let mut f = Fixture::new();
            let d = f.owner.create(b"data", &mut f.recorder).unwrap();
            let mut g = rebind_facts();
            *match index {
                0 => &mut g.control.authorized,
                1 => &mut g.control.scope,
                2 => &mut g.control.continuous,
                3 => &mut g.control.live,
                4 => &mut g.control.not_stopped,
                5 => &mut g.control.binding_coverage_complete,
                6 => &mut g.control.writer_coverage_complete,
                7 => &mut g.control.binding_activation_allowed,
                8 => &mut g.control.identity_allowed,
                9 => &mut g.control.companions_ready,
                10 => &mut g.control.versions_available,
                11 => &mut g.control.reply_ready,
                12 => &mut g.control.resources_ready,
                13 => &mut g.control.disposition_readable,
                14 => &mut g.destination_allowed,
                15 => &mut g.rebind_allowed,
                16 => &mut g.owner_control_allowed,
                17 => &mut g.preparation_reserved,
                18 => &mut g.ordinary_bindings_current,
                _ => &mut g.ordinary_identity_current,
            } = denied;
            let generation = f.owner.generation();
            let usage = f.owner.resource_usage();
            let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
            assert_eq!(
                f.owner.rebind_source_document(
                    &d.document,
                    f.port.source_binding_target(b"out").unwrap(),
                    SourceRebindOptionsV1 {
                        expected_binding: None,
                        max_binding_history: 0,
                        pair: None
                    },
                    &g,
                    &mut f.recorder
                ),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.resource_usage(), usage);
            assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
        }
    }
}
#[test]
fn rebind_refuses_pending_exports_and_unresolved_old_or_new_effects_without_using_quiescence_as_disposition()
 {
    for key in [b"source".as_slice(), b"next".as_slice()] {
        let mut f = Fixture::new();
        let d = f.doc.clone();
        let peer = queue(&mut f, &d, key);
        assert_eq!(
            rebind(&mut f, &d, b"next", Some(0), 1),
            Err(ResourceError::Retained)
        );
        f.owner.stop_save(&peer, &mut f.recorder).unwrap();
        assert_eq!(
            rebind(&mut f, &d, b"next", Some(0), 1),
            Err(ResourceError::Retained)
        );
        f.owner
            .release_source_export_reservation(&peer, &control(), &mut f.recorder)
            .unwrap();
        assert_eq!(rebind(&mut f, &d, b"next", Some(0), 1), Ok(true));
    }
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let pair = f.pair().unwrap();
    let task = f
        .owner
        .queue_pinned_save_with_pair(&d, requirements(), pair, &mut f.recorder)
        .unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &task,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        rebind(&mut f, &d, b"next", Some(0), 1),
        Err(ResourceError::Retained)
    );
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port.no_future_effects(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    // Settled I/O does not itself discharge the separately admitted baseline
    // promise. Release that proven impossible promise before moving its binding.
    assert_eq!(
        rebind(&mut f, &d, b"next", Some(0), 1),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner
            .release_source_save_capacity_v1(&task, &mut f.recorder)
            .unwrap(),
        vellum_engine::resource::BaselineCapacityState::NotWritten
    );
    assert_eq!(rebind(&mut f, &d, b"next", Some(0), 1), Ok(true));
}
#[test]
fn rebind_requires_resolved_distinct_ordinary_targets_and_never_steals_a_foreign_owner() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let peer = f
        .owner
        .load_resource(
            f.port.read_result(b"peer", b"p0", b"peer").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_ne!(peer.document, d.document);
    assert_eq!(
        rebind(&mut f, &d, b"peer", Some(0), 1),
        Err(ResourceError::Conflict)
    );
    assert_eq!(
        rebind(&mut f, &d, b"comments", Some(0), 1),
        Err(ResourceError::Conflict)
    );
    assert_eq!(
        f.owner.rebind_source_document(
            &d.document,
            f.port.source_binding_target(b"next").unwrap(),
            SourceRebindOptionsV1 {
                expected_binding: Some(0),
                max_binding_history: 1,
                pair: None
            },
            &rebind_facts(),
            &mut f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    );
    assert_eq!(
        rebind(&mut f, &d, b"next", Some(0), 0),
        Err(ResourceError::Limit)
    );
    assert_eq!(rebind(&mut f, &d, b"next", Some(0), 1), Ok(true));
}
#[test]
fn rebind_capture_is_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(128), Some(1)] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"PRIVATE", &mut f.recorder).unwrap();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        assert_eq!(rebind(&mut f, &d, b"PRIVATE_PATH", None, 0), Ok(true));
        assert_eq!(rebind(&mut f, &d, b"PRIVATE_PATH", Some(1), 0), Ok(false));
        let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{records:?}").contains("PRIVATE"));
        if capacity == Some(128) {
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::SourceRebindPublication)
            );
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
        assert_eq!(f.owner.resource_usage().issued_lifetime, 0);
        assert_eq!(f.owner.resource_usage().source_binding_history_entries, 0);
    }
}
#[test]
fn explicit_empty_association_history_release_is_unchanged_but_a_stale_binding_still_refuses() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let pair = f.pair();
    let before = f.owner.generation();
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            Some(0),
            pair,
            0,
            &history_release(),
            &mut f.recorder
        ),
        Ok(false)
    );
    assert_eq!(f.owner.generation(), before);
    let pair = f.pair();
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            None,
            pair,
            0,
            &history_release(),
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
}
fn archived_producer() -> (Fixture, DocumentVersion, SaveId, SaveDispatch, SaveId) {
    let mut f = Fixture::new();
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let old = save_as(&mut f, &d, b"first");
    let dispatch = written(&mut f, &old, b"first", Some(b"v1"));
    f.owner
        .activate_source_save_as(
            &old,
            f.port.source_save_as_probe(b"first", Some(b"v1")).unwrap(),
            None,
            0,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    let new = save_as(&mut f, &d, b"second");
    let _new_dispatch = written(&mut f, &new, b"second", Some(b"v2"));
    f.owner
        .activate_source_save_as(
            &new,
            f.port.source_save_as_probe(b"second", Some(b"v2")).unwrap(),
            None,
            1,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    (f, d, old, dispatch, new)
}
#[test]
fn explicit_history_release_removes_only_the_root_consumer_not_the_producer_or_current_baseline() {
    let (mut f, d, old, dispatch, new) = archived_producer();
    let before = f.owner.resource_usage();
    let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    let old_outcome = f.owner.save_outcome(&old, &mut f.recorder).unwrap();
    let new_outcome = f.owner.save_outcome(&new, &mut f.recorder).unwrap();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner.retire_save(&old, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            Some(2),
            None,
            1,
            &history_release(),
            &mut f.recorder
        ),
        Ok(true)
    );
    assert_eq!(f.owner.generation(), generation + 1);
    assert!(
        f.owner
            .semantic_root_usage(&mut f.recorder)
            .unwrap()
            .total_bytes
            < root.total_bytes
    );
    let mut expected = before;
    expected.source_binding_history_entries = 0;
    assert_eq!(f.owner.resource_usage(), expected);
    assert_eq!(
        f.owner.save_outcome(&old, &mut f.recorder).unwrap(),
        old_outcome
    );
    assert_eq!(
        f.owner.save_outcome(&new, &mut f.recorder).unwrap(),
        new_outcome
    );
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(
        f.owner.retire_save(&old, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner
        .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
        .unwrap();
    drop(dispatch);
    f.owner.retire_save(&old, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            Some(1),
            None,
            0,
            &history_release(),
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            Some(2),
            None,
            0,
            &history_release(),
            &mut f.recorder
        ),
        Ok(false)
    );
    assert_eq!(f.owner.generation(), generation + 1);
}
#[test]
fn all_history_release_discharge_and_current_gates_refuse_exactly_without_touching_retention() {
    for index in 0..24 {
        for denied in [None, Some(false)] {
            let (mut f, d, _, _dispatch, _) = archived_producer();
            let mut g = history_release();
            *match index {
                0 => &mut g.control.authorized,
                1 => &mut g.control.scope,
                2 => &mut g.control.continuous,
                3 => &mut g.control.live,
                4 => &mut g.control.not_stopped,
                5 => &mut g.control.binding_coverage_complete,
                6 => &mut g.control.writer_coverage_complete,
                7 => &mut g.control.binding_activation_allowed,
                8 => &mut g.control.identity_allowed,
                9 => &mut g.control.companions_ready,
                10 => &mut g.control.versions_available,
                11 => &mut g.control.reply_ready,
                12 => &mut g.control.resources_ready,
                13 => &mut g.control.disposition_readable,
                14 => &mut g.owner_control_allowed,
                15 => &mut g.preparation_reserved,
                16 => &mut g.ordinary_bindings_current,
                17 => &mut g.ordinary_identity_current,
                18 => &mut g.binding_history_release_allowed,
                19 => &mut g.history_evidence_released,
                20 => &mut g.snapshot_evidence_released,
                21 => &mut g.read_evidence_released,
                22 => &mut g.write_evidence_released,
                _ => &mut g.provider_evidence_released,
            } = denied;
            let usage = f.owner.resource_usage();
            let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
            let generation = f.owner.generation();
            assert_eq!(
                f.owner.release_source_binding_history(
                    &d.document,
                    Some(2),
                    None,
                    1,
                    &g,
                    &mut f.recorder
                ),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.resource_usage(), usage);
            assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
            assert_eq!(f.owner.generation(), generation);
        }
    }
}
#[test]
fn history_release_requires_current_pair_and_archive_profile_even_when_no_write_rights_are_requested()
 {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let id = save_as(&mut f, &d, b"next");
    let _dispatch = written(&mut f, &id, b"next", Some(b"v1"));
    let pair = export_pair(&mut f, &d, b"next");
    f.owner
        .activate_source_save_as(
            &id,
            f.port.source_save_as_probe(b"next", Some(b"v1")).unwrap(),
            pair,
            1,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            Some(1),
            None,
            1,
            &history_release(),
            &mut f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    );
    let pair = f.pair();
    assert_eq!(
        f.owner.release_source_binding_history(
            &d.document,
            Some(1),
            pair,
            0,
            &history_release(),
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
    let mut g = history_release();
    g.control.reservation_release_allowed = None;
    let pair = f.pair();
    assert!(
        f.owner
            .release_source_binding_history(&d.document, Some(1), pair, 1, &g, &mut f.recorder)
            .unwrap()
    );
}
#[test]
fn history_release_capture_modes_are_bounded_private_and_do_not_change_control_semantics() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(128), Some(1)] {
        let (mut f, d, _, _dispatch, _) = archived_producer();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        assert!(
            f.owner
                .release_source_binding_history(
                    &d.document,
                    Some(2),
                    None,
                    1,
                    &history_release(),
                    &mut f.recorder
                )
                .unwrap()
        );
        assert!(
            !f.owner
                .release_source_binding_history(
                    &d.document,
                    Some(2),
                    None,
                    0,
                    &history_release(),
                    &mut f.recorder
                )
                .unwrap()
        );
        assert_eq!(f.owner.resource_usage().source_binding_history_entries, 0);
        let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        if capacity == Some(128) {
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::SourceBindingHistoryRelease)
            );
        }
        assert!(!format!("{records:?}").contains("first"));
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}
fn save_as(f: &mut Fixture, d: &DocumentVersion, key: &[u8]) -> SaveId {
    let p = prepare(f, d);
    let pair = export_pair(f, d, key);
    let mut options = options(SourceExportPurposeV1::SaveAs);
    options.pair = pair;
    f.owner
        .queue_source_export(
            &p,
            present(&f.port, key),
            options,
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
}
fn export_pair(f: &mut Fixture, d: &DocumentVersion, key: &[u8]) -> Option<OrdinaryPairEvidence> {
    f.owner
        .source_export_pair_check(&d.document, &present(&f.port, key), &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        })
}
fn written(f: &mut Fixture, id: &SaveId, key: &[u8], version: Option<&[u8]>) -> SaveDispatch {
    let d = f.owner.save_outcome(id, &mut f.recorder).unwrap().captured;
    let pair = export_pair(f, &d, key);
    let dispatch = f
        .owner
        .dispatch_source_export(id, present(&f.port, key), pair, &facts(), &mut f.recorder)
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::Written,
                    dispatch.bytes().len(),
                    Synchronization::NotRequested,
                    version,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port.no_future_effects(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    dispatch
}
#[test]
fn qualified_scratch_save_as_installs_a_baseline_without_editing_or_inventing_history() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"scratch", &mut f.recorder).unwrap();
    let id = save_as(&mut f, &d, b"first");
    let dispatch = written(&mut f, &id, b"first", Some(b"v1"));
    let before = f.owner.generation();
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"first", Some(b"v1")).unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(f.owner.generation(), before + 1);
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    assert_eq!(f.owner.resource_usage().source_export_reservations, 0);
    assert_eq!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .source_save_as_binding_applied,
        Some(true)
    );
    assert_eq!(
        f.owner.save_outcome(&id, &mut f.recorder).unwrap().captured,
        d
    );
    assert!(
        !f.owner
            .activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"first", Some(b"v1")).unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(f.owner.generation(), before + 1);
    drop(dispatch);
}

#[test]
fn save_as_archives_the_prior_baseline_and_keeps_later_edits_and_ordinary_saves() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let pair = f.pair().unwrap();
    let old = f
        .owner
        .queue_pinned_save_with_pair(&d, requirements(), pair, &mut f.recorder)
        .unwrap();
    let old_pair = f
        .owner
        .ordinary_pair_check(&d.document, &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        })
        .unwrap();
    let old_dispatch = f
        .owner
        .dispatch_save_with_pair(
            &old,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            old_pair,
            &mut f.recorder,
        )
        .unwrap();
    for e in [
        f.port
            .result(
                &old_dispatch,
                Application::Written,
                4,
                Synchronization::NotRequested,
                Some(b"s1"),
            )
            .unwrap(),
        f.port.no_future_effects(&old_dispatch).unwrap(),
        f.port.quiescence(&old_dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    let pair = f
        .owner
        .ordinary_pair_check(&d.document, &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        })
        .unwrap();
    f.owner
        .publish_save_baseline_with_pair(&old, pair, &mut f.recorder)
        .unwrap();
    drop(old_dispatch);
    let id = save_as(&mut f, &d, b"next");
    let dispatch = written(&mut f, &id, b"next", Some(b"n1"));
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
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
    f.owner.commit(edit, &mut f.recorder).unwrap();
    let pair = export_pair(&mut f, &d, b"next");
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"next", Some(b"n1")).unwrap(),
                pair,
                1,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(f.owner.resource_usage().source_binding_history_entries, 1);
    assert_eq!(
        f.owner.retire_save(&old, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Modified
    );
    let current = f.owner.read(&d.document, &mut f.recorder).unwrap().version;
    let pair = f.pair().unwrap();
    let ordinary = f
        .owner
        .queue_pinned_save_with_pair(&current, requirements(), pair, &mut f.recorder)
        .unwrap();
    let pair = f
        .owner
        .ordinary_pair_check(&d.document, &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        })
        .unwrap();
    let normal = f
        .owner
        .dispatch_save_with_pair(
            &ordinary,
            f.port.read_result(b"next", b"n1", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(normal.bytes(), b"later");
    assert_eq!(normal.expected_validator(), Some(b"n1".as_slice()));
    assert_eq!(
        f.owner
            .save_outcome(&ordinary, &mut f.recorder)
            .unwrap()
            .binding_generation,
        Some(1)
    );
    drop(dispatch);
}

#[test]
fn same_context_activation_keeps_binding_generation_and_no_archive_and_can_refine_a_missing_version()
 {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let id = save_as(&mut f, &d, b"source");
    let dispatch = written(&mut f, &id, b"source", None);
    let pair = export_pair(&mut f, &d, b"source");
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"source", None).unwrap(),
                pair,
                0,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
    assert_eq!(f.owner.resource_usage().source_binding_history_entries, 0);
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::NotRequested,
                    Some(b"s1"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f
        .owner
        .ordinary_pair_check(&d.document, &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        })
        .unwrap();
    f.owner
        .publish_save_baseline_with_pair(&id, pair, &mut f.recorder)
        .unwrap();
    let pair = f.pair().unwrap();
    let ordinary = f
        .owner
        .queue_pinned_save_with_pair(&d, requirements(), pair, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .save_outcome(&ordinary, &mut f.recorder)
            .unwrap()
            .binding_generation,
        Some(0)
    );
    let pair = f
        .owner
        .ordinary_pair_check(&d.document, &mut f.recorder)
        .unwrap()
        .map(|c| {
            f.port
                .attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        })
        .unwrap();
    let normal = f
        .owner
        .dispatch_save_with_pair(
            &ordinary,
            f.port.read_result(b"source", b"s1", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &normal,
                    Application::Written,
                    4,
                    Synchronization::NotRequested,
                    Some(b"s2"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    assert_eq!(
        f.owner
            .publish_save_baseline_with_pair(&id, pair, &mut f.recorder),
        Err(ResourceError::Superseded)
    );
}

#[test]
fn late_contradiction_cannot_be_hidden_by_repeated_activation_or_retirement() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let id = save_as(&mut f, &d, b"out");
    let dispatch = written(&mut f, &id, b"out", Some(b"v1"));
    f.owner
        .activate_source_save_as(
            &id,
            f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
            None,
            0,
            &activation_facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let generation = f.owner.generation();
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            )
            .is_err()
    );
    let out = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
    assert!(out.contested);
    assert_eq!(out.source_save_as_binding_applied, Some(true));
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner.retire_save(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn all_current_activation_facts_are_exact_and_refusal_preserves_write_and_reservation() {
    for index in 0..17 {
        for denied in [None, Some(false)] {
            let mut f = Fixture::new();
            let d = f.owner.create(b"data", &mut f.recorder).unwrap();
            let id = save_as(&mut f, &d, b"out");
            let _dispatch = written(&mut f, &id, b"out", Some(b"v1"));
            let mut g = activation_facts();
            *match index {
                0 => &mut g.control.authorized,
                1 => &mut g.control.scope,
                2 => &mut g.control.continuous,
                3 => &mut g.control.live,
                4 => &mut g.control.not_stopped,
                5 => &mut g.control.binding_coverage_complete,
                6 => &mut g.control.writer_coverage_complete,
                7 => &mut g.control.binding_activation_allowed,
                8 => &mut g.control.identity_allowed,
                9 => &mut g.control.companions_ready,
                10 => &mut g.control.versions_available,
                11 => &mut g.control.reply_ready,
                12 => &mut g.control.resources_ready,
                13 => &mut g.control.disposition_readable,
                14 => &mut g.probe_attested,
                15 => &mut g.resource_continuity_attested,
                _ => &mut g.overwrite_allowed,
            } = denied;
            let generation = f.owner.generation();
            let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
            assert!(
                f.owner
                    .activate_source_save_as(
                        &id,
                        f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
                        None,
                        0,
                        &g,
                        &mut f.recorder
                    )
                    .is_err()
            );
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
            let out = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
            assert_eq!(out.application, Application::Written);
            assert_eq!(out.source_export_reservation_released, Some(false));
        }
    }
}

#[test]
fn activation_requires_disposition_and_current_original_probe_not_quiescence_or_cancelled_success()
{
    for mode in [
        "unknown",
        "no-disposition",
        "cancelled",
        "released",
        "wrong-key",
        "wrong-version",
        "wrong-provider",
        "copy",
    ] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"data", &mut f.recorder).unwrap();
        let id = if mode == "copy" {
            queue(&mut f, &d, b"out")
        } else {
            save_as(&mut f, &d, b"out")
        };
        let dispatch = f
            .owner
            .dispatch_source_export(
                &id,
                present(&f.port, b"out"),
                None,
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        if mode != "unknown" {
            f.owner
                .record_save_evidence(
                    f.port
                        .result(
                            &dispatch,
                            Application::Written,
                            4,
                            Synchronization::NotRequested,
                            Some(b"v1"),
                        )
                        .unwrap(),
                    &mut f.recorder,
                )
                .unwrap();
        }
        if mode != "no-disposition" {
            f.owner
                .record_save_evidence(
                    f.port.no_future_effects(&dispatch).unwrap(),
                    &mut f.recorder,
                )
                .unwrap();
        }
        if mode == "cancelled" {
            f.owner.stop_save(&id, &mut f.recorder).unwrap();
        }
        if mode == "released" {
            f.owner
                .release_source_export_reservation(&id, &control(), &mut f.recorder)
                .unwrap();
        }
        let other = f
            .owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                Some(&f.port.identity_domain()),
                &mut f.recorder,
            )
            .unwrap();
        let issuer = if mode == "wrong-provider" {
            &other
        } else {
            &f.port
        };
        let probe = issuer
            .source_save_as_probe(
                if mode == "wrong-key" {
                    b"other"
                } else {
                    b"out"
                },
                Some(if mode == "wrong-version" {
                    b"v0"
                } else {
                    b"v1"
                }),
            )
            .unwrap();
        let generation = f.owner.generation();
        assert!(
            f.owner
                .activate_source_save_as(&id, probe, None, 0, &activation_facts(), &mut f.recorder)
                .is_err(),
            "{mode}"
        );
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(
            f.owner
                .saved_state(&d.document, &mut f.recorder)
                .unwrap()
                .content,
            ContentComparison::Unbound
        );
    }
}

#[test]
fn old_and_new_destination_barriers_and_origin_reservations_are_checked_at_activation() {
    for old_destination in [false, true] {
        let mut f = Fixture::new();
        let d = f.doc.clone();
        let id = save_as(&mut f, &d, b"out");
        let _dispatch = written(&mut f, &id, b"out", Some(b"v1"));
        let peer = queue(&mut f, &d, if old_destination { b"source" } else { b"out" });
        let pair = export_pair(&mut f, &d, b"out");
        assert_eq!(
            f.owner.activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
                pair,
                1,
                &activation_facts(),
                &mut f.recorder
            ),
            Err(ResourceError::Retained)
        );
        f.owner.stop_save(&peer, &mut f.recorder).unwrap();
        if old_destination {
            let pair = export_pair(&mut f, &d, b"out");
            assert_eq!(
                f.owner.activate_source_save_as(
                    &id,
                    f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
                    pair,
                    1,
                    &activation_facts(),
                    &mut f.recorder
                ),
                Err(ResourceError::Retained)
            );
            f.owner
                .release_source_export_reservation(&peer, &control(), &mut f.recorder)
                .unwrap();
        }
        let pair = export_pair(&mut f, &d, b"out");
        assert!(
            f.owner
                .activate_source_save_as(
                    &id,
                    f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
                    pair,
                    1,
                    &activation_facts(),
                    &mut f.recorder
                )
                .unwrap()
        );
        assert_eq!(
            f.owner.resource_usage().source_export_reservations,
            usize::from(!old_destination)
        );
    }
}

#[test]
fn save_as_requires_final_pair_and_archive_headroom_without_erasing_a_successful_write() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let id = save_as(&mut f, &d, b"out");
    let _dispatch = written(&mut f, &id, b"out", Some(b"v1"));
    let generation = f.owner.generation();
    assert_eq!(
        f.owner.activate_source_save_as(
            &id,
            f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
            None,
            1,
            &activation_facts(),
            &mut f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    );
    let check = f
        .owner
        .source_export_pair_check(&d.document, &present(&f.port, b"out"), &mut f.recorder)
        .unwrap()
        .unwrap();
    let equivalent = f
        .port
        .attest_ordinary_pair(&check, ResourceRelation::Equivalent)
        .unwrap();
    assert_eq!(
        f.owner.activate_source_save_as(
            &id,
            f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
            Some(equivalent),
            1,
            &activation_facts(),
            &mut f.recorder
        ),
        Err(ResourceError::Conflict)
    );
    let pair = export_pair(&mut f, &d, b"out");
    assert_eq!(
        f.owner.activate_source_save_as(
            &id,
            f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
            pair,
            0,
            &activation_facts(),
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.resource_usage().source_binding_history_entries, 0);
    assert_eq!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .application,
        Application::Written
    );
    let pair = export_pair(&mut f, &d, b"out");
    assert!(
        f.owner
            .activate_source_save_as(
                &id,
                f.port.source_save_as_probe(b"out", Some(b"v1")).unwrap(),
                pair,
                1,
                &activation_facts(),
                &mut f.recorder
            )
            .unwrap()
    );
}

#[test]
fn activation_diagnostics_are_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(128), Some(1)] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"PRIVATE_SOURCE", &mut f.recorder).unwrap();
        let id = save_as(&mut f, &d, b"PRIVATE_PATH");
        let _dispatch = written(&mut f, &id, b"PRIVATE_PATH", Some(b"PRIVATE_VERSION"));
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        assert!(
            f.owner
                .activate_source_save_as(
                    &id,
                    f.port
                        .source_save_as_probe(b"PRIVATE_PATH", Some(b"PRIVATE_VERSION"))
                        .unwrap(),
                    None,
                    0,
                    &activation_facts(),
                    &mut f.recorder
                )
                .unwrap()
        );
        assert_eq!(
            f.owner
                .saved_state(&d.document, &mut f.recorder)
                .unwrap()
                .content,
            ContentComparison::Unchanged
        );
        assert_eq!(f.owner.resource_usage().source_export_reservations, 0);
        let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{events:?}").contains("PRIVATE"));
        if capacity == Some(128) {
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceSaveAsActivation)
            );
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn absent_save_as_rechecks_create_and_synchronization_without_needing_overwrite_or_release_grants()
{
    for synchronized in [false, true] {
        let mut f = Fixture::new();
        let port = f
            .owner
            .register_provider_with_destination_capabilities(
                ProviderProfile {
                    synchronization_supported: synchronized,
                },
                ProviderDestinationCapabilitiesV1 {
                    exclusive_create: true,
                    atomic_conditional_write: false,
                },
                None,
                &mut f.recorder,
            )
            .unwrap();
        let d = f.owner.create(b"data", &mut f.recorder).unwrap();
        let p = prepare(&mut f, &d);
        let id = f
            .owner
            .queue_source_export(
                &p,
                port.absent_source_export_destination(b"new").unwrap(),
                SourceExportOptionsV1 {
                    requirements: SaveRequirements {
                        protection: Protection::AtomicConditional,
                        acknowledgment: if synchronized {
                            Acknowledgment::Synchronized
                        } else {
                            Acknowledgment::Ordinary
                        },
                    },
                    ..options(SourceExportPurposeV1::SaveAs)
                },
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        let dispatch = f
            .owner
            .dispatch_source_export(
                &id,
                port.absent_source_export_destination(b"new").unwrap(),
                None,
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(
                port.result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::Unknown,
                    Some(b"v1"),
                )
                .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        let mut g = activation_facts();
        g.control.reservation_release_allowed = None;
        g.overwrite_allowed = None;
        if synchronized {
            assert_eq!(
                f.owner.activate_source_save_as(
                    &id,
                    port.source_save_as_probe(b"new", Some(b"v1")).unwrap(),
                    None,
                    0,
                    &g,
                    &mut f.recorder
                ),
                Err(ResourceError::AcknowledgmentUnsatisfied)
            );
            f.owner
                .record_save_evidence(
                    port.result(
                        &dispatch,
                        Application::Written,
                        4,
                        Synchronization::Succeeded,
                        Some(b"v1"),
                    )
                    .unwrap(),
                    &mut f.recorder,
                )
                .unwrap();
        }
        for denied in [None, Some(false)] {
            g.create_allowed = denied;
            assert_eq!(
                f.owner.activate_source_save_as(
                    &id,
                    port.source_save_as_probe(b"new", Some(b"v1")).unwrap(),
                    None,
                    0,
                    &g,
                    &mut f.recorder
                ),
                Err(ResourceError::Permission)
            );
        }
        g.create_allowed = Some(true);
        assert!(
            f.owner
                .activate_source_save_as(
                    &id,
                    port.source_save_as_probe(b"new", Some(b"v1")).unwrap(),
                    None,
                    0,
                    &g,
                    &mut f.recorder
                )
                .unwrap()
        );
    }
}

#[test]
fn absent_copy_and_save_as_intent_require_exclusive_create_without_a_fake_validator_or_implicit_binding()
 {
    for purpose in [
        SourceExportPurposeV1::CopyExport,
        SourceExportPurposeV1::SaveAs,
    ] {
        let mut f = Fixture::new();
        let port = f
            .owner
            .register_provider_with_destination_capabilities(
                ProviderProfile {
                    synchronization_supported: false,
                },
                ProviderDestinationCapabilitiesV1 {
                    exclusive_create: true,
                    atomic_conditional_write: false,
                },
                None,
                &mut f.recorder,
            )
            .unwrap();
        let d = f.owner.create(b"new", &mut f.recorder).unwrap();
        let p = prepare(&mut f, &d);
        let mut grant = facts();
        grant.overwrite_allowed = None;
        let id = f
            .owner
            .queue_source_export(
                &p,
                port.absent_source_export_destination(b"new").unwrap(),
                options(purpose),
                &grant,
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(
            f.owner.load_resource(
                port.read_result(b"new", b"v1", b"new").unwrap(),
                &mut f.recorder
            ),
            Err(ResourceError::Blocked)
        );
        let dispatch = f
            .owner
            .dispatch_source_export(
                &id,
                port.absent_source_export_destination(b"new").unwrap(),
                None,
                &grant,
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(
            dispatch.condition_v1().unwrap(),
            SaveWriteConditionV1::Absent
        );
        assert_eq!(dispatch.expected_validator(), None);
        assert_eq!(dispatch.expected_bytes(), None);
        f.owner
            .record_save_evidence(
                port.result(
                    &dispatch,
                    Application::Written,
                    3,
                    Synchronization::NotRequested,
                    Some(b"v1"),
                )
                .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner.retire_save(&id, &mut f.recorder),
            Err(ResourceError::Retained)
        );
        assert!(
            f.owner
                .release_source_export_reservation(&id, &control(), &mut f.recorder)
                .unwrap()
        );
        // Reservation release is not producer quiescence or Save As activation.
        assert_eq!(
            f.owner.retire_save(&id, &mut f.recorder),
            Err(ResourceError::Retained)
        );
        assert_eq!(
            f.owner
                .saved_state(&d.document, &mut f.recorder)
                .unwrap()
                .content,
            ContentComparison::Unbound
        );
        f.owner
            .record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        drop(dispatch);
        f.owner.retire_save(&id, &mut f.recorder).unwrap();
        let loaded = f
            .owner
            .load_resource(
                port.read_result(b"new", b"v1", b"new").unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        assert_ne!(loaded.document, d.document);
    }
}

#[test]
fn missing_current_admission_facts_and_independent_create_overwrite_or_capabilities_never_issue_work()
 {
    for index in 0..17 {
        for denied in [None, Some(false)] {
            let mut f = Fixture::new();
            let d = f.owner.create(b"data", &mut f.recorder).unwrap();
            let p = prepare(&mut f, &d);
            let mut g = facts();
            *match index {
                0 => &mut g.authorized,
                1 => &mut g.scope,
                2 => &mut g.continuous,
                3 => &mut g.live,
                4 => &mut g.not_stopped,
                5 => &mut g.source_readable,
                6 => &mut g.write_allowed,
                7 => &mut g.binding_coverage_complete,
                8 => &mut g.writer_coverage_complete,
                9 => &mut g.provider_available,
                10 => &mut g.resources_ready,
                11 => &mut g.launch_allowed,
                12 => &mut g.destination_allowed,
                13 => &mut g.identity_allowed,
                14 => &mut g.companions_ready,
                15 => &mut g.versions_available,
                _ => &mut g.reply_ready,
            } = denied;
            let root = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
            let generation = f.owner.generation();
            assert_eq!(
                f.owner.queue_source_export(
                    &p,
                    present(&f.port, b"out"),
                    options(SourceExportPurposeV1::CopyExport),
                    &g,
                    &mut f.recorder
                ),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.semantic_root_usage(&mut f.recorder).unwrap(), root);
            assert_eq!(f.owner.generation(), generation);
            let u = f.owner.resource_usage();
            assert_eq!(
                (
                    u.issued_lifetime,
                    u.issued_artifacts_lifetime,
                    u.source_export_reservations,
                    u.prepared_sources,
                    u.held_artifact_bytes
                ),
                (0, 0, 0, 0, 0)
            );
            assert!(!f.owner.is_faulted());
        }
    }
    for case in 0..5 {
        let mut f = Fixture::new();
        let d = f.owner.create(b"data", &mut f.recorder).unwrap();
        let p = prepare(&mut f, &d);
        let mut g = facts();
        let mut opt = options(SourceExportPurposeV1::CopyExport);
        let dest = if case < 2 {
            f.port.absent_source_export_destination(b"out").unwrap()
        } else {
            present(&f.port, b"out")
        };
        let error = match case {
            0 => ResourceError::Unsupported, // no advertised exclusive create
            1 => {
                g.create_allowed = None;
                ResourceError::Permission
            }
            2 => {
                g.overwrite_allowed = Some(false);
                ResourceError::Permission
            }
            3 => {
                opt.requirements.protection = Protection::AtomicConditional;
                ResourceError::Unsupported
            }
            _ => {
                opt.requirements.acknowledgment = Acknowledgment::Synchronized;
                ResourceError::Unsupported
            }
        };
        assert_eq!(
            f.owner
                .queue_source_export(&p, dest, opt, &g, &mut f.recorder),
            Err(error)
        );
        assert_eq!(f.owner.resource_usage().issued_lifetime, 0);
        assert_eq!(f.owner.resource_usage().prepared_sources, 0);
    }
}

#[test]
fn fresh_launch_checks_do_not_repair_conditions_or_recapture_after_edits_and_legacy_dispatch_cannot_bypass_them()
 {
    use vellum_engine::source::Replacement;
    let mut f = Fixture::new();
    let d = f.owner.create(b"frozen", &mut f.recorder).unwrap();
    let id = queue(&mut f, &d, b"out");
    let change = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 6,
                    expected: "frozen".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(change, &mut f.recorder).unwrap();
    f.owner
        .set_read_only(&d.document, true, &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    assert!(
        f.owner
            .dispatch_save(
                &id,
                f.port.read_result(b"out", b"v0", b"OLD").unwrap(),
                &mut f.recorder
            )
            .is_err()
    );
    let mut denied = facts();
    denied.launch_allowed = Some(false);
    assert!(
        f.owner
            .dispatch_source_export(
                &id,
                present(&f.port, b"out"),
                None,
                &denied,
                &mut f.recorder
            )
            .is_err()
    );
    assert_eq!(
        f.owner
            .dispatch_source_export(
                &id,
                f.port
                    .read_result(b"out", b"v1", b"OLD")
                    .unwrap()
                    .into_source_export_destination(),
                None,
                &facts(),
                &mut f.recorder
            )
            .unwrap_err(),
        ResourceError::Conflict
    );
    assert_eq!(f.owner.resource_usage(), before);
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            present(&f.port, b"out"),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"frozen");
    assert_eq!(
        f.owner
            .read(&d.document, &mut f.recorder)
            .unwrap()
            .source
            .text(),
        "later"
    );
    assert!(!f.owner.is_faulted());
}

#[test]
fn export_shares_ordinary_fifo_and_quiescence_or_stop_never_bypasses_unknown_application() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    let pair = f.pair().unwrap();
    let ordinary = f
        .owner
        .queue_pinned_save_with_pair(&d, requirements(), pair, &mut f.recorder)
        .unwrap();
    let p = prepare(&mut f, &d);
    let export = f
        .owner
        .queue_source_export(
            &p,
            f.port
                .read_result(b"source", b"s0", b"text")
                .unwrap()
                .into_source_export_destination(),
            options(SourceExportPurposeV1::CopyExport),
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &ordinary,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    f.owner.stop_save(&ordinary, &mut f.recorder).unwrap();
    f.owner
        .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .dispatch_source_export(
                &export,
                f.port
                    .read_result(b"source", b"s0", b"text")
                    .unwrap()
                    .into_source_export_destination(),
                None,
                &facts(),
                &mut f.recorder
            )
            .unwrap_err(),
        ResourceError::Blocked
    );
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port.no_future_effects(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert!(
        f.owner
            .dispatch_source_export(
                &export,
                f.port
                    .read_result(b"source", b"s0", b"text")
                    .unwrap()
                    .into_source_export_destination(),
                None,
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
}

#[test]
fn reservations_are_per_origin_and_block_alias_birth_and_retirement_until_explicit_release() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let first = queue(&mut f, &d, b"out");
    let second = queue(&mut f, &d, b"out");
    let alias = f
        .owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            Some(&f.port.identity_domain()),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.load_resource(
            alias.read_result(b"out", b"v0", b"OLD").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Blocked)
    );
    f.owner.stop_save(&first, &mut f.recorder).unwrap();
    let generation = f.owner.generation();
    for index in 0..15 {
        for denied in [None, Some(false)] {
            let mut c = control();
            *match index {
                0 => &mut c.authorized,
                1 => &mut c.scope,
                2 => &mut c.continuous,
                3 => &mut c.live,
                4 => &mut c.not_stopped,
                5 => &mut c.binding_coverage_complete,
                6 => &mut c.writer_coverage_complete,
                7 => &mut c.binding_activation_allowed,
                8 => &mut c.identity_allowed,
                9 => &mut c.companions_ready,
                10 => &mut c.versions_available,
                11 => &mut c.reply_ready,
                12 => &mut c.resources_ready,
                13 => &mut c.disposition_readable,
                _ => &mut c.reservation_release_allowed,
            } = denied;
            assert_eq!(
                f.owner
                    .release_source_export_reservation(&first, &c, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.resource_usage().source_export_reservations, 2);
            assert_eq!(f.owner.generation(), generation);
        }
    }
    assert_eq!(
        f.owner.retire_save(&first, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert!(
        f.owner
            .release_source_export_reservation(&first, &control(), &mut f.recorder)
            .unwrap()
    );
    let generation = f.owner.generation();
    assert!(
        !f.owner
            .release_source_export_reservation(&first, &control(), &mut f.recorder)
            .unwrap()
    );
    assert_eq!(f.owner.generation(), generation);
    f.owner.retire_save(&first, &mut f.recorder).unwrap();
    assert_eq!(f.owner.resource_usage().source_export_reservations, 1);
    assert_eq!(
        f.owner.load_resource(
            alias.read_result(b"out", b"v0", b"OLD").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Blocked)
    );
    f.owner.stop_save(&second, &mut f.recorder).unwrap();
    f.owner
        .release_source_export_reservation(&second, &control(), &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .load_resource(
                alias.read_result(b"out", b"v0", b"OLD").unwrap(),
                &mut f.recorder
            )
            .is_ok()
    );
}

#[test]
fn root_capacity_refusal_precedes_codec_and_writer_issuance_and_discards_only_the_preparation() {
    fn ready() -> (Fixture, DocumentVersion, PreparedSourceArtifactIdV1) {
        let mut f = Fixture::new();
        let d = f.owner.create(b"data", &mut f.recorder).unwrap();
        let p = prepare(&mut f, &d);
        (f, d, p)
    }
    let (mut probe, _, p) = ready();
    probe
        .owner
        .queue_source_export(
            &p,
            present(&probe.port, b"out"),
            options(SourceExportPurposeV1::CopyExport),
            &facts(),
            &mut probe.recorder,
        )
        .unwrap();
    let exact = probe
        .owner
        .semantic_root_usage(&mut probe.recorder)
        .unwrap()
        .total_bytes;
    for delta in [-1, 0, 1] {
        let (mut f, d, p) = ready();
        f.owner
            .configure_semantic_root_budget(
                exact.checked_add_signed(delta).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let before = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
        let generation = f.owner.generation();
        let result = f.owner.queue_source_export(
            &p,
            present(&f.port, b"out"),
            options(SourceExportPurposeV1::CopyExport),
            &facts(),
            &mut f.recorder,
        );
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(
                f.owner.semantic_root_usage(&mut f.recorder).unwrap(),
                before
            );
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.resource_usage().issued_artifacts_lifetime, 0);
            assert_eq!(f.owner.resource_usage().issued_lifetime, 0);
            assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
        } else {
            result.unwrap();
            assert_eq!(
                f.owner
                    .semantic_root_usage(&mut f.recorder)
                    .unwrap()
                    .total_bytes,
                exact
            );
        }
        assert_eq!(f.owner.resource_usage().prepared_sources, 0);
        assert_eq!(
            f.owner.read(&d.document, &mut f.recorder).unwrap().version,
            d
        );
    }
}

#[test]
fn contradictory_late_evidence_faults_the_destination_without_reopening_released_intent() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let id = queue(&mut f, &d, b"out");
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            present(&f.port, b"out"),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port
                .result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::NotRequested,
                    Some(b"v1"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.port.no_future_effects(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .release_source_export_reservation(&id, &control(), &mut f.recorder)
        .unwrap();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner
            .record_save_evidence(
                f.port
                    .result(
                        &dispatch,
                        Application::NotWritten,
                        0,
                        Synchronization::NotRequested,
                        None
                    )
                    .unwrap(),
                &mut f.recorder
            )
            .unwrap(),
        EvidenceDisposition::Contradiction
    );
    assert_eq!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .source_export_reservation_released,
        Some(true)
    );
    assert_eq!(f.owner.resource_usage().source_export_reservations, 0);
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner.load_resource(
            f.port.read_result(b"out", b"v1", b"data").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Contested)
    );
    assert_eq!(
        f.owner.retire_save(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn disposed_owner_stops_launch_but_cannot_release_provenance_or_task_before_its_destination_intent()
{
    let mut f = Fixture::new();
    let d = f.owner.create(b"data", &mut f.recorder).unwrap();
    let id = queue(&mut f, &d, b"out");
    let disposal = f
        .owner
        .prepare_retained_disposal(&d, 0, &mut f.recorder)
        .unwrap();
    let state = f
        .owner
        .commit_retained_disposal(disposal, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .dispatch_source_export(
                &id,
                present(&f.port, b"out"),
                None,
                &facts(),
                &mut f.recorder
            )
            .unwrap_err(),
        ResourceError::Unavailable
    );
    f.owner.stop_save(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .release_disposed_provenance(&d.document, state, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner
        .release_source_export_reservation(&id, &control(), &mut f.recorder)
        .unwrap();
    assert!(
        f.owner
            .release_disposed_provenance(&d.document, state, &mut f.recorder)
            .unwrap()
    );
    f.owner.retire_save(&id, &mut f.recorder).unwrap();
}

#[test]
fn explicit_capabilities_remain_independent_and_sync_failure_does_not_erase_a_known_write() {
    for absent in [false, true] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"data", &mut f.recorder).unwrap();
        let port = f
            .owner
            .register_provider_with_destination_capabilities(
                ProviderProfile {
                    synchronization_supported: true,
                },
                ProviderDestinationCapabilitiesV1 {
                    exclusive_create: absent,
                    atomic_conditional_write: !absent,
                },
                None,
                &mut f.recorder,
            )
            .unwrap();
        let target = || {
            if absent {
                port.absent_source_export_destination(b"out").unwrap()
            } else {
                present(&port, b"out")
            }
        };
        let p = prepare(&mut f, &d);
        let mut opt = options(SourceExportPurposeV1::SaveAs);
        opt.requirements = SaveRequirements {
            protection: Protection::AtomicConditional,
            acknowledgment: Acknowledgment::Synchronized,
        };
        let id = f
            .owner
            .queue_source_export(&p, target(), opt, &facts(), &mut f.recorder)
            .unwrap();
        let dispatch = f
            .owner
            .dispatch_source_export(&id, target(), None, &facts(), &mut f.recorder)
            .unwrap();
        assert_eq!(dispatch.protection(), Protection::AtomicConditional);
        assert!(dispatch.synchronization_required());
        f.owner
            .record_save_evidence(
                port.result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::Failed,
                    Some(b"v1"),
                )
                .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        let out = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
        assert_eq!(out.application, Application::Written);
        assert_eq!(out.synchronization, Synchronization::Failed);
        f.owner
            .release_source_export_reservation(&id, &control(), &mut f.recorder)
            .unwrap();
        assert_eq!(
            f.owner
                .saved_state(&d.document, &mut f.recorder)
                .unwrap()
                .content,
            ContentComparison::Unbound
        );
    }
}

#[test]
fn source_export_off_on_and_saturated_diagnostics_preserve_semantics_without_content_labels() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(128), Some(1)] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"PRIVATE_EXPORT", &mut f.recorder).unwrap();
        let p = prepare(&mut f, &d);
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = f
            .owner
            .queue_source_export(
                &p,
                present(&f.port, b"PRIVATE_DEST"),
                options(SourceExportPurposeV1::CopyExport),
                &facts(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner.stop_save(&id, &mut f.recorder).unwrap();
        f.owner
            .release_source_export_reservation(&id, &control(), &mut f.recorder)
            .unwrap();
        f.owner.retire_save(&id, &mut f.recorder).unwrap();
        let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{events:?}").contains("PRIVATE"));
        if capacity == Some(128) {
            for b in [
                Boundary::SourceExportAdmission,
                Boundary::SourceExportReservationRelease,
            ] {
                assert!(events.iter().any(|e| e.boundary == b));
            }
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
        assert_eq!(f.owner.resource_usage().source_export_reservations, 0);
        assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
        assert_eq!(f.owner.resource_usage().issued_lifetime, 1);
    }
}

#[test]
fn save_as_requires_structured_distinction_and_copy_does_not_gain_binding_authority() {
    let mut f = Fixture::new();
    let d = f.doc.clone();
    for relation in [
        None,
        Some(ResourceRelation::Equivalent),
        Some(ResourceRelation::Unknown),
        Some(ResourceRelation::Distinct),
    ] {
        let p = prepare(&mut f, &d);
        let destination = present(&f.port, b"new-source");
        let check = f
            .owner
            .source_export_pair_check(&d.document, &destination, &mut f.recorder)
            .unwrap()
            .unwrap();
        let pair = relation.map(|r| f.port.attest_ordinary_pair(&check, r).unwrap());
        let result = f.owner.queue_source_export(
            &p,
            destination,
            SourceExportOptionsV1 {
                pair,
                ..options(SourceExportPurposeV1::SaveAs)
            },
            &facts(),
            &mut f.recorder,
        );
        if relation == Some(ResourceRelation::Distinct) {
            let id = result.unwrap();
            let dest = present(&f.port, b"new-source");
            let check = f
                .owner
                .source_export_pair_check(&d.document, &dest, &mut f.recorder)
                .unwrap()
                .unwrap();
            let pair = f
                .port
                .attest_ordinary_pair(&check, ResourceRelation::Distinct)
                .unwrap();
            assert!(
                f.owner
                    .dispatch_source_export(
                        &id,
                        present(&f.port, b"new-source"),
                        None,
                        &facts(),
                        &mut f.recorder
                    )
                    .is_err()
            );
            assert!(
                f.owner
                    .dispatch_source_export(&id, dest, Some(pair), &facts(), &mut f.recorder)
                    .is_ok()
            );
        } else {
            assert!(result.is_err());
            assert_eq!(f.owner.resource_usage().issued_lifetime, 0);
        }
    }
    let p = prepare(&mut f, &d);
    assert_eq!(
        f.owner.queue_source_export(
            &p,
            present(&f.port, b"comments"),
            options(SourceExportPurposeV1::SaveAs),
            &facts(),
            &mut f.recorder
        ),
        Err(ResourceError::Conflict)
    );
    let p = prepare(&mut f, &d);
    assert!(
        f.owner
            .queue_source_export(
                &p,
                present(&f.port, b"comments"),
                options(SourceExportPurposeV1::CopyExport),
                &facts(),
                &mut f.recorder
            )
            .is_ok()
    );
}
#[test]
fn given_scratch_copy_export_then_shared_task_writes_exact_capture_without_activating_a_binding() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"payload", &mut f.recorder).unwrap();
    let p = prepare(&mut f, &d);
    let destination = f
        .port
        .read_result(b"dest", b"v0", b"OLD")
        .unwrap()
        .into_source_export_destination();
    let generation = f.owner.generation();
    let id = f
        .owner
        .queue_source_export(
            &p,
            destination,
            SourceExportOptionsV1 {
                purpose: SourceExportPurposeV1::CopyExport,
                requirements: requirements(),
                pair: None,
            },
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.owner.generation(), generation + 1);
    assert_eq!(f.owner.resource_usage().source_export_reservations, 1);
    assert_eq!(f.owner.resource_usage().prepared_sources, 0);
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unbound
    );
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            f.port
                .read_result(b"dest", b"v0", b"OLD")
                .unwrap()
                .into_source_export_destination(),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(dispatch.bytes(), b"payload");
    assert_eq!(
        dispatch.condition_v1().unwrap(),
        SaveWriteConditionV1::Present {
            bytes: b"OLD",
            validator: b"v0"
        }
    );
    for e in [
        f.port
            .result(
                &dispatch,
                Application::Written,
                7,
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
        f.port.no_future_effects(&dispatch).unwrap(),
        f.port.quiescence(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(e, &mut f.recorder).unwrap();
    }
    assert_eq!(
        f.owner.publish_save_baseline(&id, &mut f.recorder),
        Err(ResourceError::Unsupported)
    );
    assert!(
        f.owner
            .release_source_export_reservation(&id, &control(), &mut f.recorder)
            .unwrap()
    );
    drop(dispatch);
    f.owner.retire_save(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unbound
    );
}
