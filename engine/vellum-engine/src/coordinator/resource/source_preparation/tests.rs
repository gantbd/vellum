use super::*;
fn ready() -> (SourceCoordinator, Recorder, DocumentVersion) {
    let (mut o, mut r, d, _) = super::super::serialization_tests::fixture();
    let serialization = o.source_serialization(&d.document, &mut r).unwrap();
    assert!(o.prepare_source_artifact(&d, serialization, &mut r).is_ok());
    (o, r, d)
}

#[test]
fn export_capture_consumes_exact_codec_bytes_and_releases_the_temporary_charge_once() {
    let (mut owner, mut recorder, source) = ready();
    let prepared = owner
        .resources
        .source_preparation
        .as_ref()
        .unwrap()
        .id
        .clone();
    let before = owner.resource_usage();
    assert_eq!(before.prepared_sources, 1);
    assert_eq!(before.prepared_source_bytes, 1);

    let capture = owner
        .capture_prepared_source_serialization(&prepared, &mut recorder)
        .unwrap();
    assert_eq!(capture.bytes(), b"A");
    assert_eq!(capture.source(), &source);
    assert_eq!(
        capture.serialization(),
        owner
            .source_serialization(&source.document, &mut recorder)
            .unwrap()
    );
    assert_eq!(
        format!("{capture:?}"),
        "CapturedSourceSerializationV1(<redacted>)"
    );
    assert_eq!(owner.resource_usage().prepared_sources, 0);
    assert_eq!(owner.resource_usage().prepared_source_bytes, 0);
    assert_eq!(owner.resource_usage().held_artifact_bytes, 0);
    assert!(matches!(
        owner.capture_prepared_source_serialization(&prepared, &mut recorder),
        Err(ResourceError::Unavailable)
    ));
    assert!(!owner.is_faulted());
}

#[test]
fn retained_export_capture_preserves_the_exact_preparation_for_native_admission() {
    let (mut owner, mut recorder, source) = ready();
    let prepared = owner
        .resources
        .source_preparation
        .as_ref()
        .unwrap()
        .id
        .clone();
    let before = owner.resource_usage();

    let capture = owner
        .capture_prepared_source_serialization_retained(&prepared, &mut recorder)
        .unwrap();
    assert_eq!(capture.bytes(), b"A");
    assert_eq!(capture.source(), &source);
    assert_eq!(owner.resource_usage(), before);
    assert!(matches!(
        owner.capture_prepared_source_serialization_retained(&prepared, &mut recorder),
        Err(ResourceError::Unavailable)
    ));
    assert!(matches!(
        owner.capture_prepared_source_serialization(&prepared, &mut recorder),
        Err(ResourceError::Unavailable)
    ));
    assert_eq!(owner.resource_usage(), before);

    let artifact = owner
        .retain_prepared_source_artifact(&prepared, &mut recorder)
        .unwrap();
    assert_eq!(owner.resource_usage().prepared_sources, 0);
    assert_eq!(owner.resource_usage().retained_artifacts, 1);
    owner
        .retire_resource_artifact(&artifact, &mut recorder)
        .unwrap();
}

