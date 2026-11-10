use vellum_engine::attachment::{AttachmentTarget, LocationEvidence, Placement};
use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
mod fixture;
use fixture::Fixture;
#[path = "annotation_restore_behaviors/load_facade.rs"]
mod load_facade;
#[path = "annotation_restore_behaviors/owned.rs"]
mod owned;
#[path = "annotation_restore_behaviors/root_admission.rs"]
mod root_admission;
fn captured(
    f: &mut Fixture,
) -> (
    AnnotationReadId,
    AnnotationReadDispatch,
    CapturedAnnotationRead,
) {
    let set = f
        .owner
        .annotation_set_version(&f.doc.document, &mut f.recorder)
        .unwrap();
    let id = if set == 0 {
        f.admit()
    } else {
        let pair = f.pair();
        f.owner
            .admit_annotation_read(
                &f.doc,
                set,
                f.port.annotation_read_target(b"comments").unwrap(),
                ReadStrength::CapturedRead,
                pair,
                &mut f.recorder,
            )
            .unwrap()
    };
    let dispatch = f.submit(&id);
    for fact in [
        f.port.annotation_read_start(&dispatch, b"v0").unwrap(),
        f.port
            .annotation_read_chunk(&dispatch, 0, 0, b"opaque")
            .unwrap(),
        f.port.annotation_read_end(&dispatch, b"v0", 1, 6).unwrap(),
        f.port.annotation_read_quiescence(&dispatch).unwrap(),
    ] {
        f.record(fact).unwrap();
    }
    let capture = f.capture(&id).unwrap();
    (id, dispatch, capture)
}
fn source(f: &mut Fixture) -> AnnotationSnapshot {
    let set = f
        .owner
        .annotation_set_version(&f.doc.document, &mut f.recorder)
        .unwrap();
    f.owner
        .capture_annotations(&f.doc, set, &mut f.recorder)
        .unwrap()
}
fn row(port: &AnnotationRestorerPort, source: &DocumentVersion) -> RestoredAnnotation {
    RestoredAnnotation {
        identity: port.identity(b"approved-record", 7).unwrap(),
        body: "loaded body".into(),
        content_version: 5,
        target_generation: 9,
        authored: SourceTarget {
            source: source.clone(),
            basis: TargetBasis::Before,
            target: AttachmentTarget::ContentRange { start: 0, end: 4 },
        },
        placement: Placement::Located {
            target: AttachmentTarget::ContentRange { start: 0, end: 4 },
            evidence: LocationEvidence::Affected,
        },
    }
}
#[test]
fn initial_restoration_preserves_floors_and_source_and_pins_the_actual_consumed_read() {
    let mut f = Fixture::new();
    let (id, dispatch, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let proof = port
        .seal(capture, vec![witness], vec![row(&port, &f.doc)])
        .unwrap();
    let generation = f.owner.generation();
    let pair = f.pair();
    let outcome = f
        .owner
        .restore_annotations(proof, pair, &mut f.recorder)
        .unwrap();
    assert!(outcome.changed);
    assert_eq!(outcome.annotations.len(), 1);
    assert_eq!(f.owner.generation(), generation + 1);
    let record = f
        .owner
        .annotation(&outcome.annotations[0].annotation, &mut f.recorder)
        .unwrap();
    assert_eq!(record.body, "loaded body");
    assert_eq!(record.content_version, 5);
    assert_eq!(record.target_generation, 9);
    assert_eq!(record.restoration_identity.unwrap().incarnation(), 7);
    assert_eq!(
        f.owner
            .read(&f.doc.document, &mut f.recorder)
            .unwrap()
            .version,
        f.doc
    );
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    drop(dispatch);
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let retained = f
        .owner
        .annotation_restoration_outcome(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(retained.annotations, outcome.annotations);
    assert!(!retained.loaded_baseline_adopted);
    let p = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: outcome.annotations[0].annotation.clone(),
                expected_content: 5,
                expected_target: 9,
            }],
            &mut f.recorder,
        )
        .unwrap();
    let deleted = f.owner.commit(p, &mut f.recorder).unwrap();
    f.owner
        .retire_history_group(&deleted.history_group.unwrap(), &mut f.recorder)
        .unwrap();
    let before = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    let resources = f.owner.resource_usage();
    f.owner
        .retire_absent_annotation(
            &deleted.annotation_changes[0],
            &TrustedAnnotationRetirementV1 {
                authorized: Some(true),
                membership_current: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
                session_references_released: Some(true),
                authored_disposition_ready: Some(true),
                history_released: Some(true),
                versions_available: Some(true),
                resources_ready: Some(true),
                bookkeeping_ready: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    let after = f.owner.semantic_root_usage(&mut f.recorder).unwrap();
    // The persistent restoration key remains funded. Only the body leaves the
    // annotation root; original read provenance/result ownership is untouched.
    assert_eq!(
        before.annotations_bytes - after.annotations_bytes,
        "loaded body".len()
    );
    assert_eq!(f.owner.resource_usage(), resources);
    assert_eq!(f.owner.annotation_usage().issued_lifetime, 1);
    assert_eq!(f.owner.annotation_usage().retained_records, 0);
    assert_eq!(
        f.owner
            .annotation_restoration_outcome(&id, &mut f.recorder)
            .unwrap()
            .annotations,
        outcome.annotations
    );
}
#[test]
fn pending_lifetime_slots_are_not_a_known_empty_inventory_and_refusal_spends_no_restore_ids() {
    let mut f = Fixture::new();
    let (id, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let pending = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "pending".into(),
                target: SourceTarget {
                    source: f.doc.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
            }],
            &mut f.recorder,
        )
        .unwrap();
    drop(pending);
    let proof = port
        .seal(capture, vec![witness.clone()], vec![row(&port, &f.doc)])
        .unwrap();
    let before = f.owner.annotation_usage();
    let generation = f.owner.generation();
    let pair = f.pair();
    assert!(matches!(
        f.owner.restore_annotations(proof, pair, &mut f.recorder),
        Err(ResourceError::Retained)
    ));
    assert_eq!(f.owner.annotation_usage(), before);
    assert_eq!(f.owner.generation(), generation);
    assert!(
        !f.owner
            .annotation_read_status(&id, &mut f.recorder)
            .unwrap()
            .restored
    );
}

