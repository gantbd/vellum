use super::*;
#[test]
fn shutdown_inventory_reports_actual_journal_and_pin_dependencies_not_only_inspection_contexts() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, a, b, group, _, _) = fixture();
    o.pin_history(&group, &mut r).unwrap();
    let usage = o.history_usage();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::History, &[&a, &b]);
    assert_eq!(
        row.retained_records,
        Some(usage.retained_groups + usage.active_pins)
    );
    assert_eq!(row.held_bytes, Some(usage.journal_bytes));
    assert_eq!(row.reserved_bytes, None);
    assert_eq!(o.history_usage(), usage);
}
#[test]
fn shutdown_inventory_keeps_captured_hidden_history_participant_dependencies() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, a, b, _, s, p) = fixture();
    o.acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::HistoryInspection,
        &[&a, &b],
    );
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
}
#[test]
fn document_read_pins_refuse_an_orphan_history_pin_instead_of_reporting_zero() {
    let (mut o, mut r, a, _, group, _, _) = fixture();
    o.pin_history(&group, &mut r).unwrap();
    o.history.groups.retain(|g| g.id != group);
    assert_eq!(
        o.document_read_pins_v1(&a, &mut r),
        Err(CoordinatorError::Faulted)
    );
}
#[test]
fn document_read_pins_count_captured_hidden_group_participants_until_inspection_retirement() {
    let (mut o, mut r, a, b, _, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    for doc in [&a, &b] {
        assert_eq!(
            o.document_read_pins_v1(doc, &mut r)
                .unwrap()
                .history_inspections,
            1
        );
    }
    assert!(o.stop_history_inspection_v1(&id, &control(&s), &mut r));
    assert_eq!(
        o.document_read_pins_v1(&b, &mut r)
            .unwrap()
            .history_inspections,
        1
    );
    assert!(o.retire_history_inspection_v1(&id, &control(&s), &mut r));
    for doc in [&a, &b] {
        assert_eq!(
            o.document_read_pins_v1(doc, &mut r)
                .unwrap()
                .history_inspections,
            0
        );
    }
}
fn scope() -> HistoryInspectionScopeV1 {
    HistoryInspectionScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "authority".into(),
            engine: "engine".into(),
            recipient: "recipient".into(),
            purpose: "purpose".into(),
            destination: "destination".into(),
            profile: "history/1".into(),
            continuity: "continuity".into(),
        },
        policy: "policy".into(),
    }
}
fn limits() -> HistoryInspectionLimitsV1 {
    HistoryInspectionLimitsV1 {
        max_contexts: 4,
        max_issued: 8,
        basis: HistoryBasisLimitsV1 {
            max_groups: 16,
            max_scopes: 16,
            max_participants: 8,
            max_basis_bytes: 32768,
        },
        max_details: 8,
        max_capture_bytes: 32768,
        max_reply_bytes: 4096,
        max_held_bytes: 1_000_000,
    }
}
fn facts(s: &HistoryInspectionScopeV1) -> TrustedHistoryInspectionFactsV1<'_> {
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
fn control(s: &HistoryInspectionScopeV1) -> HistoryInspectionControlV1<'_> {
    HistoryInspectionControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn owner() -> (SourceCoordinator, Recorder) {
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
    o.configure_retention_budget(2_000_000, &mut r).unwrap();
    (o, r)
}
fn change(
    o: &mut SourceCoordinator,
    edits: &[(&DocumentId, &str, &str)],
    r: &mut Recorder,
) -> HistoryGroupId {
    let writes = edits
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
    let p = o.prepare(&writes, &[], r).unwrap();
    o.commit(p, r).unwrap().history_group.unwrap()
}
fn reverse(o: &mut SourceCoordinator, g: &HistoryGroupId, d: HistoryDirection, r: &mut Recorder) {
    let p = o.prepare_reversal(g, d, r).unwrap();
    o.commit(p, r).unwrap();
}
fn projection(a: &DocumentId, g: &HistoryGroupId) -> HistoryProjectionV1 {
    HistoryProjectionV1 {
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
    }
}
fn fixture() -> (
    SourceCoordinator,
    Recorder,
    DocumentId,
    DocumentId,
    HistoryGroupId,
    HistoryInspectionScopeV1,
    HistoryProjectionV1,
) {
    let (mut o, mut r) = owner();
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let g = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r);
    let s = scope();
    o.configure_history_inspection_v1(&s, limits(), Some(true), &mut r)
        .unwrap();
    let p = projection(&a, &g);
    (o, r, a, b, g, s, p)
}
fn query(
    o: &mut SourceCoordinator,
    id: &HistoryInspectionIdV1,
    f: &TrustedHistoryInspectionFactsV1<'_>,
    r: &mut Recorder,
) -> HistoryInspectionReplyV1 {
    o.query_history_inspection_v1(
        id,
        HistoryInspectionQueryV1::Availability {
            scope: "visible",
            direction: HistoryDirection::Undo,
            group: None,
        },
        f,
        r,
    )
}
fn value(
    id: &HistoryInspectionIdV1,
    v: HistoryInspectionAvailabilityV1,
) -> HistoryInspectionReplyV1 {
    HistoryInspectionReplyV1::Availability {
        context: id.clone(),
        value: v,
    }
}
#[test]
fn scoped_hidden_change_is_absorbing_but_unrelated_publication_is_not() {
    let (mut o, mut r, _, b, _, s, p) = fixture();
    let f = facts(&s);
    let id = o.acquire_history_inspection_v1(&p, &f, &mut r).unwrap();
    let c = o.create(b"C", &mut r).unwrap().document;
    change(&mut o, &[(&c, "C", "c")], &mut r);
    assert_eq!(
        query(&mut o, &id, &f, &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Eligible)
    );
    let h = change(&mut o, &[(&b, "b", "hidden")], &mut r);
    assert_eq!(
        query(&mut o, &id, &f, &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    reverse(&mut o, &h, HistoryDirection::Undo, &mut r); // redo metadata still differs
    assert_eq!(
        query(&mut o, &id, &f, &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    assert_eq!(o.history_inspection_usage_v1().live, 0);
    assert_eq!(o.history_inspection_usage_v1().contexts, 1);
}
#[test]
fn hidden_blockers_affect_advice_but_never_leak_into_detail() {
    let (mut o, mut r, a, b, g, s, _) = fixture();
    let h = change(&mut o, &[(&b, "b", "later")], &mut r);
    let p = projection(&a, &g);
    let f = facts(&s);
    let id = o.acquire_history_inspection_v1(&p, &f, &mut r).unwrap();
    assert_eq!(
        query(&mut o, &id, &f, &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Blocked)
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::PermittedBlockingDetail {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: None
            },
            &f,
            &mut r
        ),
        HistoryInspectionReplyV1::PermittedBlockingDetail {
            context: id.clone(),
            blockers: vec![]
        }
    );
    let mut visible = projection(&a, &g);
    visible.scopes.push(HistoryScopeProjectionV1 {
        scope: HistoryBasisScopeV1::Source(b),
        alias: "blocker".into(),
        undo_complete: true,
        redo_complete: true,
    });
    visible.groups.push(HistoryGroupProjectionV1 {
        group: h,
        alias: "later".into(),
        evidence_ready: true,
    });
    let id = o
        .acquire_history_inspection_v1(&visible, &f, &mut r)
        .unwrap();
    let q = HistoryInspectionQueryV1::PermittedBlockingDetail {
        scope: "visible",
        direction: HistoryDirection::Undo,
        group: None,
    };
    assert_eq!(
        o.query_history_inspection_v1(&id, q, &f, &mut r),
        HistoryInspectionReplyV1::PermittedBlockingDetail {
            context: id,
            blockers: vec![HistoryBlockerV1 {
                scope: "blocker".into(),
                group: "later".into()
            }]
        }
    );
}
#[test]
fn all_acquisition_gates_are_required_and_do_not_spend_on_refusal() {
    let (mut o, mut r, _, _, _, s, p) = fixture();
    for i in 0..11 {
        for denied in [None, Some(false)] {
            let mut f = facts(&s);
            let fields = [
                &mut f.granted,
                &mut f.retained,
                &mut f.live,
                &mut f.continuous,
                &mut f.transfer_ready,
                &mut f.history_current,
                &mut f.capture_complete,
                &mut f.preparation_reserved,
                &mut f.registry_ready,
                &mut f.retention_ready,
                &mut f.not_stopped,
            ];
            *fields.into_iter().nth(i).unwrap() = denied;
            assert_eq!(
                o.acquire_history_inspection_v1(&p, &f, &mut r),
                Err(HistoryError::Unavailable)
            );
            assert_eq!(o.history_inspection_usage_v1().issued_lifetime, 0);
        }
    }
    let mut f = facts(&s);
    f.reply_ready = None; // not an acquisition gate
    assert!(o.acquire_history_inspection_v1(&p, &f, &mut r).is_ok());
}
#[test]
fn release_unknowns_withhold_while_explicit_bound_loss_invalidates() {
    for i in 0..7 {
        for denied in [None, Some(false)] {
            let (mut o, mut r, _, _, _, s, p) = fixture();
            let id = o
                .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
                .unwrap();
            let mut f = facts(&s);
            let fields = [
                &mut f.granted,
                &mut f.retained,
                &mut f.live,
                &mut f.continuous,
                &mut f.history_current,
                &mut f.transfer_ready,
                &mut f.reply_ready,
            ];
            *fields.into_iter().nth(i).unwrap() = denied;
            assert_eq!(
                query(&mut o, &id, &f, &mut r),
                HistoryInspectionReplyV1::Unavailable
            );
            let revived = query(&mut o, &id, &facts(&s), &mut r);
            if i < 5 && denied == Some(false) {
                assert_eq!(revived, HistoryInspectionReplyV1::Unavailable);
            } else {
                assert_eq!(
                    revived,
                    value(&id, HistoryInspectionAvailabilityV1::Eligible)
                );
            }
            assert!(!o.is_faulted());
        }
    }
}
#[test]
fn unknown_alias_foreign_handle_and_wrong_context_do_not_poison_entries() {
    let (mut o, mut r, _, _, _, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    let mut denied = facts(&s);
    denied.granted = Some(false);
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Availability {
                scope: "unknown",
                direction: HistoryDirection::Undo,
                group: None
            },
            &denied,
            &mut r
        ),
        HistoryInspectionReplyV1::Unavailable
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Availability {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: Some("unknown")
            },
            &denied,
            &mut r
        ),
        HistoryInspectionReplyV1::Unavailable
    );
    let mut other = s.clone();
    other.context.recipient = "other".into();
    assert_eq!(
        query(&mut o, &id, &facts(&other), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    let (mut foreign, mut fr, _, _, _, fs, fp) = fixture();
    let fid = foreign
        .acquire_history_inspection_v1(&fp, &facts(&fs), &mut fr)
        .unwrap();
    assert_eq!(
        query(&mut o, &fid, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Eligible)
    );
}
#[test]
fn empty_requires_retained_presence_complete_floor_and_no_lost_history() {
    let (mut o, mut r) = owner();
    let a = o.create(b"A", &mut r).unwrap().document;
    let s = scope();
    o.configure_history_inspection_v1(&s, limits(), Some(true), &mut r)
        .unwrap();
    let p = HistoryProjectionV1 {
        details: true,
        scopes: vec![HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(a.clone()),
            alias: "visible".into(),
            undo_complete: true,
            redo_complete: false,
        }],
        groups: vec![],
    };
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Empty)
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Frontier {
                scope: "visible",
                direction: HistoryDirection::Undo
            },
            &facts(&s),
            &mut r
        ),
        HistoryInspectionReplyV1::Frontier {
            context: id.clone(),
            group: None
        }
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Availability {
                scope: "visible",
                direction: HistoryDirection::Redo,
                group: None
            },
            &facts(&s),
            &mut r
        ),
        HistoryInspectionReplyV1::Unavailable
    );
    o.documents[a.slot].as_mut().unwrap().history.undo_lost = true;
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    o.documents[a.slot] = None;
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
}
#[test]
fn withheld_or_invalid_group_and_missing_evidence_are_not_empty() {
    for mode in 0..3 {
        let (mut o, mut r, _, _, g, s, mut p) = fixture();
        match mode {
            0 => p.groups.clear(),
            1 => p.groups[0].evidence_ready = false,
            _ => {
                o.history
                    .groups
                    .iter_mut()
                    .find(|row| row.id == g)
                    .unwrap()
                    .state = GroupState::Invalidated
            }
        }
        let id = o
            .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
            .unwrap();
        assert_eq!(
            query(&mut o, &id, &facts(&s), &mut r),
            HistoryInspectionReplyV1::Unavailable
        );
        assert_eq!(
            o.query_history_inspection_v1(
                &id,
                HistoryInspectionQueryV1::Frontier {
                    scope: "visible",
                    direction: HistoryDirection::Undo
                },
                &facts(&s),
                &mut r
            ),
            HistoryInspectionReplyV1::Unavailable
        );
    }
}
#[test]
fn detail_permission_count_and_reply_limits_refuse_whole_result_without_invalidation() {
    for mode in 0..3 {
        let (mut o, mut r, a, b, g, s, _) = fixture();
        let h = change(&mut o, &[(&b, "b", "later")], &mut r);
        let mut p = projection(&a, &g);
        p.scopes.push(HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(b),
            alias: "blocker".into(),
            undo_complete: true,
            redo_complete: true,
        });
        p.groups.push(HistoryGroupProjectionV1 {
            group: h,
            alias: "later".into(),
            evidence_ready: true,
        });
        let State::Active(pool) = &mut o.history_inspection else {
            panic!()
        };
        match mode {
            0 => p.details = false,
            1 => pool.limits.max_details = 0,
            _ => pool.limits.max_reply_bytes = size_of::<HistoryInspectionReplyV1>(),
        }
        let id = o
            .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
            .unwrap();
        assert_eq!(
            o.query_history_inspection_v1(
                &id,
                HistoryInspectionQueryV1::PermittedBlockingDetail {
                    scope: "visible",
                    direction: HistoryDirection::Undo,
                    group: None
                },
                &facts(&s),
                &mut r
            ),
            HistoryInspectionReplyV1::Unavailable
        );
        assert_eq!(
            query(&mut o, &id, &facts(&s), &mut r),
            value(&id, HistoryInspectionAvailabilityV1::Blocked)
        );
        assert_eq!(o.history_inspection_usage_v1().live, 1);
    }
}
#[test]
fn stop_retirement_and_pool_discharge_preserve_issuance_and_need_no_read_grant() {
    let (mut o, mut r, _, _, _, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    let mut c = control(&s);
    let reserved = o.retention_usage().reserved;
    c.delivery_drained = None;
    assert!(!o.retire_history_inspection_v1(&id, &c, &mut r));
    assert!(o.stop_history_inspection_v1(&id, &c, &mut r));
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    assert_eq!(o.retention_usage().reserved, reserved);
    c.delivery_drained = Some(true);
    assert!(o.retire_history_inspection_v1(&id, &c, &mut r));
    let next = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_ne!(id, next);
    assert!(!o.retire_history_inspection_owner_v1(&c, &mut r));
    assert!(o.stop_history_inspection_owner_v1(&c, &mut r));
    assert!(!o.retire_history_inspection_owner_v1(&c, &mut r));
    assert!(o.retire_history_inspection_v1(&next, &c, &mut r));
    assert!(o.retire_history_inspection_owner_v1(&c, &mut r));
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.history_inspection_usage_v1().issued_lifetime, 2);
    assert_eq!(
        o.configure_history_inspection_v1(&s, limits(), Some(true), &mut r),
        Err(HistoryError::Unavailable)
    );
}
#[test]
fn faulted_owner_cannot_deliver_but_protected_retirement_still_works() {
    let (mut o, mut r, _, _, _, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    o.faulted = true;
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    assert!(o.stop_history_inspection_owner_v1(&control(&s), &mut r));
    assert!(o.retire_history_inspection_v1(&id, &control(&s), &mut r));
    assert!(o.retire_history_inspection_owner_v1(&control(&s), &mut r));
    assert!(o.is_faulted());
}
#[test]
fn advice_neither_pins_inverse_history_nor_authorizes_or_cancels_reversal() {
    let (mut o, mut r, a, _, g, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    o.set_read_only(&a, true, &mut r).unwrap();
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Eligible)
    );
    assert_eq!(
        o.prepare_reversal(&g, HistoryDirection::Undo, &mut r)
            .unwrap_err(),
        HistoryError::Permission
    );
    o.set_read_only(&a, false, &mut r).unwrap();
    let plan = o
        .prepare_reversal(&g, HistoryDirection::Undo, &mut r)
        .unwrap();
    assert!(o.stop_history_inspection_v1(&id, &control(&s), &mut r));
    o.commit(plan, &mut r).unwrap();
    assert_eq!(o.history_usage().active_pins, 0);
    o.retire_history_group(&g, &mut r).unwrap();
}
#[test]
fn duplicates_limits_parent_pressure_and_lifetime_exhaustion_do_not_leak_capacity() {
    let (mut o, mut r, a, _, g, s, mut p) = fixture();
    p.scopes.push(HistoryScopeProjectionV1 {
        scope: HistoryBasisScopeV1::Source(a.clone()),
        alias: "duplicate".into(),
        undo_complete: true,
        redo_complete: true,
    });
    assert_eq!(
        o.acquire_history_inspection_v1(&p, &facts(&s), &mut r),
        Err(HistoryError::Unavailable)
    );
    let p = projection(&a, &g);
    let State::Active(pool) = &mut o.history_inspection else {
        panic!()
    };
    pool.limits.max_issued = 1;
    pool.limits.max_contexts = 1;
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.acquire_history_inspection_v1(&p, &facts(&s), &mut r),
        Err(HistoryError::Limit)
    );
    assert!(o.retire_history_inspection_v1(&id, &control(&s), &mut r));
    assert_eq!(
        o.acquire_history_inspection_v1(&p, &facts(&s), &mut r),
        Err(HistoryError::Limit)
    );
    assert_eq!(o.history_inspection_usage_v1().issued_lifetime, 1);
    assert!(
        HistoryInspectionLimitsV1 {
            max_contexts: usize::MAX,
            ..limits()
        }
        .required_pool_bytes()
        .is_err()
    );
    let (mut tiny, mut tr) = owner();
    let _other_pool = tiny.retention.reserve(1_999_999).unwrap();
    assert_eq!(
        tiny.configure_history_inspection_v1(&s, limits(), Some(true), &mut tr),
        Err(HistoryError::Limit)
    );
    assert_eq!(tiny.retention_usage().reserved, 1_999_999);
}
#[test]
fn diagnostics_are_optional_bounded_redacted_and_not_authority() {
    for capacity in [0, 1, 32] {
        let (mut o, _, _, b, _, s, p) = fixture();
        let mut r = if capacity == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(capacity).unwrap()
        };
        let id = o
            .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
            .unwrap();
        assert_eq!(
            query(&mut o, &id, &facts(&s), &mut r),
            value(&id, HistoryInspectionAvailabilityV1::Eligible)
        );
        change(&mut o, &[(&b, "b", "later")], &mut r);
        assert_eq!(
            query(&mut o, &id, &facts(&s), &mut r),
            HistoryInspectionReplyV1::Unavailable
        );
        assert!(!format!("{id:?} {s:?} {:?}", o.history_inspection).contains("recipient"));
        assert!(!o.is_faulted());
    }
}

