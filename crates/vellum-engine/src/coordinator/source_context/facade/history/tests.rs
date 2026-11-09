use super::*;
#[test]
fn decoded_history_stop_preserves_original_context_binding_on_both_routes_under_the_fault_fence() {
    use ReadObservationStopEnvelopeValueV1::{Context, Data, Object};
    for dynamic in [false, true] {
        let mut f = owner();
        let other = owner();
        f.o.faulted = true;
        let c = HistoryInspectionControlV1 {
            retention_released: None,
            delivery_drained: Some(false),
            ..control(&f.s)
        };
        let native = ReadObservationStopPortV1::History {
            context: &f.id,
            control: &c,
        };
        for foreign in [true, false] {
            let target = if foreign { &other.id } else { &f.id };
            let args = [
                ("contract", Data(InertValueV1::Text("read-context/1"))),
                ("operation", Data(InertValueV1::Text("stop-observation"))),
                (
                    "context",
                    Context(ReadObservationStopTargetV1::History(target)),
                ),
            ];
            let header = [
                (
                    if dynamic { "name" } else { "family" },
                    Data(if dynamic {
                        InertValueV1::Text("control-observation")
                    } else {
                        InertValueV1::Number(18.0)
                    }),
                ),
                ("variant", Data(InertValueV1::Text("stop-observation"))),
                ("version", Data(InertValueV1::Number(1.0))),
                ("arguments", Object(&args)),
            ];
            let outer = [
                (
                    "route",
                    Data(InertValueV1::Text(if dynamic {
                        "dynamic"
                    } else {
                        "typed"
                    })),
                ),
                ("request", Object(&header)),
            ];
            assert_eq!(
                f.o.stop_decoded_read_observation_facade_v1(
                    DecodedReadObservationStopRequestV1 {
                        envelope: &Object(&outer),
                        native
                    },
                    &mut f.r
                ),
                if foreign {
                    Err(Error::Unavailable)
                } else {
                    Ok(())
                }
            );
            assert!(f.o.is_faulted());
        }
        assert_eq!(f.o.history_inspection_usage_v1().live, 0);
        assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
    }
}
fn scope() -> HistoryInspectionScopeV1 {
    HistoryInspectionScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "host".into(),
            engine: "engine".into(),
            recipient: "reader".into(),
            purpose: "history".into(),
            destination: "agent".into(),
            profile: "history/1".into(),
            continuity: "session".into(),
        },
        policy: "policy".into(),
    }
}
pub(in crate::coordinator) fn facts(
    s: &HistoryInspectionScopeV1,
) -> TrustedHistoryInspectionFactsV1<'_> {
    TrustedHistoryInspectionFactsV1 {
        scope: s,
        granted: Some(true),
        retained: Some(true),
        live: Some(true),
        continuous: Some(true),
        transfer_ready: Some(true),
        history_current: Some(true),
        capture_complete: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
        not_stopped: Some(true),
        reply_ready: Some(true),
    }
}
pub(in crate::coordinator) fn control(
    s: &HistoryInspectionScopeV1,
) -> HistoryInspectionControlV1<'_> {
    HistoryInspectionControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 512,
        max_depth: 16,
    }
}
fn pool_limits() -> HistoryInspectionLimitsV1 {
    HistoryInspectionLimitsV1 {
        max_contexts: 4,
        max_issued: 8,
        basis: HistoryBasisLimitsV1 {
            max_groups: 16,
            max_scopes: 16,
            max_participants: 8,
            max_basis_bytes: 16384,
        },
        max_details: 8,
        max_capture_bytes: 4096,
        max_reply_bytes: 4096,
        max_held_bytes: 500_000,
    }
}
pub(in crate::coordinator) struct Fixture {
    pub(in crate::coordinator) o: SourceCoordinator,
    pub(in crate::coordinator) r: Recorder,
    pub(in crate::coordinator) s: HistoryInspectionScopeV1,
    pub(in crate::coordinator) id: HistoryInspectionIdV1,
    pub(super) a: DocumentId,
    pub(super) b: DocumentId,
    pub(super) g: HistoryGroupId,
}
fn change(
    o: &mut SourceCoordinator,
    edits: &[(&DocumentId, &str, &str)],
    r: &mut Recorder,
) -> HistoryGroupId {
    let edits = edits
        .iter()
        .map(|(id, from, to)| DocumentEdits {
            expected: o.read(id, r).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: (*from).into(),
                inserted: (*to).into(),
            }],
        })
        .collect::<Vec<_>>();
    let p = o.prepare(&edits, &[], r).unwrap();
    o.commit(p, r).unwrap().history_group.unwrap()
}
fn owner_with(p: HistoryInspectionLimitsV1) -> Fixture {
    let mut o = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 16,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 2048,
            },
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(1_000_000, &mut r).unwrap();
    let s = scope();
    o.configure_history_inspection_v1(&s, p, Some(true), &mut r)
        .unwrap();
    o.set_settings_parent_limit_for_test(0); // not a history request-byte ceiling
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let g = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r);
    let projection = HistoryProjectionV1 {
        details: true,
        scopes: vec![HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(a.clone()),
            alias: "visible".into(),
            undo_complete: true,
            redo_complete: true,
        }],
        groups: vec![HistoryGroupProjectionV1 {
            group: g.clone(),
            alias: "group".into(),
            evidence_ready: true,
        }],
    };
    let id = o
        .acquire_history_inspection_v1(&projection, &facts(&s), &mut r)
        .unwrap();
    Fixture {
        o,
        r,
        s,
        id,
        a,
        b,
        g,
    }
}
pub(in crate::coordinator) fn owner() -> Fixture {
    owner_with(pool_limits())
}
#[test]
fn native_history_stop_facade_is_protected_metadata_control_not_a_read_or_retirement() {
    for faulted in [false, true] {
        let mut f = owner();
        let base = f.o.retention_usage().reserved;
        let generation = f.o.generation();
        let mut c = control(&f.s);
        c.owner_control_allowed = None;
        let port = ReadObservationStopPortV1::History {
            context: &f.id,
            control: &c,
        };
        assert_eq!(
            f.o.stop_read_observation_facade_v1(port, &mut f.r),
            Err(Error::Unavailable)
        );
        let other = owner();
        assert_eq!(
            f.o.stop_read_observation_facade_v1(
                ReadObservationStopPortV1::History {
                    context: &other.id,
                    control: &control(&f.s)
                },
                &mut f.r
            ),
            Err(Error::Unavailable)
        );
        f.o.faulted = faulted;
        f.o.source_facade_workspace = 7;
        c.owner_control_allowed = Some(true);
        c.retention_released = None;
        c.delivery_drained = Some(false);
        let pressure = f.o.retention_usage().maximum.unwrap() - base;
        let mut held = f.o.retention.reserve(pressure).unwrap();
        let before = f.o.history_inspection_usage_v1();
        for _ in 0..2 {
            assert_eq!(
                f.o.stop_read_observation_facade_v1(
                    ReadObservationStopPortV1::History {
                        context: &f.id,
                        control: &c
                    },
                    &mut f.r
                ),
                Ok(())
            );
        }
        let after = f.o.history_inspection_usage_v1();
        assert_eq!(after.contexts, before.contexts);
        assert_eq!(after.issued_lifetime, before.issued_lifetime);
        assert_eq!(after.live, before.live - 1);
        assert!(!f.o.retire_history_inspection_v1(&f.id, &c, &mut f.r));
        assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
        assert_eq!(f.o.is_faulted(), faulted);
        assert_eq!(f.o.generation(), generation);
        f.o.retention.release(&mut held).unwrap();
        assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
        assert_eq!(f.o.retention_usage().reserved, base);
        assert_eq!(f.o.is_faulted(), faulted);
    }
}
#[test]
fn native_history_stop_facade_late_fault_is_not_rollback_and_wrong_scope_never_selects_the_context()
{
    for fault in [Fault::BeforeNative, Fault::AfterNative] {
        let mut f = owner();
        let mut wrong = f.s.clone();
        wrong.context.purpose.push('x');
        assert_eq!(
            f.o.stop_read_observation_facade_v1(
                ReadObservationStopPortV1::History {
                    context: &f.id,
                    control: &control(&wrong)
                },
                &mut f.r
            ),
            Err(Error::Unavailable)
        );
        let before = f.o.history_inspection_usage_v1();
        f.r = Recorder::try_with_capacity(32).unwrap();
        f.o.source_facade_fault = Some(fault);
        assert_eq!(
            f.o.stop_read_observation_facade_v1(
                ReadObservationStopPortV1::History {
                    context: &f.id,
                    control: &control(&f.s)
                },
                &mut f.r
            ),
            Err(Error::Faulted)
        );
        assert!(f.o.is_faulted());
        let after = f.o.history_inspection_usage_v1();
        assert_eq!(after.contexts, before.contexts);
        assert_eq!(after.issued_lifetime, before.issued_lifetime);
        assert_eq!(
            after.live,
            before.live - usize::from(fault == Fault::AfterNative)
        );
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        assert_eq!(
            events.last().unwrap().boundary,
            Boundary::ReadObservationStopFacade
        );
        assert_eq!(events.last().unwrap().disposition, Disposition::Unknown);
        f.o.source_facade_fault = None;
        assert_eq!(
            f.o.stop_read_observation_facade_v1(
                ReadObservationStopPortV1::History {
                    context: &f.id,
                    control: &control(&f.s)
                },
                &mut f.r
            ),
            Ok(())
        );
        assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
        assert!(f.o.is_faulted());
    }
}
fn query(kind: usize) -> HistoryInspectionQueryV1<'static> {
    match kind {
        0 => HistoryInspectionQueryV1::Frontier {
            scope: "visible",
            direction: HistoryDirection::Undo,
        },
        1 => HistoryInspectionQueryV1::Availability {
            scope: "visible",
            direction: HistoryDirection::Undo,
            group: None,
        },
        _ => HistoryInspectionQueryV1::PermittedBlockingDetail {
            scope: "visible",
            direction: HistoryDirection::Undo,
            group: Some("group"),
        },
    }
}
fn invoke(f: &mut Fixture, kind: usize, l: CaptureLimitsV1) -> Option<HistoryInspectionReplyV1> {
    let facts = facts(&f.s);
    f.o.query_history_facade_v1(
        HistoryFacadeRequestV1 {
            context: &f.id,
            scope: &f.s,
            actor: &f.s,
            query: query(kind),
            limits: l,
            facts: &facts,
        },
        &mut f.r,
    )
}
#[test]
fn typed_history_facade_reports_budgets_separately_from_unavailable_current_facts() {
    for kind in 0..3 {
        let mut f = owner();
        f.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        crate::coordinator::release_measurements::tests::enable(&mut f.o, &mut f.r);
        let base = f.o.retention_usage().reserved;
        let current = facts(&f.s);
        let request = HistoryFacadeRequestV1 {
            context: &f.id,
            scope: &f.s,
            actor: &f.s,
            query: query(kind),
            limits: limits(),
            facts: &current,
        };
        assert!(matches!(
            f.o.try_query_history_facade_v1(
                HistoryFacadeRequestV1 {
                    limits: CaptureLimitsV1 {
                        max_bytes: 1,
                        ..limits()
                    },
                    ..request
                },
                &mut f.r
            ),
            Err(Error::Limit)
        ));
        let unknown = TrustedHistoryInspectionFactsV1 {
            granted: None,
            ..facts(&f.s)
        };
        assert!(matches!(
            f.o.try_query_history_facade_v1(
                HistoryFacadeRequestV1 {
                    facts: &unknown,
                    ..request
                },
                &mut f.r
            ),
            Err(Error::Unavailable)
        ));
        assert_eq!(
            f.o.try_query_history_facade_v1(request, &mut f.r),
            Ok(expected(&f, kind))
        );
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        assert!(
            events
                .iter()
                .any(|e| e.boundary == Boundary::HistoryFacadeRelease && e.service_time.is_some())
        );
        assert!(
            events
                .iter()
                .filter(|e| e.boundary != Boundary::HistoryFacadeRelease)
                .all(|e| e.service_time.is_none())
        );
        idle(&f, base);
        assert_eq!(
            crate::coordinator::release_measurements::tests::totals(&mut f.o, &mut f.r),
            [1, 0, 2, 0, 0, 0]
        );
    }
}
fn idle(f: &Fixture, reserved: usize) {
    assert_eq!(
        f.o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
    assert_eq!(f.o.retention_usage().reserved, reserved);
}
fn expected(f: &Fixture, kind: usize) -> HistoryInspectionReplyV1 {
    match kind {
        0 => HistoryInspectionReplyV1::Frontier {
            context: f.id.clone(),
            group: Some("group".into()),
        },
        1 => HistoryInspectionReplyV1::Availability {
            context: f.id.clone(),
            value: HistoryInspectionAvailabilityV1::Eligible,
        },
        _ => HistoryInspectionReplyV1::PermittedBlockingDetail {
            context: f.id.clone(),
            blockers: vec![],
        },
    }
}
#[test]
fn all_three_fixed_ports_preserve_original_handles_and_do_not_use_settings_or_capture_ceiling() {
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    assert!(limits().max_bytes > pool_limits().max_capture_bytes);
    assert_eq!(f.o.settings_job_parent_limit(), 0);
    for kind in 0..3 {
        assert_eq!(invoke(&mut f, kind, limits()), Some(expected(&f, kind)));
        idle(&f, base);
    }
    assert_eq!(f.o.history_inspection_usage_v1().issued_lifetime, 1);
}
#[test]
fn copied_false_and_unknown_facts_preserve_native_loss_meaning() {
    for i in 0..7 {
        for denied in [None, Some(false)] {
            let mut f = owner();
            let mut current = facts(&f.s);
            let fields = [
                &mut current.granted,
                &mut current.retained,
                &mut current.live,
                &mut current.continuous,
                &mut current.history_current,
                &mut current.transfer_ready,
                &mut current.reply_ready,
            ];
            *fields.into_iter().nth(i).unwrap() = denied;
            let base = f.o.retention_usage().reserved;
            assert!(
                f.o.query_history_facade_v1(
                    HistoryFacadeRequestV1 {
                        context: &f.id,
                        scope: &f.s,
                        actor: &f.s,
                        query: query(1),
                        limits: limits(),
                        facts: &current
                    },
                    &mut f.r
                )
                .is_none()
            );
            idle(&f, base);
            let next = invoke(&mut f, 1, limits());
            if i < 5 && denied == Some(false) {
                assert!(next.is_none());
                assert_eq!(f.o.history_inspection_usage_v1().live, 0);
            } else {
                assert_eq!(next, Some(expected(&f, 1)));
            }
            assert!(!f.o.is_faulted());
        }
    }
}
#[test]
fn wrong_eight_field_actor_fact_scope_and_unknown_alias_cannot_lend_loss() {
    for mode in 0..11 {
        let mut f = owner();
        let mut actor = f.s.clone();
        let mut fact_scope = f.s.clone();
        let mut q = query(1);
        let labels = [
            &mut actor.context.authority,
            &mut actor.context.engine,
            &mut actor.context.recipient,
            &mut actor.context.purpose,
            &mut actor.context.destination,
            &mut actor.context.profile,
            &mut actor.context.continuity,
            &mut actor.policy,
        ];
        if mode < 8 {
            *labels.into_iter().nth(mode).unwrap() = "other".into();
        } else if mode == 8 {
            fact_scope.policy = "other".into();
        } else if mode == 9 {
            q = HistoryInspectionQueryV1::Availability {
                scope: "unknown",
                direction: HistoryDirection::Undo,
                group: None,
            };
        } else {
            q = HistoryInspectionQueryV1::Availability {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: Some("unknown"),
            };
        }
        let mut current = facts(&fact_scope);
        current.granted = Some(false);
        let base = f.o.retention_usage().reserved;
        assert!(
            f.o.query_history_facade_v1(
                HistoryFacadeRequestV1 {
                    context: &f.id,
                    scope: &f.s,
                    actor: &actor,
                    query: q,
                    limits: limits(),
                    facts: &current
                },
                &mut f.r
            )
            .is_none()
        );
        idle(&f, base);
        assert_eq!(invoke(&mut f, 1, limits()), Some(expected(&f, 1)));
        assert_eq!(f.o.history_inspection_usage_v1().live, 1);
    }
}
#[test]
fn inert_byte_node_and_depth_bounds_refuse_without_poisoning() {
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    for l in [
        CaptureLimitsV1 {
            max_bytes: 0,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 22,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: 2,
            ..limits()
        },
        CaptureLimitsV1 {
            max_bytes: usize::MAX,
            ..limits()
        },
    ] {
        assert!(invoke(&mut f, 1, l).is_none());
        idle(&f, base);
        assert_eq!(f.o.history_inspection_usage_v1().live, 1);
        assert!(!f.o.is_faulted());
    }
    assert!(
        invoke(
            &mut f,
            1,
            CaptureLimitsV1 {
                max_nodes: 23,
                max_depth: 3,
                ..limits()
            }
        )
        .is_some()
    );
    // Find the independent native logical byte boundary across all copied frames.
    let (mut lo, mut hi) = (0, limits().max_bytes);
    while lo + 1 < hi {
        let mid = lo + (hi - lo) / 2;
        if invoke(
            &mut f,
            1,
            CaptureLimitsV1 {
                max_bytes: mid,
                ..limits()
            },
        )
        .is_some()
        {
            hi = mid
        } else {
            lo = mid
        }
    }
    assert!(
        invoke(
            &mut f,
            1,
            CaptureLimitsV1 {
                max_bytes: hi,
                ..limits()
            }
        )
        .is_some()
    );
    assert!(
        invoke(
            &mut f,
            1,
            CaptureLimitsV1 {
                max_bytes: hi - 1,
                ..limits()
            }
        )
        .is_none()
    );
    idle(&f, base);
}
#[test]
fn exact_owner_headroom_and_actual_parent_budget_are_both_required() {
    let f = owner();
    let base = f.o.history_inspection_usage_v1().reserved_pool_bytes;
    let total = size_of::<CaptureLimitsV1>() + 7 * limits().max_bytes;
    for (headroom, success) in [
        (total, true),
        (total - 1, false),
        (size_of::<CaptureLimitsV1>() - 1, false),
    ] {
        let mut f = owner_with(HistoryInspectionLimitsV1 {
            max_held_bytes: base + headroom,
            ..pool_limits()
        });
        let reserved = f.o.retention_usage().reserved;
        assert_eq!(invoke(&mut f, 1, limits()).is_some(), success);
        idle(&f, reserved);
        assert!(!f.o.is_faulted());
    }
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    let mut other = f.o.retention.reserve(1_000_000 - base - total + 1).unwrap();
    let held = f.o.retention_usage().reserved;
    assert!(invoke(&mut f, 1, limits()).is_none());
    idle(&f, held);
    assert_eq!(f.o.history_inspection_usage_v1().live, 1);
    f.o.retention.release(&mut other).unwrap();
    assert!(invoke(&mut f, 1, limits()).is_some());
    idle(&f, base);
}
#[test]
fn observed_staleness_survives_later_endpoint_equality_through_facade() {
    let mut f = owner();
    let before = f.o.history_inspection_usage_v1();
    let plan =
        f.o.prepare_reversal(&f.g, HistoryDirection::Undo, &mut f.r)
            .unwrap();
    f.o.commit(plan, &mut f.r).unwrap();
    assert!(invoke(&mut f, 1, limits()).is_none());
    let plan =
        f.o.prepare_reversal(&f.g, HistoryDirection::Redo, &mut f.r)
            .unwrap();
    f.o.commit(plan, &mut f.r).unwrap();
    assert!(invoke(&mut f, 1, limits()).is_none());
    let now = f.o.history_inspection_usage_v1();
    assert_eq!(now.contexts, before.contexts);
    assert_eq!(now.issued_lifetime, before.issued_lifetime);
    assert_eq!(now.live, 0);
}
#[test]
fn linked_hidden_work_invalidates_but_unrelated_work_does_not() {
    let mut f = owner();
    let c = f.o.create(b"C", &mut f.r).unwrap().document;
    change(&mut f.o, &[(&c, "C", "c")], &mut f.r);
    assert!(invoke(&mut f, 1, limits()).is_some());
    change(&mut f.o, &[(&f.b, "b", "hidden")], &mut f.r);
    assert!(invoke(&mut f, 1, limits()).is_none());
    assert_eq!(f.o.history_inspection_usage_v1().live, 0);
}
#[test]
fn all_fault_stages_fence_release_refund_only_temporary_charge_and_keep_cleanup() {
    for stage in [
        Fault::Classification,
        Fault::Workspace,
        Fault::BeforeNative,
        Fault::AfterNative,
        Fault::BadResult,
        Fault::RefundDrift,
    ] {
        for kind in 0..3 {
            let mut f = owner();
            let base = f.o.retention_usage().reserved;
            let before = f.o.history_inspection_usage_v1();
            f.o.source_facade_fault = Some(stage);
            assert!(invoke(&mut f, kind, limits()).is_none());
            assert!(f.o.is_faulted());
            idle(&f, base);
            assert_eq!(f.o.history_inspection_usage_v1(), before);
            if stage == Fault::BadResult {
                assert_eq!(
                    f.o.read_result_invariant(),
                    Some(ReadResultInvariantV1::NativeHistoryFacadeResultInvariant)
                );
            }
            assert!(f.o.stop_history_inspection_owner_v1(&control(&f.s), &mut f.r));
            assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
            assert!(f.o.retire_history_inspection_owner_v1(&control(&f.s), &mut f.r));
            assert!(f.o.is_faulted());
        }
    }
}
#[test]
fn reentry_stopped_owner_and_outstanding_unowned_workspace_are_not_admitted() {
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    f.o.source_facade_fault = Some(Fault::Reentry);
    assert!(invoke(&mut f, 1, limits()).is_some());
    idle(&f, base);
    assert!(!f.o.is_faulted());
    f.o.source_facade_fault = None;
    assert!(f.o.stop_history_inspection_owner_v1(&control(&f.s), &mut f.r));
    assert!(invoke(&mut f, 1, limits()).is_none());
    idle(&f, base);
    let mut f = owner();
    let base = f.o.retention_usage().reserved;
    f.o.source_facade_workspace = 7;
    assert!(invoke(&mut f, 1, limits()).is_none());
    assert!(f.o.is_faulted());
    assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
    assert_eq!(f.o.retention_usage().reserved, base);
}
#[test]
fn optional_saturated_events_neither_change_results_nor_leak_labels() {
    for capacity in [0, 1, 64] {
        let mut f = owner();
        f.r = if capacity == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(capacity).unwrap()
        };
        let base = f.o.retention_usage().reserved;
        assert_eq!(invoke(&mut f, 1, limits()), Some(expected(&f, 1)));
        idle(&f, base);
        let mut events = Vec::new();
        while let Some(e) = f.r.pop_front() {
            assert_eq!(e.service_time, None);
            events.push((e.boundary, e.disposition));
        }
        if capacity == 64 {
            for b in [
                Boundary::HistoryFacadeAdmission,
                Boundary::HistoryFacadeCapture,
                Boundary::HistoryFacadeNativeHandoff,
                Boundary::HistoryFacadeResultValidation,
                Boundary::HistoryFacadeRefund,
                Boundary::HistoryFacadeRelease,
            ] {
                assert!(events.contains(&(b, Disposition::Completed)));
            }
        }
        assert!(
            !format!(
                "{:?}",
                HistoryFacadeRequestV1 {
                    context: &f.id,
                    scope: &f.s,
                    actor: &f.s,
                    query: query(1),
                    limits: limits(),
                    facts: &facts(&f.s)
                }
            )
            .contains("reader")
        );
    }
}

