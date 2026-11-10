use super::*;
fn limits() -> ValueLimitsV1 {
    ValueLimitsV1 {
        max_integer: 10,
        max_seconds: 20.0,
    }
}
fn state(value: ValueV1) -> PointStateV1 {
    PointStateV1::Observed {
        freshness: FreshnessV1::Current,
        coverage: CoverageV1::Complete,
        value,
    }
}
fn counter(total: u64) -> PointStateV1 {
    state(ValueV1::Counter(CounterV1 { total, exact: true }))
}
fn dimensions(m: MeasureV1) -> DimensionsV1 {
    match m.descriptor().dimensions {
        [] => DimensionsV1::None,
        ["admission"] => DimensionsV1::Admission(AdmissionV1::FirstAdmitted),
        ["disposition"] => DimensionsV1::Disposition(DispositionV1::Success),
        ["artifact"] => DimensionsV1::Artifact(ArtifactV1::Source),
        ["evidence"] => DimensionsV1::Evidence(EvidenceV1::Progress),
        ["derived"] => DimensionsV1::Derived(DerivedV1::Accepted),
        ["diagnostic"] => DimensionsV1::Diagnostic(DiagnosticV1::Offered),
        ["owner"] => DimensionsV1::Owner(OwnerV1::Invocation),
        ["artifact", "disposition"] => {
            DimensionsV1::ArtifactDisposition(ArtifactV1::Source, DispositionV1::Success)
        }
        ["owner", "coordinate"] => DimensionsV1::OwnerCoordinate(
            OwnerV1::Invocation,
            if m == MeasureV1::ResourceConsumed {
                CoordinateV1::WorkSteps
            } else {
                CoordinateV1::Slots
            },
        ),
        _ => panic!("unsupported catalog shape"),
    }
}
fn zero(m: MeasureV1) -> PointStateV1 {
    state(match m.descriptor().kind {
        ValueKindV1::Counter => ValueV1::Counter(CounterV1 {
            total: 0,
            exact: true,
        }),
        ValueKindV1::Gauge => ValueV1::Gauge { amount: 0 },
        ValueKindV1::Duration => ValueV1::Duration {
            seconds: 0.0,
            endpoint: EndpointV1::Completed,
        },
    })
}
#[test]
fn all_catalog_values_keep_observed_zero_distinct_from_absence_and_refuse_type_confusion() {
    for m in MeasureV1::ALL {
        let mut series = MeasurementSeriesV1::new(m, dimensions(m), limits()).unwrap();
        let p = series.observe(zero(m)).unwrap();
        assert_eq!(p.sequence(), 0);
        assert_eq!(p.state(), zero(m));
        for status in [
            PointStateV1::Off,
            PointStateV1::Unavailable,
            PointStateV1::Unknown,
        ] {
            assert_eq!(series.observe(status).unwrap().state(), status);
        }
        let bad = match m.descriptor().kind {
            ValueKindV1::Counter => ValueV1::Gauge { amount: 0 },
            _ => ValueV1::Counter(CounterV1 {
                total: 0,
                exact: true,
            }),
        };
        assert_eq!(
            series.observe(state(bad)),
            Err(MeasurementErrorV1::InvalidValue)
        );
        assert_eq!(series.observe(zero(m)).unwrap().sequence(), 4);
    }
}
#[test]
fn invalid_dimensions_and_resource_coordinates_refuse_without_silent_unit_conversion() {
    for m in MeasureV1::ALL {
        if !m.descriptor().dimensions.is_empty() {
            assert!(matches!(
                MeasurementSeriesV1::new(m, DimensionsV1::None, limits()),
                Err(MeasurementErrorV1::InvalidDimensions)
            ));
        }
    }
    for m in [
        MeasureV1::ResourceOccupancy,
        MeasureV1::ResourceCommitments,
        MeasureV1::ResourceConsumed,
        MeasureV1::ResourceCapacity,
    ] {
        for c in CoordinateV1::ALL {
            let consumed = matches!(c, CoordinateV1::WorkSteps | CoordinateV1::ChildGrants);
            let valid = m == MeasureV1::ResourceCapacity
                || (if m == MeasureV1::ResourceConsumed {
                    consumed
                } else {
                    !consumed
                });
            assert_eq!(
                m.accepts(DimensionsV1::OwnerCoordinate(OwnerV1::History, c)),
                valid
            );
        }
    }
    assert!(
        !MeasureV1::ArtifactDispositions.accepts(DimensionsV1::Disposition(DispositionV1::Success))
    );
}
#[test]
fn bounds_and_nonfinite_durations_are_checked_before_observation_issuance() {
    for p in [
        ValueLimitsV1 {
            max_integer: MAX_INTEGER_V1 + 1,
            ..limits()
        },
        ValueLimitsV1 {
            max_seconds: -1.0,
            ..limits()
        },
        ValueLimitsV1 {
            max_seconds: f64::NAN,
            ..limits()
        },
        ValueLimitsV1 {
            max_seconds: f64::INFINITY,
            ..limits()
        },
    ] {
        assert_eq!(p.validate(), Err(MeasurementErrorV1::InvalidProfile));
    }
    let mut s =
        MeasurementSeriesV1::new(MeasureV1::AdmissionDuration, DimensionsV1::None, limits())
            .unwrap();
    for seconds in [-1.0, f64::NAN, f64::INFINITY, 20.1] {
        assert_eq!(
            s.observe(state(ValueV1::Duration {
                seconds,
                endpoint: EndpointV1::Completed
            })),
            Err(MeasurementErrorV1::InvalidValue)
        );
    }
    for endpoint in [
        EndpointV1::Completed,
        EndpointV1::Pending,
        EndpointV1::Censored,
    ] {
        assert!(
            s.observe(state(ValueV1::Duration {
                seconds: 20.0,
                endpoint
            }))
            .is_ok()
        );
    }
    let mut s = MeasurementSeriesV1::new(
        MeasureV1::ArtifactSize,
        DimensionsV1::Artifact(ArtifactV1::Annotation),
        limits(),
    )
    .unwrap();
    assert_eq!(
        s.observe(state(ValueV1::Gauge { amount: 11 })),
        Err(MeasurementErrorV1::InvalidValue)
    );
    assert_eq!(
        s.observe(state(ValueV1::Gauge { amount: 10 }))
            .unwrap()
            .sequence(),
        0
    );
}
#[test]
fn sampled_or_inexact_coverage_cannot_claim_exact_cumulative_totals() {
    let mut s =
        MeasurementSeriesV1::new(MeasureV1::RequestOffers, DimensionsV1::None, limits()).unwrap();
    for coverage in [
        CoverageV1::Sampled,
        CoverageV1::Inexact,
        CoverageV1::Unknown,
    ] {
        assert_eq!(
            s.observe(PointStateV1::Observed {
                freshness: FreshnessV1::Current,
                coverage,
                value: ValueV1::Counter(CounterV1 {
                    total: 0,
                    exact: true
                })
            }),
            Err(MeasurementErrorV1::InvalidValue)
        );
        assert!(
            s.observe(PointStateV1::Observed {
                freshness: FreshnessV1::Current,
                coverage,
                value: ValueV1::Counter(CounterV1 {
                    total: 0,
                    exact: false
                })
            })
            .is_ok()
        );
    }
}
#[test]
fn counter_saturation_is_sticky_and_never_wraps_or_confiscates_existing_totals() {
    assert_eq!(
        counter_step_v1(
            CounterV1 {
                total: 0,
                exact: true
            },
            u64::MAX,
            MAX_INTEGER_V1
        ),
        Err(MeasurementErrorV1::InvalidValue)
    );
    for total in 0..=10 {
        for amount in 0..=12 {
            for exact in [true, false] {
                let actual = counter_step_v1(CounterV1 { total, exact }, amount, 10).unwrap();
                assert_eq!(
                    actual,
                    CounterV1 {
                        total: (total + amount).min(10),
                        exact: exact && total + amount <= 10
                    }
                );
            }
        }
    }
    assert_eq!(
        counter_step_v1(
            CounterV1 {
                total: MAX_INTEGER_V1,
                exact: true
            },
            MAX_INTEGER_V1,
            MAX_INTEGER_V1
        )
        .unwrap(),
        CounterV1 {
            total: MAX_INTEGER_V1,
            exact: false
        }
    );
    assert_eq!(
        counter_step_v1(
            CounterV1 {
                total: 2,
                exact: true
            },
            0,
            1
        ),
        Err(MeasurementErrorV1::InvalidValue)
    );
}
#[test]
fn observation_ordinals_exhaust_without_wrapping_or_issuing_a_replacement_generation() {
    let mut s = MeasurementSeriesV1::new(
        MeasureV1::RequestOffers,
        DimensionsV1::None,
        ValueLimitsV1 {
            max_integer: 0,
            max_seconds: 0.0,
        },
    )
    .unwrap();
    let p = s.observe(counter(0)).unwrap();
    assert_eq!(p.sequence(), 0);
    assert_eq!(s.observe(counter(0)), Err(MeasurementErrorV1::Exhausted));
    assert_eq!(p.generation(), &s.generation);
    let other =
        MeasurementSeriesV1::new(MeasureV1::RequestOffers, DimensionsV1::None, limits()).unwrap();
    assert_ne!(p.generation(), &other.generation);
}
#[test]
fn counter_delta_requires_exact_current_same_series_and_whole_interval_continuity() {
    let mut s =
        MeasurementSeriesV1::new(MeasureV1::RequestOffers, DimensionsV1::None, limits()).unwrap();
    let a = s.observe(counter(2)).unwrap();
    let b = s.observe(counter(5)).unwrap();
    assert_eq!(counter_delta_v1(&a, &b, Some(true)), Some(3));
    assert_eq!(counter_delta_v1(&a, &b, None), None);
    assert_eq!(counter_delta_v1(&a, &b, Some(false)), None);
    assert_eq!(counter_delta_v1(&b, &a, Some(true)), None);
    for mode in 0..8 {
        let mut bad = b.clone();
        match mode {
            0 => bad.measure = MeasureV1::InvocationAdmissions,
            1 => bad.dimensions = DimensionsV1::Diagnostic(DiagnosticV1::Captured),
            2 => bad.generation = MeasurementGenerationV1(Arc::new(())),
            3 => bad.sequence = a.sequence,
            4 => {
                bad.state = PointStateV1::Observed {
                    freshness: FreshnessV1::Stale,
                    coverage: CoverageV1::Complete,
                    value: ValueV1::Counter(CounterV1 {
                        total: 5,
                        exact: true,
                    }),
                }
            }
            5 => {
                bad.state = PointStateV1::Observed {
                    freshness: FreshnessV1::Current,
                    coverage: CoverageV1::Sampled,
                    value: ValueV1::Counter(CounterV1 {
                        total: 5,
                        exact: false,
                    }),
                }
            }
            6 => bad.state = counter(1),
            _ => bad.state = PointStateV1::Unknown,
        }
        assert_eq!(counter_delta_v1(&a, &bad, Some(true)), None);
    }
}
#[test]
fn clock_conversion_requires_valid_ordered_same_domain_epoch_and_known_scale() {
    let mut c = MeasurementClockV1::new(0.5, 10).unwrap();
    let a = c.stamp(2).unwrap();
    let b = c.stamp(6).unwrap();
    assert_eq!(c.seconds(&a, &b, Some(true), 2.0), Some(2.0));
    assert_eq!(c.seconds(&a, &b, None, 20.0), None);
    assert_eq!(c.seconds(&b, &a, Some(true), 20.0), None);
    assert_eq!(c.seconds(&a, &b, Some(true), 1.9), None);
    assert!(c.stamp(11).is_none());
    let foreign = MeasurementClockV1::new(0.5, 10).unwrap();
    assert_eq!(
        c.seconds(&a, &foreign.stamp(6).unwrap(), Some(true), 20.0),
        None
    );
    c.new_epoch();
    assert_eq!(c.seconds(&a, &b, Some(true), 20.0), None);
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(MeasurementClockV1::new(scale, 10).is_err());
    }
    let c = MeasurementClockV1::new(f64::MAX, 10).unwrap();
    assert_eq!(
        c.seconds(
            &c.stamp(0).unwrap(),
            &c.stamp(10).unwrap(),
            Some(true),
            f64::MAX
        ),
        None
    );
}
#[test]
fn first_result_classification_does_not_finish_on_progress_quiescence_or_duplicate() {
    for kind in EvidenceV1::ALL {
        for attributed in [None, Some(false), Some(true)] {
            for seen in [false, true] {
                let wanted = attributed == Some(true)
                    && !seen
                    && matches!(
                        kind,
                        EvidenceV1::Acknowledgment
                            | EvidenceV1::Failure
                            | EvidenceV1::Unknown
                            | EvidenceV1::Contradiction
                    );
                assert_eq!(first_result_v1(kind, attributed, seen), wanted);
            }
        }
    }
}

