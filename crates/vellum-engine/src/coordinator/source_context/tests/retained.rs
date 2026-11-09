use super::*;

#[test]
fn retained_request_counts_the_original_selector_and_rejects_oversize_before_observing_loss() {
    let (_, _, s) = base();
    let bound = scope_bytes(&s).unwrap()
        + std::mem::size_of::<DocumentId>()
        + std::mem::size_of::<SourceReadContextId>();
    for delta in [-1isize, 0, 1] {
        let p = SourceReadContextLimitsV1 {
            max_request_bytes: bound.checked_add_signed(delta).unwrap(),
            ..limits()
        };
        let (mut o, mut r, s) = configured(p);
        let original = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let before = o.retention_usage();
        let mut f = facts(&s);
        if delta < 0 {
            f.source_readable = Some(false);
        }
        let child = o.capture_retained_source_read_context(&original, &s, &f, &mut r);
        assert_eq!(child.is_some(), delta >= 0);
        assert!(
            !o.source_pool().unwrap().rows[original.slot]
                .as_ref()
                .unwrap()
                .closed
        );
        assert_eq!(o.retention_usage(), before);
        assert_eq!(
            o.source_read_context_usage().issued_lifetime,
            if delta < 0 { 1 } else { 2 }
        );
        assert!(!o.faulted);
    }
}

#[test]
fn retained_copy_uses_only_the_original_projection_not_a_newer_whole_root_or_current_source_revision()
 {
    let (probe, mut r, _) = base();
    let root = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    let (mut o, mut r, s) = configured(SourceReadContextLimitsV1 {
        max_root_bytes: root,
        ..limits()
    });
    let original = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let version = o.source_context_row(&original, &s).unwrap().version.clone();
    let plan = o
        .prepare(
            &[DocumentEdits {
                expected: version.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 4,
                    expected: "a\r\nx".into(),
                    inserted: "longer".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(plan, &mut r).unwrap();
    assert!(o.semantic_root_usage(&mut r).unwrap().total_bytes > root);
    assert!(
        o.capture_source_read_context(&s, &facts(&s), &mut r)
            .is_none()
    );
    let child = o
        .capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
        .unwrap();
    let result = o
        .query_source_read_context(&child, &s, range(4), &facts(&s), &mut r)
        .unwrap();
    let SourceContextContentV1::Range(result) = result.content else {
        panic!("wrong product")
    };
    assert_eq!(result.bytes(), b"a\r\nx");
    assert_eq!(result.source, version);
    assert_eq!(o.read(&s.document, &mut r).unwrap().source.text(), "longer");
}

#[test]
fn faulted_retained_acquisition_closes_delivery_without_discharging_a_real_hold_or_claiming_recovery()
 {
    let (mut o, mut r, s) = configured(limits());
    let original = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let hold = o
        .hold_source_read_context(
            &original,
            &s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.faulted = true;
    let before = o.retention_usage();
    assert!(
        o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
            .is_none()
    );
    assert!(
        o.source_pool().unwrap().rows[original.slot]
            .as_ref()
            .unwrap()
            .closed
    );
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
    assert_eq!(o.retention_usage(), before);
    assert!(o.release_source_read_hold(&hold, &discharge(&s), &mut r));
    assert!(o.retire_source_read_context(&original, &discharge(&s), &mut r));
    assert!(o.retire_source_read_contexts(&controls(), &mut r).unwrap());
    assert!(o.faulted);
}