#[test]
fn private_alias_bridge_revalidates_and_cannot_revive_or_authorize() {
    let (mut o, mut r, a, b, g, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.resolve_history_group_alias_v1(&id, "group", &facts(&s), &mut r),
        Some(g.clone())
    );
    assert_eq!(
        o.resolve_history_scope_alias_v1(&id, "visible", &facts(&s), &mut r),
        Some(HistoryBasisScopeV1::Source(a.clone()))
    );
    let mut f = facts(&s);
    f.granted = Some(false);
    assert_eq!(
        o.resolve_history_group_alias_v1(&id, "unknown", &f, &mut r),
        None
    );
    assert_eq!(o.history_inspection_usage_v1().live, 1);
    o.set_read_only(&a, true, &mut r).unwrap();
    let group = o
        .resolve_history_group_alias_v1(&id, "group", &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap_err(),
        HistoryError::Permission
    );
    change(&mut o, &[(&b, "b", "later")], &mut r);
    assert_eq!(
        o.resolve_history_scope_alias_v1(&id, "visible", &facts(&s), &mut r),
        None
    );
    assert_eq!(o.history_inspection_usage_v1().live, 0);
}

#[test]
fn observed_basis_loss_is_absorbing_after_both_stacks_return_to_exact_original() {
    let (mut o, mut r, a, _, g, s, p) = fixture();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    let basis = o
        .capture_history_basis_v1(
            std::slice::from_ref(&g),
            &[HistoryBasisScopeV1::Source(a)],
            limits().basis,
            &mut r,
        )
        .unwrap();
    reverse(&mut o, &g, HistoryDirection::Undo, &mut r);
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    reverse(&mut o, &g, HistoryDirection::Redo, &mut r);
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    assert_eq!(
        o.resolve_history_group_alias_v1(&id, "group", &facts(&s), &mut r),
        None
    );
    let fresh = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        query(&mut o, &fresh, &facts(&s), &mut r),
        value(&fresh, HistoryInspectionAvailabilityV1::Eligible)
    );
}

