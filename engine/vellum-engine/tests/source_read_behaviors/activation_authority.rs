use super::*;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
fn facts() -> SourceLoadAuthorityFactsV1 {
    SourceLoadAuthorityFactsV1 {
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn completed_task_authority_is_stable_opaque_read_only_and_does_not_pin_a_retired_task() {
    for capacity in [0, 1, 128] {
        let mut f = setup();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let (id, dispatch) = submitted(&mut f, b"PRIVATE-RESOURCE");
        assert!(
            f.owner
                .borrow_source_load_authority_v1(&id, &facts(), &mut f.recorder)
                .is_err()
        );
        complete(&mut f, &dispatch, b"\xef\xbb\xbfEnglish\r\nDeutsch\n");
        let generation = f.owner.generation();
        let usage = f.owner.source_reader_usage_v1().unwrap();
        let funding = f.owner.retention_usage();
        let authority = f
            .owner
            .borrow_source_load_authority_v1(&id, &facts(), &mut f.recorder)
            .unwrap();
        let repeated = f
            .owner
            .borrow_source_load_authority_v1(&id, &facts(), &mut f.recorder)
            .unwrap();
        assert_eq!(authority, repeated);
        assert_eq!(
            f.owner
                .source_load_authority_current_v1(&authority, &facts(), &mut f.recorder),
            Ok(())
        );
        assert_eq!(f.owner.source_reader_usage_v1().unwrap(), usage);
        assert_eq!(f.owner.retention_usage(), funding);
        assert_eq!(f.owner.generation(), generation);
        f.owner
            .record_source_read_v1(
                f.port.source_read_quiescence_v1(&dispatch).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        drop(dispatch);
        f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
        assert_eq!(
            f.owner
                .source_load_authority_current_v1(&authority, &facts(), &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(
            f.owner
                .source_load_authority_current_v1(&repeated, &facts(), &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(f.owner.source_reader_usage_v1().unwrap().live, 0);
        let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{records:?} {authority:?}").contains("PRIVATE"));
        if capacity == 128 {
            for boundary in [
                Boundary::SourceLoadAuthorityBorrow,
                Boundary::SourceLoadAuthorityValidation,
            ] {
                assert!(
                    records
                        .iter()
                        .any(|r| r.boundary == boundary && r.service_time.is_some())
                );
            }
        }
    }
}
#[test]
fn authority_requires_every_current_control_fact_and_cannot_cross_owners_tasks_or_revocation() {
    let mut f = setup();
    let (id, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let authority = f
        .owner
        .borrow_source_load_authority_v1(&id, &facts(), &mut f.recorder)
        .unwrap();
    for field in 0..3 {
        for denied in [None, Some(false)] {
            let mut current = facts();
            match field {
                0 => current.owner_control_allowed = denied,
                1 => current.preparation_reserved = denied,
                _ => current.resources_ready = denied,
            }
            assert_eq!(
                f.owner
                    .borrow_source_load_authority_v1(&id, &current, &mut f.recorder)
                    .err(),
                Some(ResourceError::Unavailable)
            );
            assert_eq!(
                f.owner
                    .source_load_authority_current_v1(&authority, &current, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
        }
    }
    assert_eq!(
        f.owner
            .source_load_authority_current_v1(&authority, &facts(), &mut f.recorder),
        Ok(())
    );
    let (peer, peer_dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &peer_dispatch, b"candidate");
    let other = f
        .owner
        .borrow_source_load_authority_v1(&peer, &facts(), &mut f.recorder)
        .unwrap();
    assert_ne!(authority, other);
    let mut foreign = setup();
    assert_eq!(
        foreign
            .owner
            .source_load_authority_current_v1(&authority, &facts(), &mut foreign.recorder),
        Err(ResourceError::Unavailable)
    );
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .source_load_authority_current_v1(&authority, &facts(), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(
        f.owner
            .source_load_authority_current_v1(&other, &facts(), &mut f.recorder),
        Ok(())
    );
}
#[test]
fn late_conflict_revokes_the_codec_witness_without_activating_or_disclosing_source() {
    let mut f = setup();
    let (id, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let authority = f
        .owner
        .borrow_source_load_authority_v1(&id, &facts(), &mut f.recorder)
        .unwrap();
    let generation = f.owner.generation();
    f.owner
        .record_source_read_v1(
            f.port
                .source_read_chunk_v1(&dispatch, 0, 0, b"different")
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert!(
        f.owner
            .source_load_authority_current_v1(&authority, &facts(), &mut f.recorder)
            .is_err()
    );
    assert!(
        f.owner
            .borrow_source_load_authority_v1(&id, &facts(), &mut f.recorder)
            .is_err()
    );
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner
            .source_read_status_v1(&id, &mut f.recorder)
            .unwrap()
            .activated,
        None
    );
}
