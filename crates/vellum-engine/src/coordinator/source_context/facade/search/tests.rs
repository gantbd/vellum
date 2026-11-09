use super::*;
use crate::coordinator::source_context::search::result_contract::tests::{
    holds, last_result, owner as native_owner, qfacts, request, retire, with_facts,
};
use crate::coordinator::source_query::result_contract::tests::Fixture;
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
#[test]
fn native_search_timing_distinguishes_facade_and_owner_service_without_timing_inner_markers() {
    for kind in 0..3 {
        let mut f = owner();
        f.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        let q = request();
        let id = evaluated(&mut f, kind, &q);
        for op in 1..4 {
            assert!(run(&mut f, kind, port(op, &id), &q).is_some());
        }
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        let samples: Vec<_> = events.iter().filter(|e| e.service_time.is_some()).collect();
        // Each port has one whole-facade duration and one native-owner duration.
        // These are nested, not additive; intermediate markers remain untimed.
        assert_eq!(samples.len(), 8);
        for (boundary, count) in [
            (Boundary::SearchFacadeRelease, 4),
            (Boundary::SourceSearchEvaluation, 1),
            (Boundary::SourceSearchRelease, 1),
            (Boundary::SearchOccurrenceResolution, 1),
            (Boundary::SourceSearchNavigation, 1),
        ] {
            assert_eq!(
                samples.iter().filter(|e| e.boundary == boundary).count(),
                count
            );
        }
        assert!(
            events
                .iter()
                .filter(|e| matches!(
                    e.boundary,
                    Boundary::SearchFacadeAdmission
                        | Boundary::SearchFacadeCapture
                        | Boundary::SearchFacadeNativeHandoff
                        | Boundary::SearchFacadeResultValidation
                ))
                .all(|e| e.service_time.is_none())
        );
        assert!(!f.o.is_faulted());
    }
}
fn run(
    f: &mut Fixture,
    kind: usize,
    port: SearchFacadePortV1<'_>,
    q: &SearchRequest,
) -> Option<SearchFacadeReplyV1> {
    invoke(f, kind, port, q, |o, request, r| {
        o.search_facade_v1(request, r)
    })
}
fn invoke(
    f: &mut Fixture,
    kind: usize,
    port: SearchFacadePortV1<'_>,
    q: &SearchRequest,
    call: impl for<'a> FnOnce(
        &mut SourceCoordinator,
        SearchFacadeRequestV1<'a>,
        &mut Recorder,
    ) -> Option<SearchFacadeReplyV1>,
) -> Option<SearchFacadeReplyV1> {
    with_facts(f, kind, |o, source, scope, facts, r| {
        let mut actor = scope.context.clone();
        actor.recipient = "separate-headless-actor".into();
        call(
            o,
            SearchFacadeRequestV1 {
                source,
                scope,
                actor: &actor,
                port,
                query: q,
                current: &qfacts(q),
                facts,
                limits: limits(),
            },
            r,
        )
    })
}
fn evaluated(f: &mut Fixture, kind: usize, q: &SearchRequest) -> SourceSearchResultId {
    let Some(SearchFacadeReplyV1::Evaluated(id)) = run(f, kind, SearchFacadePortV1::Evaluate, q)
    else {
        panic!("evaluation")
    };
    id
}
fn port(op: usize, id: &SourceSearchResultId) -> SearchFacadePortV1<'_> {
    match op {
        0 => SearchFacadePortV1::Evaluate,
        1 => SearchFacadePortV1::Release { result: id },
        2 => SearchFacadePortV1::Occurrence {
            result: id,
            index: 0,
        },
        _ => SearchFacadePortV1::Navigate {
            result: id,
            navigation: SourceSearchNavigationV1 {
                direction: SearchDirection::Next,
                start: 0,
                current: None,
                wrap: false,
            },
        },
    }
}
#[test]
fn typed_search_outcomes_preserve_success_for_every_port_and_original_issuer() {
    for kind in 0..3 {
        let mut f = owner();
        let q = request();
        let id = evaluated(&mut f, kind, &q);
        for op in 0..4 {
            invoke(&mut f, kind, port(op, &id), &q, |o, request, r| {
                let out = o.try_search_facade_v1(request, r).unwrap();
                assert!(matches!(
                    (op, &out),
                    (0, SearchFacadeReplyV1::Evaluated(_))
                        | (1, SearchFacadeReplyV1::Released(_))
                        | (2, SearchFacadeReplyV1::Occurrence(_))
                        | (3, SearchFacadeReplyV1::Navigation(_))
                ));
                Some(out)
            });
        }
        assert!(!f.o.is_faulted());
        assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
    }
}
#[test]
fn typed_search_outcomes_distinguish_adapter_shape_budget_and_native_navigation_errors() {
    for kind in 0..3 {
        for mode in 0..4 {
            let mut f = owner();
            let q = request();
            let id = evaluated(&mut f, kind, &q);
            let base = f.o.retention_usage().reserved;
            invoke(&mut f, kind, port(3, &id), &q, |o, mut req, r| {
                let expected = match mode {
                    0 => {
                        req.limits.max_nodes = 0;
                        Error::InvalidRequest
                    }
                    1 => {
                        req.limits.max_bytes = 1;
                        Error::Limit
                    }
                    2 => {
                        req.port = SearchFacadePortV1::Navigate {
                            result: req.port.result().unwrap(),
                            navigation: SourceSearchNavigationV1 {
                                direction: SearchDirection::Next,
                                start: 999,
                                current: None,
                                wrap: false,
                            },
                        };
                        Error::InvalidRequest
                    }
                    _ => {
                        let current = TrustedSourceSearchFactsV1 {
                            grant: None,
                            ..qfacts(&q)
                        };
                        assert!(matches!(
                            o.try_search_facade_v1(
                                SearchFacadeRequestV1 {
                                    current: &current,
                                    ..req
                                },
                                r
                            ),
                            Err(Error::Unavailable)
                        ));
                        return None;
                    }
                };
                assert!(matches!(o.try_search_facade_v1(req, r), Err(e) if e == expected));
                None
            });
            assert!(!f.o.is_faulted());
            assert_eq!(f.o.retention_usage().reserved, base);
            assert!(
                run(
                    &mut f,
                    kind,
                    SearchFacadePortV1::Release { result: &id },
                    &q
                )
                .is_some()
            );
        }
    }
}
#[test]
fn typed_search_outcomes_preserve_native_limits_instead_of_collapsing_to_unavailable() {
    for kind in 0..3 {
        let mut f = owner();
        let q = SearchRequest {
            query: "x".repeat(1024),
            scope: SearchScope::Document,
        };
        let base = f.o.retention_usage().reserved;
        invoke(
            &mut f,
            kind,
            SearchFacadePortV1::Evaluate,
            &q,
            |o, req, r| {
                assert!(matches!(o.try_search_facade_v1(req, r), Err(Error::Limit)));
                None
            },
        );
        assert_eq!(f.o.source_search_usage().issued_lifetime, 0);
        assert_eq!(f.o.retention_usage().reserved, base);
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn typed_search_outcomes_do_not_depend_on_diagnostic_capture_or_disclose_authority_premises() {
    for kind in 0..3 {
        for capacity in [None, Some(0), Some(1), Some(128)] {
            for denial in 0..7 {
                let mut f = owner();
                let q = request();
                let id = evaluated(&mut f, kind, &q);
                f.r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                let base = f.o.retention_usage().reserved;
                invoke(&mut f, kind, port(1, &id), &q, |o, req, r| {
                    let mut current = qfacts(req.current.request);
                    match denial {
                        0 => current.grant = Some(false),
                        1 => current.scope = Some(false),
                        2 => current.continuity = Some(false),
                        3 => current.capacity = Some(false),
                        4 => current.retained = Some(false),
                        5 => current.not_stopped = Some(false),
                        _ => current.grant = None,
                    }
                    assert!(matches!(
                        o.try_search_facade_v1(
                            SearchFacadeRequestV1 {
                                current: &current,
                                ..req
                            },
                            r
                        ),
                        Err(Error::Unavailable)
                    ));
                    None
                });
                assert_eq!(f.o.retention_usage().reserved, base);
                assert!(!f.o.is_faulted());
                assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
                assert!(f.r.queued() <= capacity.unwrap_or(0));
                while let Some(event) = f.r.pop_front() {
                    assert!(event.service_time.is_none());
                }
            }
        }
    }
}
#[test]
fn typed_search_outcomes_refuse_busy_owners_without_reentry_and_fence_unowned_workspace() {
    for kind in 0..3 {
        let mut f = owner();
        let q = request();
        let base = f.o.retention_usage().reserved;
        f.o.source_facade_busy = true;
        invoke(
            &mut f,
            kind,
            SearchFacadePortV1::Evaluate,
            &q,
            |o, req, r| {
                assert!(matches!(
                    o.try_search_facade_v1(req, r),
                    Err(Error::Unavailable)
                ));
                None
            },
        );
        assert!(f.o.read_facade_usage().busy);
        assert!(!f.o.is_faulted());
        f.o.source_facade_busy = false;
        f.o.source_facade_workspace = 1;
        f.r = Recorder::try_with_capacity(4).unwrap();
        for _ in 0..2 {
            invoke(
                &mut f,
                kind,
                SearchFacadePortV1::Evaluate,
                &q,
                |o, req, r| {
                    assert!(matches!(
                        o.try_search_facade_v1(req, r),
                        Err(Error::Faulted)
                    ));
                    None
                },
            );
        }
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        assert_eq!(events.len(), 2);
        assert!(
            events
                .iter()
                .all(|e| e.boundary == Boundary::SearchFacadeAdmission
                    && e.disposition == Disposition::Unknown)
        );
        assert_eq!(f.o.read_facade_usage().workspace_bytes, 1);
        assert_eq!(f.o.retention_usage().reserved, base);
        assert_eq!(f.o.source_search_usage().issued_lifetime, 0);
    }
}
#[test]
fn search_facade_preserves_all_three_original_issuers_across_native_result_lifetimes() {
    for kind in 0..3 {
        let mut f = owner();
        let q = request();
        let base = f.o.retention_usage().reserved;
        let before = holds(&f);
        let Some(SearchFacadeReplyV1::Evaluated(id)) =
            run(&mut f, kind, SearchFacadePortV1::Evaluate, &q)
        else {
            panic!("native evaluation")
        };
        let held = holds(&f);
        let issued = f.o.source_search_usage().issued_lifetime;
        let Some(SearchFacadeReplyV1::Released(reply)) = run(
            &mut f,
            kind,
            SearchFacadePortV1::Release { result: &id },
            &q,
        ) else {
            panic!("literal result")
        };
        assert_eq!(reply.result, id);
        assert_eq!(reply.matches, vec![ByteRange { start: 1, end: 2 }]);
        assert!(matches!(
            run(
                &mut f,
                kind,
                SearchFacadePortV1::Occurrence {
                    result: &id,
                    index: 0
                },
                &q
            ),
            Some(SearchFacadeReplyV1::Occurrence(_))
        ));
        assert!(matches!(
            run(
                &mut f,
                kind,
                SearchFacadePortV1::Navigate {
                    result: &id,
                    navigation: SourceSearchNavigationV1 {
                        direction: SearchDirection::Next,
                        start: 0,
                        current: None,
                        wrap: false
                    }
                },
                &q
            ),
            Some(SearchFacadeReplyV1::Navigation(_))
        ));
        assert_eq!(holds(&f), held);
        assert_eq!(f.o.source_search_usage().issued_lifetime, issued);
        assert_eq!(f.o.retention_usage().reserved, base);
        retire(&mut f, kind, &id);
        assert_eq!(holds(&f), before);
        assert!(!f.o.is_faulted());
        let q = SearchRequest {
            query: "o".into(),
            scope: SearchScope::Captured(vec![ByteRange { start: 0, end: 1 }]),
        };
        let id = evaluated(&mut f, kind, &q);
        let Some(SearchFacadeReplyV1::Released(v)) = run(
            &mut f,
            kind,
            SearchFacadePortV1::Release { result: &id },
            &q,
        ) else {
            panic!("captured-scope result")
        };
        assert!(v.matches.is_empty());
        let Some(SearchFacadeReplyV1::Navigation(v)) = run(&mut f, kind, port(3, &id), &q) else {
            panic!("available empty navigation")
        };
        assert!(v.found.is_none());
        retire(&mut f, kind, &id);
    }
}
#[test]
fn search_facade_current_request_is_independently_copied_and_never_repaired_from_arguments() {
    for kind in 0..3 {
        let mut f = owner();
        let q = request();
        let other = SearchRequest {
            query: "o".into(),
            scope: SearchScope::Captured(Vec::new()),
        };
        assert!(
            invoke(
                &mut f,
                kind,
                SearchFacadePortV1::Evaluate,
                &q,
                |o, req, r| {
                    let current = qfacts(&other);
                    o.search_facade_v1(
                        SearchFacadeRequestV1 {
                            current: &current,
                            ..req
                        },
                        r,
                    )
                }
            )
            .is_none()
        );
        assert_eq!(f.o.source_search_usage().issued_lifetime, 0);
        assert!(run(&mut f, kind, SearchFacadePortV1::Evaluate, &q).is_some());
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn search_facade_foreign_issuer_or_result_cannot_redirect_or_poison_the_original() {
    let mut f = owner();
    let q = request();
    let id = evaluated(&mut f, 0, &q);
    let usage = f.o.source_search_usage();
    let held = holds(&f);
    for kind in [1, 2] {
        assert!(
            run(
                &mut f,
                kind,
                SearchFacadePortV1::Release { result: &id },
                &q
            )
            .is_none()
        );
    }
    let mut foreign = owner();
    let foreign_id = evaluated(&mut foreign, 0, &q);
    let Some(SearchFacadeReplyV1::Occurrence(foreign_occurrence)) = run(
        &mut foreign,
        0,
        SearchFacadePortV1::Occurrence {
            result: &foreign_id,
            index: 0,
        },
        &q,
    ) else {
        panic!("foreign occurrence")
    };
    assert!(
        run(
            &mut f,
            0,
            SearchFacadePortV1::Release {
                result: &foreign_id
            },
            &q
        )
        .is_none()
    );
    assert_eq!(f.o.source_search_usage(), usage);
    assert_eq!(holds(&f), held);
    assert!(!f.o.is_faulted());
    assert!(
        invoke(
            &mut f,
            0,
            SearchFacadePortV1::Navigate {
                result: &id,
                navigation: SourceSearchNavigationV1 {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: Some(&foreign_occurrence),
                    wrap: false
                }
            },
            &q,
            |o, req, r| {
                let SearchReadFactsV1::Source(sf) = req.facts else {
                    panic!("source facts")
                };
                let sf = TrustedSourceReadFactsV1 {
                    authorized: Some(false),
                    ..*sf
                };
                o.search_facade_v1(
                    SearchFacadeRequestV1 {
                        facts: SearchReadFactsV1::Source(&sf),
                        ..req
                    },
                    r,
                )
            }
        )
        .is_none()
    );
    assert!(run(&mut f, 0, SearchFacadePortV1::Release { result: &id }, &q).is_some());
}
#[test]
fn search_facade_unknown_and_capacity_refusal_do_not_establish_loss_but_bound_stop_does() {
    for kind in 0..3 {
        let mut f = owner();
        let q = request();
        let id = evaluated(&mut f, kind, &q);
        for mode in 0..2 {
            assert!(
                invoke(
                    &mut f,
                    kind,
                    SearchFacadePortV1::Release { result: &id },
                    &q,
                    |o, req, r| {
                        let mut current = qfacts(req.current.request);
                        if mode == 0 {
                            current.grant = None;
                        } else {
                            current.capacity = Some(false);
                        }
                        o.search_facade_v1(
                            SearchFacadeRequestV1 {
                                current: &current,
                                ..req
                            },
                            r,
                        )
                    }
                )
                .is_none()
            );
            assert!(
                run(
                    &mut f,
                    kind,
                    SearchFacadePortV1::Release { result: &id },
                    &q
                )
                .is_some()
            );
        }
        assert!(
            invoke(
                &mut f,
                kind,
                SearchFacadePortV1::Release { result: &id },
                &q,
                |o, req, r| {
                    let mut current = qfacts(req.current.request);
                    current.not_stopped = Some(false);
                    current.capacity = Some(false);
                    o.search_facade_v1(
                        SearchFacadeRequestV1 {
                            current: &current,
                            ..req
                        },
                        r,
                    )
                }
            )
            .is_none()
        );
        assert_eq!(f.o.source_search_usage().invalidated, 1);
        assert!(
            run(
                &mut f,
                kind,
                SearchFacadePortV1::Release { result: &id },
                &q
            )
            .is_none()
        );
        retire(&mut f, kind, &id);
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn search_facade_source_loss_precedes_query_capacity_and_malformed_requests_cannot_lend_loss() {
    for malformed in [true, false] {
        let mut f = owner();
        let q = request();
        let other = SearchRequest {
            query: "other".into(),
            scope: SearchScope::Document,
        };
        assert!(
            invoke(&mut f, 0, SearchFacadePortV1::Evaluate, &q, |o, req, r| {
                let SearchReadFactsV1::Source(sf) = req.facts else {
                    panic!("source")
                };
                let sf = TrustedSourceReadFactsV1 {
                    authorized: Some(false),
                    ..*sf
                };
                let mut current = qfacts(if malformed {
                    &other
                } else {
                    req.current.request
                });
                current.capacity = Some(false);
                o.search_facade_v1(
                    SearchFacadeRequestV1 {
                        facts: SearchReadFactsV1::Source(&sf),
                        current: &current,
                        ..req
                    },
                    r,
                )
            })
            .is_none()
        );
        assert_eq!(
            run(&mut f, 0, SearchFacadePortV1::Evaluate, &q).is_some(),
            malformed
        );
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn search_facade_copy_limits_refuse_before_loss_and_refund_original_parent_headroom() {
    for kind in 0..3 {
        for axis in 0..5 {
            let mut f = owner();
            let q = request();
            let base = f.o.retention_usage().reserved;
            let before = holds(&f);
            assert!(
                invoke(
                    &mut f,
                    kind,
                    SearchFacadePortV1::Evaluate,
                    &q,
                    |o, req, r| {
                        let mut limits = req.limits;
                        match axis {
                            0 => limits.max_bytes = 1,
                            1 => limits.max_nodes = 1,
                            2 => limits.max_depth = 1,
                            3 => limits.max_nodes = 0,
                            _ => o.set_settings_parent_limit_for_test(1),
                        }
                        let mut current = qfacts(req.current.request);
                        current.grant = Some(false);
                        o.search_facade_v1(
                            SearchFacadeRequestV1 {
                                limits,
                                current: &current,
                                ..req
                            },
                            r,
                        )
                    }
                )
                .is_none()
            );
            assert_eq!(f.o.source_search_usage().issued_lifetime, 0);
            assert_eq!(holds(&f), before);
            assert_eq!(f.o.retention_usage().reserved, base);
            assert!(!f.o.is_faulted());
            f.o.set_settings_parent_limit_for_test(16384);
            assert!(run(&mut f, kind, SearchFacadePortV1::Evaluate, &q).is_some());
        }
    }
}
#[test]
fn search_facade_malformed_ranges_navigation_and_actor_never_poison_or_fault_the_owner() {
    for mode in 0..5 {
        let mut f = owner();
        let good = request();
        let id = evaluated(&mut f, 0, &good);
        let q = if mode == 0 {
            SearchRequest {
                query: "o".into(),
                scope: SearchScope::Captured(vec![ByteRange { start: 3, end: 1 }]),
            }
        } else {
            request()
        };
        let p = if mode == 1 {
            SearchFacadePortV1::Occurrence {
                result: &id,
                index: usize::MAX,
            }
        } else if mode == 2 {
            SearchFacadePortV1::Navigate {
                result: &id,
                navigation: SourceSearchNavigationV1 {
                    direction: SearchDirection::Next,
                    start: usize::MAX,
                    current: None,
                    wrap: false,
                },
            }
        } else {
            SearchFacadePortV1::Evaluate
        };
        assert!(
            invoke(&mut f, 0, p, &q, |o, req, r| {
                let mut actor = req.actor.clone();
                if mode == 3 {
                    actor.engine = "foreign".into();
                } else if mode == 4 {
                    actor.recipient.clear();
                }
                o.search_facade_v1(
                    SearchFacadeRequestV1 {
                        actor: &actor,
                        ..req
                    },
                    r,
                )
            })
            .is_none()
        );
        assert!(!f.o.is_faulted());
        assert!(
            run(
                &mut f,
                0,
                SearchFacadePortV1::Release { result: &id },
                &good
            )
            .is_some()
        );
    }
}
#[test]
fn search_facade_exact_parent_admission_uses_eight_frames_for_every_port_and_issuer() {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 8 * limits().max_bytes;
    for kind in 0..3 {
        for op in 0..4 {
            for available in [fixed - 1, fixed, total - 1, total] {
                let mut f = owner();
                let q = request();
                let id = evaluated(&mut f, kind, &q);
                let base = f.o.retention_usage().reserved;
                let pressure = f.o.retention_usage().maximum.unwrap() - base - available;
                let mut held = f.o.retention.reserve(pressure).unwrap();
                let before = f.o.retention_usage().reserved;
                let usage = f.o.source_search_usage();
                assert_eq!(
                    invoke(&mut f, kind, port(op, &id), &q, |o, req, r| {
                        let out = o.try_search_facade_v1(req, r);
                        if available < total {
                            assert!(matches!(out, Err(Error::Limit)));
                        }
                        out.ok()
                    })
                    .is_some(),
                    available >= total
                );
                assert_eq!(f.o.retention_usage().reserved, before);
                assert!(!f.o.is_faulted());
                if available < total {
                    assert_eq!(f.o.source_search_usage(), usage);
                }
                f.o.retention.release(&mut held).unwrap();
                assert_eq!(f.o.retention_usage().reserved, base);
            }
        }
    }
}
#[test]
fn search_facade_faults_refund_only_workspace_and_preserve_native_result_hold_and_issuance() {
    for kind in 0..3 {
        for op in 0..4 {
            for fault in [
                Fault::Classification,
                Fault::Workspace,
                Fault::BeforeNative,
                Fault::AfterNative,
                Fault::BadResult,
                Fault::RefundDrift,
            ] {
                let mut f = owner();
                let q = request();
                let id = if op > 0 {
                    Some(evaluated(&mut f, kind, &q))
                } else {
                    None
                };
                let p = id
                    .as_ref()
                    .map_or(SearchFacadePortV1::Evaluate, |id| port(op, id));
                let base = f.o.retention_usage().reserved;
                let baseline = holds(&f);
                let usage = f.o.source_search_usage();
                let generation = f.o.generation();
                f.o.source_facade_fault = Some(fault);
                invoke(&mut f, kind, p, &q, |o, req, r| {
                    assert!(
                        matches!(o.try_search_facade_v1(req, r), Err(Error::Faulted)),
                        "{kind}/{op}/{fault:?}"
                    );
                    None
                });
                assert!(f.o.is_faulted());
                assert_eq!(f.o.retention_usage().reserved, base);
                assert_eq!(
                    f.o.read_facade_usage(),
                    ReadFacadeUsageV1 {
                        busy: false,
                        workspace_bytes: 0
                    }
                );
                assert_eq!(f.o.generation(), generation);
                let installed = usize::from(
                    op == 0
                        && matches!(
                            fault,
                            Fault::AfterNative | Fault::BadResult | Fault::RefundDrift
                        ),
                );
                assert_eq!(
                    f.o.source_search_usage().issued_lifetime,
                    usage.issued_lifetime + installed
                );
                assert_eq!(f.o.source_search_usage().results, usage.results + installed);
                if fault == Fault::BadResult {
                    assert_eq!(
                        f.o.read_result_invariant(),
                        Some(ReadResultInvariantV1::NativeSearchFacadeResultInvariant)
                    );
                }
                f.o.source_facade_fault = None;
                if let Some(id) = id {
                    retire(&mut f, kind, &id);
                } else if installed > 0 {
                    let id = last_result(&f.o).unwrap();
                    retire(&mut f, kind, &id);
                }
                if op == 0 {
                    assert_eq!(holds(&f), baseline);
                } else {
                    assert_eq!(holds(&f), [0, 0, 0]);
                }
                assert!(f.o.is_faulted());
            }
        }
    }
}
#[test]
fn search_facade_lower_native_invariant_is_not_overwritten_by_adapter_fault_diagnosis() {
    use crate::coordinator::source_context::search::result_contract::Fault as Native;
    let mut f = owner();
    let q = request();
    let base = f.o.retention_usage().reserved;
    f.o.search_result_fault = Some(Native::ProducerRange);
    assert!(run(&mut f, 0, SearchFacadePortV1::Evaluate, &q).is_none());
    assert!(f.o.is_faulted());
    assert_eq!(
        f.o.read_result_invariant(),
        Some(ReadResultInvariantV1::NativeSearchResultInvariant)
    );
    assert_eq!(f.o.retention_usage().reserved, base);
    assert_eq!(holds(&f), [0, 0, 0]);
}
#[test]
fn search_facade_nonfirst_coherent_member_retains_selected_scope_and_complete_peer_authority() {
    use crate::coordinator::source_query::result_contract::tests::{cleanup, member_facts};
    fn capture(scope: &SourceReadScopeV1) -> TrustedAnnotationCaptureFactsV1<'_> {
        TrustedAnnotationCaptureFactsV1 {
            scope,
            grants: &[],
            authorized: Some(true),
            scope_allowed: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            source_readable: Some(true),
            annotation_readable: Some(true),
            visibility_complete: Some(true),
            projection_order_attested: Some(true),
            resources_ready: Some(true),
        }
    }
    let mut f = owner();
    let peer_version = f.o.create(b"other", &mut f.r).unwrap();
    let mut peer = f.s.clone();
    peer.document = peer_version.document.clone();
    let participants = [
        VersionSetParticipantV1 {
            scope: &f.s,
            data: AnnotationProjectionClassesV1::Source,
        },
        VersionSetParticipantV1 {
            scope: &peer,
            data: AnnotationProjectionClassesV1::Source,
        },
    ];
    let prepared =
        f.o.prepare_version_set(&participants, &[capture(&f.s), capture(&peer)], &mut f.r)
            .unwrap();
    let set =
        f.o.install_version_set(
            &prepared,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &prepared,
                members: &[member_facts(&f.s), member_facts(&peer)],
            },
            &mut f.r,
        )
        .unwrap();
    let member = &set.participants[1].snapshot;
    let q = request();
    let mut actor = peer.context.clone();
    actor.recipient = "separate-actor".into();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &set.context,
        members: &[member_facts(&f.s), member_facts(&peer)],
    };
    let selected = TrustedVersionSetSearchFactsV1 {
        member,
        scope: &peer,
        parent: &parent,
    };
    let req = SearchFacadeRequestV1 {
        source: SearchReadContextV1::VersionSet(member),
        scope: &peer,
        actor: &actor,
        port: SearchFacadePortV1::Evaluate,
        query: &q,
        current: &qfacts(&q),
        facts: SearchReadFactsV1::VersionSet(&selected),
        limits: limits(),
    };
    let Some(SearchFacadeReplyV1::Evaluated(result)) = f.o.search_facade_v1(req, &mut f.r) else {
        panic!("selected nonfirst member")
    };
    let release = SearchFacadeRequestV1 {
        port: SearchFacadePortV1::Release { result: &result },
        ..req
    };
    let Some(SearchFacadeReplyV1::Released(reply)) = f.o.search_facade_v1(release, &mut f.r) else {
        panic!("selected release")
    };
    assert_eq!(reply.version, peer_version);
    assert_eq!(reply.matches, vec![ByteRange { start: 0, end: 1 }]);
    let mut denied = member_facts(&f.s);
    denied.authorized = Some(false);
    let parent = TrustedVersionSetContextFactsV1 {
        context: &set.context,
        members: &[denied, member_facts(&peer)],
    };
    let selected = TrustedVersionSetSearchFactsV1 {
        member,
        scope: &peer,
        parent: &parent,
    };
    let mut current = qfacts(&q);
    current.capacity = Some(false);
    assert!(
        f.o.search_facade_v1(
            SearchFacadeRequestV1 {
                facts: SearchReadFactsV1::VersionSet(&selected),
                current: &current,
                ..release
            },
            &mut f.r
        )
        .is_none()
    );
    assert!(f.o.search_facade_v1(release, &mut f.r).is_none());
    assert!(!f.o.is_faulted());
    assert!(f.o.retire_managed_search_result(
        &result,
        SearchResultControlV1::VersionSet(&[cleanup(&f.s), cleanup(&peer)]),
        &mut f.r
    ));
}
#[test]
fn search_facade_reentry_and_unowned_workspace_debt_never_create_another_execution() {
    let mut f = owner();
    let q = request();
    f.o.source_facade_fault = Some(Fault::Reentry);
    assert!(run(&mut f, 0, SearchFacadePortV1::Evaluate, &q).is_some());
    assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
    f.o.source_facade_fault = None;
    f.o.source_facade_workspace = 7;
    let base = f.o.retention_usage().reserved;
    assert!(run(&mut f, 0, SearchFacadePortV1::Evaluate, &q).is_none());
    assert!(f.o.is_faulted());
    assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
    assert_eq!(f.o.retention_usage().reserved, base);
    assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
}
#[test]
fn search_facade_diagnostics_off_available_and_full_preserve_results_and_native_holds() {
    for cap in [None, Some(128), Some(1)] {
        let mut f = owner();
        let q = request();
        f.r = cap.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = evaluated(&mut f, 0, &q);
        assert!(run(&mut f, 0, SearchFacadePortV1::Release { result: &id }, &q).is_some());
        assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
        assert_eq!(holds(&f), [1, 0, 0]);
        assert!(!f.o.is_faulted());
        match cap {
            None => assert_eq!(f.r.queued(), 0),
            Some(1) => assert!(f.r.dropped() > 0),
            _ => {
                let mut seen = false;
                while let Some(e) = f.r.pop_front() {
                    seen |= e.boundary == Boundary::SearchFacadeResultValidation
                        && e.disposition == Disposition::Completed;
                    assert!(e.service_time.is_none());
                }
                assert!(seen);
            }
        }
        retire(&mut f, 0, &id);
    }
}
