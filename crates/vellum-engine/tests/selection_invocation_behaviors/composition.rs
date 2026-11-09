//! Accepted replacements are engine behavior; preedit sessions are not.
use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
use vellum_engine::source::Replacement;

fn actor() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "owner".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "edit".into(),
        destination: "host".into(),
        profile: PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1.into(),
        continuity: "session".into(),
    }
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 128,
        max_depth: 8,
    }
}
fn with_arguments<T>(
    id: &InvocationId,
    c: &LogicalViewContext,
    edit: &Replacement,
    consume: impl FnOnce(CompositionEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use CompositionEnvelopeValueV1 as V;
    let span = [
        ("lo", D::Number(edit.start as f64)),
        ("hi", D::Number(edit.end as f64)),
        ("expected", D::Text(&edit.expected)),
    ];
    let arguments = [
        ("contract", V::Data(D::Text("composition-commit/1"))),
        ("operation", V::Data(D::Text("accepted-composition-commit"))),
        ("invocation", V::Invocation(id)),
        ("document", V::Document(&c.source.document)),
        ("view", V::View(&c.view)),
        (
            "sourceRevision",
            V::Data(D::Number(c.source.revision as f64)),
        ),
        (
            "expectedSelection",
            V::Data(D::Number(c.stamp.selection_generation as f64)),
        ),
        (
            "profile",
            V::Data(D::Text(PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1)),
        ),
        ("replacement", V::Data(D::Object(&span))),
        ("text", V::Data(D::Text(&edit.inserted))),
    ];
    consume(V::Object(&arguments))
}

fn replacement(start: usize, end: usize, expected: &str, inserted: &str) -> Replacement {
    Replacement {
        start,
        end,
        expected: expected.into(),
        inserted: inserted.into(),
    }
}
fn range(anchor: usize, head: usize, affinity: Affinity) -> Selection {
    Selection {
        anchor: BytePosition(anchor),
        head: BytePosition(head),
        affinity,
    }
}
fn select(
    owner: &mut SourceCoordinator,
    context: &LogicalViewContext,
    selections: Vec<Selection>,
    primary: usize,
    r: &mut Recorder,
) -> LogicalViewContext {
    let id = owner.issue_invocation(r).unwrap();
    owner
        .submit_selection_command(
            &id,
            context,
            &SelectionCommandV1::Set {
                selections,
                primary,
            },
            r,
        )
        .unwrap();
    owner.run(&id, r).unwrap();
    owner
        .logical_view(&context.view, r)
        .unwrap()
        .context()
        .unwrap()
}

#[test]
fn surrounding_replacement_is_single_and_only_the_original_primary_collapses() {
    let (mut owner, context, mut r) = fixture();
    let context = select(
        &mut owner,
        &context,
        vec![caret(0), range(5, 7, Affinity::Before)],
        1,
        &mut r,
    );
    let peer = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&peer, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(
            &peer,
            stamp,
            &context.source,
            &[range(5, 7, Affinity::Before)],
            0,
            &mut r,
        )
        .unwrap();
    // Deliberately not the primary extent: accepted surrounding text replaces b,c.
    let plan = owner
        .prepare_composition_replacement_v1(&context, &replacement(1, 3, "bc", "é"), &mut r)
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "aé\r\ndef"
    );
    assert_eq!(outcome.changes.len(), 1);
    assert!(outcome.history_group.is_some());
    assert_eq!(owner.history_usage().retained_groups, 1);
    let view = owner.logical_view(&context.view, &mut r).unwrap();
    assert_eq!(
        view.selections.selections(),
        &[caret(0), range(3, 3, Affinity::Before)]
    );
    assert_eq!(view.selections.primary(), 1);
    assert_eq!(
        owner
            .logical_view(&peer, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[range(5, 7, Affinity::Before)]
    );
}

#[test]
fn primary_override_precedes_normalization_and_preserves_original_affinity() {
    let (mut owner, context, mut r) = fixture();
    let context = select(
        &mut owner,
        &context,
        vec![range(0, 1, Affinity::After), range(2, 3, Affinity::Before)],
        1,
        &mut r,
    );
    let plan = owner
        .prepare_composition_replacement_v1(&context, &replacement(0, 3, "abc", "X"), &mut r)
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    let view = owner.logical_view(&context.view, &mut r).unwrap();
    // Both ranges would map to [0,1] and merge if normalized too early.
    // Override the original primary first. The normalizer deliberately keeps
    // a caret at a range boundary distinct, preserving the secondary range.
    assert_eq!(
        view.selections.selections(),
        &[range(0, 1, Affinity::After), range(1, 1, Affinity::Before)]
    );
    assert_eq!(view.selections.primary(), 1);
}

#[test]
fn equal_bytes_move_only_the_primary_and_full_noop_has_no_history_or_publication() {
    let (mut owner, context, mut r) = fixture();
    let context = select(
        &mut owner,
        &context,
        vec![range(1, 3, Affinity::Before)],
        0,
        &mut r,
    );
    let edit = replacement(1, 3, "bc", "bc");
    let plan = owner
        .prepare_composition_replacement_v1(&context, &edit, &mut r)
        .unwrap();
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert!(outcome.changes.is_empty());
    assert!(outcome.history_group.is_none());
    assert!(outcome.operation.is_some());
    let view = owner.logical_view(&context.view, &mut r).unwrap();
    assert_eq!(
        view.selections.selections(),
        &[range(3, 3, Affinity::Before)]
    );
    let next = view.context().unwrap();
    assert_eq!(next.source, context.source);
    let plan = owner
        .prepare_composition_replacement_v1(&next, &edit, &mut r)
        .unwrap();
    let noop = owner.commit(plan, &mut r).unwrap();
    assert!(noop.operation.is_none());
    assert_eq!(noop.generation, outcome.generation);
    assert_eq!(owner.history_usage().retained_groups, 0);
}

#[test]
fn stale_peer_or_changed_write_policy_refuses_without_a_partial_commit() {
    for policy_change in [false, true] {
        let (mut owner, context, mut r) = fixture();
        let peer = owner.create_logical_view(&mut r).unwrap();
        let stamp = owner.logical_view(&peer, &mut r).unwrap().stamp;
        owner
            .attach_logical_view(&peer, stamp, &context.source, &[caret(0)], 0, &mut r)
            .unwrap();
        let plan = owner
            .prepare_composition_replacement_v1(&context, &replacement(1, 3, "bc", "X"), &mut r)
            .unwrap();
        if policy_change {
            owner
                .set_read_only(&context.source.document, true, &mut r)
                .unwrap();
        } else {
            let peer_context = owner
                .logical_view(&peer, &mut r)
                .unwrap()
                .context()
                .unwrap();
            select(&mut owner, &peer_context, vec![caret(2)], 0, &mut r);
        }
        assert!(owner.commit(plan, &mut r).is_err());
        let source = owner.read(&context.source.document, &mut r).unwrap();
        assert_eq!(source.source.text(), "abc\r\ndef");
        assert_eq!(source.version, context.source);
        assert_eq!(owner.history_usage().retained_groups, 0);
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[caret(1)]
        );
    }
}

