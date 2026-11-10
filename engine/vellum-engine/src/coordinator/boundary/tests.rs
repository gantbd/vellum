use super::*;
fn limits() -> BoundaryLimits {
    BoundaryLimits {
        max_issued: 8,
        max_retained: 4,
        max_source_bytes: 1024,
        max_boundaries: 64,
        max_words: 8,
        max_evidence_bytes: 4096,
        max_retained_bytes: 8192,
    }
}
fn fixture(bounds: BoundaryLimits) -> (SourceCoordinator, DocumentVersion, Recorder) {
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 16,
            max_edit_payload_bytes: 1024,
        },
    });
    let mut r = Recorder::disabled();
    owner.configure_boundaries(bounds, &mut r).unwrap();
    let source = owner.create(b"abc", &mut r).unwrap();
    (owner, source, r)
}
fn table() -> BoundaryTableV1 {
    BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: vec![0, 1, 2, 3].into_iter().map(BytePosition).collect(),
        words: vec![BytePosition(0), BytePosition(3)],
        significant_words: vec![ByteRange { start: 0, end: 3 }],
    }
}
#[test]
fn exact_native_byte_and_vector_caps_refuse_before_installing_any_table_prefix() {
    let required = std::mem::size_of::<Record>() + 3 + table_bytes(&table()).unwrap();
    for bounds in [
        BoundaryLimits {
            max_evidence_bytes: required - 1,
            ..limits()
        },
        BoundaryLimits {
            max_retained_bytes: required - 1,
            ..limits()
        },
        BoundaryLimits {
            max_boundaries: 7,
            ..limits()
        },
        BoundaryLimits {
            max_words: 0,
            ..limits()
        },
    ] {
        let (mut owner, source, mut r) = fixture(bounds);
        let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
        let before = owner.boundary_usage();
        assert_eq!(
            owner.accept_boundary_evidence(&capture, &table(), &mut r),
            Err(BoundaryError::Limit)
        );
        assert_eq!(owner.boundary_usage(), before);
        assert_eq!(owner.boundary_source(&capture, &mut r).unwrap().text, "abc");
    }
    let (mut owner, source, mut r) = fixture(BoundaryLimits {
        max_evidence_bytes: required,
        max_retained_bytes: required,
        max_boundaries: 8,
        max_words: 1,
        ..limits()
    });
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    owner
        .accept_boundary_evidence(&capture, &table(), &mut r)
        .unwrap();
    assert_eq!(owner.boundary_usage().retained_bytes, required);
}
#[test]
fn total_live_capture_bytes_and_slots_do_not_get_refunded_as_lifetime_identity() {
    let size = std::mem::size_of::<Record>() + 3;
    let (mut owner, source, mut r) = fixture(BoundaryLimits {
        max_issued: 2,
        max_retained: 1,
        max_retained_bytes: size,
        ..limits()
    });
    let first = owner.begin_boundary_capture(&source, &mut r).unwrap();
    assert_eq!(
        owner.begin_boundary_capture(&source, &mut r),
        Err(BoundaryError::Limit)
    );
    owner.retire_boundary_capture(&first, &mut r).unwrap();
    let second = owner.begin_boundary_capture(&source, &mut r).unwrap();
    assert_ne!(first, second);
    owner.retire_boundary_capture(&second, &mut r).unwrap();
    assert_eq!(owner.boundary_usage().retained_bytes, 0);
    assert_eq!(
        owner.begin_boundary_capture(&source, &mut r),
        Err(BoundaryError::Limit)
    );
}
#[test]
fn unexpected_owned_shape_drift_fences_mutation_but_protected_retirement_still_frees_it() {
    for mode in 0..3 {
        let (mut owner, source, mut r) = fixture(limits());
        let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
        let evidence = owner
            .accept_boundary_evidence(&capture, &table(), &mut r)
            .unwrap();
        match mode {
            0 => {
                owner.boundaries.rows[evidence.slot]
                    .as_mut()
                    .unwrap()
                    .table
                    .as_mut()
                    .unwrap()
                    .graphemes[1] = BytePosition(99)
            }
            1 => owner.boundaries.limits = None,
            _ => {
                owner.boundaries.rows[evidence.slot]
                    .as_mut()
                    .unwrap()
                    .version
                    .document
                    .namespace = Arc::new(())
            }
        }
        let generation = owner.generation();
        assert_eq!(
            owner.begin_boundary_capture(&source, &mut r),
            Err(BoundaryError::Faulted)
        );
        assert!(owner.is_faulted());
        assert_eq!(owner.generation(), generation);
        assert_eq!(
            owner.boundary_evidence(&evidence, &mut r).err(),
            Some(BoundaryError::Faulted)
        );
        owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
        assert_eq!(owner.boundary_usage().retained_bytes, 0);
        assert!(owner.is_faulted());
    }
}
#[test]
fn empty_source_uses_single_zero_endpoints_and_no_unused_publication_counter() {
    let (mut owner, _, mut r) = fixture(limits());
    let source = owner.create(b"", &mut r).unwrap();
    owner.generation = u64::MAX;
    let capture = owner.begin_boundary_capture(&source, &mut r).unwrap();
    let data = BoundaryTableV1 {
        profile: BoundaryProfileV1::Unicode18Uax29Revision49,
        graphemes: vec![BytePosition(0)],
        words: vec![BytePosition(0)],
        significant_words: vec![],
    };
    let evidence = owner
        .accept_boundary_evidence(&capture, &data, &mut r)
        .unwrap();
    assert_eq!(
        owner.boundary_evidence(&evidence, &mut r).unwrap().version,
        source
    );
    owner.retire_boundary_evidence(&evidence, &mut r).unwrap();
    assert_eq!(owner.generation(), u64::MAX);
}