#[test]
fn telemetry_separates_validation_invalidation_refusal_and_protected_discharge() {
    let (mut o, _, _, _, _, s, p) = fixture();
    let mut r = Recorder::try_with_capacity(32).unwrap();
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    query(&mut o, &id, &facts(&s), &mut r);
    let mut denied = facts(&s);
    denied.granted = Some(false);
    query(&mut o, &id, &denied, &mut r);
    assert!(o.retire_history_inspection_v1(&id, &control(&s), &mut r));
    let mut events = Vec::new();
    while let Some(e) = r.pop_front() {
        assert_eq!(e.service_time, None);
        events.push((e.boundary, e.disposition));
    }
    for expected in [
        (
            Boundary::HistoryInspectionAcquisition,
            Disposition::Completed,
        ),
        (
            Boundary::HistoryInspectionValidation,
            Disposition::Completed,
        ),
        (Boundary::HistoryInspectionQuery, Disposition::Completed),
        (
            Boundary::HistoryInspectionInvalidation,
            Disposition::Completed,
        ),
        (Boundary::HistoryInspectionQuery, Disposition::Refused),
        (
            Boundary::HistoryInspectionRetirement,
            Disposition::Completed,
        ),
    ] {
        assert!(events.contains(&expected), "{expected:?}");
    }
}