#[test]
fn advice_through_adapter_is_not_write_permission_or_ownership_of_a_reversal_plan() {
    let mut f = owner();
    f.o.set_read_only(&f.a, true, &mut f.r).unwrap();
    assert_eq!(invoke(&mut f, 1, limits()), Some(expected(&f, 1)));
    assert_eq!(
        f.o.prepare_reversal(&f.g, HistoryDirection::Undo, &mut f.r)
            .unwrap_err(),
        HistoryError::Permission
    );
    f.o.set_read_only(&f.a, false, &mut f.r).unwrap();
    let plan =
        f.o.prepare_reversal(&f.g, HistoryDirection::Undo, &mut f.r)
            .unwrap();
    assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
    f.o.commit(plan, &mut f.r).unwrap();
    assert_eq!(f.o.read(&f.a, &mut f.r).unwrap().source.text(), "A");
    assert_eq!(f.o.history_usage().active_pins, 0);
}

#[test]
fn late_fault_preserves_observed_invalidation_instead_of_rolling_it_back() {
    for stage in [Fault::BeforeNative, Fault::AfterNative, Fault::RefundDrift] {
        let mut f = owner();
        change(&mut f.o, &[(&f.b, "b", "later")], &mut f.r);
        let before = f.o.history_inspection_usage_v1();
        let base = f.o.retention_usage().reserved;
        f.o.source_facade_fault = Some(stage);
        assert!(invoke(&mut f, 1, limits()).is_none());
        assert!(f.o.is_faulted());
        idle(&f, base);
        let now = f.o.history_inspection_usage_v1();
        assert_eq!(now.contexts, before.contexts);
        assert_eq!(now.issued_lifetime, before.issued_lifetime);
        assert_eq!(now.reserved_pool_bytes, before.reserved_pool_bytes);
        assert_eq!(now.live, usize::from(stage == Fault::BeforeNative));
        assert!(f.o.retire_history_inspection_v1(&f.id, &control(&f.s), &mut f.r));
    }
}