#[test]
fn historical_selectors_need_real_managed_source_witnesses_and_source_undo_preserves_loaded_authorship()
 {
    let mut f = Fixture::new();
    let old = source(&mut f);
    let old_version = f.doc.clone();
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: f.doc.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 4,
                    expected: "text".into(),
                    inserted: "changed".into(),
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
    f.doc = f
        .owner
        .read(&f.doc.document, &mut f.recorder)
        .unwrap()
        .version;
    let (id, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let current = source(&mut f);
    let mut imported = row(&port, &old_version);
    imported.placement = Placement::Detached;
    let missing = port
        .seal(capture.clone(), vec![current.clone()], vec![imported])
        .unwrap();
    let pair = f.pair();
    assert!(matches!(
        f.owner.restore_annotations(missing, pair, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    ));
    assert_eq!(f.owner.annotation_usage().issued_lifetime, 0);
    let mut imported = row(&port, &old_version);
    imported.placement = Placement::Detached;
    let package = port
        .seal(capture, vec![current, old], vec![imported])
        .unwrap();
    let before = f.owner.history_usage();
    let pair = f.pair();
    let outcome = f
        .owner
        .restore_annotations(package, pair, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.history_usage().retained_groups,
        before.retained_groups
    );
    let undo = f
        .owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut f.recorder)
        .unwrap();
    f.owner.commit(undo, &mut f.recorder).unwrap();
    let record = f
        .owner
        .annotation(&outcome.annotations[0].annotation, &mut f.recorder)
        .unwrap();
    assert_eq!(record.body, "loaded body");
    assert_eq!(record.authored.source, old_version);
    assert_eq!(record.placement, Placement::Detached);
    assert_eq!(
        f.owner
            .annotation_restoration_outcome(&id, &mut f.recorder)
            .unwrap()
            .annotations,
        outcome.annotations
    );
}

#[test]
fn an_invalid_last_record_refuses_the_complete_set_without_issuing_the_valid_prefix() {
    let mut f = Fixture::new();
    let (_, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let valid = row(&port, &f.doc);
    let mut invalid = row(&port, &f.doc);
    invalid.identity = port.identity(b"second", 2).unwrap();
    invalid.authored.target = AttachmentTarget::ContentRange { start: 0, end: 99 };
    let package = port
        .seal(capture, vec![witness], vec![valid, invalid])
        .unwrap();
    let generation = f.owner.generation();
    let pair = f.pair();
    assert!(matches!(
        f.owner.restore_annotations(package, pair, &mut f.recorder),
        Err(ResourceError::Malformed)
    ));
    let usage = f.owner.annotation_usage();
    assert_eq!(usage.issued_lifetime, 0);
    assert_eq!(usage.present_records, 0);
    assert_eq!(f.owner.generation(), generation);
}

#[test]
fn empty_hydration_consumes_once_without_publication_and_does_not_become_a_create_group() {
    let mut f = Fixture::new();
    let (id, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let package = port
        .seal(capture.clone(), vec![witness.clone()], vec![])
        .unwrap();
    let generation = f.owner.generation();
    let pair = f.pair();
    let outcome = f
        .owner
        .restore_annotations(package, pair, &mut f.recorder)
        .unwrap();
    assert!(!outcome.changed);
    assert!(outcome.annotations.is_empty());
    assert_eq!(f.owner.generation(), generation);
    let replay = port.seal(capture, vec![witness], vec![]).unwrap();
    let pair = f.pair();
    assert!(matches!(
        f.owner.restore_annotations(replay, pair, &mut f.recorder),
        Err(ResourceError::Retained)
    ));
    assert!(
        f.owner
            .annotation_read_status(&id, &mut f.recorder)
            .unwrap()
            .restored
    );
    assert_eq!(f.owner.history_usage().retained_groups, 0);
}

#[test]
fn retained_absent_records_are_not_a_fresh_inventory_even_after_the_visible_set_is_empty() {
    let mut f = Fixture::new();
    let create = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "old".into(),
                target: SourceTarget {
                    source: f.doc.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
            }],
            &mut f.recorder,
        )
        .unwrap();
    let old = create.created_annotations().next().unwrap().clone();
    f.owner.commit(create, &mut f.recorder).unwrap();
    let delete = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: old,
                expected_content: 1,
                expected_target: 1,
            }],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(delete, &mut f.recorder).unwrap();
    let (_, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    assert!(witness.records().is_empty());
    let package = port
        .seal(capture, vec![witness], vec![row(&port, &f.doc)])
        .unwrap();
    let pair = f.pair();
    assert!(matches!(
        f.owner.restore_annotations(package, pair, &mut f.recorder),
        Err(ResourceError::Retained)
    ));
}