#[test]
fn retained_group_choice_and_redo_have_distinct_frontier_meanings() {
    let (mut o, mut r, a, _, g, s, _) = fixture();
    let h = change(&mut o, &[(&a, "a", "later")], &mut r);
    let mut p = projection(&a, &g);
    p.groups.push(HistoryGroupProjectionV1 {
        group: h.clone(),
        alias: "head".into(),
        evidence_ready: true,
    });
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Eligible)
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Availability {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: Some("group")
            },
            &facts(&s),
            &mut r
        ),
        value(&id, HistoryInspectionAvailabilityV1::Blocked)
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::PermittedBlockingDetail {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: Some("group")
            },
            &facts(&s),
            &mut r
        ),
        HistoryInspectionReplyV1::PermittedBlockingDetail {
            context: id,
            blockers: vec![HistoryBlockerV1 {
                scope: "visible".into(),
                group: "head".into()
            }]
        }
    );
    reverse(&mut o, &h, HistoryDirection::Undo, &mut r);
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Frontier {
                scope: "visible",
                direction: HistoryDirection::Redo
            },
            &facts(&s),
            &mut r
        ),
        HistoryInspectionReplyV1::Frontier {
            context: id.clone(),
            group: Some("head".into())
        }
    );
    assert_eq!(
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Availability {
                scope: "visible",
                direction: HistoryDirection::Redo,
                group: None
            },
            &facts(&s),
            &mut r
        ),
        value(&id, HistoryInspectionAvailabilityV1::Eligible)
    );
}

