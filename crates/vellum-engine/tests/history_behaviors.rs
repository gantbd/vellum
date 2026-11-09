//! BDD for source-scoped linked frontier history, not UI or selective undo.
#[path = "history_behaviors/dependent_reversal.rs"]
mod dependent_reversal;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/inspection_read_fixture.rs"]
mod inspection_read;
#[path = "support/invocation_observation_fixture.rs"]
mod observation_support;
#[path = "history_behaviors/reversal_results.rs"]
mod reversal_results;
use vellum_engine::capture::CaptureLimitsV1;
use vellum_engine::capture::InertValueV1;
use vellum_engine::coordinator::{
    CommitOutcome, CoordinatorError, CoordinatorLimits, DocumentEdits, DocumentId,
    HistoryAvailability, HistoryBasisLimitsV1, HistoryBasisScopeV1, HistoryDirection, HistoryError,
    HistoryGroupId, HistoryLimits, SourceCoordinator,
};
use vellum_engine::coordinator::{DecodedHistoryFacadeRequestV1, HistoryEnvelopeValueV1};
use vellum_engine::coordinator::{HistoryFacadeRequestV1, HistoryInspectionIdV1};

fn decoded_history(
    o: &mut SourceCoordinator,
    id: &HistoryInspectionIdV1,
    route: &str,
    variant: &str,
    version: f64,
    current: &TrustedHistoryInspectionFactsV1<'_>,
    r: &mut Recorder,
) -> Option<HistoryInspectionReplyV1> {
    decoded_history_via(
        o,
        id,
        route,
        variant,
        version,
        current,
        r,
        |o, request, r| o.query_decoded_history_facade_v1(request, r),
    )
}
#[allow(clippy::too_many_arguments)]
fn decoded_history_via(
    o: &mut SourceCoordinator,
    id: &HistoryInspectionIdV1,
    route: &str,
    variant: &str,
    version: f64,
    current: &TrustedHistoryInspectionFactsV1<'_>,
    r: &mut Recorder,
    dispatch: impl FnOnce(
        &mut SourceCoordinator,
        DecodedHistoryFacadeRequestV1<'_, '_>,
        &mut Recorder,
    ) -> Option<HistoryInspectionReplyV1>,
) -> Option<HistoryInspectionReplyV1> {
    let s = current.scope;
    use HistoryEnvelopeValueV1::{Context, Data, Object};
    let arguments = [
        ("contract", Data(InertValueV1::Text("history-query/1"))),
        ("operation", Data(InertValueV1::Text(variant))),
        ("context", Context(id)),
        ("scope", Data(InertValueV1::Text("document"))),
        ("direction", Data(InertValueV1::Text("undo"))),
        ("profile", Data(InertValueV1::Text(&s.context.profile))),
        ("group", Data(InertValueV1::Null)),
    ];
    let request = [
        (
            if route == "typed" { "family" } else { "name" },
            Data(if route == "typed" {
                InertValueV1::Number(3.0)
            } else {
                InertValueV1::Text("inspect-history")
            }),
        ),
        ("variant", Data(InertValueV1::Text(variant))),
        ("version", Data(InertValueV1::Number(version))),
        (
            "arguments",
            Object(&arguments[..if variant == "frontier" { 6 } else { 7 }]),
        ),
    ];
    let envelope = [
        ("route", Data(InertValueV1::Text(route))),
        ("request", Object(&request)),
    ];
    dispatch(
        o,
        DecodedHistoryFacadeRequestV1 {
            envelope: &Object(&envelope),
            actor: s,
            limits: facade_limits(),
            facts: current,
        },
        r,
    )
}
#[test]
fn given_discovered_history_queries_when_released_then_original_context_and_filtered_products_survive_metadata_retirement()
 {
    for route in ["typed", "dynamic"] {
        let (mut o, mut r, s, id, _) = facade_fixture();
        let binding = inspection_read::Reader::install(&mut o, &s.context, true, &mut r);
        let current = inspection_facts(&s);
        let before = (
            o.retention_usage(),
            o.generation(),
            o.history_inspection_usage_v1(),
        );
        for (index, op) in ["frontier", "availability", "permitted-blocking-detail"]
            .into_iter()
            .enumerate()
        {
            let result = decoded_history_via(
                &mut o,
                &id,
                route,
                op,
                1.0,
                &current,
                &mut r,
                |o, request, r| binding.history(o, index, request, r),
            )
            .unwrap();
            match result {
                HistoryInspectionReplyV1::Frontier { context, .. }
                | HistoryInspectionReplyV1::Availability { context, .. }
                | HistoryInspectionReplyV1::PermittedBlockingDetail { context, .. } => {
                    assert_eq!(context, id)
                }
                _ => panic!("guarded history product"),
            }
        }
        assert_eq!(
            (
                o.retention_usage(),
                o.generation(),
                o.history_inspection_usage_v1()
            ),
            before
        );
        binding.retire(&mut o, &mut r);
        assert!(decoded_history(&mut o, &id, route, "frontier", 1.0, &current, &mut r).is_some());
        assert!(o.stop_history_inspection_v1(&id, &inspection_control(&s), &mut r));
        assert!(o.retire_history_inspection_v1(&id, &inspection_control(&s), &mut r));
    }
}