#[test]
fn late_contestation_never_rolls_back_loaded_bodies_or_subsequent_authored_edits() {
    let mut f = Fixture::new();
    let (id, dispatch, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let package = port
        .seal(capture, vec![witness], vec![row(&port, &f.doc)])
        .unwrap();
    let pair = f.pair();
    let outcome = f
        .owner
        .restore_annotations(package, pair, &mut f.recorder)
        .unwrap();
    let annotation = outcome.annotations[0].annotation.clone();
    let edit = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: annotation.clone(),
                expected_content: 5,
                body: "later authored".into(),
            }],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(edit, &mut f.recorder).unwrap();
    let generation = f.owner.generation();
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.record(
            f.port
                .annotation_read_start(&dispatch, b"different")
                .unwrap()
        ),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert_eq!(
        f.owner
            .annotation(&annotation, &mut f.recorder)
            .unwrap()
            .body,
        "later authored"
    );
    let retained = f
        .owner
        .annotation_restoration_outcome(&id, &mut f.recorder)
        .unwrap();
    assert!(retained.evidence_contested);
    assert_eq!(retained.annotations[0].content_version, 5);
    assert_eq!(f.owner.generation(), generation);
}

#[test]
fn imported_maximum_floors_refuse_subsequent_increment_without_rounding_or_wrapping() {
    let mut f = Fixture::new();
    let (_, _, capture) = captured(&mut f);
    let port = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = source(&mut f);
    let mut imported = row(&port, &f.doc);
    imported.content_version = u64::MAX;
    imported.target_generation = u64::MAX;
    let package = port.seal(capture, vec![witness], vec![imported]).unwrap();
    let pair = f.pair();
    let outcome = f
        .owner
        .restore_annotations(package, pair, &mut f.recorder)
        .unwrap();
    let annotation = outcome.annotations[0].annotation.clone();
    let generation = f.owner.generation();
    assert!(matches!(
        f.owner.prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: annotation.clone(),
                expected_content: u64::MAX,
                body: "cannot wrap".into()
            }],
            &mut f.recorder
        ),
        Err(CoordinatorError::VersionExhausted)
    ));
    assert_eq!(
        f.owner
            .annotation(&annotation, &mut f.recorder)
            .unwrap()
            .content_version,
        u64::MAX
    );
    assert_eq!(f.owner.generation(), generation);
}

#[test]
fn current_policy_pair_stop_and_producer_guards_refuse_without_erasing_unrelated_peers() {
    for case in 0..4 {
        let mut f = Fixture::new();
        let (id, _, capture) = captured(&mut f);
        let port = f
            .owner
            .register_annotation_restorer(&mut f.recorder)
            .unwrap();
        let witness = source(&mut f);
        let mut foreign = Fixture::new();
        let foreign_port = foreign
            .owner
            .register_annotation_restorer(&mut foreign.recorder)
            .unwrap();
        assert!(matches!(
            foreign_port.seal(capture.clone(), vec![witness.clone()], vec![]),
            Err(ResourceError::Unavailable)
        ));
        let package = port
            .seal(capture, vec![witness], vec![row(&port, &f.doc)])
            .unwrap();
        let peer = f.owner.create(b"peer survives", &mut f.recorder).unwrap();
        if case == 0 {
            f.owner
                .set_annotations_read_only(&f.doc.document, true, &mut f.recorder)
                .unwrap();
        }
        if case == 1 {
            f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
        }
        let pair = if case == 2 { None } else { f.pair() };
        let before = f.owner.generation();
        let result = f.owner.restore_annotations(package, pair, &mut f.recorder);
        match case {
            0 => assert!(matches!(result, Err(ResourceError::Permission))),
            1 => assert!(matches!(result, Err(ResourceError::Unavailable))),
            2 => assert!(matches!(result, Err(ResourceError::MissingEvidence))),
            _ => assert!(result.unwrap().changed),
        }
        assert_eq!(
            f.owner
                .read(&peer.document, &mut f.recorder)
                .unwrap()
                .source
                .text(),
            "peer survives"
        );
        assert_eq!(f.owner.generation(), before + u64::from(case == 3));
    }
}