#[test]
fn duplicate_aliases_foreign_targets_and_duplicate_groups_never_install() {
    for mode in 0..6 {
        let (mut o, mut r, a, b, g, s, _) = fixture();
        let h = change(&mut o, &[(&b, "b", "later")], &mut r);
        let mut p = projection(&a, &g);
        let (other, _, foreign, _, fg, _, _) = fixture(); // retain distinct actual namespace
        assert!(!other.is_faulted());
        match mode {
            0 => p.scopes.push(HistoryScopeProjectionV1 {
                scope: HistoryBasisScopeV1::Source(b),
                alias: "visible".into(),
                undo_complete: true,
                redo_complete: true,
            }),
            1 => p.groups.push(HistoryGroupProjectionV1 {
                group: h,
                alias: "group".into(),
                evidence_ready: true,
            }),
            2 => p.groups.push(HistoryGroupProjectionV1 {
                group: g,
                alias: "different".into(),
                evidence_ready: true,
            }),
            3 => p.scopes[0].scope = HistoryBasisScopeV1::Source(foreign),
            4 => p.groups[0].group = fg,
            _ => p.groups[0].alias.clear(),
        }
        assert_eq!(
            o.acquire_history_inspection_v1(&p, &facts(&s), &mut r),
            Err(HistoryError::Unavailable)
        );
        assert_eq!(o.history_inspection_usage_v1().issued_lifetime, 0);
        assert!(!o.is_faulted());
    }
}