#[test]
fn given_typed_or_dynamic_history_arguments_when_dispatched_then_both_keep_the_original_context_and_native_meaning()
 {
    let (mut o, mut r, s, id, _) = facade_fixture();
    let current = inspection_facts(&s);
    let baseline = o.retention_usage().reserved;
    for variant in ["frontier", "availability", "permitted-blocking-detail"] {
        let typed = decoded_history(&mut o, &id, "typed", variant, 1.0, &current, &mut r);
        let dynamic = decoded_history(&mut o, &id, "dynamic", variant, 1.0, &current, &mut r);
        assert_eq!(typed, dynamic);
        assert!(typed.is_some());
        match typed.unwrap() {
            HistoryInspectionReplyV1::Frontier { context, .. }
            | HistoryInspectionReplyV1::Availability { context, .. }
            | HistoryInspectionReplyV1::PermittedBlockingDetail { context, .. } => {
                assert_eq!(context, id)
            }
            _ => panic!("native product"),
        }
        assert_eq!(o.retention_usage().reserved, baseline);
        assert_eq!(o.read_facade_usage().workspace_bytes, 0);
        assert!(!o.read_facade_usage().busy);
    }
}

#[test]
fn given_an_unsupported_history_envelope_when_current_loss_is_supplied_then_decoder_refusal_cannot_revoke_the_context()
 {
    let (mut o, mut r, s, id, _) = facade_fixture();
    let mut denied = inspection_facts(&s);
    denied.history_current = Some(false);
    assert!(
        decoded_history(&mut o, &id, "dynamic", "availability", 2.0, &denied, &mut r).is_none()
    );
    assert_eq!(o.history_inspection_usage_v1().live, 1);
    assert!(
        decoded_history(
            &mut o,
            &id,
            "typed",
            "availability",
            1.0,
            &inspection_facts(&s),
            &mut r
        )
        .is_some()
    );
    assert!(decoded_history(&mut o, &id, "typed", "availability", 1.0, &denied, &mut r).is_none());
    assert!(
        decoded_history(
            &mut o,
            &id,
            "dynamic",
            "availability",
            1.0,
            &inspection_facts(&s),
            &mut r
        )
        .is_none()
    );
    assert!(!o.is_faulted());
}
use vellum_engine::coordinator::{
    HistoryGroupProjectionV1, HistoryInspectionAvailabilityV1, HistoryInspectionControlV1,
    HistoryInspectionLimitsV1, HistoryInspectionQueryV1, HistoryInspectionReplyV1,
    HistoryInspectionScopeV1, HistoryProjectionV1, HistoryScopeProjectionV1,
    SettingsExecutionContextV1, TrustedHistoryInspectionFactsV1,
};
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::{Limits, Replacement};

fn facade_fixture() -> (
    SourceCoordinator,
    Recorder,
    HistoryInspectionScopeV1,
    HistoryInspectionIdV1,
    DocumentId,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let s = inspection_scope();
    configure_inspection(&mut o, &s, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let g = change(&mut o, &[(&d, "A", "a")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let p = HistoryProjectionV1 {
        details: true,
        scopes: vec![HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(d.clone()),
            alias: "document".into(),
            undo_complete: true,
            redo_complete: true,
        }],
        groups: vec![HistoryGroupProjectionV1 {
            group: g,
            alias: "edit".into(),
            evidence_ready: true,
        }],
    };
    let id = o
        .acquire_history_inspection_v1(&p, &inspection_facts(&s), &mut r)
        .unwrap();
    (o, r, s, id, d)
}
fn facade_limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 512,
        max_depth: 16,
    }
}