#[test]
fn native_and_both_normalized_routes_replay_original_intent_and_retire_once() {
    use CompositionEnvelopeValueV1 as V;
    for route in ["native", "typed", "dynamic"] {
        let (mut owner, context, mut r) = fixture_with_intent_limit(16384);
        let id = owner.issue_invocation(&mut r).unwrap();
        let edit = replacement(1, 3, "bc", "é");
        let actor = actor();
        let submit = |owner: &mut SourceCoordinator, r: &mut Recorder| {
            if route == "native" {
                return owner.submit_composition_replacement_v1(
                    &id,
                    &context,
                    &edit,
                    &actor,
                    limits(),
                    r,
                );
            }
            with_arguments(&id, &context, &edit, |arguments| {
                let header = [
                    (
                        if route == "typed" { "family" } else { "name" },
                        if route == "typed" {
                            V::Data(D::Number(10.0))
                        } else {
                            V::Data(D::Text("change-source"))
                        },
                    ),
                    ("variant", V::Data(D::Text("accepted-composition-commit"))),
                    ("version", V::Data(D::Number(1.0))),
                    ("arguments", arguments),
                ];
                let outer = [
                    ("route", V::Data(D::Text(route))),
                    ("request", V::Object(&header)),
                ];
                let q = normalize_composition_envelope_v1(
                    V::Object(&outer),
                    &context,
                    &actor,
                    limits(),
                    r,
                )?;
                owner.submit_composition_arguments_v1(&q, r)
            })
        };
        assert_eq!(submit(&mut owner, &mut r), Ok(InvocationState::Pending));
        assert_eq!(submit(&mut owner, &mut r), Ok(InvocationState::Pending));
        let mut changed_actor = actor.clone();
        changed_actor.recipient = "someone-else".into();
        assert_eq!(
            owner.submit_composition_replacement_v1(
                &id,
                &context,
                &edit,
                &changed_actor,
                limits(),
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            owner.submit_composition_replacement_v1(
                &id,
                &context,
                &edit,
                &actor,
                CaptureLimitsV1 {
                    max_nodes: 127,
                    ..limits()
                },
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            owner.submit_composition_replacement_v1(
                &id,
                &context,
                &replacement(1, 3, "bc", "X"),
                &actor,
                limits(),
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        owner.run(&id, &mut r).unwrap();
        let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
        else {
            panic!("not committed");
        };
        assert_eq!(outcome.changes.len(), 1);
        assert_eq!(owner.history_usage().retained_groups, 1);
        assert_eq!(submit(&mut owner, &mut r), Ok(InvocationState::Finished));
        owner.forget_invocation_outcome(&id, &mut r).unwrap();
        assert_eq!(submit(&mut owner, &mut r), Ok(InvocationState::Finished));
        owner.run(&id, &mut r).unwrap();
        assert_eq!(owner.history_usage().retained_groups, 1);
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            "aé\r\ndef"
        );
    }
}

#[test]
fn refused_preparation_or_capture_keeps_the_original_ticket_unbound() {
    let (mut owner, context, mut r) = fixture_with_intent_limit(16384);
    let id = owner.issue_invocation(&mut r).unwrap();
    let edit = replacement(1, 3, "bc", "X");
    for budget in [
        CaptureLimitsV1 {
            max_bytes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: 1,
            ..limits()
        },
    ] {
        assert_eq!(
            owner.submit_composition_replacement_v1(&id, &context, &edit, &actor(), budget, &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(
            owner.invocation_state(&id, &mut r),
            Ok(InvocationState::Issued)
        );
    }
    let invalid = replacement(1, 3, "no", "X");
    assert!(
        owner
            .submit_composition_replacement_v1(&id, &context, &invalid, &actor(), limits(), &mut r)
            .is_err()
    );
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.submit_composition_replacement_v1(&id, &context, &edit, &actor(), limits(), &mut r),
        Ok(InvocationState::Pending)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&context.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "aX\r\ndef"
    );
}

#[test]
fn malformed_composition_fields_refuse_before_admission() {
    use CompositionEnvelopeValueV1 as V;
    let (mut owner, context, mut r) = fixture_with_intent_limit(16384);
    let id = owner.issue_invocation(&mut r).unwrap();
    let edit = replacement(1, 3, "bc", "X");
    for malformed in 0..7 {
        with_arguments(&id, &context, &edit, |args| {
            let V::Object(rows) = args else { panic!() };
            let mut rows = rows.to_vec();
            match malformed {
                0 => rows.push(("extra", V::Data(D::Null))),
                1 => rows[0] = ("operation", V::Data(D::Text("accepted-composition-commit"))),
                2 => rows[1].1 = V::Data(D::Text("ordinary-edit")),
                3 => rows[2].1 = V::Data(D::Text("invented-invocation")),
                4 => rows[5].1 = V::Data(D::Number(f64::NAN)),
                5 => rows[7].1 = V::Data(D::Text("another-profile")),
                _ => rows[8].1 = V::Data(D::Object(&[])),
            }
            assert!(matches!(
                normalize_composition_arguments_v1(
                    V::Object(&rows),
                    &context,
                    &actor(),
                    limits(),
                    &mut r
                ),
                Err(InvocationError::MalformedIntent)
            ));
        });
    }
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
}

#[test]
fn installed_composition_executes_and_protected_stop_settles_without_publication() {
    use CompositionEnvelopeValueV1 as V;
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1, Observation};
    for stop in [false, true] {
        for capture in 0..4 {
            let (mut owner, context, _) = fixture_with_intent_limit(16384);
            ordinary_result::fund(&mut owner, &mut Recorder::disabled());
            let mut r = match capture {
                0 => Recorder::disabled(),
                1 => Recorder::try_with_capacity(4096).unwrap(),
                2 => Recorder::try_with_capacity(4096)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                _ => Recorder::try_with_capacity(1)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            };
            if capture == 3 {
                r.record(Observation {
                    boundary: Boundary::InvocationAdmission,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let route = selection_route::Route::install_composition(&mut owner, &mut r);
            let id = owner.issue_invocation(&mut r).unwrap();
            ordinary_result::bind_in_context(&mut owner, &id, &actor(), &mut r);
            let edit = replacement(1, 3, "bc", "é");
            let actor = actor();
            with_arguments(&id, &context, &edit, |args| {
                let typed = capture % 2 == 0;
                let header = [
                    (
                        if typed { "family" } else { "name" },
                        if typed {
                            V::Data(D::Number(10.0))
                        } else {
                            V::Data(D::Text("change-source"))
                        },
                    ),
                    ("variant", V::Data(D::Text("accepted-composition-commit"))),
                    ("version", V::Data(D::Number(1.0))),
                    ("arguments", args),
                ];
                let outer = [
                    (
                        "route",
                        V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
                    ),
                    ("request", V::Object(&header)),
                ];
                let q = normalize_composition_envelope_v1(
                    V::Object(&outer),
                    &context,
                    &actor,
                    limits(),
                    &mut r,
                )
                .unwrap();
                assert_eq!(
                    route.submit_composition(&mut owner, &q, &mut r),
                    Ok(InvocationState::Pending)
                );
            });
            if stop {
                owner
                    .request_invocation_stop(
                        &id,
                        &TrustedInvocationControlV1 {
                            context: &actor,
                            owner_control_allowed: Some(true),
                            control_context_current: Some(true),
                            control_scope_valid: Some(true),
                            protected_control_ready: Some(true),
                        },
                        &mut r,
                    )
                    .unwrap();
            }
            // Discovery retirement cannot revoke the original ticket, its
            // protected control or the native publisher/retained outcome.
            route.retire(&mut owner, &mut r);
            owner.run(&id, &mut r).unwrap();
            if stop {
                assert!(matches!(
                    owner.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::StoppedBeforePublication
                ));
                assert_eq!(owner.history_usage().retained_groups, 0);
            } else {
                assert!(matches!(
                    owner.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::Committed(_)
                ));
                assert_eq!(owner.history_usage().retained_groups, 1);
            }
            assert_eq!(
                owner
                    .read(&context.source.document, &mut r)
                    .unwrap()
                    .source
                    .text(),
                if stop { "abc\r\ndef" } else { "aé\r\ndef" }
            );
            ordinary_result::check_in_context(
                &mut owner,
                &id,
                &actor,
                if stop {
                    InvocationPublicationV1::NotApplied
                } else {
                    InvocationPublicationV1::Applied
                },
                &mut r,
            );
            owner.forget_invocation_outcome(&id, &mut r).unwrap();
            let mut found = [false; 4];
            while let Some(record) = r.pop_front() {
                let index = match record.boundary {
                    Boundary::CompositionArgumentNormalization => Some(0),
                    Boundary::CompositionReplacementPreparation => Some(1),
                    Boundary::CompositionAdmission => Some(2),
                    Boundary::DiscoveryCompositionDispatch => Some(3),
                    _ => None,
                };
                if let Some(index) = index {
                    found[index] = true;
                    assert_eq!(record.disposition, Disposition::Completed);
                    assert_eq!(record.service_time.is_some(), capture == 2);
                }
            }
            assert_eq!(found, [capture == 1 || capture == 2; 4]);
        }
    }
}
