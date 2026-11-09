//! Native measurement contract and real engine invariance, not exporter evidence.
#[path = "measurement_behaviors/responsiveness.rs"]
mod responsiveness;
use vellum_engine::coordinator::{
    CoordinatorError, DocumentId, PublicationMeasurementControlV1, PublicationMeasurementLimitsV1,
};
use vellum_engine::coordinator::{CoordinatorLimits, DocumentEdits, SourceCoordinator};
use vellum_engine::coordinator::{
    InvocationLimits, InvocationMeasurementLimitsV1, InvocationMeasurementsV1, SourceIntent,
};
use vellum_engine::instrumentation::measurement::*;
use vellum_engine::instrumentation::{Boundary, Capture, Disposition, Observation, Recorder};
use vellum_engine::source::{Limits, Replacement};

fn invocation_owner(r: &mut Recorder) -> (SourceCoordinator, DocumentId) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 8192,
        },
    );
    o.configure_retention_budget(100_000, r).unwrap();
    let d = o.create(b"A", r).unwrap().document;
    (o, d)
}
fn enable_invocation(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_invocation_measurements_v1(
        InvocationMeasurementLimitsV1 {
            values: ValueLimitsV1 {
                max_integer: 100,
                max_seconds: 0.0,
            },
            max_pool_bytes: InvocationMeasurementLimitsV1::required_pool_bytes(16).unwrap(),
        },
        Some(true),
        r,
    )
    .unwrap();
}
fn source_intent(
    o: &SourceCoordinator,
    d: &DocumentId,
    from: &str,
    to: &str,
    r: &mut Recorder,
) -> SourceIntent {
    SourceIntent {
        writes: vec![DocumentEdits {
            expected: o.read(d, r).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: from.into(),
                inserted: to.into(),
            }],
        }],
        reads: vec![],
    }
}
fn invocation_totals(p: InvocationMeasurementsV1) -> [u64; 7] {
    let total = |p: MeasurementPointV1| match p.state() {
        PointStateV1::Observed {
            value: ValueV1::Counter(c),
            ..
        } => c.total,
        _ => panic!("observed counter"),
    };
    let mut v = [0; 7];
    for (i, p) in p.dispositions.into_iter().enumerate() {
        v[i] = total(p);
    }
    v[6] = total(p.admissions);
    v
}
#[test]
fn given_one_admitted_invocation_when_it_is_replayed_then_admission_and_completion_count_once_despite_event_loss()
 {
    let mut r = Recorder::try_with_capacity(1).unwrap();
    let (mut o, d) = invocation_owner(&mut r);
    enable_invocation(&mut o, &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        invocation_totals(o.invocation_measurements_v1(&mut r).unwrap().unwrap()),
        [0; 7]
    );
    let q = source_intent(&o, &d, "A", "a", &mut r);
    o.submit(&id, &q, &mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        invocation_totals(o.invocation_measurements_v1(&mut r).unwrap().unwrap()),
        [1, 0, 0, 0, 0, 0, 1]
    );
    assert!(r.dropped() > 0);
}
#[test]
fn given_late_collection_activation_when_old_and_new_admissions_complete_then_only_the_actual_new_cohort_is_counted()
 {
    let mut r = Recorder::disabled();
    let (mut o, d) = invocation_owner(&mut r);
    let old = o.issue_invocation(&mut r).unwrap();
    let q = source_intent(&o, &d, "A", "a", &mut r);
    o.submit(&old, &q, &mut r).unwrap();
    let issued = o.issue_invocation(&mut r).unwrap();
    enable_invocation(&mut o, &mut r);
    o.run(&old, &mut r).unwrap();
    assert_eq!(
        invocation_totals(o.invocation_measurements_v1(&mut r).unwrap().unwrap()),
        [0; 7]
    );
    let q = source_intent(&o, &d, "a", "b", &mut r);
    o.submit(&issued, &q, &mut r).unwrap();
    o.run(&issued, &mut r).unwrap();
    assert_eq!(
        invocation_totals(o.invocation_measurements_v1(&mut r).unwrap().unwrap()),
        [1, 0, 0, 0, 0, 0, 1]
    );
}
#[test]
fn given_retired_outcome_storage_when_the_invocation_is_already_spent_then_measurements_do_not_erase_execution_history()
 {
    let mut r = Recorder::disabled();
    let (mut o, d) = invocation_owner(&mut r);
    enable_invocation(&mut o, &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    let q = source_intent(&o, &d, "A", "a", &mut r);
    o.submit(&id, &q, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(
        invocation_totals(o.invocation_measurements_v1(&mut r).unwrap().unwrap()),
        [1, 0, 0, 0, 0, 0, 1]
    );
    assert!(o.submit(&id, &q, &mut r).is_err());
    assert_eq!(o.read(&d, &mut r).unwrap().source.text(), "a");
}

fn publication_owner(r: &mut Recorder) -> (SourceCoordinator, DocumentId) {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 8,
            max_edit_payload_bytes: 4096,
        },
    });
    o.configure_retention_budget(100_000, r).unwrap();
    o.configure_publication_measurements_v1(
        PublicationMeasurementLimitsV1 {
            values: ValueLimitsV1 {
                max_integer: 1000,
                max_seconds: 0.0,
            },
            max_pool_bytes: PublicationMeasurementLimitsV1::required_pool_bytes(),
        },
        Some(true),
        r,
    )
    .unwrap();
    let d = o.create(b"A", r).unwrap().document;
    (o, d)
}
fn prepared(
    o: &SourceCoordinator,
    d: &DocumentId,
    from: &str,
    to: &str,
    r: &mut Recorder,
) -> vellum_engine::coordinator::PreparedTransaction {
    o.prepare(
        &[DocumentEdits {
            expected: o.read(d, r).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: from.into(),
                inserted: to.into(),
            }],
        }],
        &[],
        r,
    )
    .unwrap()
}
fn publication_totals(points: [MeasurementPointV1; 6]) -> [u64; 6] {
    points.map(|p| match p.state() {
        PointStateV1::Observed {
            value: ValueV1::Counter(c),
            ..
        } => c.total,
        _ => panic!("observed publication count"),
    })
}
#[test]
fn given_actual_publications_when_events_are_disabled_or_full_then_useful_noop_and_refusal_counts_remain_distinct()
 {
    for mode in 0..3 {
        let mut r = match mode {
            0 => Recorder::disabled(),
            1 => Recorder::try_with_capacity(128).unwrap(),
            _ => Recorder::try_with_capacity(1).unwrap(),
        };
        let (mut o, d) = publication_owner(&mut r);
        let stale = prepared(&o, &d, "A", "x", &mut r);
        let change = prepared(&o, &d, "A", "a", &mut r);
        o.commit(change, &mut r).unwrap();
        let noop = prepared(&o, &d, "a", "a", &mut r);
        assert!(o.commit(noop, &mut r).unwrap().operation.is_none());
        assert_eq!(o.commit(stale, &mut r), Err(CoordinatorError::Stale));
        assert_eq!(
            publication_totals(o.publication_measurements_v1(&mut r).unwrap().unwrap()),
            [1, 1, 1, 0, 0, 0]
        );
        assert_eq!(o.read(&d, &mut r).unwrap().source.text(), "a");
        assert!(!o.is_faulted());
        if mode == 2 {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn given_publication_collection_stop_when_its_pool_retires_then_edits_continue_without_resetting_that_observer()
 {
    let mut r = Recorder::disabled();
    let (mut o, d) = publication_owner(&mut r);
    let reserved = o.retention_usage().reserved;
    assert!(o.stop_publication_measurements_v1(Some(true), &mut r));
    let p = prepared(&o, &d, "A", "a", &mut r);
    o.commit(p, &mut r).unwrap();
    assert!(
        o.publication_measurements_v1(&mut r)
            .unwrap()
            .unwrap()
            .iter()
            .all(|p| p.state() == PointStateV1::Off)
    );
    let control = PublicationMeasurementControlV1 {
        owner_control_allowed: Some(true),
        delivery_drained: Some(true),
        retention_released: Some(true),
    };
    assert!(o.retire_publication_measurements_v1(&control, &mut r));
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(reserved > 0);
    assert_eq!(
        o.publication_measurements_v1(&mut r),
        Err(MeasurementErrorV1::Unavailable)
    );
    let p = prepared(&o, &d, "a", "b", &mut r);
    o.commit(p, &mut r).unwrap();
    assert_eq!(o.read(&d, &mut r).unwrap().source.text(), "b");
    assert!(!o.is_faulted());
}
#[test]
fn given_two_actual_owners_sharing_a_recorder_when_publications_occur_then_series_are_not_merged() {
    let mut r = Recorder::try_with_capacity(1).unwrap();
    let (mut a, da) = publication_owner(&mut r);
    let (mut b, db) = publication_owner(&mut r);
    let before = a.publication_measurements_v1(&mut r).unwrap().unwrap();
    let pa = prepared(&a, &da, "A", "a", &mut r);
    let pb = prepared(&b, &db, "A", "b", &mut r);
    a.commit(pa, &mut r).unwrap();
    b.commit(pb, &mut r).unwrap();
    let after = a.publication_measurements_v1(&mut r).unwrap().unwrap();
    let other = b.publication_measurements_v1(&mut r).unwrap().unwrap();
    assert_eq!(counter_delta_v1(&before[0], &after[0], Some(true)), Some(1));
    assert_eq!(counter_delta_v1(&before[0], &other[0], Some(true)), None);
    assert_ne!(after[0].generation(), other[0].generation());
}

#[test]
fn native_descriptors_and_finite_dimension_labels_match_the_adopted_catalog() {
    let catalog = include_str!("../../../formal/engine-measurements.json");
    let rows = catalog
        .lines()
        .filter(|l| l.trim_start().starts_with("{\"id\":"))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), MeasureV1::ALL.len());
    for (m, line) in MeasureV1::ALL.into_iter().zip(rows) {
        let d = m.descriptor();
        let kind = match d.kind {
            ValueKindV1::Counter => "counter",
            ValueKindV1::Gauge => "gauge",
            ValueKindV1::Duration => "duration",
        };
        let dimensions = d
            .dimensions
            .iter()
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(",");
        let expected = format!(
            "{{\"id\":\"{}\",\"site\":\"{}\",\"type\":\"{}\",\"unit\":\"{}\",\"population\":\"{}\",\"boundary\":\"{}\",\"dimensions\":[{}]}}",
            d.id, d.site, kind, d.unit, d.population, d.boundary, dimensions
        );
        assert_eq!(line.trim().trim_end_matches(','), expected);
        assert!(!matches!(d.site, "INS-10" | "INS-11"));
    }
    let dictionaries = [
        ("admission", AdmissionV1::ALL.map(|v| v.label()).to_vec()),
        (
            "disposition",
            DispositionV1::ALL.map(|v| v.label()).to_vec(),
        ),
        ("artifact", ArtifactV1::ALL.map(|v| v.label()).to_vec()),
        ("evidence", EvidenceV1::ALL.map(|v| v.label()).to_vec()),
        ("derived", DerivedV1::ALL.map(|v| v.label()).to_vec()),
        ("diagnostic", DiagnosticV1::ALL.map(|v| v.label()).to_vec()),
        ("owner", OwnerV1::ALL.map(|v| v.label()).to_vec()),
        ("coordinate", CoordinateV1::ALL.map(|v| v.label()).to_vec()),
    ];
    for (name, values) in dictionaries {
        let prefix = format!("\"{name}\":");
        let line = catalog
            .lines()
            .find(|l| l.trim_start().starts_with(&prefix))
            .unwrap()
            .trim()
            .trim_end_matches(',');
        let values = values
            .iter()
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(line, format!("{prefix}[{values}]"));
    }
}
fn profile() -> DiagnosticMeasurementLimitsV1 {
    DiagnosticMeasurementLimitsV1 {
        values: ValueLimitsV1 {
            max_integer: 1000,
            max_seconds: 10.0,
        },
        max_state_bytes: DiagnosticMeasurementLimitsV1::required_state_bytes(),
    }
}
#[test]
fn given_real_source_edits_when_metrics_are_off_available_or_saturated_then_authoritative_results_match()
 {
    let mut results = Vec::new();
    for mode in 0..3 {
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_diagnostic_measurements(if mode == 1 { 128 } else { 1 }, profile())
                .unwrap()
        };
        let mut o = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 1024,
            },
        });
        let source = o.create(b"prose and code", &mut r).unwrap();
        let plan = o
            .prepare(
                &[DocumentEdits {
                    expected: source.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 5,
                        expected: "prose".into(),
                        inserted: "notes".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        let committed = o.commit(plan, &mut r).unwrap();
        let read = o.read(&source.document, &mut r).unwrap();
        results.push((
            read.source.text().to_owned(),
            read.version.revision,
            committed.generation,
            committed.operation.is_some(),
            o.is_faulted(),
        ));
        if mode == 0 {
            assert_eq!(r.diagnostic_measurements().unwrap(), None);
        } else {
            let points = r.diagnostic_measurements().unwrap().unwrap();
            let total = |i: usize| match points[i].state() {
                PointStateV1::Observed {
                    value: ValueV1::Counter(c),
                    ..
                } => c.total,
                _ => panic!("counter"),
            };
            assert!(total(0) > 1);
            assert_eq!(total(1), r.queued() as u64);
            assert_eq!(total(3), r.dropped());
            if mode == 2 {
                assert!(total(3) > 0);
            }
        }
    }
    assert!(results.windows(2).all(|v| v[0] == v[1]));
    assert_eq!(results[0].0, "notes and code");
}
#[test]
fn given_a_full_event_queue_when_it_is_drained_then_counters_distinguish_loss_and_delivery() {
    let mut r = Recorder::try_with_diagnostic_measurements(1, profile()).unwrap();
    let observation = Observation {
        boundary: Boundary::Query,
        disposition: Disposition::Completed,
        service_time: None,
    };
    assert_eq!(r.record(observation), Capture::Recorded);
    assert_eq!(r.record(observation), Capture::Dropped);
    assert_eq!(r.pop_front(), Some(observation));
    assert_eq!(r.pop_front(), None);
    let points = r.diagnostic_measurements().unwrap().unwrap();
    let totals = points.map(|p| match p.state() {
        PointStateV1::Observed {
            value: ValueV1::Counter(c),
            ..
        } => c.total,
        _ => panic!("counter"),
    });
    assert_eq!(totals, [2, 1, 1, 1]);
    assert_eq!(r.queued(), 0);
    assert_eq!(r.dropped(), 1);
}