#[test]
fn history_and_source_share_the_same_reentry_fence() {
    let mut h = owner();
    let base = h.o.retention_usage().reserved;
    {
        let mut guard = Gateway::new(&mut h.o, &mut h.r);
        let current = facts(&h.s);
        assert!(
            guard
                .owner
                .query_history_facade_v1(
                    HistoryFacadeRequestV1 {
                        context: &h.id,
                        scope: &h.s,
                        actor: &h.s,
                        query: query(1),
                        limits: limits(),
                        facts: &current
                    },
                    guard.recorder
                )
                .is_none()
        );
        guard.finish(&Ok::<(), Error>(()));
    }
    idle(&h, base);
    assert!(invoke(&mut h, 1, limits()).is_some());
    assert!(!h.o.is_faulted());
    let mut f = crate::coordinator::source_query::result_contract::tests::owner();
    let base = f.o.retention_usage().reserved;
    {
        let mut guard = Gateway::for_history(&mut f.o, &mut f.r);
        let current = crate::coordinator::source_query::result_contract::tests::source_facts(&f.s);
        assert!(
            guard
                .owner
                .query_source_facade_v1(
                    SourceFacadeRequestV1 {
                        source: SearchReadContextV1::Source(&f.source),
                        scope: &f.s,
                        actor: &f.s.context,
                        query: SourceContextQueryV1::Line { index: 0 },
                        limits: limits(),
                        facts: SearchReadFactsV1::Source(&current)
                    },
                    guard.recorder
                )
                .is_none()
        );
        guard.finish(&Ok::<(), Error>(()));
    }
    assert_eq!(
        f.o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
    assert_eq!(f.o.retention_usage().reserved, base);
    assert!(!f.o.is_faulted());
}

#[test]
fn existing_native_fault_cause_is_never_replaced_by_facade_rejection() {
    let mut f = owner();
    let before = f.o.history_inspection_usage_v1();
    f.o.finish_read_result(
        false,
        ReadResultInvariantV1::NativeHistoryResultInvariant,
        Boundary::HistoryInspectionResultValidation,
        &mut f.r,
    );
    assert!(invoke(&mut f, 1, limits()).is_none());
    assert_eq!(
        f.o.read_result_invariant(),
        Some(ReadResultInvariantV1::NativeHistoryResultInvariant)
    );
    assert_eq!(f.o.history_inspection_usage_v1(), before);
}