#[test]
fn foreign_export_capture_cannot_consume_the_local_preparation() {
    let (mut owner, mut recorder, _) = ready();
    let (peer, _, _) = ready();
    let foreign = peer
        .resources
        .source_preparation
        .as_ref()
        .unwrap()
        .id
        .clone();
    let before = owner.resource_usage();
    assert!(matches!(
        owner.capture_prepared_source_serialization(&foreign, &mut recorder),
        Err(ResourceError::Unavailable)
    ));
    assert_eq!(owner.resource_usage(), before);
}
#[test]
fn read_writer_attachment_charge_is_preserved_by_source_artifact_preparation_and_discard() {
    let (mut o, mut r, d, _) = super::super::serialization_tests::fixture();
    o.attach_annotation_read_baselines(&mut r).unwrap();
    let original = o.resource_usage().held_artifact_bytes;
    assert!(original > 0);
    let serialization = o.source_serialization(&d.document, &mut r).unwrap();
    let prepared = o
        .prepare_source_artifact(&d, serialization, &mut r)
        .unwrap();
    assert_eq!(o.resource_usage().held_artifact_bytes, original + 1);
    o.discard_source_artifact_preparation(&prepared, &mut r)
        .unwrap();
    assert_eq!(o.resource_usage().held_artifact_bytes, original);
    assert!(o.resources.read_baseline_attachment.is_some());
    assert!(!o.is_faulted());
}
#[test]
fn live_and_issued_credit_cannot_be_stolen_by_other_artifact_capture() {
    for issued in [false, true] {
        let (mut o, mut r, _) = ready();
        let p = o.resources.source_preparation.as_ref().unwrap().id.clone();
        let l = o.resources.limits.as_mut().unwrap();
        if issued {
            l.max_issued_artifacts = 1;
        } else {
            l.max_live_artifacts = 1;
        }
        assert_eq!(o.preflight_artifact(0), Err(ResourceError::Limit));
        let before = o.resource_usage();
        let id = o.retain_prepared_source_artifact(&p, &mut r).unwrap();
        assert_eq!(
            o.resource_usage().held_artifact_bytes,
            before.held_artifact_bytes
        );
        assert_eq!(o.resource_usage().prepared_sources, 0);
        assert_eq!(o.resource_usage().issued_artifacts_lifetime, 1);
        o.retire_resource_artifact(&id, &mut r).unwrap();
        if issued {
            assert_eq!(o.preflight_artifact(0), Err(ResourceError::Limit));
        } else {
            assert!(o.preflight_artifact(0).is_ok());
        }
    }
}
#[test]
fn competing_capture_preserves_prepaid_vector_space_and_byte_charge() {
    let (mut o, mut r, _) = ready();
    let p = o.resources.source_preparation.as_ref().unwrap().id.clone();
    let basis = o.resources.source_preparation.as_ref().unwrap();
    let source = basis.source.clone();
    let serialization = basis.serialization;
    let original = o.resource_usage().held_artifact_bytes;
    o.preflight_artifact(1).unwrap();
    let other = o.install_artifact(ArtifactRow {
        catalog: None,
        loaded_condition: None,
        basis: ArtifactBasis::Source {
            source,
            serialization,
        },
        bytes: Blob(Arc::from(&b"B"[..])),
        stopped: false,
    });
    assert!(o.resources.artifacts.capacity() > o.resources.artifacts.len());
    let id = o.retain_prepared_source_artifact(&p, &mut r).unwrap();
    assert_eq!(o.resource_usage().held_artifact_bytes, original + 1);
    o.retire_resource_artifact(&other, &mut r).unwrap();
    assert_eq!(o.resource_usage().held_artifact_bytes, original);
    o.retire_resource_artifact(&id, &mut r).unwrap();
    assert_eq!(o.resource_usage().held_artifact_bytes, 0);
}
#[test]
fn faulted_discard_releases_only_exact_charge_and_never_heals_or_erases_debt() {
    for drift in [false, true] {
        let (mut o, mut r, _) = ready();
        let p = o.resources.source_preparation.as_ref().unwrap().id.clone();
        o.faulted = true;
        if drift {
            o.resources.held_artifact_bytes += 7;
        }
        let before = o.resource_usage();
        if drift {
            assert_eq!(
                o.discard_source_artifact_preparation(&p, &mut r),
                Err(ResourceError::Faulted)
            );
            assert_eq!(o.resource_usage(), before);
        } else {
            o.discard_source_artifact_preparation(&p, &mut r).unwrap();
            assert_eq!(o.resource_usage().held_artifact_bytes, 0);
            assert_eq!(o.resource_usage().prepared_sources, 0);
        }
        assert!(o.is_faulted());
        assert_eq!(o.resource_usage().issued_artifacts_lifetime, 0);
    }
}
