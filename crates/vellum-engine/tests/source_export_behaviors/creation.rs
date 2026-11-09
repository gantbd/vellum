use super::*;
use vellum_engine::instrumentation::{Boundary, Recorder};

fn version(token: &[u8]) -> ResourceVersionV1<'_> {
    ResourceVersionV1 {
        epoch: b"epoch",
        token,
    }
}
fn provider(f: &mut Fixture, causal: bool, create: bool) -> ProviderPort {
    f.owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: false,
            },
            ProviderDestinationCapabilitiesV1 {
                exclusive_create: create,
                atomic_conditional_write: false,
            },
            ProviderCausalCapabilitiesV1 {
                causal_receipts: causal,
                non_reuse_tokens: causal,
            },
            None,
            &mut f.recorder,
        )
        .unwrap()
}
fn launched(
    f: &mut Fixture,
    p: &ProviderPort,
    purpose: SourceExportPurposeV1,
    typed: bool,
) -> (DocumentVersion, SaveId, SaveDispatch) {
    let d = f.owner.create(b"new", &mut f.recorder).unwrap();
    let prepared = prepare(f, &d);
    let destination = p.absent_source_export_destination(b"new").unwrap();
    let id = if typed {
        f.owner.queue_source_export_v2(
            &prepared,
            destination,
            SourceExportOptionsV2 {
                intent: options(purpose),
                result_version_profile: SaveResultVersionProfileV1::EpochToken,
            },
            &facts(),
            &mut f.recorder,
        )
    } else {
        f.owner.queue_source_export(
            &prepared,
            destination,
            options(purpose),
            &facts(),
            &mut f.recorder,
        )
    }
    .unwrap();
    let dispatch = f
        .owner
        .dispatch_source_export(
            &id,
            p.absent_source_export_destination(b"new").unwrap(),
            None,
            &facts(),
            &mut f.recorder,
        )
        .unwrap();
    (d, id, dispatch)
}

