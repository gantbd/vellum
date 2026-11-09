use super::*;
#[test]
fn shutdown_inventory_reports_actual_publication_measurement_owner_until_native_retirement() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::PublicationMeasurements,
        &[],
    );
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(0)); // No issued managed entries; still an owner.
    assert!(row.reserved_bytes.unwrap() > 0);
    assert!(o.stop_publication_measurements_v1(Some(true), &mut r));
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::PublicationMeasurements,
        &[],
    );
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert!(row.admission_stopped);
    assert!(o.retire_publication_measurements_v1(&control(), &mut r));
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::PublicationMeasurements,
        &[],
    );
    assert_eq!(row.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(row.reserved_bytes, None);
}
fn profile(maximum: u64) -> PublicationMeasurementLimitsV1 {
    PublicationMeasurementLimitsV1 {
        values: m::ValueLimitsV1 {
            max_integer: maximum,
            max_seconds: 0.0,
        },
        max_pool_bytes: PublicationMeasurementLimitsV1::required_pool_bytes(),
    }
}
fn control() -> PublicationMeasurementControlV1 {
    PublicationMeasurementControlV1 {
        owner_control_allowed: Some(true),
        delivery_drained: Some(true),
        retention_released: Some(true),
    }
}
fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 8192,
        }),
        history: Some(HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        }),
        resources: None,
        search: None,
        annotations: Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 4096,
            max_mapping_steps: 65536,
            max_live_snapshots: 8,
            max_snapshot_bytes: 4096,
        }),
    });
    let mut r = Recorder::disabled();
    o.configure_retention_budget(200_000, &mut r).unwrap();
    (o, r)
}
fn enable(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_publication_measurements_v1(profile(100), Some(true), r)
        .unwrap();
}
fn edits(
    o: &SourceCoordinator,
    d: &DocumentId,
    from: &str,
    to: &str,
    r: &mut Recorder,
) -> DocumentEdits {
    DocumentEdits {
        expected: o.read(d, r).unwrap().version,
        edits: vec![Replacement {
            start: 0,
            end: from.len(),
            expected: from.into(),
            inserted: to.into(),
        }],
    }
}
fn change(
    o: &mut SourceCoordinator,
    d: &DocumentId,
    from: &str,
    to: &str,
    r: &mut Recorder,
) -> CommitOutcome {
    let edit = edits(o, d, from, to, r);
    let p = o.prepare(&[edit], &[], r).unwrap();
    o.commit(p, r).unwrap()
}
fn counts(o: &mut SourceCoordinator, r: &mut Recorder) -> [m::CounterV1; 6] {
    o.publication_measurements_v1(r)
        .unwrap()
        .unwrap()
        .map(|p| match p.state() {
            m::PointStateV1::Observed {
                value: m::ValueV1::Counter(v),
                ..
            } => v,
            _ => panic!("counter"),
        })
}
fn totals(o: &mut SourceCoordinator, r: &mut Recorder) -> [u64; 6] {
    counts(o, r).map(|v| v.total)
}
#[test]
fn compound_publication_counts_once_not_per_participant_and_noop_is_distinct() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let writes = [
        edits(&o, &a, "A", "a", &mut r),
        edits(&o, &b, "B", "b", &mut r),
    ];
    let p = o.prepare(&writes, &[], &mut r).unwrap();
    o.commit(p, &mut r).unwrap();
    change(&mut o, &a, "a", "a", &mut r);
    assert_eq!(totals(&mut o, &mut r), [1, 1, 0, 0, 0, 0]);
    assert!(!o.is_faulted());
}
#[test]
fn stale_readonly_and_foreign_preparations_count_known_refusal_not_fault() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let e = edits(&o, &a, "A", "x", &mut r);
    let stale = o.prepare(&[e], &[], &mut r).unwrap();
    change(&mut o, &a, "A", "a", &mut r);
    assert_eq!(o.commit(stale, &mut r), Err(CoordinatorError::Stale));
    let e = edits(&o, &a, "a", "x", &mut r);
    let p = o.prepare(&[e], &[], &mut r).unwrap();
    o.set_read_only(&a, true, &mut r).unwrap();
    assert!(o.commit(p, &mut r).is_err());
    let (other, mut rr) = owner();
    let foreign = other.prepare(&[], &[], &mut rr).unwrap();
    assert_eq!(
        o.commit(foreign, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(totals(&mut o, &mut r), [1, 0, 3, 0, 0, 0]);
    assert!(!o.is_faulted());
}
#[test]
fn native_invocation_replay_and_preparation_refusal_are_not_extra_publication_attempts() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let intent = SourceIntent {
        writes: vec![edits(&o, &a, "A", "a", &mut r)],
        reads: vec![],
    };
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &intent, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    o.submit(&id, &intent, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &intent, &mut r).unwrap();
    o.run(&id, &mut r).unwrap(); // stale during preparation, no final kernel entry
    assert_eq!(totals(&mut o, &mut r), [1, 0, 0, 0, 0, 0]);
}
#[test]
fn pending_stop_is_not_mislabeled_as_an_entered_publication() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let intent = SourceIntent {
        writes: vec![edits(&o, &a, "A", "a", &mut r)],
        reads: vec![],
    };
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &intent, &mut r).unwrap();
    o.stop_invocation(&id, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [0; 6]);
    assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "A");
}
#[test]
fn mixed_annotation_source_and_reversal_use_the_same_whole_transaction_boundary() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap();
    let write = edits(&o, &a.document, "A", "a", &mut r);
    let p = o
        .prepare_compound(
            &[write],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: a,
                    basis: TargetBasis::Before,
                    target: crate::attachment::AttachmentTarget::Document,
                },
                body: "comment".into(),
            }],
            &mut r,
        )
        .unwrap();
    let result = o.commit(p, &mut r).unwrap();
    let g = result.history_group.unwrap();
    let p = o
        .prepare_reversal(&g, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let p = o
        .prepare_reversal(&g, HistoryDirection::Redo, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [3, 0, 0, 0, 0, 0]);
}
#[test]
fn line_derived_publication_enters_only_once() {
    let (mut o, context, mut r) = views::line_test_fixture();
    o.configure_retention_budget(200_000, &mut r).unwrap();
    enable(&mut o, &mut r);
    let p = o
        .prepare_line_edit(
            &context,
            &LineEditV1::Indent {
                unit: " ".into(),
                tab_width: 4,
            },
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [1, 0, 0, 0, 0, 0]);
}
#[test]
fn unwind_counts_one_fault_preserves_fencing_and_does_not_claim_rollback() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let writes = [
        edits(&o, &a, "A", "a", &mut r),
        edits(&o, &b, "B", "b", &mut r),
    ];
    let p = o.prepare(&writes, &[], &mut r).unwrap();
    o.publication_fault_after = Some(1);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.commit(p, &mut r))).is_err()
    );
    assert!(o.is_faulted());
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 1]);
    assert_eq!(o.documents[a.slot].as_ref().unwrap().source.text(), "a");
    assert_eq!(o.documents[b.slot].as_ref().unwrap().source.text(), "B");
    assert!(o.stop_publication_measurements_v1(Some(true), &mut r));
    assert!(o.retire_publication_measurements_v1(&control(), &mut r));
    assert!(o.is_faulted());
}
#[test]
fn returned_integrity_fault_counts_once_and_measurement_does_not_heal_it() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let e = edits(&o, &a, "A", "a", &mut r);
    let mut p = o.prepare(&[e], &[], &mut r).unwrap();
    p.candidates[0].edits[0].inserted = "wrong".into();
    assert_eq!(o.commit(p, &mut r), Err(CoordinatorError::Faulted));
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 1]);
    assert!(o.is_faulted());
}
#[test]
fn pool_profile_parent_bounds_and_fixed_root_accounting_are_independent() {
    let (mut o, mut r) = owner();
    let before = o.semantic_root_usage(&mut r).unwrap();
    let bytes = PublicationMeasurementLimitsV1::required_pool_bytes();
    assert_eq!(
        o.configure_publication_measurements_v1(
            PublicationMeasurementLimitsV1 {
                max_pool_bytes: bytes - 1,
                ..profile(100)
            },
            Some(true),
            &mut r
        ),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    let mut other = o.retention.reserve(200_000 - bytes + 1).unwrap();
    assert_eq!(
        o.configure_publication_measurements_v1(profile(100), Some(true), &mut r),
        Err(Error::Limit)
    );
    o.retention.release(&mut other).unwrap();
    enable(&mut o, &mut r);
    assert_eq!(o.retention_usage().reserved, bytes);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    assert!(!o.is_faulted());
}
#[test]
fn configuration_is_nonrenewing_and_stopped_collection_cannot_restart() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let before = o.publication_measurements_v1(&mut r).unwrap().unwrap();
    assert_eq!(
        o.configure_publication_measurements_v1(profile(100), Some(true), &mut r),
        Ok(false)
    );
    assert_eq!(
        o.configure_publication_measurements_v1(profile(101), Some(true), &mut r),
        Err(Error::Unavailable)
    );
    let after = o.publication_measurements_v1(&mut r).unwrap().unwrap();
    assert_eq!(before[0].generation(), after[0].generation());
    assert_eq!(after[0].sequence(), before[0].sequence() + 1);
    assert!(!o.stop_publication_measurements_v1(None, &mut r));
    assert!(o.stop_publication_measurements_v1(Some(true), &mut r));
    assert_eq!(
        o.configure_publication_measurements_v1(profile(100), Some(true), &mut r),
        Ok(false)
    );
    let p = o.prepare(&[], &[], &mut r).unwrap();
    o.commit(p, &mut r).unwrap();
    assert!(
        o.publication_measurements_v1(&mut r)
            .unwrap()
            .unwrap()
            .iter()
            .all(|p| p.state() == m::PointStateV1::Off)
    );
    assert!(!o.is_faulted());
}
#[test]
fn protected_retirement_waits_for_actual_probe_and_never_refunds_twice_or_rearms() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let bytes = o.retention_usage().reserved;
    let probe = o.publication_measurements.probe();
    assert!(o.stop_publication_measurements_v1(Some(true), &mut r));
    assert!(!o.retire_publication_measurements_v1(&control(), &mut r));
    assert_eq!(o.retention_usage().reserved, bytes);
    drop(probe);
    let mut c = control();
    c.delivery_drained = None;
    assert!(!o.retire_publication_measurements_v1(&c, &mut r));
    assert_eq!(o.retention_usage().reserved, bytes);
    c.delivery_drained = Some(true);
    assert!(o.retire_publication_measurements_v1(&c, &mut r));
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(!o.retire_publication_measurements_v1(&c, &mut r));
    assert_eq!(
        o.configure_publication_measurements_v1(profile(100), Some(true), &mut r),
        Err(Error::Unavailable)
    );
}
#[test]
fn counter_saturation_and_snapshot_exhaustion_do_not_reject_real_edits() {
    let (mut o, mut r) = owner();
    o.configure_publication_measurements_v1(profile(1), Some(true), &mut r)
        .unwrap();
    let a = o.create(b"A", &mut r).unwrap().document;
    let first = o.publication_measurements_v1(&mut r).unwrap().unwrap();
    change(&mut o, &a, "A", "a", &mut r);
    change(&mut o, &a, "a", "b", &mut r);
    let second = o.publication_measurements_v1(&mut r).unwrap().unwrap();
    assert!(matches!(
        second[0].state(),
        m::PointStateV1::Observed {
            coverage: m::CoverageV1::Inexact,
            value: m::ValueV1::Counter(m::CounterV1 {
                total: 1,
                exact: false
            }),
            ..
        }
    ));
    assert_eq!(m::counter_delta_v1(&first[0], &second[0], Some(true)), None);
    assert_eq!(o.publication_measurements_v1(&mut r), Err(Error::Exhausted));
    change(&mut o, &a, "b", "c", &mut r);
    assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "c");
    assert!(!o.is_faulted());
}
#[test]
fn snapshots_and_ordinary_metric_refusals_do_not_publish_or_grant_engine_authority() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<SourceCoordinator>();
    let (mut o, mut r) = owner();
    assert_eq!(o.publication_measurements_v1(&mut r), Ok(None));
    assert_eq!(
        o.configure_publication_measurements_v1(profile(100), None, &mut r),
        Err(Error::Unavailable)
    );
    enable(&mut o, &mut r);
    let generation = o.generation();
    let before = o.history_usage();
    let points = o.publication_measurements_v1(&mut r).unwrap().unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(o.history_usage(), before);
    assert!(
        points
            .iter()
            .all(|p| p.measure() == m::MeasureV1::PublicationDispositions)
    );
}