fn diagnostic_limits(maximum: u64) -> DiagnosticMeasurementLimitsV1 {
    DiagnosticMeasurementLimitsV1 {
        values: ValueLimitsV1 {
            max_integer: maximum,
            max_seconds: 0.0,
        },
        max_state_bytes: DiagnosticMeasurementLimitsV1::required_state_bytes(),
    }
}
fn event() -> crate::instrumentation::Observation {
    crate::instrumentation::Observation {
        boundary: crate::instrumentation::Boundary::Admission,
        disposition: crate::instrumentation::Disposition::Completed,
        service_time: None,
    }
}
fn totals(points: &[MeasurementPointV1; 4]) -> [CounterV1; 4] {
    points.each_ref().map(|p| match p.state() {
        PointStateV1::Observed {
            value: ValueV1::Counter(v),
            ..
        } => v,
        _ => panic!("observed stage counter"),
    })
}
#[test]
fn actual_queue_stage_counters_follow_offers_capture_drain_and_drop_not_engine_traffic() {
    use crate::instrumentation::{Capture, Recorder};
    let mut r = Recorder::try_with_diagnostic_measurements(1, diagnostic_limits(10)).unwrap();
    let storage = r.capacity();
    assert_eq!(r.record(event()), Capture::Recorded);
    assert_eq!(r.record(event()), Capture::Dropped);
    assert_eq!(r.record(event()), Capture::Dropped);
    assert!(r.pop_front().is_some());
    assert!(r.pop_front().is_none());
    assert_eq!(r.capacity(), storage);
    let points = r.diagnostic_measurements().unwrap().unwrap();
    assert_eq!(
        totals(&points),
        [
            CounterV1 {
                total: 3,
                exact: true
            },
            CounterV1 {
                total: 1,
                exact: true
            },
            CounterV1 {
                total: 1,
                exact: true
            },
            CounterV1 {
                total: 2,
                exact: true
            }
        ]
    );
    assert_eq!(r.dropped(), 2);
    assert_eq!(r.queued(), 0);
    for (p, d) in points.iter().zip([
        DiagnosticV1::Offered,
        DiagnosticV1::Captured,
        DiagnosticV1::Drained,
        DiagnosticV1::Dropped,
    ]) {
        assert_eq!(p.measure(), MeasureV1::DiagnosticRecords);
        assert_eq!(p.dimensions(), DimensionsV1::Diagnostic(d));
        assert_eq!(p.profile(), PROFILE_V1);
    }
}
#[test]
fn metrics_are_separately_opt_in_and_do_not_reclassify_disabled_capture_as_dropped() {
    use crate::instrumentation::{Capture, Recorder};
    assert_eq!(
        Recorder::disabled().diagnostic_measurements().unwrap(),
        None
    );
    let mut r = Recorder::try_with_diagnostic_measurements(0, diagnostic_limits(10)).unwrap();
    assert_eq!(r.record(event()), Capture::Disabled);
    assert_eq!(r.dropped(), 0);
    let p = r.diagnostic_measurements().unwrap().unwrap();
    assert_eq!(totals(&p).map(|v| v.total), [1, 0, 0, 0]);
}
#[test]
fn native_state_reservation_and_failed_activation_are_explicit() {
    use crate::instrumentation::Recorder;
    let exact = diagnostic_limits(10);
    assert!(Recorder::try_with_diagnostic_measurements(1, exact).is_ok());
    assert!(matches!(
        Recorder::try_with_diagnostic_measurements(
            1,
            DiagnosticMeasurementLimitsV1 {
                max_state_bytes: exact.max_state_bytes - 1,
                ..exact
            }
        ),
        Err(MeasurementErrorV1::Limit)
    ));
    assert!(matches!(
        Recorder::try_with_diagnostic_measurements(usize::MAX, exact),
        Err(MeasurementErrorV1::Capacity)
    ));
    assert!(matches!(
        Recorder::try_with_diagnostic_measurements(
            1,
            DiagnosticMeasurementLimitsV1 {
                values: ValueLimitsV1 {
                    max_integer: MAX_INTEGER_V1 + 1,
                    ..exact.values
                },
                ..exact
            }
        ),
        Err(MeasurementErrorV1::InvalidProfile)
    ));
}
#[test]
fn saturated_diagnostic_totals_are_inexact_and_snapshot_exhaustion_never_resets_them() {
    use crate::instrumentation::Recorder;
    let mut r = Recorder::try_with_diagnostic_measurements(1, diagnostic_limits(1)).unwrap();
    let before = r.diagnostic_measurements().unwrap().unwrap();
    for _ in 0..3 {
        r.record(event());
    }
    let after = r.diagnostic_measurements().unwrap().unwrap();
    assert_eq!(
        totals(&after)[0],
        CounterV1 {
            total: 1,
            exact: false
        }
    );
    assert!(matches!(
        after[0].state(),
        PointStateV1::Observed {
            coverage: CoverageV1::Inexact,
            ..
        }
    ));
    assert_eq!(counter_delta_v1(&before[0], &after[0], Some(true)), None);
    assert_eq!(
        r.diagnostic_measurements(),
        Err(MeasurementErrorV1::Exhausted)
    );
    r.record(event());
    assert_eq!(
        r.diagnostic_measurements(),
        Err(MeasurementErrorV1::Exhausted)
    );
    assert_eq!(r.dropped(), 3);
}
#[test]
fn whole_snapshot_preflight_prevents_partial_ordinal_consumption() {
    let mut d = DiagnosticMeasurementsV1::new(diagnostic_limits(10)).unwrap();
    d.series[3].next = 11;
    assert_eq!(d.snapshot(), Err(MeasurementErrorV1::Exhausted));
    assert_eq!(d.series[0].next, 0);
    assert_eq!(d.series[1].next, 0);
    assert_eq!(d.series[2].next, 0);
    d.series[3].next = 0;
    d.counts[3].total = 11;
    assert_eq!(d.snapshot(), Err(MeasurementErrorV1::InvalidValue));
    assert_eq!(d.series[0].next, 0);
}
#[test]
fn distinct_actual_recorders_cannot_be_merged_by_identical_dimensions_or_counter_values() {
    use crate::instrumentation::Recorder;
    let mut a = Recorder::try_with_diagnostic_measurements(1, diagnostic_limits(10)).unwrap();
    let mut b = Recorder::try_with_diagnostic_measurements(1, diagnostic_limits(10)).unwrap();
    let p = a.diagnostic_measurements().unwrap().unwrap();
    b.record(event());
    let q = b.diagnostic_measurements().unwrap().unwrap();
    assert_eq!(p[0].measure(), q[0].measure());
    assert_eq!(p[0].dimensions(), q[0].dimensions());
    assert_ne!(p[0].generation(), q[0].generation());
    assert_eq!(counter_delta_v1(&p[0], &q[0], Some(true)), None);
}
