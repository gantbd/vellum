use super::*;
#[test]
fn source_load_authority_binds_the_exact_codec_witness_and_corruption_fails_before_activation() {
    for mode in ["witness", "codec", "transcript"] {
        let (mut o, mut r, _, port) = super::super::serialization_tests::fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        o.configure_source_reader_v1(limits(), &mut r).unwrap();
        let id = o
            .admit_source_read_v1(
                port.source_read_target_v1(b"new").unwrap(),
                ReadStrength::CapturedRead,
                &mut r,
            )
            .unwrap();
        let dispatch = o.submit_source_read_v1(&id, &mut r).unwrap();
        let version = SourceReadVersionV1::Opaque(b"v1");
        for evidence in [
            port.source_read_start_v1(&dispatch, version).unwrap(),
            port.source_read_chunk_v1(&dispatch, 0, 0, b"candidate")
                .unwrap(),
            port.source_read_end_v1(&dispatch, version, 1, 9).unwrap(),
        ] {
            o.record_source_read_v1(evidence, &mut r).unwrap();
        }
        let facts = SourceLoadAuthorityFactsV1 {
            owner_control_allowed: Some(true),
            preparation_reserved: Some(true),
            resources_ready: Some(true),
        };
        let original = o
            .borrow_source_load_authority_v1(&id, &facts, &mut r)
            .unwrap();
        let generation = o.generation();
        let documents = o.documents.len();
        match mode {
            "witness" => o.source_read_task_mut(&id).unwrap().candidate_marker = Some(Arc::new(())),
            "codec" => {
                o.source_read_task_mut(&id).unwrap().candidate =
                    Some(Utf8Source::decode_observed(b"different", 128, &mut r).unwrap())
            }
            _ => o.source_read_task_mut(&id).unwrap().bytes.push(b'!'),
        }
        assert_eq!(
            o.source_load_authority_current_v1(&original, &facts, &mut r),
            Err(if mode == "witness" {
                ResourceError::Unavailable
            } else {
                ResourceError::Faulted
            })
        );
        if mode == "witness" {
            let replacement = o
                .borrow_source_load_authority_v1(&id, &facts, &mut r)
                .unwrap();
            assert_ne!(original, replacement);
            assert_eq!(
                o.source_load_authority_current_v1(&replacement, &facts, &mut r),
                Ok(())
            );
            assert!(!o.is_faulted());
        } else {
            assert!(o.is_faulted());
            assert_eq!(
                o.activate_source_read_v1(&id, &mut r),
                Err(ResourceError::Faulted)
            );
        }
        assert_eq!(o.generation(), generation);
        assert_eq!(o.documents.len(), documents);
    }
}
pub(super) fn limits() -> SourceReaderLimitsV1 {
    SourceReaderLimitsV1 {
        max_live: 2,
        max_issued: 4,
        max_input_bytes: 128,
        max_chunks: 8,
        max_held_bytes: 65536,
        max_store_bytes: 131072,
    }
}
#[test]
fn exact_child_store_and_real_shared_sponsorship_boundaries_precede_issuance() {
    for field in 0..3 {
        for delta in [-1, 0, 1] {
            let (mut o, mut r, _, p) = super::super::serialization_tests::fixture();
            let mut l = limits();
            let child = task_charge(
                l,
                p.registration.limits,
                3,
                o.limits.source.max_source_bytes,
            )
            .unwrap();
            if field == 0 {
                l.max_held_bytes = child.checked_add_signed(delta).unwrap();
            }
            let total = l.reserved_bytes().unwrap();
            if field == 1 {
                l.max_store_bytes = total.checked_add_signed(delta).unwrap();
            }
            let budget = if field == 2 {
                total.checked_add_signed(delta).unwrap()
            } else {
                1048576
            };
            o.configure_retention_budget(budget, &mut r).unwrap();
            let configured = o.configure_source_reader_v1(l, &mut r);
            if field != 0 && delta < 0 {
                assert_eq!(configured, Err(ResourceError::Limit));
                assert_eq!(o.retention_usage().reserved, 0);
                continue;
            }
            configured.unwrap();
            let sponsored = o.retention_usage();
            let result = o.admit_source_read_v1(
                p.source_read_target_v1(b"new").unwrap(),
                ReadStrength::CapturedRead,
                &mut r,
            );
            if field == 0 && delta < 0 {
                assert_eq!(result, Err(ResourceError::Limit));
                assert_eq!(o.source_reader_usage_v1().unwrap().issued, 0);
            } else {
                result.unwrap();
            }
            assert_eq!(o.retention_usage(), sponsored);
        }
    }
}
#[test]
fn exhausted_publication_keeps_candidate_and_integrity_failure_never_refunds_reader_debt() {
    let (mut o, mut r, _, p) = super::super::serialization_tests::fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_source_reader_v1(limits(), &mut r).unwrap();
    let id = o
        .admit_source_read_v1(
            p.source_read_target_v1(b"new").unwrap(),
            ReadStrength::CapturedRead,
            &mut r,
        )
        .unwrap();
    let dispatch = o.submit_source_read_v1(&id, &mut r).unwrap();
    for e in [
        p.source_read_start_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"))
            .unwrap(),
        p.source_read_end_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"), 0, 0)
            .unwrap(),
    ] {
        o.record_source_read_v1(e, &mut r).unwrap();
    }
    o.generation = u64::MAX;
    let documents = o.documents.len();
    let debt = o.retention_usage();
    assert_eq!(
        o.activate_source_read_v1(&id, &mut r),
        Err(ResourceError::VersionExhausted)
    );
    assert_eq!(o.documents.len(), documents);
    assert!(
        o.source_read_status_v1(&id, &mut r)
            .unwrap()
            .candidate_ready
    );
    o.source_reader_mut().unwrap().held = 0;
    assert_eq!(
        o.retire_source_read_v1(&id, &mut r),
        Err(ResourceError::Faulted)
    );
    assert!(o.is_faulted());
    assert!(o.source_read_task(&id).is_ok());
    assert_eq!(o.retention_usage(), debt);
    o.record_source_read_v1(p.source_read_quiescence_v1(&dispatch).unwrap(), &mut r)
        .unwrap();
    assert!(o.source_read_status_v1(&id, &mut r).unwrap().quiescent);
    assert_eq!(o.retention_usage(), debt);
}