#[test]
fn given_native_history_advice_when_read_through_the_facade_then_all_three_products_keep_the_original_context()
 {
    let (mut o, mut r, s, id, d) = facade_fixture();
    let facts = inspection_facts(&s);
    let before = o.retention_usage().reserved;
    let queries = [
        HistoryInspectionQueryV1::Frontier {
            scope: "document",
            direction: HistoryDirection::Undo,
        },
        HistoryInspectionQueryV1::Availability {
            scope: "document",
            direction: HistoryDirection::Undo,
            group: None,
        },
        HistoryInspectionQueryV1::PermittedBlockingDetail {
            scope: "document",
            direction: HistoryDirection::Undo,
            group: Some("edit"),
        },
    ];
    let expected = [
        HistoryInspectionReplyV1::Frontier {
            context: id.clone(),
            group: Some("edit".into()),
        },
        HistoryInspectionReplyV1::Availability {
            context: id.clone(),
            value: HistoryInspectionAvailabilityV1::Eligible,
        },
        HistoryInspectionReplyV1::PermittedBlockingDetail {
            context: id.clone(),
            blockers: vec![],
        },
    ];
    for (query, expected) in queries.into_iter().zip(expected) {
        assert_eq!(
            o.query_history_facade_v1(
                HistoryFacadeRequestV1 {
                    context: &id,
                    scope: &s,
                    actor: &s,
                    query,
                    limits: facade_limits(),
                    facts: &facts
                },
                &mut r
            ),
            Some(expected)
        );
        assert!(!o.read_facade_usage().busy);
        assert_eq!(o.read_facade_usage().workspace_bytes, 0);
        assert_eq!(o.retention_usage().reserved, before);
    }
    change(&mut o, &[(&d, "a", "later")], &mut r).unwrap();
    assert!(
        o.query_history_facade_v1(
            HistoryFacadeRequestV1 {
                context: &id,
                scope: &s,
                actor: &s,
                query: queries[1],
                limits: facade_limits(),
                facts: &facts
            },
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.history_inspection_usage_v1().live, 0);
    assert!(o.retire_history_inspection_v1(&id, &inspection_control(&s), &mut r));
    assert!(!o.is_faulted());
}

#[test]
fn given_foreign_actor_denial_when_facade_rejects_it_then_the_owned_history_context_is_not_revoked()
{
    let (mut o, mut r, s, id, _) = facade_fixture();
    let mut actor = s.clone();
    actor.policy = "foreign policy".into();
    let mut denied = inspection_facts(&s);
    denied.granted = Some(false);
    let query = HistoryInspectionQueryV1::Availability {
        scope: "document",
        direction: HistoryDirection::Undo,
        group: None,
    };
    assert!(
        o.query_history_facade_v1(
            HistoryFacadeRequestV1 {
                context: &id,
                scope: &s,
                actor: &actor,
                query,
                limits: facade_limits(),
                facts: &denied
            },
            &mut r
        )
        .is_none()
    );
    let current = inspection_facts(&s);
    assert_eq!(
        o.query_history_facade_v1(
            HistoryFacadeRequestV1 {
                context: &id,
                scope: &s,
                actor: &s,
                query,
                limits: facade_limits(),
                facts: &current
            },
            &mut r
        ),
        Some(HistoryInspectionReplyV1::Availability {
            context: id,
            value: HistoryInspectionAvailabilityV1::Eligible
        })
    );
    assert_eq!(o.history_inspection_usage_v1().issued_lifetime, 1);
    assert!(!o.is_faulted());
}

fn inspection_scope() -> HistoryInspectionScopeV1 {
    HistoryInspectionScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "host".into(),
            engine: "engine".into(),
            recipient: "reader".into(),
            purpose: "history advice".into(),
            destination: "agent".into(),
            profile: "history/1".into(),
            continuity: "session".into(),
        },
        policy: "policy".into(),
    }
}
fn inspection_facts(s: &HistoryInspectionScopeV1) -> TrustedHistoryInspectionFactsV1<'_> {
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
fn inspection_control(s: &HistoryInspectionScopeV1) -> HistoryInspectionControlV1<'_> {
    HistoryInspectionControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn configure_inspection(o: &mut SourceCoordinator, s: &HistoryInspectionScopeV1, r: &mut Recorder) {
    o.configure_retention_budget(1_000_000, r).unwrap();
    o.configure_history_inspection_v1(
        s,
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
            max_capture_bytes: 16384,
            max_reply_bytes: 4096,
            max_held_bytes: 500_000,
        },
        Some(true),
        r,
    )
    .unwrap();
}