#[test]
fn exact_pool_capture_basis_and_reply_native_charges_are_enforced() {
    let s = scope();
    let mut l = limits();
    let exact = l.required_pool_bytes().unwrap() + scope_bytes(&s).unwrap()
        - size_of::<HistoryInspectionScopeV1>();
    for delta in [0, 1] {
        let (mut o, mut r) = owner();
        l.max_held_bytes = exact - delta;
        let out = o.configure_history_inspection_v1(&s, l, Some(true), &mut r);
        if delta == 0 {
            assert_eq!(out, Ok(true));
            assert_eq!(o.retention_usage().reserved, exact);
        } else {
            assert_eq!(out, Err(HistoryError::Limit));
            assert_eq!(o.retention_usage().reserved, 0);
        }
    }
    for mode in 0..3 {
        for delta in [0, 1] {
            let (mut o, mut r, _, _, _, s, p) = fixture();
            let id = o
                .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
                .unwrap();
            let State::Active(pool) = &mut o.history_inspection else {
                panic!()
            };
            let basis_bytes = pool.rows[id.slot]
                .as_ref()
                .unwrap()
                .basis
                .usage()
                .logical_bytes;
            match mode {
                0 => {
                    pool.limits.max_capture_bytes = projection_bytes(&p).unwrap()
                        + size_of::<HistoryGroupId>()
                        + size_of::<HistoryBasisScopeV1>()
                        + size_of::<Vec<HistoryGroupId>>()
                        + size_of::<Vec<HistoryBasisScopeV1>>()
                        - delta
                }
                1 => pool.limits.basis.max_basis_bytes = basis_bytes - delta,
                _ => {
                    pool.limits.max_reply_bytes =
                        size_of::<HistoryInspectionReplyV1>() + "group".len() - delta
                }
            }
            if mode < 2 {
                let result = o.acquire_history_inspection_v1(&p, &facts(&s), &mut r);
                if delta == 0 {
                    assert!(result.is_ok());
                } else {
                    assert_eq!(result, Err(HistoryError::Limit));
                }
            } else {
                let reply = o.query_history_inspection_v1(
                    &id,
                    HistoryInspectionQueryV1::Frontier {
                        scope: "visible",
                        direction: HistoryDirection::Undo,
                    },
                    &facts(&s),
                    &mut r,
                );
                if delta == 0 {
                    assert_eq!(
                        reply,
                        HistoryInspectionReplyV1::Frontier {
                            context: id,
                            group: Some("group".into())
                        }
                    );
                } else {
                    assert_eq!(reply, HistoryInspectionReplyV1::Unavailable);
                    assert_eq!(o.history_inspection_usage_v1().live, 1);
                }
            }
            assert!(!o.is_faulted());
        }
    }
}

