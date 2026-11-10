use super::*;
#[test]
fn shutdown_inventory_reports_actual_release_measurement_funding_not_empty_closure() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let mut f = owner();
    enable(&mut f.o, &mut f.r);
    let row = inventory_owner(
        &f.o,
        &mut f.r,
        ShutdownOwnerRoleV1::ReleaseMeasurements,
        &[],
    );
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(0));
    assert_eq!(
        row.reserved_bytes,
        Some(ReleaseMeasurementLimitsV1::required_pool_bytes())
    );
}
use crate::capture::CaptureLimitsV1;
use crate::coordinator::source_context::facade::Fault;
use crate::coordinator::source_context::search::result_contract::tests::{
    owner as native_owner, qfacts, request, with_facts,
};
use crate::coordinator::source_query::result_contract::tests::{Fixture, source_facts};
fn profile(maximum: u64) -> ReleaseMeasurementLimitsV1 {
    ReleaseMeasurementLimitsV1 {
        values: m::ValueLimitsV1 {
            max_integer: maximum,
            max_seconds: 0.0,
        },
        max_pool_bytes: ReleaseMeasurementLimitsV1::required_pool_bytes(),
    }
}
fn control() -> ReleaseMeasurementControlV1 {
    ReleaseMeasurementControlV1 {
        owner_control_allowed: Some(true),
        delivery_drained: Some(true),
        retention_released: Some(true),
    }
}
pub(in crate::coordinator) fn enable(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_release_measurements_v1(profile(1000), Some(true), r)
        .unwrap();
}
pub(in crate::coordinator) fn totals(o: &mut SourceCoordinator, r: &mut Recorder) -> [u64; 6] {
    o.release_measurements_v1(r)
        .unwrap()
        .unwrap()
        .map(|p| match p.state() {
            m::PointStateV1::Observed {
                value: m::ValueV1::Counter(v),
                ..
            } => v.total,
            _ => panic!("counter"),
        })
}
fn owner() -> Fixture {
    let mut f = native_owner();
    f.o.set_settings_parent_limit_for_test(16384);
    f
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 512,
        max_depth: 16,
    }
}
fn query(
    f: &mut Fixture,
    kind: usize,
    line: usize,
    compatibility: bool,
) -> Result<(), SourceReadContextErrorV1> {
    with_facts(f, kind, |o, source, scope, facts, r| {
        let req = SourceFacadeRequestV1 {
            source,
            scope,
            facts,
            actor: &scope.context,
            query: SourceContextQueryV1::Line { index: line },
            limits: limits(),
        };
        if compatibility {
            o.query_source_facade_v1(req, r)
                .map(|_| ())
                .ok_or(SourceReadContextErrorV1::Unavailable)
        } else {
            o.try_query_source_facade_v1(req, r).map(|_| ())
        }
    })
}
#[test]
fn release_counts_start_prospectively_and_count_whole_handoffs_not_diagnostics_or_native_helpers() {
    let mut f = owner();
    query(&mut f, 0, 0, false).unwrap();
    assert_eq!(f.o.release_measurements_v1(&mut f.r), Ok(None));
    enable(&mut f.o, &mut f.r);
    assert_eq!(totals(&mut f.o, &mut f.r), [0; 6]);
    for kind in 0..3 {
        query(&mut f, kind, 0, false).unwrap();
    }
    query(&mut f, 0, 0, true).unwrap();
    assert_eq!(
        query(&mut f, 0, 999, false),
        Err(SourceReadContextErrorV1::InvalidRequest)
    );
    f.o.query_source_read_context(
        &f.source,
        &f.s,
        SourceContextQueryV1::Line { index: 0 },
        &source_facts(&f.s),
        &mut f.r,
    )
    .unwrap();
    assert_eq!(totals(&mut f.o, &mut f.r), [4, 0, 1, 0, 0, 0]);
    let first = f.o.release_measurements_v1(&mut f.r).unwrap().unwrap();
    let next = f.o.release_measurements_v1(&mut f.r).unwrap().unwrap();
    for (i, (a, b)) in first.iter().zip(&next).enumerate() {
        assert_eq!(a.measure(), m::MeasureV1::ReleaseDispositions);
        assert_eq!(
            a.dimensions(),
            m::DimensionsV1::Disposition(m::DispositionV1::ALL[i])
        );
        assert_eq!(a.generation(), b.generation());
        assert_eq!(b.sequence(), a.sequence() + 1);
        assert_eq!(a.state(), b.state());
    }
    assert_eq!(totals(&mut f.o, &mut f.r), [4, 0, 1, 0, 0, 0]);
}
#[test]
fn release_counts_cover_all_four_search_ports_once_for_each_original_issuer() {
    let mut f = owner();
    enable(&mut f.o, &mut f.r);
    let q = request();
    for kind in 0..3 {
        with_facts(&mut f, kind, |o, source, scope, facts, r| {
            let req = SearchFacadeRequestV1 {
                source,
                scope,
                facts,
                actor: &scope.context,
                query: &q,
                current: &qfacts(&q),
                port: SearchFacadePortV1::Evaluate,
                limits: limits(),
            };
            let Ok(SearchFacadeReplyV1::Evaluated(id)) = o.try_search_facade_v1(req, r) else {
                panic!("evaluate");
            };
            for port in [
                SearchFacadePortV1::Release { result: &id },
                SearchFacadePortV1::Occurrence {
                    result: &id,
                    index: 0,
                },
                SearchFacadePortV1::Navigate {
                    result: &id,
                    navigation: SourceSearchNavigationV1 {
                        direction: SearchDirection::Next,
                        start: 0,
                        current: None,
                        wrap: false,
                    },
                },
            ] {
                assert!(
                    o.search_facade_v1(SearchFacadeRequestV1 { port, ..req }, r)
                        .is_some()
                );
            }
        });
    }
    assert_eq!(totals(&mut f.o, &mut f.r), [12, 0, 0, 0, 0, 0]);
}
#[test]
fn release_faults_are_counted_after_cleanup_not_as_successful_delivery_and_remain_readable_under_the_fence()
 {
    for kind in 0..3 {
        for fault in [
            Fault::Classification,
            Fault::Workspace,
            Fault::BeforeNative,
            Fault::AfterNative,
            Fault::BadResult,
            Fault::RefundDrift,
        ] {
            let mut f = owner();
            enable(&mut f.o, &mut f.r);
            let base = f.o.retention_usage().reserved;
            f.o.source_facade_fault = Some(fault);
            assert_eq!(
                query(&mut f, kind, 0, false),
                Err(SourceReadContextErrorV1::Faulted)
            );
            assert_eq!(f.o.retention_usage().reserved, base);
            assert_eq!(totals(&mut f.o, &mut f.r), [0, 0, 0, 0, 0, 1]);
            assert_eq!(
                query(&mut f, kind, 0, true),
                Err(SourceReadContextErrorV1::Unavailable)
            );
            assert_eq!(totals(&mut f.o, &mut f.r), [0, 0, 0, 0, 0, 2]);
            assert!(f.o.is_faulted());
            assert!(f.o.stop_release_measurements_v1(Some(true), &mut f.r));
            assert!(f.o.retire_release_measurements_v1(&control(), &mut f.r));
            assert!(f.o.is_faulted());
        }
    }
}
#[test]
fn release_counter_saturation_and_snapshot_exhaustion_never_refuse_actual_reads() {
    let mut f = owner();
    f.o.configure_release_measurements_v1(profile(1), Some(true), &mut f.r)
        .unwrap();
    let first = f.o.release_measurements_v1(&mut f.r).unwrap().unwrap();
    for _ in 0..3 {
        query(&mut f, 0, 0, false).unwrap();
    }
    let next = f.o.release_measurements_v1(&mut f.r).unwrap().unwrap();
    assert!(matches!(
        next[0].state(),
        m::PointStateV1::Observed {
            coverage: m::CoverageV1::Inexact,
            value: m::ValueV1::Counter(m::CounterV1 {
                total: 1,
                exact: false
            }),
            ..
        }
    ));
    assert_eq!(m::counter_delta_v1(&first[0], &next[0], Some(true)), None);
    assert_eq!(f.o.release_measurements_v1(&mut f.r), Err(Error::Exhausted));
    query(&mut f, 0, 0, false).unwrap();
    assert!(!f.o.is_faulted());
}
#[test]
fn release_pool_requires_its_own_exact_profile_and_actual_parent_reservation() {
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    let root = f.o.semantic_root_usage(&mut f.r).unwrap();
    let bytes = ReleaseMeasurementLimitsV1::required_pool_bytes();
    assert_eq!(
        f.o.configure_release_measurements_v1(
            ReleaseMeasurementLimitsV1 {
                max_pool_bytes: bytes - 1,
                ..profile(1000)
            },
            Some(true),
            &mut f.r
        ),
        Err(Error::Limit)
    );
    let pressure = f.o.retention_usage().maximum.unwrap() - base - bytes + 1;
    let mut other = f.o.retention.reserve(pressure).unwrap();
    assert_eq!(
        f.o.configure_release_measurements_v1(profile(1000), Some(true), &mut f.r),
        Err(Error::Limit)
    );
    f.o.retention.release(&mut other).unwrap();
    assert_eq!(f.o.retention_usage().reserved, base);
    enable(&mut f.o, &mut f.r);
    assert_eq!(f.o.retention_usage().reserved, base + bytes);
    assert_eq!(f.o.semantic_root_usage(&mut f.r).unwrap(), root);
    assert!(!f.o.is_faulted());
}
#[test]
fn release_collection_stop_is_nonrenewing_and_retirement_never_refunds_twice() {
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    enable(&mut f.o, &mut f.r);
    query(&mut f, 0, 0, false).unwrap();
    assert!(!f.o.retire_release_measurements_v1(&control(), &mut f.r));
    assert!(!f.o.stop_release_measurements_v1(None, &mut f.r));
    assert!(f.o.stop_release_measurements_v1(Some(true), &mut f.r));
    assert_eq!(
        f.o.configure_release_measurements_v1(profile(1000), Some(true), &mut f.r),
        Ok(false)
    );
    query(&mut f, 0, 0, false).unwrap();
    assert!(
        f.o.release_measurements_v1(&mut f.r)
            .unwrap()
            .unwrap()
            .iter()
            .all(|p| p.state() == m::PointStateV1::Off)
    );
    let State::Active(p) = &f.o.release_measurements else {
        panic!("pool");
    };
    assert_eq!(p.counts[0].total, 1);
    for axis in 0..3 {
        let c = ReleaseMeasurementControlV1 {
            owner_control_allowed: if axis == 0 { None } else { Some(true) },
            delivery_drained: if axis == 1 { Some(false) } else { Some(true) },
            retention_released: if axis == 2 { None } else { Some(true) },
        };
        assert!(!f.o.retire_release_measurements_v1(&c, &mut f.r));
    }
    assert!(f.o.retire_release_measurements_v1(&control(), &mut f.r));
    assert_eq!(f.o.retention_usage().reserved, base);
    assert!(!f.o.retire_release_measurements_v1(&control(), &mut f.r));
    assert_eq!(
        f.o.configure_release_measurements_v1(profile(1000), Some(true), &mut f.r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        f.o.release_measurements_v1(&mut f.r),
        Err(Error::Unavailable)
    );
    query(&mut f, 0, 0, false).unwrap();
}
#[test]
fn release_measurements_and_diagnostic_capture_have_independent_saturation_and_no_payload_dimensions()
 {
    for active in [false, true] {
        for capacity in [None, Some(0), Some(1), Some(128)] {
            let mut f = owner();
            f.r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            if active {
                enable(&mut f.o, &mut f.r);
            }
            let body = with_facts(&mut f, 0, |o, source, scope, facts, r| {
                let reply = o
                    .try_query_source_facade_v1(
                        SourceFacadeRequestV1 {
                            source,
                            scope,
                            facts,
                            actor: &scope.context,
                            query: SourceContextQueryV1::Line { index: 0 },
                            limits: limits(),
                        },
                        r,
                    )
                    .unwrap();
                let SourceFacadeReplyV1::Source(reply) = reply else {
                    panic!("source");
                };
                let SourceContextContentV1::Line(line) = reply.content else {
                    panic!("line");
                };
                line.body().to_vec()
            });
            assert_eq!(body, b"body");
            if active {
                assert_eq!(totals(&mut f.o, &mut f.r), [1, 0, 0, 0, 0, 0]);
            } else {
                assert_eq!(f.o.release_measurements_v1(&mut f.r), Ok(None));
            }
            assert!(!f.o.is_faulted());
            while let Some(event) = f.r.pop_front() {
                assert!(event.service_time.is_none());
            }
        }
    }
}
#[test]
fn optional_release_profile_refusals_never_grant_authority_reset_counts_or_fault_reads() {
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    for authority in [None, Some(false)] {
        assert_eq!(
            f.o.configure_release_measurements_v1(profile(1000), authority, &mut f.r),
            Err(Error::Unavailable)
        );
    }
    assert_eq!(
        f.o.configure_release_measurements_v1(
            ReleaseMeasurementLimitsV1 {
                values: m::ValueLimitsV1 {
                    max_integer: u64::MAX,
                    max_seconds: 0.0
                },
                ..profile(1000)
            },
            Some(true),
            &mut f.r
        ),
        Err(Error::InvalidProfile)
    );
    assert_eq!(f.o.retention_usage().reserved, base);
    query(&mut f, 0, 0, false).unwrap();
    enable(&mut f.o, &mut f.r);
    query(&mut f, 0, 0, false).unwrap();
    assert_eq!(
        f.o.configure_release_measurements_v1(profile(1000), Some(true), &mut f.r),
        Ok(false)
    );
    assert_eq!(
        f.o.configure_release_measurements_v1(profile(99), Some(true), &mut f.r),
        Err(Error::Unavailable)
    );
    assert_eq!(totals(&mut f.o, &mut f.r), [1, 0, 0, 0, 0, 0]);
    assert!(!f.o.is_faulted());
}
#[test]
fn release_snapshot_exhaustion_is_preflighted_for_all_six_series_without_partial_advance() {
    let mut f = owner();
    f.o.configure_release_measurements_v1(profile(1), Some(true), &mut f.r)
        .unwrap();
    let zero = m::PointStateV1::Observed {
        freshness: m::FreshnessV1::Current,
        coverage: m::CoverageV1::Complete,
        value: m::ValueV1::Counter(m::CounterV1 {
            total: 0,
            exact: true,
        }),
    };
    let State::Active(p) = &mut f.o.release_measurements else {
        panic!("pool");
    };
    p.series[5].observe(zero).unwrap();
    p.series[5].observe(zero).unwrap();
    assert_eq!(f.o.release_measurements_v1(&mut f.r), Err(Error::Exhausted));
    let State::Active(p) = &mut f.o.release_measurements else {
        panic!("pool");
    };
    assert_eq!(p.series[0].observe(zero).unwrap().sequence(), 0);
    query(&mut f, 0, 0, false).unwrap();
    assert!(!f.o.is_faulted());
}
#[test]
fn early_busy_and_unowned_workspace_attempts_are_counted_without_adopting_or_erasing_their_obligations()
 {
    let mut f = owner();
    enable(&mut f.o, &mut f.r);
    f.o.source_facade_busy = true;
    assert_eq!(
        query(&mut f, 0, 0, false),
        Err(SourceReadContextErrorV1::Unavailable)
    );
    assert_eq!(totals(&mut f.o, &mut f.r), [0, 0, 1, 0, 0, 0]);
    assert!(!f.o.stop_release_measurements_v1(Some(true), &mut f.r));
    assert_eq!(
        f.o.configure_release_measurements_v1(profile(1000), Some(true), &mut f.r),
        Err(Error::Unavailable)
    );
    assert!(f.o.read_facade_usage().busy);
    f.o.source_facade_busy = false;
    f.o.source_facade_workspace = 7;
    assert_eq!(
        query(&mut f, 0, 0, false),
        Err(SourceReadContextErrorV1::Faulted)
    );
    assert_eq!(totals(&mut f.o, &mut f.r), [0, 0, 1, 0, 0, 1]);
    assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
    assert!(f.o.stop_release_measurements_v1(Some(true), &mut f.r));
    assert!(f.o.retire_release_measurements_v1(&control(), &mut f.r));
    assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
    assert!(f.o.is_faulted());
}