#[test]
fn given_owned_history_advice_when_hidden_work_is_observed_then_later_undo_does_not_revive_it() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let s = inspection_scope();
    configure_inspection(&mut o, &s, &mut r);
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let g = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let p = HistoryProjectionV1 {
        details: true,
        scopes: vec![HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(a),
            alias: "visible".into(),
            undo_complete: true,
            redo_complete: true,
        }],
        groups: vec![HistoryGroupProjectionV1 {
            group: g,
            alias: "linked".into(),
            evidence_ready: true,
        }],
    };
    let f = inspection_facts(&s);
    let id = o.acquire_history_inspection_v1(&p, &f, &mut r).unwrap();
    let query = |o: &mut SourceCoordinator, r: &mut Recorder| {
        o.query_history_inspection_v1(
            &id,
            HistoryInspectionQueryV1::Availability {
                scope: "visible",
                direction: HistoryDirection::Undo,
                group: None,
            },
            &f,
            r,
        )
    };
    assert_eq!(
        query(&mut o, &mut r),
        HistoryInspectionReplyV1::Availability {
            context: id.clone(),
            value: HistoryInspectionAvailabilityV1::Eligible
        }
    );
    let later = change(&mut o, &[(&b, "b", "later")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    assert_eq!(query(&mut o, &mut r), HistoryInspectionReplyV1::Unavailable);
    reverse(&mut o, &later, HistoryDirection::Undo, &mut r);
    assert_eq!(query(&mut o, &mut r), HistoryInspectionReplyV1::Unavailable);
    assert_eq!(o.history_inspection_usage_v1().live, 0);
    assert_eq!(o.history_inspection_usage_v1().contexts, 1);
    assert!(o.retire_history_inspection_v1(&id, &inspection_control(&s), &mut r));
    assert_eq!(o.history_inspection_usage_v1().issued_lifetime, 1);
}

#[test]
fn given_a_prepared_reversal_when_inspection_is_retired_then_the_independent_plan_still_executes() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let s = inspection_scope();
    configure_inspection(&mut o, &s, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let g = change(&mut o, &[(&d, "A", "a")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let p = HistoryProjectionV1 {
        details: false,
        scopes: vec![HistoryScopeProjectionV1 {
            scope: HistoryBasisScopeV1::Source(d.clone()),
            alias: "document".into(),
            undo_complete: true,
            redo_complete: true,
        }],
        groups: vec![HistoryGroupProjectionV1 {
            group: g.clone(),
            alias: "edit".into(),
            evidence_ready: true,
        }],
    };
    let id = o
        .acquire_history_inspection_v1(&p, &inspection_facts(&s), &mut r)
        .unwrap();
    let resolved = o
        .resolve_history_group_alias_v1(&id, "edit", &inspection_facts(&s), &mut r)
        .unwrap();
    assert_eq!(resolved, g);
    let plan = o
        .prepare_reversal(&resolved, HistoryDirection::Undo, &mut r)
        .unwrap();
    assert!(o.stop_history_inspection_v1(&id, &inspection_control(&s), &mut r));
    assert!(o.retire_history_inspection_v1(&id, &inspection_control(&s), &mut r));
    o.commit(plan, &mut r).unwrap();
    assert_eq!(o.read(&d, &mut r).unwrap().source.text(), "A");
    assert_eq!(o.history_usage().active_pins, 0);
    assert!(!o.is_faulted());
}

fn owner_with(history: HistoryLimits) -> SourceCoordinator {
    SourceCoordinator::with_history_limits(
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
        history,
    )
}

fn owner() -> SourceCoordinator {
    owner_with(HistoryLimits {
        max_retained_groups: 16,
        max_journal_bytes: 65536,
        max_pins: 8,
    })
}

fn change(
    owner: &mut SourceCoordinator,
    edits: &[(&DocumentId, &str, &str)],
    telemetry: &mut Recorder,
) -> Result<CommitOutcome, CoordinatorError> {
    let writes = edits
        .iter()
        .map(|(document, from, to)| DocumentEdits {
            expected: owner.read(document, telemetry).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: (*from).into(),
                inserted: (*to).into(),
            }],
        })
        .collect::<Vec<_>>();
    let prepared = owner.prepare(&writes, &[], telemetry)?;
    owner.commit(prepared, telemetry)
}

fn reverse(
    owner: &mut SourceCoordinator,
    group: &HistoryGroupId,
    direction: HistoryDirection,
    telemetry: &mut Recorder,
) -> CommitOutcome {
    let prepared = owner.prepare_reversal(group, direction, telemetry).unwrap();
    owner.commit(prepared, telemetry).unwrap()
}
#[test]
fn given_a_linked_group_when_hidden_participant_history_changes_then_its_scoped_basis_is_stale() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let group = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let basis = o
        .capture_history_basis_v1(
            &[group],
            &[HistoryBasisScopeV1::Source(a)],
            HistoryBasisLimitsV1 {
                max_groups: 16,
                max_scopes: 16,
                max_participants: 8,
                max_basis_bytes: 32768,
            },
            &mut r,
        )
        .unwrap();
    let c = o.create(b"C", &mut r).unwrap().document;
    change(&mut o, &[(&c, "C", "unrelated")], &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
    change(&mut o, &[(&b, "b", "hidden later work")], &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
    assert!(!o.is_faulted());
}
#[test]
fn given_current_history_metadata_when_reversal_is_denied_or_history_retired_then_basis_is_not_authority_or_a_pin()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let d = o.create(b"A", &mut r).unwrap().document;
    let group = change(&mut o, &[(&d, "A", "a")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let basis = o
        .capture_history_basis_v1(
            std::slice::from_ref(&group),
            &[HistoryBasisScopeV1::Source(d.clone())],
            HistoryBasisLimitsV1 {
                max_groups: 16,
                max_scopes: 16,
                max_participants: 8,
                max_basis_bytes: 32768,
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(o.history_usage().active_pins, 0);
    o.set_read_only(&d, true, &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
    assert_eq!(
        o.prepare_reversal(&group, HistoryDirection::Undo, &mut r)
            .unwrap_err(),
        HistoryError::Permission
    );
    o.retire_history_group(&group, &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
    assert!(!o.is_faulted());
}

#[test]
fn given_a_linked_edit_when_undone_and_redone_then_content_and_frontiers_move_together() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let b = owner.create(b"B", &mut telemetry).unwrap().document;
    let original = change(
        &mut owner,
        &[(&a, "A", "a"), (&b, "B", "b")],
        &mut telemetry,
    )
    .unwrap();
    let group = original.history_group.clone().unwrap();
    let undone = reverse(&mut owner, &group, HistoryDirection::Undo, &mut telemetry);
    assert_ne!(undone.operation, original.operation);
    assert_eq!(undone.history_group, Some(group.clone()));
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "A");
    assert_eq!(owner.read(&b, &mut telemetry).unwrap().source.text(), "B");
    assert!(
        undone
            .changes
            .iter()
            .all(|c| c.after.revision == c.before.revision + 1)
    );
    let redone = reverse(&mut owner, &group, HistoryDirection::Redo, &mut telemetry);
    assert_ne!(redone.operation, undone.operation);
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "a");
    assert_eq!(owner.read(&b, &mut telemetry).unwrap().source.text(), "b");
    assert_eq!(owner.history_usage().retained_groups, 1);
}

#[test]
fn given_a_later_edit_in_one_participant_then_linked_undo_blocks_until_it_is_explicitly_undone() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let b = owner.create(b"B", &mut telemetry).unwrap().document;
    let g = change(
        &mut owner,
        &[(&a, "A", "a"), (&b, "B", "b")],
        &mut telemetry,
    )
    .unwrap()
    .history_group
    .unwrap();
    let h = change(&mut owner, &[(&b, "b", "later")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let before = owner.generation();
    assert_eq!(
        owner.history_status(&a, HistoryDirection::Undo, &mut telemetry),
        Ok(HistoryAvailability::Blocked(g.clone()))
    );
    assert_eq!(
        owner
            .prepare_reversal(&g, HistoryDirection::Undo, &mut telemetry)
            .err(),
        Some(HistoryError::Blocked)
    );
    assert_eq!(owner.generation(), before);
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "a");
    reverse(&mut owner, &h, HistoryDirection::Undo, &mut telemetry);
    assert_eq!(
        owner.history_status(&a, HistoryDirection::Undo, &mut telemetry),
        Ok(HistoryAvailability::Eligible(g.clone()))
    );
    reverse(&mut owner, &g, HistoryDirection::Undo, &mut telemetry);
    assert_eq!(owner.read(&b, &mut telemetry).unwrap().source.text(), "B");
}

#[test]
fn given_unrelated_work_after_reversal_preparation_then_it_does_not_block_the_original_group() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let c = owner.create(b"C", &mut telemetry).unwrap().document;
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let undo = owner
        .prepare_reversal(&g, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    change(&mut owner, &[(&c, "C", "c")], &mut telemetry).unwrap();
    owner.commit(undo, &mut telemetry).unwrap();
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "A");
    assert_eq!(owner.read(&c, &mut telemetry).unwrap().source.text(), "c");
}

#[test]
fn given_a_policy_change_after_reversal_preparation_then_no_participant_or_frontier_moves() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let b = owner.create(b"B", &mut telemetry).unwrap().document;
    let g = change(
        &mut owner,
        &[(&a, "A", "a"), (&b, "B", "b")],
        &mut telemetry,
    )
    .unwrap()
    .history_group
    .unwrap();
    let undo = owner
        .prepare_reversal(&g, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    owner.set_read_only(&b, true, &mut telemetry).unwrap();
    let before = owner.generation();
    assert_eq!(
        owner.commit(undo, &mut telemetry),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(owner.generation(), before);
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "a");
    assert_eq!(
        owner.history_status(&a, HistoryDirection::Undo, &mut telemetry),
        Ok(HistoryAvailability::Eligible(g))
    );
}

#[test]
fn given_history_capacity_is_full_then_new_work_refuses_but_undo_redo_and_noop_still_work() {
    let mut owner = owner_with(HistoryLimits {
        max_retained_groups: 1,
        max_journal_bytes: 65536,
        max_pins: 1,
    });
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let before = owner.generation();
    assert_eq!(
        change(&mut owner, &[(&a, "a", "b")], &mut telemetry),
        Err(CoordinatorError::History(HistoryError::Limit))
    );
    let noop = change(&mut owner, &[(&a, "a", "a")], &mut telemetry).unwrap();
    assert!(noop.operation.is_none());
    assert!(noop.history_group.is_none());
    assert_eq!(owner.generation(), before);
    reverse(&mut owner, &g, HistoryDirection::Undo, &mut telemetry);
    reverse(&mut owner, &g, HistoryDirection::Redo, &mut telemetry);
    assert_eq!(owner.history_usage().retained_groups, 1);
}

#[test]
fn given_journal_byte_pressure_then_source_is_not_published_without_history() {
    let mut owner = owner_with(HistoryLimits {
        max_retained_groups: 8,
        max_journal_bytes: 1,
        max_pins: 1,
    });
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let before = owner.generation();
    assert_eq!(
        change(&mut owner, &[(&a, "A", "a")], &mut telemetry),
        Err(CoordinatorError::History(HistoryError::Limit))
    );
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "A");
    assert_eq!(owner.generation(), before);
    assert_eq!(owner.history_usage().retained_groups, 0);
}

#[test]
fn given_new_work_after_undo_then_linked_and_dependent_redo_are_invalidated_but_unrelated_redo_survives()
 {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let b = owner.create(b"B", &mut telemetry).unwrap().document;
    let c = owner.create(b"C", &mut telemetry).unwrap().document;
    let d = owner.create(b"D", &mut telemetry).unwrap().document;
    let g = change(
        &mut owner,
        &[(&a, "A", "a"), (&b, "B", "b")],
        &mut telemetry,
    )
    .unwrap()
    .history_group
    .unwrap();
    let r = change(
        &mut owner,
        &[(&b, "b", "later"), (&c, "C", "c")],
        &mut telemetry,
    )
    .unwrap()
    .history_group
    .unwrap();
    let independent = change(&mut owner, &[(&d, "D", "d")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let pin = owner.pin_history(&g, &mut telemetry).unwrap();
    reverse(&mut owner, &r, HistoryDirection::Undo, &mut telemetry);
    reverse(&mut owner, &g, HistoryDirection::Undo, &mut telemetry);
    reverse(
        &mut owner,
        &independent,
        HistoryDirection::Undo,
        &mut telemetry,
    );
    change(&mut owner, &[(&a, "A", "new")], &mut telemetry).unwrap();
    for group in [&g, &r] {
        assert_eq!(
            owner
                .prepare_reversal(group, HistoryDirection::Redo, &mut telemetry)
                .err(),
            Some(HistoryError::Unavailable)
        );
    }
    assert_eq!(
        owner.retire_history_group(&g, &mut telemetry),
        Err(HistoryError::Pinned)
    );
    owner.release_history_pin(&pin, &mut telemetry).unwrap();
    reverse(
        &mut owner,
        &independent,
        HistoryDirection::Redo,
        &mut telemetry,
    );
    assert_eq!(owner.read(&b, &mut telemetry).unwrap().source.text(), "B");
    assert_eq!(owner.read(&c, &mut telemetry).unwrap().source.text(), "C");
    assert_eq!(owner.read(&d, &mut telemetry).unwrap().source.text(), "d");
}

#[test]
fn given_pinned_closure_member_then_history_retirement_refuses_as_one_unit() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let b = owner.create(b"B", &mut telemetry).unwrap().document;
    let g = change(
        &mut owner,
        &[(&a, "A", "a"), (&b, "B", "b")],
        &mut telemetry,
    )
    .unwrap()
    .history_group
    .unwrap();
    let h = change(&mut owner, &[(&b, "b", "later")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let pin = owner.pin_history(&g, &mut telemetry).unwrap();
    let before = owner.generation();
    assert_eq!(
        owner.retire_history_group(&h, &mut telemetry),
        Err(HistoryError::Pinned)
    );
    assert_eq!(owner.generation(), before);
    assert_eq!(owner.history_usage().retained_groups, 2);
    owner.release_history_pin(&pin, &mut telemetry).unwrap();
    assert_eq!(owner.retire_history_group(&h, &mut telemetry), Ok(2));
    assert_eq!(
        owner.history_status(&a, HistoryDirection::Undo, &mut telemetry),
        Ok(HistoryAvailability::Unavailable)
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.read(&b, &mut telemetry).unwrap().source.text(),
        "later"
    );
}

#[test]
fn given_retained_history_then_disposal_requires_explicit_safe_retirement() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    assert_eq!(
        owner.history_status(&a, HistoryDirection::Undo, &mut telemetry),
        Ok(HistoryAvailability::Empty)
    );
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    assert_eq!(
        owner.dispose(&a, &mut telemetry),
        Err(CoordinatorError::History(HistoryError::Retained))
    );
    owner.retire_history_group(&g, &mut telemetry).unwrap();
    owner.dispose(&a, &mut telemetry).unwrap();
    let new = owner.create(b"a", &mut telemetry).unwrap().document;
    assert_ne!(a, new);
    assert_eq!(
        owner
            .prepare_reversal(&g, HistoryDirection::Undo, &mut telemetry)
            .err(),
        Some(HistoryError::Unavailable)
    );
}

#[test]
fn given_adjacent_deletions_then_ordered_inverse_insertions_restore_unicode_and_bom_losslessly() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let bytes = "\u{feff}aü終\r\n".as_bytes();
    let version = owner.create(bytes, &mut telemetry).unwrap();
    let edits = vec![
        Replacement {
            start: 0,
            end: 1,
            expected: "a".into(),
            inserted: "".into(),
        },
        Replacement {
            start: 1,
            end: 3,
            expected: "ü".into(),
            inserted: "".into(),
        },
    ];
    let plan = owner
        .prepare(
            &[DocumentEdits {
                expected: version.clone(),
                edits,
            }],
            &[],
            &mut telemetry,
        )
        .unwrap();
    let g = owner
        .commit(plan, &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    assert_eq!(
        owner
            .read(&version.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "終\r\n"
    );
    reverse(&mut owner, &g, HistoryDirection::Undo, &mut telemetry);
    assert_eq!(
        owner
            .read(&version.document, &mut telemetry)
            .unwrap()
            .source
            .encode()
            .unwrap(),
        bytes
    );
    reverse(&mut owner, &g, HistoryDirection::Redo, &mut telemetry);
    assert_eq!(
        owner
            .read(&version.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "終\r\n"
    );
}

#[test]
fn given_frontier_retirement_after_preparation_then_unchanged_source_revision_is_not_enough() {
    let mut owner = owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let h = change(&mut owner, &[(&a, "a", "later")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let prepared = owner
        .prepare_reversal(&h, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    owner.retire_history_group(&g, &mut telemetry).unwrap();
    let before = owner.generation();
    assert_eq!(
        owner.commit(prepared, &mut telemetry),
        Err(CoordinatorError::History(HistoryError::Stale))
    );
    assert_eq!(owner.generation(), before);
    assert_eq!(
        owner.read(&a, &mut telemetry).unwrap().source.text(),
        "later"
    );
    reverse(&mut owner, &h, HistoryDirection::Undo, &mut telemetry);
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "a");
    assert_eq!(
        owner.history_status(&a, HistoryDirection::Undo, &mut telemetry),
        Ok(HistoryAvailability::Unavailable)
    );
}

#[test]
fn given_reused_pin_storage_then_an_old_pin_cannot_release_the_new_retention_promise() {
    let mut owner = owner_with(HistoryLimits {
        max_retained_groups: 8,
        max_journal_bytes: 65536,
        max_pins: 1,
    });
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let first = owner.pin_history(&g, &mut telemetry).unwrap();
    assert_eq!(
        owner.pin_history(&g, &mut telemetry).err(),
        Some(HistoryError::Limit)
    );
    let retained_old_token = first.clone();
    owner.release_history_pin(&first, &mut telemetry).unwrap();
    let second = owner.pin_history(&g, &mut telemetry).unwrap();
    assert_eq!(
        owner.release_history_pin(&retained_old_token, &mut telemetry),
        Err(HistoryError::Unavailable)
    );
    assert_eq!(
        owner.retire_history_group(&g, &mut telemetry),
        Err(HistoryError::Pinned)
    );
    owner.release_history_pin(&second, &mut telemetry).unwrap();
    owner.retire_history_group(&g, &mut telemetry).unwrap();
    assert_eq!(owner.history_usage().journal_bytes, 0);
}

#[test]
fn given_foreign_history_handles_then_they_neither_reverse_nor_control_another_owner() {
    let mut first = owner();
    let mut second = owner();
    let mut telemetry = Recorder::disabled();
    let a = first.create(b"A", &mut telemetry).unwrap().document;
    let b = second.create(b"B", &mut telemetry).unwrap().document;
    let g = change(&mut first, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let own_group = change(&mut second, &[(&b, "B", "b")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    assert_ne!(g, own_group);
    let pin = first.pin_history(&g, &mut telemetry).unwrap();
    assert_eq!(
        second
            .prepare_reversal(&g, HistoryDirection::Undo, &mut telemetry)
            .err(),
        Some(HistoryError::Unavailable)
    );
    assert_eq!(
        second.retire_history_group(&g, &mut telemetry),
        Err(HistoryError::Unavailable)
    );
    assert_eq!(
        second.release_history_pin(&pin, &mut telemetry),
        Err(HistoryError::Unavailable)
    );
    assert_eq!(second.read(&b, &mut telemetry).unwrap().source.text(), "b");
}

#[test]
fn given_diagnostics_off_available_or_full_then_undo_redo_and_retirement_are_identical() {
    use vellum_engine::instrumentation::{Boundary, Disposition, Observation};
    let mut results = Vec::new();
    for capacity in [0, 1, 64] {
        let mut owner = owner();
        let mut telemetry = Recorder::try_with_capacity(capacity).unwrap();
        telemetry.record(Observation {
            boundary: Boundary::Query,
            disposition: Disposition::Completed,
            service_time: None,
        });
        let a = owner
            .create(b"PRIVATE_HISTORY", &mut telemetry)
            .unwrap()
            .document;
        let g = change(
            &mut owner,
            &[(&a, "PRIVATE_HISTORY", "edited")],
            &mut telemetry,
        )
        .unwrap()
        .history_group
        .unwrap();
        reverse(&mut owner, &g, HistoryDirection::Undo, &mut telemetry);
        reverse(&mut owner, &g, HistoryDirection::Redo, &mut telemetry);
        let generation = owner.generation();
        let source = owner
            .read(&a, &mut telemetry)
            .unwrap()
            .source
            .text()
            .to_owned();
        owner.retire_history_group(&g, &mut telemetry).unwrap();
        results.push((generation, source, owner.history_usage()));
        let records: Vec<_> = std::iter::from_fn(|| telemetry.pop_front()).collect();
        assert!(!format!("{records:?}").contains("PRIVATE_HISTORY"));
        if capacity == 64 {
            assert_eq!(
                records
                    .iter()
                    .filter(|r| r.boundary == Boundary::Publication)
                    .count(),
                3
            );
        }
        if capacity == 1 {
            assert!(telemetry.dropped() > 0);
        }
    }
    assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
}

fn invocation_owner() -> SourceCoordinator {
    use vellum_engine::coordinator::InvocationLimits;
    SourceCoordinator::with_invocation_and_history_limits(
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
        InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 8,
            max_intent_bytes: 4096,
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        },
    )
}

#[test]
fn given_reversal_invocation_retries_then_original_outcomes_do_not_reexecute_or_refresh_history() {
    use vellum_engine::coordinator::{InvocationError, InvocationOutcome, InvocationState};
    let mut owner = invocation_owner();
    let mut telemetry = Recorder::try_with_capacity(64).unwrap();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let undo = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .submit_reversal(&undo, &g, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    owner
        .submit_reversal(&undo, &g, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    owner.run(&undo, &mut telemetry).unwrap();
    let original_undo_generation = owner.generation();
    let redo = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .submit_reversal(&redo, &g, HistoryDirection::Redo, &mut telemetry)
        .unwrap();
    owner.run(&redo, &mut telemetry).unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.submit_reversal(&undo, &g, HistoryDirection::Redo, &mut telemetry),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        owner.submit_reversal(&undo, &g, HistoryDirection::Undo, &mut telemetry),
        Ok(InvocationState::Finished)
    );
    owner.run(&undo, &mut telemetry).unwrap();
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "a");
    match owner.invocation_outcome(&undo, &mut telemetry).unwrap() {
        InvocationOutcome::Committed(outcome) => {
            assert_eq!(outcome.generation, original_undo_generation)
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(owner.history_usage().retained_groups, 1);
}

#[test]
fn given_source_invocation_with_history_then_matching_retry_creates_only_one_group() {
    use vellum_engine::coordinator::{InvocationError, SourceIntent};
    let mut owner = invocation_owner();
    let mut telemetry = Recorder::disabled();
    let version = owner.create(b"A", &mut telemetry).unwrap();
    let id = owner.issue_invocation(&mut telemetry).unwrap();
    let request = SourceIntent {
        writes: vec![DocumentEdits {
            expected: version.clone(),
            edits: vec![Replacement {
                start: 0,
                end: 1,
                expected: "A".into(),
                inserted: "a".into(),
            }],
        }],
        reads: vec![],
    };
    owner.submit(&id, &request, &mut telemetry).unwrap();
    owner.run(&id, &mut telemetry).unwrap();
    owner.submit(&id, &request, &mut telemetry).unwrap();
    owner.run(&id, &mut telemetry).unwrap();
    let group = match owner
        .history_status(&version.document, HistoryDirection::Undo, &mut telemetry)
        .unwrap()
    {
        HistoryAvailability::Eligible(group) => group,
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(
        owner.submit_reversal(&id, &group, HistoryDirection::Undo, &mut telemetry),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(owner.history_usage().retained_groups, 1);
    let undo = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .submit_reversal(&undo, &group, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    assert_eq!(
        owner.submit(&undo, &request, &mut telemetry),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&undo, &mut telemetry).unwrap();
    assert_eq!(
        owner
            .read(&version.document, &mut telemetry)
            .unwrap()
            .source
            .text(),
        "A"
    );
}

#[test]
fn given_a_retired_group_after_invocation_admission_then_final_history_dependency_refuses() {
    use vellum_engine::coordinator::InvocationOutcome;
    let mut owner = invocation_owner();
    let mut telemetry = Recorder::disabled();
    let a = owner.create(b"A", &mut telemetry).unwrap().document;
    let g = change(&mut owner, &[(&a, "A", "a")], &mut telemetry)
        .unwrap()
        .history_group
        .unwrap();
    let undo = owner.issue_invocation(&mut telemetry).unwrap();
    owner
        .submit_reversal(&undo, &g, HistoryDirection::Undo, &mut telemetry)
        .unwrap();
    owner.retire_history_group(&g, &mut telemetry).unwrap();
    owner.run(&undo, &mut telemetry).unwrap();
    assert_eq!(
        owner.invocation_outcome(&undo, &mut telemetry).unwrap(),
        &InvocationOutcome::Rejected(CoordinatorError::History(HistoryError::Unavailable))
    );
    assert_eq!(owner.read(&a, &mut telemetry).unwrap().source.text(), "a");
}

#[test]
fn generated_small_unicode_edit_pairs_roundtrip_through_the_actual_history_journal() {
    let text = "aü終b";
    let boundaries = [0, 1, 3, 6, 7];
    for lo1 in 0..=4 {
        for hi1 in lo1..=4 {
            for lo2 in 0..=4 {
                for hi2 in lo2..=4 {
                    let (a, b, c, d) = (
                        boundaries[lo1],
                        boundaries[hi1],
                        boundaries[lo2],
                        boundaries[hi2],
                    );
                    let ambiguous = if a == b {
                        c <= a && a <= d
                    } else if c == d {
                        a <= c && c <= b
                    } else {
                        a.max(c) < b.min(d)
                    };
                    if ambiguous {
                        continue;
                    }
                    for first in ["", "X", "é\u{301}"] {
                        for second in ["", "Y", "ü"] {
                            let mut owner = owner();
                            let mut telemetry = Recorder::disabled();
                            let encoded = format!("\u{feff}{text}");
                            let version = owner.create(encoded.as_bytes(), &mut telemetry).unwrap();
                            let writes = [DocumentEdits {
                                expected: version.clone(),
                                edits: vec![
                                    Replacement {
                                        start: c,
                                        end: d,
                                        expected: text[c..d].into(),
                                        inserted: second.into(),
                                    },
                                    Replacement {
                                        start: a,
                                        end: b,
                                        expected: text[a..b].into(),
                                        inserted: first.into(),
                                    },
                                ],
                            }];
                            let prepared = owner.prepare(&writes, &[], &mut telemetry).unwrap();
                            let result = owner.commit(prepared, &mut telemetry).unwrap();
                            let after = owner
                                .read(&version.document, &mut telemetry)
                                .unwrap()
                                .source
                                .encode()
                                .unwrap();
                            let Some(group) = result.history_group else {
                                assert_eq!(after, encoded.as_bytes());
                                continue;
                            };
                            reverse(&mut owner, &group, HistoryDirection::Undo, &mut telemetry);
                            assert_eq!(
                                owner
                                    .read(&version.document, &mut telemetry)
                                    .unwrap()
                                    .source
                                    .encode()
                                    .unwrap(),
                                encoded.as_bytes()
                            );
                            reverse(&mut owner, &group, HistoryDirection::Redo, &mut telemetry);
                            assert_eq!(
                                owner
                                    .read(&version.document, &mut telemetry)
                                    .unwrap()
                                    .source
                                    .encode()
                                    .unwrap(),
                                after
                            );
                        }
                    }
                }
            }
        }
    }
}