#[test]
fn projected_details_use_reference_utf16_order_and_independent_owned_values() {
    let (mut o, mut r) = owner();
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let c = o.create(b"C", &mut r).unwrap().document;
    let g = change(
        &mut o,
        &[(&a, "A", "a"), (&b, "B", "b"), (&c, "C", "c")],
        &mut r,
    );
    let h = change(&mut o, &[(&b, "b", "B")], &mut r);
    let i = change(&mut o, &[(&c, "c", "C")], &mut r);
    let s = scope();
    o.configure_history_inspection_v1(&s, limits(), Some(true), &mut r)
        .unwrap();
    let mut p = projection(&a, &g);
    for (doc, alias) in [(b, "\u{e000}"), (c, "\u{10000}")] {
        p.scopes.push(HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(doc),
            alias: alias.into(),
            undo_complete: true,
            redo_complete: true,
        });
    }
    for (group, alias) in [(h, "h"), (i, "i")] {
        p.groups.push(HistoryGroupProjectionV1 {
            group,
            alias: alias.into(),
            evidence_ready: true,
        });
    }
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    p.groups[0].alias = "tampered".into();
    p.scopes[0].alias = "tampered".into();
    let read = |o: &mut SourceCoordinator, r: &mut Recorder| {
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::PermittedBlockingDetail {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: Some("group"),
            },
            &facts(&s),
            r,
        )
    };
    let expected = vec![
        HistoryBlockerV1 {
            scope: "\u{10000}".into(),
            group: "i".into(),
        },
        HistoryBlockerV1 {
            scope: "\u{e000}".into(),
            group: "h".into(),
        },
    ];
    let mut reply = read(&mut o, &mut r);
    assert_eq!(
        reply,
        HistoryInspectionReplyV1::PermittedBlockingDetail {
            context: id.clone(),
            blockers: expected
        }
    );
    let HistoryInspectionReplyV1::PermittedBlockingDetail { blockers, .. } = &mut reply else {
        panic!()
    };
    blockers[0].scope = "tampered".into();
    assert_ne!(read(&mut o, &mut r), reply);
}

#[test]
fn observed_global_metadata_pressure_invalidates_but_registry_pressure_does_not() {
    let (mut o, mut r, _, _, _, s, p) = fixture();
    let State::Active(pool) = &mut o.history_inspection else {
        panic!()
    };
    pool.limits.basis.max_scopes = 2;
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    let State::Active(pool) = &mut o.history_inspection else {
        panic!()
    };
    pool.limits.max_contexts = 1;
    assert_eq!(
        o.acquire_history_inspection_v1(&p, &facts(&s), &mut r),
        Err(HistoryError::Limit)
    );
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        value(&id, HistoryInspectionAvailabilityV1::Eligible)
    );
    // Captured global metadata ceilings can refuse unrelated operational pressure.
    o.create(b"unrelated pressure", &mut r).unwrap();
    assert_eq!(
        query(&mut o, &id, &facts(&s), &mut r),
        HistoryInspectionReplyV1::Unavailable
    );
    assert_eq!(o.history_inspection_usage_v1().live, 0);
}