#[test]
fn legacy_absent_create_does_not_infer_epoch_profile_from_provider_capabilities() {
    let mut f = Fixture::new();
    let p = provider(&mut f, true, true);
    let (d, id, dispatch) = launched(&mut f, &p, SourceExportPurposeV1::CopyExport, false);
    assert_eq!(
        dispatch.result_version_profile(),
        SaveResultVersionProfileV1::Opaque
    );
    assert_eq!(dispatch.condition_v1(), Ok(SaveWriteConditionV1::Absent));
    assert!(matches!(
        p.result_causal(
            &dispatch,
            Application::Written,
            3,
            Synchronization::NotRequested,
            Some(version(b"v1"))
        ),
        Err(ResourceError::Unsupported)
    ));
    f.owner
        .record_save_evidence(
            p.result(
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
    assert_eq!(
        f.owner
            .save_outcome(&id, &mut f.recorder)
            .unwrap()
            .result_version_profile,
        SaveResultVersionProfileV1::Opaque
    );
    assert_eq!(
        f.owner
            .saved_state(&d.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unbound
    );
}

#[test]
fn typed_create_retains_before_version_contradictions_in_either_arrival_order_with_a_fixed_cap() {
    for edge_first in [false, true] {
        let mut f = Fixture::new();
        let p = provider(&mut f, true, true);
        let (_, id, dispatch) = launched(&mut f, &p, SourceExportPurposeV1::SaveAs, true);
        let edge = p
            .causal_edge(&dispatch, version(b"v0"), version(b"v1"))
            .unwrap();
        let result = p
            .result_causal(
                &dispatch,
                Application::Written,
                3,
                Synchronization::NotRequested,
                Some(version(b"v1")),
            )
            .unwrap();
        if !edge_first {
            f.owner
                .record_save_evidence(result.clone(), &mut f.recorder)
                .unwrap();
        }
        assert_eq!(
            f.owner.record_save_evidence(edge.clone(), &mut f.recorder),
            Ok(EvidenceDisposition::Contradiction)
        );
        assert_eq!(
            f.owner.record_save_evidence(edge, &mut f.recorder),
            Ok(EvidenceDisposition::Duplicate)
        );
        if edge_first {
            f.owner
                .record_save_evidence(result, &mut f.recorder)
                .unwrap();
        }
        f.owner
            .record_save_evidence(p.no_future_effects(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
        assert!(outcome.contested);
        assert_eq!(outcome.application, Application::Written);
        assert!(!outcome.causal_edge_available);
        assert_eq!(outcome.causal_conflict_witnesses, 1);
        assert_eq!(
            f.owner.activate_source_save_as(
                &id,
                p.source_save_as_probe_causal(b"new", Some(version(b"v1")))
                    .unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            ),
            Err(ResourceError::AcknowledgmentUnsatisfied)
        );
        assert_eq!(
            f.owner
                .release_source_export_reservation(&id, &control(), &mut f.recorder),
            Err(ResourceError::Retained)
        );
        let second = p
            .causal_edge(&dispatch, version(b"v2"), version(b"v3"))
            .unwrap();
        assert_eq!(
            f.owner
                .record_save_evidence(second.clone(), &mut f.recorder),
            Ok(EvidenceDisposition::Contradiction)
        );
        let before = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
        assert_eq!(
            f.owner.record_save_evidence(
                p.causal_edge(&dispatch, version(b"v4"), version(b"v5"))
                    .unwrap(),
                &mut f.recorder
            ),
            Err(ResourceError::EvidenceCapacity)
        );
        assert_eq!(f.owner.save_outcome(&id, &mut f.recorder).unwrap(), before);
        assert_eq!(
            f.owner.record_save_evidence(second, &mut f.recorder),
            Ok(EvidenceDisposition::Duplicate)
        );
        assert_eq!(
            f.owner.retire_save(&id, &mut f.recorder),
            Err(ResourceError::Retained)
        );
    }
}

#[test]
fn result_profile_and_exclusive_create_capabilities_are_independent_admission_conditions() {
    for (causal, create, expected) in [
        (false, true, ResourceError::Unsupported),
        (true, false, ResourceError::Unsupported),
    ] {
        let mut f = Fixture::new();
        let p = provider(&mut f, causal, create);
        let d = f.owner.create(b"new", &mut f.recorder).unwrap();
        let prepared = prepare(&mut f, &d);
        let mut before = f.owner.resource_usage();
        before.prepared_sources = 0;
        before.prepared_source_bytes = 0;
        before.held_artifact_bytes = 0;
        assert_eq!(
            f.owner.queue_source_export_v2(
                &prepared,
                p.absent_source_export_destination(b"new").unwrap(),
                SourceExportOptionsV2 {
                    intent: options(SourceExportPurposeV1::SaveAs),
                    result_version_profile: SaveResultVersionProfileV1::EpochToken,
                },
                &facts(),
                &mut f.recorder
            ),
            Err(expected)
        );
        assert_eq!(f.owner.resource_usage(), before);
    }
    for typed_before in [false, true] {
        let mut f = Fixture::new();
        let p = provider(&mut f, true, true);
        let d = f.owner.create(b"new", &mut f.recorder).unwrap();
        let prepared = prepare(&mut f, &d);
        let mut before = f.owner.resource_usage();
        before.prepared_sources = 0;
        before.prepared_source_bytes = 0;
        before.held_artifact_bytes = 0;
        let destination = if typed_before {
            p.read_result_causal(b"new", version(b"v0"), b"OLD")
                .unwrap()
        } else {
            p.read_result(b"new", b"v0", b"OLD").unwrap()
        }
        .into_source_export_destination();
        assert_eq!(
            f.owner.queue_source_export_v2(
                &prepared,
                destination,
                SourceExportOptionsV2 {
                    intent: options(SourceExportPurposeV1::CopyExport),
                    result_version_profile: if typed_before {
                        SaveResultVersionProfileV1::Opaque
                    } else {
                        SaveResultVersionProfileV1::EpochToken
                    },
                },
                &facts(),
                &mut f.recorder
            ),
            Err(ResourceError::Unsupported)
        );
        assert_eq!(f.owner.resource_usage(), before);
    }
}

#[test]
fn typed_create_result_refinement_is_lossless_and_optional_diagnostics_do_not_change_copy_semantics()
 {
    for capacity in [None, Some(0), Some(128)] {
        let mut f = Fixture::new();
        let p = provider(&mut f, true, true);
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let (d, id, dispatch) = launched(&mut f, &p, SourceExportPurposeV1::CopyExport, true);
        assert!(matches!(
            p.result(
                &dispatch,
                Application::Written,
                3,
                Synchronization::NotRequested,
                Some(b"v1")
            ),
            Err(ResourceError::Unsupported)
        ));
        f.owner
            .record_save_evidence(
                p.result_causal(
                    &dispatch,
                    Application::Written,
                    3,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        assert!(
            !f.owner
                .save_outcome(&id, &mut f.recorder)
                .unwrap()
                .version_available
        );
        f.owner
            .record_save_evidence(
                p.result_causal(
                    &dispatch,
                    Application::Written,
                    3,
                    Synchronization::NotRequested,
                    Some(version(b"v1")),
                )
                .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(p.no_future_effects(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        let outcome = f.owner.save_outcome(&id, &mut f.recorder).unwrap();
        assert!(outcome.version_available);
        assert!(!outcome.contested);
        assert!(!outcome.causal_edge_available);
        assert_eq!(
            outcome.result_version_profile,
            SaveResultVersionProfileV1::EpochToken
        );
        assert_eq!(
            f.owner
                .saved_state(&d.document, &mut f.recorder)
                .unwrap()
                .content,
            ContentComparison::Unbound
        );
        assert_eq!(
            f.owner.activate_source_save_as(
                &id,
                p.source_save_as_probe_causal(b"new", Some(version(b"v1")))
                    .unwrap(),
                None,
                0,
                &activation_facts(),
                &mut f.recorder
            ),
            Err(ResourceError::Unsupported)
        );
        let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{records:?}").contains("epoch"));
        if capacity == Some(128) {
            assert!(
                records
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceExportAdmission)
            );
            assert!(records.iter().any(|e| e.boundary == Boundary::SaveEvidence));
        } else {
            assert!(records.is_empty());
        }
    }
}
