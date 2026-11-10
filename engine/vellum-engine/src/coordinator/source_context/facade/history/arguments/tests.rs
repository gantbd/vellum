use super::super::tests::{Fixture, control, facts, limits, owner};
use super::*;
use HistoryEnvelopeValueV1::{Context, Data, Object};
struct Input<'a, 'h> {
    route: &'a str,
    selector: InertValueV1<'a>,
    version: InertValueV1<'a>,
    variant: &'a str,
    operation: &'a str,
    contract: &'a str,
    context: HistoryEnvelopeValueV1<'a, 'h>,
    scope: InertValueV1<'a>,
    direction: InertValueV1<'a>,
    profile: InertValueV1<'a>,
    group: InertValueV1<'a>,
    extra: bool,
    duplicate: bool,
    header_extra: bool,
    reverse: bool,
    force_group: bool,
}
impl<'a, 'h> Input<'a, 'h> {
    fn new(context: &'h HistoryInspectionIdV1, route: &'a str, variant: &'a str) -> Self {
        Self {
            route,
            selector: if route == "typed" {
                InertValueV1::Number(3.0)
            } else {
                InertValueV1::Text("inspect-history")
            },
            version: InertValueV1::Number(1.0),
            variant,
            operation: variant,
            contract: "history-query/1",
            context: Context(context),
            scope: InertValueV1::Text("visible"),
            direction: InertValueV1::Text("undo"),
            profile: InertValueV1::Text("history/1"),
            group: InertValueV1::Text("group"),
            extra: false,
            duplicate: false,
            header_extra: false,
            reverse: false,
            force_group: false,
        }
    }
}
fn with_input<'a, 'h, R>(
    i: &Input<'a, 'h>,
    f: impl for<'v> FnOnce(&'v HistoryEnvelopeValueV1<'v, 'h>) -> R,
) -> R {
    let mut args = [
        ("contract", Data(InertValueV1::Text(i.contract))),
        ("operation", Data(InertValueV1::Text(i.operation))),
        ("context", i.context),
        ("scope", Data(i.scope)),
        ("direction", Data(i.direction)),
        ("profile", Data(i.profile)),
        ("group", Data(i.group)),
        ("extra", Data(InertValueV1::Boolean(true))),
    ];
    let mut n = if i.variant == "frontier" && !i.force_group {
        6
    } else {
        7
    };
    if i.extra {
        if n == 6 {
            args[6] = args[7];
        }
        n += 1;
    }
    if i.duplicate {
        args[n - 1].0 = "contract";
    }
    if i.reverse {
        args[..n].reverse();
    }
    let request = [
        (
            if i.route == "dynamic" {
                "name"
            } else {
                "family"
            },
            Data(i.selector),
        ),
        ("variant", Data(InertValueV1::Text(i.variant))),
        ("version", Data(i.version)),
        ("arguments", Object(&args[..n])),
        ("extra", Data(InertValueV1::Null)),
    ];
    let envelope = [
        ("route", Data(InertValueV1::Text(i.route))),
        (
            "request",
            Object(&request[..if i.header_extra { 5 } else { 4 }]),
        ),
    ];
    f(&Object(&envelope))
}
fn normalized<'h>(
    i: &Input<'_, 'h>,
    s: &HistoryInspectionScopeV1,
    r: &mut Recorder,
) -> Result<NormalizedHistoryArgumentsV1<'h>, Error> {
    with_input(i, |value| {
        normalize_history_envelope_v1(value, s, limits(), limits().max_bytes, r)
    })
}
fn run(f: &mut Fixture, route: &str, variant: &str) -> Option<HistoryInspectionReplyV1> {
    let i = Input::new(&f.id, route, variant);
    let current = facts(&f.s);
    with_input(&i, |envelope| {
        f.o.query_decoded_history_facade_v1(
            DecodedHistoryFacadeRequestV1 {
                envelope,
                actor: &f.s,
                limits: limits(),
                facts: &current,
            },
            &mut f.r,
        )
    })
}
pub(in crate::coordinator) fn with_decoded_request<T>(
    id: &HistoryInspectionIdV1,
    scope: &HistoryInspectionScopeV1,
    route: &str,
    operation: &str,
    consume: impl FnOnce(DecodedHistoryFacadeRequestV1<'_, '_>) -> T,
) -> T {
    let input = Input::new(id, route, operation);
    let current = facts(scope);
    with_input(&input, |envelope| {
        consume(DecodedHistoryFacadeRequestV1 {
            envelope,
            actor: scope,
            limits: limits(),
            facts: &current,
        })
    })
}
#[test]
fn decoded_history_facade_delivers_typed_normalization_errors_on_both_routes() {
    for route in ["typed", "dynamic"] {
        for variant in ["frontier", "availability", "permitted-blocking-detail"] {
            let mut f = owner();
            f.r = Recorder::try_with_capacity(256)
                .unwrap()
                .with_native_timing(
                    crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                );
            crate::coordinator::release_measurements::tests::enable(&mut f.o, &mut f.r);
            let base = f.o.retention_usage().reserved;
            let mut input = Input::new(&f.id, route, variant);
            input.duplicate = true;
            let current = facts(&f.s);
            with_input(&input, |envelope| {
                assert!(matches!(
                    f.o.try_query_decoded_history_facade_v1(
                        DecodedHistoryFacadeRequestV1 {
                            envelope,
                            actor: &f.s,
                            facts: &current,
                            limits: limits()
                        },
                        &mut f.r
                    ),
                    Err(Error::InvalidRequest)
                ));
            });
            input.duplicate = false;
            with_input(&input, |envelope| {
                assert!(
                    f.o.try_query_decoded_history_facade_v1(
                        DecodedHistoryFacadeRequestV1 {
                            envelope,
                            actor: &f.s,
                            facts: &current,
                            limits: limits()
                        },
                        &mut f.r
                    )
                    .is_ok()
                );
            });
            assert_eq!(f.o.retention_usage().reserved, base);
            let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::HistoryFacadeRelease
                        && e.service_time.is_some())
            );
            assert!(
                events
                    .iter()
                    .filter(|e| e.boundary != Boundary::HistoryFacadeRelease)
                    .all(|e| e.service_time.is_none())
            );
            assert!(!f.o.is_faulted());
            assert_eq!(
                crate::coordinator::release_measurements::tests::totals(&mut f.o, &mut f.r),
                [1, 0, 1, 0, 0, 0]
            );
        }
    }
}
#[test]
fn typed_and_dynamic_routes_share_owned_meaning_limits_and_original_opaque_identity() {
    let mut f = owner();
    for variant in ["frontier", "availability", "permitted-blocking-detail"] {
        let a = normalized(&Input::new(&f.id, "typed", variant), &f.s, &mut f.r).unwrap();
        let mut input = Input::new(&f.id, "dynamic", variant);
        input.reverse = true;
        let b = normalized(&input, &f.s, &mut f.r).unwrap();
        assert_eq!(a, b);
        assert!(std::ptr::eq(a.context(), &f.id));
        assert_eq!(a.limits(), limits());
        assert_eq!(a.actor(), &f.s);
        assert_eq!(
            run(&mut f, "typed", variant),
            run(&mut f, "dynamic", variant)
        );
        assert!(!f.o.read_facade_usage().busy);
        assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
    }
}
#[test]
fn ordinary_values_and_actor_are_owned_after_the_original_envelope_and_strings_are_gone() {
    let mut f = owner();
    let n = {
        let scope = "visible".to_owned();
        let mut i = Input::new(&f.id, "typed", "availability");
        i.scope = InertValueV1::Text(&scope);
        normalized(&i, &f.s, &mut f.r).unwrap()
    };
    assert!(matches!(
        n.query(),
        HistoryInspectionQueryV1::Availability {
            scope: "visible",
            group: Some("group"),
            ..
        }
    ));
    assert_eq!(n.profile(), "history/1");
    assert!(std::ptr::eq(n.context(), &f.id));
    let mut foreign_actor = f.s.clone();
    foreign_actor.policy = "other".into();
    let other = normalized(
        &Input::new(&f.id, "typed", "availability"),
        &foreign_actor,
        &mut f.r,
    )
    .unwrap();
    assert_ne!(n, other);
}
#[test]
fn exact_tuple_contract_fields_and_closed_scalar_meanings_refuse_without_repair() {
    let mut f = owner();
    for mode in 0..17 {
        let mut i = Input::new(&f.id, "typed", "availability");
        match mode {
            0 => i.route = "other",
            1 => i.selector = InertValueV1::Number(2.0),
            2 => i.selector = InertValueV1::Number(3.1),
            3 => i.version = InertValueV1::Number(2.0),
            4 => i.variant = "unknown",
            5 => i.operation = "frontier",
            6 => i.contract = "history-query/2",
            7 => i.extra = true,
            8 => i.duplicate = true,
            9 => i.header_extra = true,
            10 => i.context = Data(InertValueV1::Null),
            11 => i.scope = InertValueV1::Text(""),
            12 => i.profile = InertValueV1::Boolean(true),
            13 => i.direction = InertValueV1::Text("earlier"),
            14 => i.group = InertValueV1::Text(""),
            15 => i.group = InertValueV1::Number(f64::NAN),
            _ => {
                i.variant = "frontier";
                i.operation = "frontier";
                i.force_group = true;
            }
        }
        assert!(normalized(&i, &f.s, &mut f.r).is_err(), "mode {mode}");
        assert!(!f.o.is_faulted());
    }
    let mut i = Input::new(&f.id, "dynamic", "availability");
    i.selector = InertValueV1::Text("F04");
    assert!(normalized(&i, &f.s, &mut f.r).is_err());
}
#[test]
fn unsupported_profiles_and_foreign_actual_handles_normalize_but_do_not_poison_real_contexts() {
    let mut f = owner();
    let other = owner();
    let mut denied = facts(&f.s);
    denied.granted = Some(false);
    for mode in 0..2 {
        let mut i = Input::new(
            if mode == 0 { &f.id } else { &other.id },
            "typed",
            "availability",
        );
        if mode == 0 {
            i.profile = InertValueV1::Text("unsupported");
        }
        assert!(normalized(&i, &f.s, &mut f.r).is_ok());
        assert!(
            with_input(&i, |envelope| f.o.query_decoded_history_facade_v1(
                DecodedHistoryFacadeRequestV1 {
                    envelope,
                    actor: &f.s,
                    limits: limits(),
                    facts: &denied
                },
                &mut f.r
            ))
            .is_none()
        );
    }
    assert!(run(&mut f, "typed", "availability").is_some());
    assert_eq!(f.o.history_inspection_usage_v1().live, 1);
}
#[test]
fn malformed_requests_actor_mismatch_and_fact_scope_mismatch_do_not_lend_bound_loss() {
    for mode in 0..3 {
        let mut f = owner();
        let mut i = Input::new(&f.id, "dynamic", "availability");
        let mut actor = f.s.clone();
        let mut scope = f.s.clone();
        if mode == 0 {
            i.extra = true;
        } else if mode == 1 {
            actor.context.recipient = "other".into();
        } else {
            scope.policy = "other".into();
        }
        let mut denied = facts(&scope);
        denied.history_current = Some(false);
        assert!(
            with_input(&i, |envelope| f.o.query_decoded_history_facade_v1(
                DecodedHistoryFacadeRequestV1 {
                    envelope,
                    actor: &actor,
                    limits: limits(),
                    facts: &denied
                },
                &mut f.r
            ))
            .is_none()
        );
        assert!(run(&mut f, "typed", "availability").is_some());
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn native_empty_and_absorbing_loss_are_not_decoder_default_values() {
    let mut f = owner();
    let mut i = Input::new(&f.id, "typed", "availability");
    i.direction = InertValueV1::Text("redo");
    i.group = InertValueV1::Null;
    let current = facts(&f.s);
    let reply = with_input(&i, |envelope| {
        f.o.query_decoded_history_facade_v1(
            DecodedHistoryFacadeRequestV1 {
                envelope,
                actor: &f.s,
                limits: limits(),
                facts: &current,
            },
            &mut f.r,
        )
    });
    assert_eq!(
        reply,
        Some(HistoryInspectionReplyV1::Availability {
            context: f.id.clone(),
            value: HistoryInspectionAvailabilityV1::Empty
        })
    );
    let mut denied = facts(&f.s);
    denied.history_current = Some(false);
    let i = Input::new(&f.id, "dynamic", "availability");
    assert!(
        with_input(&i, |envelope| f.o.query_decoded_history_facade_v1(
            DecodedHistoryFacadeRequestV1 {
                envelope,
                actor: &f.s,
                limits: limits(),
                facts: &denied
            },
            &mut f.r
        ))
        .is_none()
    );
    assert!(run(&mut f, "typed", "availability").is_none());
    assert_eq!(f.o.history_inspection_usage_v1().live, 0);
}

#[test]
fn hidden_blocking_and_permitted_empty_detail_come_from_the_actual_native_owner() {
    let mut f = owner();
    let source = f.o.read(&f.b, &mut f.r).unwrap().version;
    let p =
        f.o.prepare(
            &[DocumentEdits {
                expected: source,
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "b".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.r,
        )
        .unwrap();
    f.o.commit(p, &mut f.r).unwrap();
    let projection = HistoryProjectionV1 {
        details: true,
        scopes: vec![HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(f.a.clone()),
            alias: "visible".into(),
            undo_complete: true,
            redo_complete: true,
        }],
        groups: vec![HistoryGroupProjectionV1 {
            group: f.g.clone(),
            alias: "group".into(),
            evidence_ready: true,
        }],
    };
    f.id =
        f.o.acquire_history_inspection_v1(&projection, &facts(&f.s), &mut f.r)
            .unwrap();
    for route in ["typed", "dynamic"] {
        assert_eq!(
            run(&mut f, route, "availability"),
            Some(HistoryInspectionReplyV1::Availability {
                context: f.id.clone(),
                value: HistoryInspectionAvailabilityV1::Blocked
            })
        );
        assert_eq!(
            run(&mut f, route, "permitted-blocking-detail"),
            Some(HistoryInspectionReplyV1::PermittedBlockingDetail {
                context: f.id.clone(),
                blockers: vec![]
            })
        );
    }
}
#[test]
fn all_independent_copy_frames_have_exact_byte_node_depth_and_tighter_caller_limits() {
    let mut f = owner();
    for variant in ["frontier", "availability", "permitted-blocking-detail"] {
        let i = Input::new(&f.id, "typed", variant);
        let n = normalized(&i, &f.s, &mut f.r).unwrap();
        let usages = n.capture_usage();
        let nodes = usages.iter().map(|u| u.nodes).max().unwrap();
        let depth = usages.iter().map(|u| u.depth).max().unwrap();
        let bytes = usages
            .iter()
            .map(|u| u.peak_bytes)
            .max()
            .unwrap()
            .max(normalization_template_bytes())
            .max(n.owned_bytes());
        for delta in [0, 1] {
            let l = CaptureLimitsV1 {
                max_nodes: nodes - delta,
                max_depth: depth,
                ..limits()
            };
            assert_eq!(
                with_input(&i, |e| normalize_history_envelope_v1(
                    e,
                    &f.s,
                    l,
                    l.max_bytes,
                    &mut f.r
                ))
                .is_ok(),
                delta == 0
            );
            let l = CaptureLimitsV1 {
                max_depth: depth - delta,
                max_nodes: nodes,
                ..limits()
            };
            assert_eq!(
                with_input(&i, |e| normalize_history_envelope_v1(
                    e,
                    &f.s,
                    l,
                    l.max_bytes,
                    &mut f.r
                ))
                .is_ok(),
                delta == 0
            );
            assert_eq!(
                with_input(&i, |e| normalize_history_envelope_v1(
                    e,
                    &f.s,
                    limits(),
                    bytes - delta,
                    &mut f.r
                ))
                .is_ok(),
                delta == 0
            );
        }
    }
}
#[test]
fn gateway_uses_one_actual_seven_frame_reservation_and_refunds_every_fault_stage() {
    for stage in [
        Fault::Classification,
        Fault::Workspace,
        Fault::BeforeNative,
        Fault::AfterNative,
        Fault::BadResult,
        Fault::RefundDrift,
    ] {
        for variant in ["frontier", "availability", "permitted-blocking-detail"] {
            let mut f = owner();
            let base = f.o.retention_usage().reserved;
            let before = f.o.history_inspection_usage_v1();
            f.o.source_facade_fault = Some(stage);
            assert!(run(&mut f, "dynamic", variant).is_none());
            assert!(f.o.is_faulted());
            assert_eq!(f.o.retention_usage().reserved, base);
            assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
            assert!(!f.o.read_facade_usage().busy);
            assert_eq!(f.o.history_inspection_usage_v1(), before);
            assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
        }
    }
}
#[test]
fn optional_saturated_capture_and_shared_reentry_do_not_change_original_query_meaning() {
    for capacity in [0, 1, 128] {
        let mut f = owner();
        f.r = if capacity == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(capacity).unwrap()
        };
        f.o.source_facade_fault = Some(Fault::Reentry);
        assert!(run(&mut f, "dynamic", "availability").is_some());
        assert!(!f.o.is_faulted());
        let mut stages = Vec::new();
        while let Some(e) = f.r.pop_front() {
            assert_eq!(e.service_time, None);
            stages.push(e.boundary);
        }
        if capacity == 128 {
            assert!(stages.contains(&Boundary::HistoryArgumentNormalization));
            assert!(stages.contains(&Boundary::HistoryFacadeRefund));
        }
        assert_eq!(f.o.history_inspection_usage_v1().issued_lifetime, 1);
    }
}
