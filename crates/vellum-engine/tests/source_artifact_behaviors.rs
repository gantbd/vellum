use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::Replacement;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)] // Shared fixture also supplies read-transcript scenarios.
mod fixture;
use fixture::Fixture;

#[test]
fn given_exact_source_and_serialization_then_preparation_issues_nothing_until_explicit_retention() {
    let mut f = Fixture::new();
    let d = f
        .owner
        .create(b"\xef\xbb\xbf\xc3\xa9\r\n", &mut f.recorder)
        .unwrap();
    let serialization = f
        .owner
        .source_serialization(&d.document, &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    let generation = f.owner.generation();
    let p = f
        .owner
        .prepare_source_artifact(&d, serialization, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().prepared_sources, 1);
    assert_eq!(f.owner.resource_usage().prepared_source_bytes, 7);
    assert_eq!(
        f.owner.resource_usage().issued_artifacts_lifetime,
        before.issued_artifacts_lifetime
    );
    assert_eq!(
        f.owner.resource_usage().issued_lifetime,
        before.issued_lifetime
    );
    assert_eq!(f.owner.generation(), generation);
    let artifact = f
        .owner
        .retain_prepared_source_artifact(&p, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .resource_artifact_bytes(&artifact, 0, 7, &mut f.recorder)
            .unwrap(),
        b"\xef\xbb\xbf\xc3\xa9\r\n"
    );
    assert_eq!(
        f.owner.resource_usage().issued_artifacts_lifetime,
        before.issued_artifacts_lifetime + 1
    );
    assert_eq!(
        f.owner.resource_usage().issued_lifetime,
        before.issued_lifetime
    );
    assert_eq!(f.owner.generation(), generation);
    assert!(
        f.owner
            .retain_prepared_source_artifact(&p, &mut f.recorder)
            .is_err()
    );
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(edit, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .resource_artifact_bytes(&artifact, 0, 7, &mut f.recorder)
            .unwrap(),
        b"\xef\xbb\xbf\xc3\xa9\r\n"
    );
    f.owner
        .retire_resource_artifact(&artifact, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.resource_usage().held_artifact_bytes,
        before.held_artifact_bytes
    );
}

#[test]
fn given_stale_disposed_foreign_or_spent_preparation_then_retention_cannot_recapture_or_steal_it() {
    for dispose in [false, true] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"capture", &mut f.recorder).unwrap();
        let s = f
            .owner
            .source_serialization(&d.document, &mut f.recorder)
            .unwrap();
        let p = f
            .owner
            .prepare_source_artifact(&d, s, &mut f.recorder)
            .unwrap();
        let clone = p.clone();
        assert_eq!(
            f.owner
                .prepare_source_artifact(&d, s, &mut f.recorder)
                .unwrap_err(),
            ResourceError::Blocked
        );
        let mut peer = Fixture::new();
        let peer_s = peer
            .owner
            .source_serialization(&peer.doc.document, &mut peer.recorder)
            .unwrap();
        let peer_p = peer
            .owner
            .prepare_source_artifact(&peer.doc, peer_s, &mut peer.recorder)
            .unwrap();
        let peer_before = peer.owner.resource_usage();
        assert_eq!(
            peer.owner
                .retain_prepared_source_artifact(&p, &mut peer.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            peer.owner
                .discard_source_artifact_preparation(&p, &mut peer.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(peer.owner.resource_usage(), peer_before);
        if dispose {
            let plan = f
                .owner
                .prepare_retained_disposal(&d, 0, &mut f.recorder)
                .unwrap();
            f.owner
                .commit_retained_disposal(plan, &mut f.recorder)
                .unwrap();
        } else {
            let plan = f
                .owner
                .prepare(
                    &[DocumentEdits {
                        expected: d.clone(),
                        edits: vec![Replacement {
                            start: 0,
                            end: 0,
                            expected: "".into(),
                            inserted: "new".into(),
                        }],
                    }],
                    &[],
                    &mut f.recorder,
                )
                .unwrap();
            f.owner.commit(plan, &mut f.recorder).unwrap();
        }
        let before = f.owner.resource_usage();
        assert_eq!(
            f.owner.retain_prepared_source_artifact(&p, &mut f.recorder),
            Err(if dispose {
                ResourceError::Unavailable
            } else {
                ResourceError::Stale
            })
        );
        assert_eq!(f.owner.resource_usage(), before);
        f.owner
            .discard_source_artifact_preparation(&clone, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.resource_usage().prepared_sources, 0);
        assert_eq!(f.owner.resource_usage().issued_artifacts_lifetime, 0);
        let new_d = f.owner.create(b"next", &mut f.recorder).unwrap();
        let new_s = f
            .owner
            .source_serialization(&new_d.document, &mut f.recorder)
            .unwrap();
        let new_p = f
            .owner
            .prepare_source_artifact(&new_d, new_s, &mut f.recorder)
            .unwrap();
        let usage = f.owner.resource_usage();
        assert_eq!(
            f.owner.retain_prepared_source_artifact(&p, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            f.owner
                .discard_source_artifact_preparation(&p, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(f.owner.resource_usage(), usage);
        f.owner
            .discard_source_artifact_preparation(&new_p, &mut f.recorder)
            .unwrap();
        peer.owner
            .discard_source_artifact_preparation(&peer_p, &mut peer.recorder)
            .unwrap();
        assert!(!f.owner.is_faulted());
    }
}

#[test]
fn given_annotation_capture_competition_then_its_shared_codec_cannot_spend_the_source_credit() {
    let mut f = Fixture::new();
    let source = f.doc.clone();
    let s = f
        .owner
        .source_serialization(&source.document, &mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&source, 0, &mut f.recorder)
        .unwrap();
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let p = f
        .owner
        .prepare_source_artifact(&source, s, &mut f.recorder)
        .unwrap();
    let mut others = vec![];
    for _ in 0..7 {
        others.push(
            f.owner
                .retain_annotation_artifact(
                    encoder.seal(snapshot.clone(), b"[]".to_vec()).unwrap(),
                    &mut f.recorder,
                )
                .unwrap(),
        );
    }
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner.retain_annotation_artifact(
            encoder.seal(snapshot.clone(), b"[]".to_vec()).unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(f.owner.resource_usage(), before);
    let id = f
        .owner
        .retain_prepared_source_artifact(&p, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().retained_artifacts, 8);
    assert_eq!(f.owner.resource_usage().prepared_sources, 0);
    assert_eq!(
        f.owner.resource_usage().held_artifact_bytes,
        before.held_artifact_bytes
    );
    for id in others.into_iter().chain(std::iter::once(id)) {
        f.owner
            .retire_resource_artifact(&id, &mut f.recorder)
            .unwrap();
    }
    assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
    assert_eq!(f.owner.resource_usage().issued_artifacts_lifetime, 8);
}

#[test]
fn given_bad_encoding_oversize_or_serialization_basis_then_prepare_spends_no_credit_or_task() {
    let mut f = Fixture::new();
    let d = f.owner.create(b"A", &mut f.recorder).unwrap();
    let mut s = f
        .owner
        .source_serialization(&d.document, &mut f.recorder)
        .unwrap();
    s.generation += 1;
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner
            .prepare_source_artifact(&d, s, &mut f.recorder)
            .unwrap_err(),
        ResourceError::Stale
    );
    assert_eq!(f.owner.resource_usage(), before);
    let big = f.owner.create(&[b'x'; 65], &mut f.recorder).unwrap();
    let big_s = f
        .owner
        .source_serialization(&big.document, &mut f.recorder)
        .unwrap();
    f.recorder = Recorder::try_with_capacity(16).unwrap();
    assert_eq!(
        f.owner
            .prepare_source_artifact(&big, big_s, &mut f.recorder)
            .unwrap_err(),
        ResourceError::Limit
    );
    assert_eq!(f.recorder.queued(), 1); // Sizing refusal precedes codec copying.
    let plan = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "\u{feff}".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(plan, &mut f.recorder).unwrap();
    let current = f.owner.read(&d.document, &mut f.recorder).unwrap().version;
    let s = f
        .owner
        .source_serialization(&d.document, &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner
            .prepare_source_artifact(&current, s, &mut f.recorder)
            .unwrap_err(),
        ResourceError::Source(vellum_engine::source::SourceError::AmbiguousBom)
    );
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(f.owner.resource_usage().prepared_sources, 0);
}

#[test]
fn given_off_available_or_full_diagnostics_then_prepare_commit_and_discard_remain_content_free_and_equivalent()
 {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(16), Some(1)] {
        let mut f = Fixture::new();
        let d = f.owner.create(b"PRIVATE_PREPARE", &mut f.recorder).unwrap();
        let s = f
            .owner
            .source_serialization(&d.document, &mut f.recorder)
            .unwrap();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let p = f
            .owner
            .prepare_source_artifact(&d, s, &mut f.recorder)
            .unwrap();
        f.owner
            .discard_source_artifact_preparation(&p, &mut f.recorder)
            .unwrap();
        let p = f
            .owner
            .prepare_source_artifact(&d, s, &mut f.recorder)
            .unwrap();
        let id = f
            .owner
            .retain_prepared_source_artifact(&p, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.resource_usage().issued_artifacts_lifetime, 1);
        assert_eq!(f.owner.resource_usage().issued_lifetime, 0);
        let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{events:?}").contains("PRIVATE_PREPARE"));
        if capacity == Some(16) {
            for b in [
                Boundary::SourceArtifactPreparation,
                Boundary::SourceArtifactRetention,
                Boundary::SourceArtifactDiscard,
            ] {
                assert!(events.iter().any(|e| e.boundary == b));
            }
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::SourceEncode)
                    .count(),
                2
            );
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
        f.owner
            .retire_resource_artifact(&id, &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.resource_usage().held_artifact_bytes, 0);
    }
}
