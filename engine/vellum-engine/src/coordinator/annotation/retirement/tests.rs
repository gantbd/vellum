use super::*;
fn facts() -> TrustedAnnotationRetirementV1 {
    TrustedAnnotationRetirementV1 {
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
    }
}
fn ready() -> (SourceCoordinator, DocumentVersion, AnnotationFact, Recorder) {
    let mut r = Recorder::disabled();
    let (mut o, source, id) = crate::coordinator::annotation::tests::setup(&mut r);
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: id,
                expected_content: 1,
                expected_target: 1,
            }],
            &mut r,
        )
        .unwrap();
    let out = o.commit(p, &mut r).unwrap();
    o.retire_history_group(&out.history_group.unwrap(), &mut r)
        .unwrap();
    (
        o,
        source,
        out.annotation_changes.into_iter().next().unwrap(),
        r,
    )
}

#[test]
fn retirement_preserves_maximum_floors_and_frees_record_capacity_but_never_lifetime_issuance() {
    let (mut o, source, absent, mut r) = ready();
    let slot = absent.annotation.slot;
    let value = &mut o.annotations.records[slot].as_mut().unwrap().value;
    value.content = u64::MAX;
    value.target_generation = u64::MAX;
    value.placement_generation = u64::MAX;
    let absent = value.fact();
    let limits = o.annotations.limits.as_mut().unwrap();
    limits.max_records = 1;
    limits.max_issued = 2;
    o.retire_absent_annotation(&absent, &facts(), &mut r)
        .unwrap();
    assert!(o.annotations.records[slot].is_none());
    assert_eq!(o.annotations.retired[slot].as_ref().unwrap().fact, absent);
    assert!(
        o.annotations.retired[slot]
            .as_ref()
            .unwrap()
            .restored_identity
            .is_none()
    );
    let request = AnnotationMutation::Create {
        body: "next".into(),
        target: SourceTarget {
            source,
            basis: TargetBasis::Before,
            target: AttachmentTarget::ContentRange { start: 1, end: 2 },
        },
    };
    let p = o
        .prepare_compound(&[], &[], std::slice::from_ref(&request), &mut r)
        .unwrap();
    let next = p.created_annotations().next().unwrap().clone();
    o.commit(p, &mut r).unwrap();
    assert_ne!(next, absent.annotation);
    assert_eq!(o.annotation_usage().retained_records, 1);
    assert!(matches!(
        o.prepare_compound(&[], &[], &[request], &mut r),
        Err(CoordinatorError::Limit)
    ));
    assert_eq!(o.annotations.retired[slot].as_ref().unwrap().fact, absent);
    assert_eq!(o.annotation_usage().issued_lifetime, 2);
}

#[test]
fn publication_exhaustion_refuses_before_payload_or_tombstone_changes() {
    let (mut o, _, absent, mut r) = ready();
    o.generation = u64::MAX;
    let usage = o.annotation_usage();
    let root = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(
        o.retire_absent_annotation(&absent, &facts(), &mut r),
        Err(CoordinatorError::VersionExhausted)
    );
    assert_eq!(o.generation, u64::MAX);
    assert_eq!(o.annotation_usage(), usage);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), root);
    assert!(o.annotations.retired[absent.annotation.slot].is_none());
    assert!(!o.is_faulted());
}

#[test]
fn inconsistent_lifetime_inventory_fences_instead_of_reclaiming_a_record() {
    let (mut o, _, absent, mut r) = ready();
    o.annotations.retired.pop();
    let generation = o.generation;
    let usage = o.annotation_usage();
    assert_eq!(
        o.retire_absent_annotation(&absent, &facts(), &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation, generation);
    assert_eq!(o.annotation_usage(), usage);
    assert_eq!(
        o.annotations.records[absent.annotation.slot]
            .as_ref()
            .unwrap()
            .value
            .body
            .as_ref(),
        "PRIVATE_OLD"
    );
}
