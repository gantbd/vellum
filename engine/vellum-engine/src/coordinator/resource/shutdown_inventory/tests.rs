use super::*;
use crate::coordinator::ShutdownDocumentRecordV1;
fn expected(o: &SourceCoordinator, source: &DocumentVersion) -> ShutdownDocumentRecordV1 {
    let d = o.retained_document(&source.document).unwrap();
    ShutdownDocumentRecordV1 {
        source: source.clone(),
        source_binding_generation: o
            .explicit_disposal_source_binding_generation(&source.document)
            .unwrap(),
        annotation_binding_generation: o
            .shutdown_annotation_binding_generation(&source.document)
            .unwrap(),
        lifecycle: d.lifecycle,
        source_policy_generation: d.policy_generation,
        source_read_only: d.read_only,
        serialization: d.serialization,
        annotation_generation: d.annotation_generation,
        annotation_policy_generation: d.annotation_policy,
        annotation_read_only: d.annotation_read_only,
    }
}

#[test]
fn acknowledged_save_as_requires_actual_binding_activation_not_just_a_successful_export() {
    let (mut o, mut r, d, port, id) = super::super::source_export::tests::ready_activation();
    assert!(
        o.validate_shutdown_save_readiness(&expected(&o, &d), ArtifactKind::Source, &id, &mut r)
            .is_err()
    );
    let plan = o
        .prepare_source_save_as_native(
            &id,
            port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            None,
            1,
            &super::super::source_export::tests::activation(),
            &mut r,
        )
        .unwrap();
    o.commit_source_save_as_native(
        plan,
        &super::super::source_export::tests::activation(),
        &mut r,
    )
    .unwrap();
    let before = o.native_root_identity().unwrap();
    assert_eq!(
        o.validate_shutdown_save_readiness(&expected(&o, &d), ArtifactKind::Source, &id, &mut r),
        Ok(())
    );
    assert_eq!(o.native_root_identity().unwrap(), before);
}

#[test]
fn acknowledged_annotation_export_requires_exact_explicit_ordinary_baseline_adoption() {
    let (mut o, mut r, ordinary, port) = super::super::serialization_tests::pending_root_fixture();
    let d = o.save_outcome(&ordinary, &mut r).unwrap().captured;
    o.stop_save(&ordinary, &mut r).unwrap();
    let encoder = o.register_annotation_encoder(&mut r).unwrap();
    let snapshot = o.capture_annotations(&d, 0, &mut r).unwrap();
    let artifact = o
        .retain_annotation_artifact(encoder.seal(snapshot, b"[]".to_vec()).unwrap(), &mut r)
        .unwrap();
    let pair = o
        .ordinary_pair_check(&d.document, &mut r)
        .unwrap()
        .map(|p| {
            port.attest_ordinary_pair(&p, ResourceRelation::Distinct)
                .unwrap()
        });
    let id = o
        .queue_bound_annotation_export(
            &artifact,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut r,
        )
        .unwrap();
    let pair = o.ordinary_pair_check(&d.document, &mut r).unwrap().unwrap();
    let pair = port
        .attest_ordinary_pair(&pair, ResourceRelation::Distinct)
        .unwrap();
    let dispatch = o
        .dispatch_save_with_pair(
            &id,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            pair,
            &mut r,
        )
        .unwrap();
    o.record_save_evidence(
        port.result(
            &dispatch,
            Application::Written,
            2,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    assert!(
        o.validate_shutdown_save_readiness(
            &expected(&o, &d),
            ArtifactKind::Annotations,
            &id,
            &mut r
        )
        .is_err()
    );
    let pair = o
        .ordinary_pair_check(&d.document, &mut r)
        .unwrap()
        .map(|p| {
            port.attest_ordinary_pair(&p, ResourceRelation::Distinct)
                .unwrap()
        });
    o.publish_annotation_baseline(&id, pair, 65536, &mut r)
        .unwrap();
    let before = o.native_root_identity().unwrap();
    assert_eq!(
        o.validate_shutdown_save_readiness(
            &expected(&o, &d),
            ArtifactKind::Annotations,
            &id,
            &mut r
        ),
        Ok(())
    );
    assert_eq!(o.native_root_identity().unwrap(), before);
}
