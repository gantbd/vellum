use super::*;
fn fixture() -> (SourceCoordinator, LogicalViewContext, Recorder) {
    let (mut owner, _, mut r) = views::line_test_fixture();
    owner.invocations = InvocationTable::new(InvocationLimits {
        max_lifetime_issued: 4,
        max_retained: 4,
        max_intent_bytes: 16384,
    });
    // A large retained candidate makes native-envelope admission stricter than
    // the small canonical request's traversal/copy charge.
    let text = "x".repeat(1800);
    let source = owner.create(text.as_bytes(), &mut r).unwrap();
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(
            &view,
            stamp,
            &source,
            &[crate::selection::Selection {
                anchor: crate::coordinates::BytePosition(1),
                head: crate::coordinates::BytePosition(1),
                affinity: crate::attachment::Affinity::After,
            }],
            0,
            &mut r,
        )
        .unwrap();
    let view = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    (owner, view, r)
}
fn actor() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "p".into(),
        continuity: "c".into(),
    }
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 128,
        max_depth: 8,
    }
}
fn edit() -> Replacement {
    Replacement {
        start: 1,
        end: 2,
        expected: "x".into(),
        inserted: "é".into(),
    }
}

#[test]
fn exact_native_envelope_admission_refunds_failure_and_keeps_the_ticket_unbound() {
    let (mut owner, view, mut r) = fixture();
    let intent = CompositionIntent {
        view: view.clone(),
        replacement: edit(),
        actor: actor(),
        owner: owner.namespace.clone(),
        limits: limits(),
    };
    let plan = owner
        .prepare_composition_replacement_v1(&view, &edit(), &mut r)
        .unwrap();
    let required = envelope_bytes(&intent, &plan).unwrap();
    drop(plan);
    let id = owner.issue_invocation(&mut r).unwrap();
    owner.invocations.limits.max_intent_bytes = required - 1;
    assert_eq!(
        owner.submit_composition_replacement_v1(&id, &view, &edit(), &actor(), limits(), &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
    owner.invocations.limits.max_intent_bytes = required;
    assert_eq!(
        owner.submit_composition_replacement_v1(&id, &view, &edit(), &actor(), limits(), &mut r),
        Ok(InvocationState::Pending)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(owner.history_usage().retained_groups, 1);
}

#[test]
fn missing_private_seal_is_an_unknown_fault_without_a_source_or_history_prefix() {
    let (mut owner, view, mut r) = fixture();
    let id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_composition_replacement_v1(&id, &view, &edit(), &actor(), limits(), &mut r)
        .unwrap();
    let Phase::Pending {
        prepared: Ok(plan), ..
    } = &mut owner.invocations.slots[id.slot].as_mut().unwrap().phase
    else {
        panic!()
    };
    plan.derived_edit.clear();
    let generation = owner.generation;
    assert_eq!(owner.run(&id, &mut r), Err(InvocationError::Faulted));
    assert!(owner.faulted);
    assert_eq!(owner.generation, generation);
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.documents[view.source.document.slot]
            .as_ref()
            .unwrap()
            .source
            .text(),
        "x".repeat(1800)
    );
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
}