#[test]
fn malformed_native_products_fence_release_without_erasing_owned_obligations() {
    for fault in [
        ResultFault::Context,
        ResultFault::Variant,
        ResultFault::Alias,
        ResultFault::Oversized,
    ] {
        for capacity in [0, 1, 16] {
            let (mut o, _, _, _, _, s, p) = fixture();
            let mut r = if capacity == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(capacity).unwrap()
            };
            let id = o
                .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
                .unwrap();
            let before = o.history_inspection_usage_v1();
            let State::Active(pool) = &mut o.history_inspection else {
                panic!()
            };
            pool.result_fault = Some(fault);
            assert_eq!(
                o.query_history_inspection_v1(
                    &id,
                    HistoryInspectionQueryV1::Frontier {
                        scope: "visible",
                        direction: HistoryDirection::Undo
                    },
                    &facts(&s),
                    &mut r
                ),
                HistoryInspectionReplyV1::Unavailable
            );
            assert!(o.is_faulted());
            assert_eq!(
                o.read_result_invariant(),
                Some(ReadResultInvariantV1::NativeHistoryResultInvariant)
            );
            assert_eq!(
                o.read_result_invariant().unwrap().code(),
                "NATIVE_HISTORY_RESULT_INVARIANT"
            );
            assert_eq!(o.history_inspection_usage_v1(), before);
            assert_eq!(
                query(&mut o, &id, &facts(&s), &mut r),
                HistoryInspectionReplyV1::Unavailable
            );
            assert_eq!(
                o.resolve_history_group_alias_v1(&id, "group", &facts(&s), &mut r),
                None
            );
            assert_eq!(
                o.acquire_history_inspection_v1(&p, &facts(&s), &mut r),
                Err(HistoryError::Faulted)
            );
            assert!(o.stop_history_inspection_owner_v1(&control(&s), &mut r));
            assert!(o.retire_history_inspection_v1(&id, &control(&s), &mut r));
            assert!(o.retire_history_inspection_owner_v1(&control(&s), &mut r));
            assert!(o.is_faulted());
        }
    }
}

#[test]
fn result_guard_uses_original_operation_context_and_retained_alias_projection() {
    let (mut o, mut r, a, b, g, s, _) = fixture();
    let h = change(&mut o, &[(&b, "b", "later")], &mut r);
    let mut p = projection(&a, &g);
    p.scopes.push(HistoryScopeProjectionV1 {
        scope: HistoryBasisScopeV1::Source(b),
        alias: "blocker".into(),
        undo_complete: true,
        redo_complete: true,
    });
    p.groups.push(HistoryGroupProjectionV1 {
        group: h,
        alias: "head".into(),
        evidence_ready: true,
    });
    let id = o
        .acquire_history_inspection_v1(&p, &facts(&s), &mut r)
        .unwrap();
    let e = o.history_inspection_entry(&id, &s).unwrap();
    let q = HistoryInspectionQueryV1::PermittedBlockingDetail {
        scope: "visible",
        direction: HistoryDirection::Undo,
        group: None,
    };
    let out = |scope: &str, group: &str| HistoryInspectionReplyV1::PermittedBlockingDetail {
        context: id.clone(),
        blockers: vec![HistoryBlockerV1 {
            scope: scope.into(),
            group: group.into(),
        }],
    };
    assert!(valid_reply(&id, q, e, limits(), &out("blocker", "head")));
    assert!(!valid_reply(&id, q, e, limits(), &out("hidden", "head")));
    assert!(!valid_reply(&id, q, e, limits(), &out("blocker", "hidden")));
    assert!(!valid_reply(
        &id,
        q,
        e,
        HistoryInspectionLimitsV1 {
            max_details: 0,
            ..limits()
        },
        &out("blocker", "head")
    ));
    assert!(!valid_reply(
        &id,
        q,
        e,
        limits(),
        &value(&id, HistoryInspectionAvailabilityV1::Blocked)
    ));
    let duplicate = HistoryInspectionReplyV1::PermittedBlockingDetail {
        context: id.clone(),
        blockers: vec![
            HistoryBlockerV1 {
                scope: "blocker".into(),
                group: "head".into(),
            },
            HistoryBlockerV1 {
                scope: "blocker".into(),
                group: "head".into(),
            },
        ],
    };
    assert!(!valid_reply(&id, q, e, limits(), &duplicate));
    let wrong = HistoryInspectionIdV1 {
        owner: Arc::new(()),
        slot: id.slot,
    };
    assert!(!valid_reply(
        &wrong,
        q,
        e,
        limits(),
        &out("blocker", "head")
    ));
    // Valid projected aliases are not independently authenticated frontier truth.
}
