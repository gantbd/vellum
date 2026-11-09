use super::*;
use crate::instrumentation::NativeTimingProfileV1;
fn facts() -> AnnotationLoadAuthorityFactsV1 {
    AnnotationLoadAuthorityFactsV1 {
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        resources_ready: Some(true),
    }
}
fn ready() -> (
    SourceCoordinator,
    ProviderPort,
    AnnotationReadId,
    AnnotationReadDispatch,
    Recorder,
) {
    let (mut o, p, id, mut r) = super::super::tests::fixture();
    let dispatch = o.submit_annotation_read(&id, None, &mut r).unwrap();
    for e in [
        p.annotation_read_start(&dispatch, b"v0").unwrap(),
        p.annotation_read_chunk(&dispatch, 0, 0, b"PRIVATE")
            .unwrap(),
        p.annotation_read_end(&dispatch, b"v0", 1, 7).unwrap(),
    ] {
        o.record_annotation_read(e, &mut r).unwrap();
    }
    (o, p, id, dispatch, r)
}
#[test]
fn authority_is_stable_read_only_weak_and_revoked_by_real_unconsumed_retirement() {
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 128] {
            let (mut o, p, id, dispatch, _) = ready();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let usage = o.resource_usage();
            let root = o.native_root_identity().unwrap();
            let capture = o.read_task(&id).unwrap().capture.as_ref().unwrap();
            let strong = Arc::strong_count(capture);
            let a = o
                .borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
                .unwrap();
            let b = o
                .borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
                .unwrap();
            assert_eq!(a, b);
            assert_eq!(
                o.annotation_load_authority_current_v1(&a, &facts(), &mut r),
                Ok(())
            );
            assert_eq!(
                Arc::strong_count(o.read_task(&id).unwrap().capture.as_ref().unwrap()),
                strong
            );
            assert_eq!(o.resource_usage(), usage);
            assert_eq!(o.native_root_identity().unwrap(), root);
            o.record_annotation_read(p.annotation_read_quiescence(&dispatch).unwrap(), &mut r)
                .unwrap();
            drop(dispatch);
            o.retire_annotation_read(&id, &mut r).unwrap();
            assert_eq!(
                o.annotation_load_authority_current_v1(&a, &facts(), &mut r),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(
                o.annotation_load_authority_current_v1(&b, &facts(), &mut r),
                Err(ResourceError::Unavailable)
            );
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(events.len() <= capacity);
            assert!(!format!("{events:?} {a:?}").contains("PRIVATE"));
            if capacity == 128 {
                for boundary in [
                    Boundary::AnnotationLoadAuthorityBorrow,
                    Boundary::AnnotationLoadAuthorityValidation,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.disposition == Disposition::Completed
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}
#[test]
fn factory_requires_complete_input_and_every_fact_and_cannot_cross_owners_or_witnesses() {
    let (mut o, _, id, mut r) = super::super::tests::fixture();
    assert_eq!(
        o.borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
            .err(),
        Some(ResourceError::MissingEvidence)
    );
    let (mut o, _, id, _, mut r) = ready();
    let a = o
        .borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
        .unwrap();
    for field in 0..3 {
        for value in [None, Some(false)] {
            let mut f = facts();
            *match field {
                0 => &mut f.owner_control_allowed,
                1 => &mut f.preparation_reserved,
                _ => &mut f.resources_ready,
            } = value;
            assert_eq!(
                o.borrow_annotation_load_authority_v1(&id, &f, &mut r).err(),
                Some(ResourceError::Unavailable)
            );
            assert_eq!(
                o.annotation_load_authority_current_v1(&a, &f, &mut r),
                Err(ResourceError::Unavailable)
            );
        }
    }
    let (mut other, _, _, _, mut recorder) = ready();
    assert_eq!(
        other.annotation_load_authority_current_v1(&a, &facts(), &mut recorder),
        Err(ResourceError::Unavailable)
    );
    for field in 0..3 {
        let mut replaced = o
            .borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
            .unwrap();
        match field {
            0 => replaced.registration = Weak::new(),
            1 => replaced.record = Weak::new(),
            _ => replaced.capture = Weak::new(),
        }
        assert_eq!(
            o.annotation_load_authority_current_v1(&replaced, &facts(), &mut r),
            Err(ResourceError::Unavailable)
        );
    }
    o.stop_annotation_read(&id, &mut r).unwrap();
    assert_eq!(
        o.annotation_load_authority_current_v1(&a, &facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(!o.faulted);
}
#[test]
fn consumed_stop_preserves_authority_but_real_checkpoint_discharge_revokes_without_extra_pins() {
    let (mut o, id, _, mut r) = super::super::tests::checkpointed_read_mode(false);
    let a = o
        .borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
        .unwrap();
    o.stop_annotation_read(&id, &mut r).unwrap();
    assert_eq!(
        o.annotation_load_authority_current_v1(&a, &facts(), &mut r),
        Ok(())
    );
    o.retire_superseded_annotation_read(&id, &mut r).unwrap();
    assert_eq!(
        o.annotation_load_authority_current_v1(&a, &facts(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(!o.faulted);
}
#[test]
fn late_conflict_revokes_authority_and_impossible_native_transcript_drift_faults_before_publication()
 {
    for drift in 0..3 {
        let corrupt = drift != 0;
        let (mut o, p, id, dispatch, mut r) = ready();
        let generation = o.generation;
        let a = o
            .borrow_annotation_load_authority_v1(&id, &facts(), &mut r)
            .unwrap();
        if drift == 2 {
            o.resources.reads[id.slot].as_mut().unwrap().capture = None;
        } else if corrupt {
            o.resources.reads[id.slot].as_mut().unwrap().chunks[0].0 = 1;
        } else {
            o.record_annotation_read(
                p.annotation_read_chunk(&dispatch, 0, 0, b"changed")
                    .unwrap(),
                &mut r,
            )
            .unwrap();
        }
        assert_eq!(
            o.annotation_load_authority_current_v1(&a, &facts(), &mut r),
            Err(if corrupt {
                ResourceError::Faulted
            } else {
                ResourceError::Contested
            })
        );
        assert_eq!(o.faulted, corrupt);
        assert_eq!(o.generation, generation);
    }
}
