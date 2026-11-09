use super::*;
use crate::settings::*;
pub(in crate::coordinator) mod ingress;
pub(in crate::coordinator) fn limits() -> SettingsReaderLimitsV1 {
    let mut l = SettingsReaderLimitsV1 {
        max_issued: 64,
        max_packets: 4,
        max_contexts: 4,
        max_origins: 8,
        max_packet_bytes: 65536,
        max_capture_bytes: 16384,
        max_reply_bytes: 16384,
        max_context_bytes: 4096,
        max_label_bytes: 128,
        max_work: 1_048_576,
        max_depth: 32,
        max_pool_bytes: usize::MAX,
    };
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    l
}
pub(in crate::coordinator) fn scope() -> SettingsReadScopeV1 {
    SettingsReadScopeV1 {
        execution: SettingsExecutionContextV1 {
            authority: "engine".into(),
            engine: "local".into(),
            recipient: "reader".into(),
            purpose: "inspect".into(),
            destination: "client".into(),
            profile: SETTINGS_READ_PROFILE_V1.into(),
            continuity: "session".into(),
        },
        policy: "policy-1".into(),
    }
}
pub(in crate::coordinator) fn facts() -> TrustedSettingsReadFactsV1 {
    TrustedSettingsReadFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        outside_publication: Some(true),
        resources_ready: Some(true),
        transfer_ready: Some(true),
        reply_ready: Some(true),
    }
}
fn identity() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
fn setup(l: SettingsReaderLimitsV1) -> (SourceCoordinator, SettingsTargetId, Recorder) {
    setup_with_parent(l, l.max_pool_bytes)
}
pub(in crate::coordinator) fn setup_with_parent(
    l: SettingsReaderLimitsV1,
    parent_bytes: usize,
) -> (SourceCoordinator, SettingsTargetId, Recorder) {
    let mut o = super::super::tests::owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(super::super::tests::compiled(&mut r), &mut r)
        .unwrap();
    o.configure_retention_budget(parent_bytes, &mut r).unwrap();
    o.configure_settings_reader_v1("engine", "local", l, &mut r)
        .unwrap();
    let target = o.settings_target(&identity(), &mut r).unwrap();
    (o, target, r)
}
pub(in crate::coordinator) fn cut(
    o: &mut SourceCoordinator,
    t: &SettingsTargetId,
    s: &SettingsReadScopeV1,
    r: &mut Recorder,
) -> SettingsReadCutIdV1 {
    prepare(o, t, s, true, true, true, r).unwrap()
}
#[allow(clippy::too_many_arguments)]
fn prepare(
    o: &mut SourceCoordinator,
    t: &SettingsTargetId,
    s: &SettingsReadScopeV1,
    value: bool,
    origins: bool,
    declaration: bool,
    r: &mut Recorder,
) -> Result<SettingsReadCutIdV1, SettingsError> {
    let ids = [SettingProjectionOriginV1::Library];
    let grants = [SettingProjectionGrantV1 {
        key: "flag",
        visible: true,
        value,
        origins,
        declaration,
        origin_ids: if origins { &ids } else { &[] },
    }];
    o.prepare_settings_read_v1(
        t,
        s,
        &TrustedSettingsReadPreparationV1 {
            read: &facts(),
            owner_control_allowed: Some(true),
            preparation_reserved: Some(true),
            visibility_attested: Some(true),
            visibility_complete: Some(true),
            target_readable: Some(true),
            keys: &grants,
            capabilities: true,
        },
        r,
    )
}
#[allow(clippy::too_many_arguments)]
fn query(
    o: &mut SourceCoordinator,
    a: &SettingsReadAcquisitionV1,
    t: &SettingsTargetId,
    s: &SettingsReadScopeV1,
    operation: SettingsReadOperationV1<'_>,
    f: &TrustedSettingsReadFactsV1,
    r: &mut Recorder,
) -> Result<SettingsReadResultV1, SettingsError> {
    let current = cut(o, t, s, r);
    o.query_settings_read_v1(&a.context, &a.target, s, operation, &current, f, r)
}
pub(in crate::coordinator) fn control(s: &SettingsReadScopeV1) -> SettingsReadControlV1<'_> {
    SettingsReadControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn reader_delivers_owned_values_opaque_origins_and_accepted_declarations_then_cleans_up() {
    let (mut o, t, mut r) = setup(limits());
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    assert!(
        o.acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .is_err()
    );
    let SettingsReadResultV1::Value(value) = query(
        &mut o,
        &a,
        &t,
        &s,
        SettingsReadOperationV1::EffectiveConfiguration { key: "flag" },
        &facts(),
        &mut r,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(value.read().boolean(), Some(false));
    let SettingsReadResultV1::Origins(SettingsReadOriginsV1::Whole(ids)) = query(
        &mut o,
        &a,
        &t,
        &s,
        SettingsReadOperationV1::PermittedOrigin { key: "flag" },
        &facts(),
        &mut r,
    )
    .unwrap() else {
        panic!()
    };
    let SettingsReadResultV1::Origins(SettingsReadOriginsV1::Whole(again)) = query(
        &mut o,
        &a,
        &t,
        &s,
        SettingsReadOperationV1::PermittedOrigin { key: "flag" },
        &facts(),
        &mut r,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(ids, again);
    assert!(!format!("{ids:?}").contains("Library"));
    let SettingsReadResultV1::Capabilities(declarations) = query(
        &mut o,
        &a,
        &t,
        &s,
        SettingsReadOperationV1::Capabilities,
        &facts(),
        &mut r,
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].key, "flag");
    assert!(declarations[0].sensitive);
    assert!(
        o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
            .unwrap()
    );
    assert!(
        o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
            .unwrap()
    );
    assert!(o.retire_settings_reader_v1(&mut r).unwrap());
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(value.read().boolean(), Some(false));
    assert!(
        o.configure_settings_reader_v1("engine", "local", limits(), &mut r)
            .is_err()
    );
}
#[test]
fn recognized_loss_is_absorbing_but_foreign_scope_and_transient_refusal_do_not_poison() {
    let (mut o, t, mut r) = setup(limits());
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let mut lost = facts();
    lost.authorized = Some(false);
    let mut foreign = s.clone();
    foreign.policy = "another-policy".into();
    let p = cut(&mut o, &t, &s, &mut r);
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &foreign,
            SettingsReadOperationV1::Capabilities,
            &p,
            &lost,
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
    let mut transient = facts();
    transient.transfer_ready = Some(false);
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &p,
            &transient,
            &mut r
        )
        .is_err()
    );
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &p,
            &facts(),
            &mut r
        )
        .is_ok()
    );
    assert!(
        query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::Capabilities,
            &lost,
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 1);
    assert!(
        query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::Capabilities,
            &facts(),
            &mut r
        )
        .is_err()
    );
    // Loss/foreign/transient checks precede cut consumption; protected discard remains.
    let State::Active(pool) = &o.settings_reader else {
        panic!()
    };
    let cuts: Vec<_> = pool
        .rows
        .iter()
        .enumerate()
        .filter_map(|(slot, row)| {
            matches!(row, Some(Row::Cut(_))).then_some(SettingsReadCutIdV1 {
                owner: o.namespace.clone(),
                slot,
            })
        })
        .collect();
    for p in cuts {
        o.discard_settings_read_cut_v1(&p, &control(&s), &mut r)
            .unwrap();
    }
    o.faulted = true;
    assert!(
        o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
            .unwrap()
    );
    assert!(o.retire_settings_reader_v1(&mut r).unwrap());
    assert!(o.faulted);
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn captured_permissions_never_widen_and_observed_narrowing_cannot_regrant() {
    let (mut o, t, mut r) = setup(limits());
    let s = scope();
    let p = prepare(&mut o, &t, &s, false, false, true, &mut r).unwrap();
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    assert!(
        query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::EffectiveConfiguration { key: "flag" },
            &facts(),
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
    assert!(
        query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::Capabilities,
            &facts(),
            &mut r
        )
        .is_ok()
    );
    let p = prepare(&mut o, &t, &s, false, false, false, &mut r).unwrap();
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &p,
            &facts(),
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 1);
    assert!(
        query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::Capabilities,
            &facts(),
            &mut r
        )
        .is_err()
    );
}
#[test]
fn exact_parent_funding_and_lifetime_issuance_are_enforced() {
    let l = limits();
    let mut o = super::super::tests::owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(l.max_pool_bytes - 1, &mut r)
        .unwrap();
    assert_eq!(
        o.configure_settings_reader_v1("engine", "local", l, &mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.settings_reader_usage_v1(),
        SettingsReaderUsageV1::default()
    );
    let mut small = limits();
    small.max_issued = 1;
    small.max_pool_bytes = small.required_pool_bytes().unwrap();
    let (mut o, t, mut r) = setup(small);
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    o.discard_settings_read_cut_v1(&p, &control(&s), &mut r)
        .unwrap();
    assert!(prepare(&mut o, &t, &s, true, true, true, &mut r).is_err());
    assert_eq!(o.settings_reader_usage_v1().issued_lifetime, 1);
    o.retire_settings_reader_v1(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn all_permission_combinations_and_eight_scope_fields_are_independent() {
    for mask in 0..8 {
        let (mut o, t, mut r) = setup(limits());
        let s = scope();
        let p = prepare(
            &mut o,
            &t,
            &s,
            mask & 1 != 0,
            mask & 2 != 0,
            mask & 4 != 0,
            &mut r,
        )
        .unwrap();
        let a = o
            .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .unwrap();
        assert_eq!(
            query(
                &mut o,
                &a,
                &t,
                &s,
                SettingsReadOperationV1::EffectiveConfiguration { key: "flag" },
                &facts(),
                &mut r
            )
            .is_ok(),
            mask & 1 != 0
        );
        assert_eq!(
            query(
                &mut o,
                &a,
                &t,
                &s,
                SettingsReadOperationV1::PermittedOrigin { key: "flag" },
                &facts(),
                &mut r
            )
            .is_ok(),
            mask & 2 != 0
        );
        let SettingsReadResultV1::Capabilities(d) = query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::Capabilities,
            &facts(),
            &mut r,
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(d.len(), usize::from(mask & 4 != 0));
        assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
    }
    let (mut o, t, mut r) = setup(limits());
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let current = cut(&mut o, &t, &s, &mut r);
    for field in 0..8 {
        let mut foreign = s.clone();
        let label = match field {
            0 => &mut foreign.execution.authority,
            1 => &mut foreign.execution.engine,
            2 => &mut foreign.execution.recipient,
            3 => &mut foreign.execution.purpose,
            4 => &mut foreign.execution.destination,
            5 => &mut foreign.execution.profile,
            6 => &mut foreign.execution.continuity,
            _ => &mut foreign.policy,
        };
        label.push_str("-foreign");
        let mut loss = facts();
        loss.authorized = Some(false);
        assert!(
            o.query_settings_read_v1(
                &a.context,
                &a.target,
                &foreign,
                SettingsReadOperationV1::Capabilities,
                &current,
                &loss,
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
    }
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &current,
            &facts(),
            &mut r
        )
        .is_ok()
    );
}

#[test]
fn every_current_guard_refuses_unknown_and_only_attributed_losses_absorb() {
    for field in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, t, mut r) = setup(limits());
            let s = scope();
            let p = cut(&mut o, &t, &s, &mut r);
            let a = o
                .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
                .unwrap();
            let current = cut(&mut o, &t, &s, &mut r);
            let mut f = facts();
            *match field {
                0 => &mut f.authorized,
                1 => &mut f.scope,
                2 => &mut f.continuous,
                3 => &mut f.live,
                4 => &mut f.retained,
                5 => &mut f.not_stopped,
                6 => &mut f.outside_publication,
                7 => &mut f.resources_ready,
                8 => &mut f.transfer_ready,
                _ => &mut f.reply_ready,
            } = value;
            assert!(
                o.query_settings_read_v1(
                    &a.context,
                    &a.target,
                    &s,
                    SettingsReadOperationV1::Capabilities,
                    &current,
                    &f,
                    &mut r
                )
                .is_err()
            );
            let absorbing = field < 5 && value == Some(false);
            assert_eq!(
                o.settings_reader_usage_v1().ended_contexts,
                usize::from(absorbing)
            );
            assert_eq!(
                o.query_settings_read_v1(
                    &a.context,
                    &a.target,
                    &s,
                    SettingsReadOperationV1::Capabilities,
                    &current,
                    &facts(),
                    &mut r
                )
                .is_ok(),
                !absorbing
            );
        }
    }
}

fn enable_changes(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_semantic_root_budget(1_048_576, r).unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 8,
            max_request_nodes: 64,
            max_request_depth: 32,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        r,
    )
    .unwrap();
}
fn change(
    o: &mut SourceCoordinator,
    layer: SettingIdentityV1,
    generation: u64,
    value: bool,
    r: &mut Recorder,
) {
    let p = o
        .prepare_settings_change(
            &[SettingChangeV1 {
                layer,
                expected_generation: generation,
                key: "flag".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Boolean(value)),
            }],
            r,
        )
        .unwrap();
    o.commit_settings_change(p, r).unwrap();
}
#[test]
fn stale_cuts_do_not_refresh_and_observed_value_aba_requires_new_acquisition() {
    let (mut o, t, mut r) = setup(limits());
    enable_changes(&mut o, &mut r);
    let s = scope();
    let p = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let stale = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    change(&mut o, identity(), 1, true, &mut r);
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &stale,
            &facts(),
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
    let current = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &current,
            &facts(),
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 1);
    change(&mut o, identity(), 2, false, &mut r);
    let current = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &current,
            &facts(),
            &mut r
        )
        .is_err()
    );
    let fresh = o
        .acquire_settings_read_v1(&current, &s, &facts(), &mut r)
        .unwrap();
    let current = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    let SettingsReadResultV1::Value(v) = o
        .query_settings_read_v1(
            &fresh.context,
            &fresh.target,
            &s,
            SettingsReadOperationV1::EffectiveConfiguration { key: "flag" },
            &current,
            &facts(),
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(v.read().boolean(), Some(false));
    o.discard_settings_read_cut_v1(&stale, &control(&s), &mut r)
        .unwrap();
}

#[test]
fn hidden_value_changes_leave_declaration_only_context_current() {
    let (mut o, t, mut r) = setup(limits());
    enable_changes(&mut o, &mut r);
    let s = scope();
    let p = prepare(&mut o, &t, &s, false, false, true, &mut r).unwrap();
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    change(&mut o, identity(), 1, true, &mut r);
    let current = prepare(&mut o, &t, &s, false, false, true, &mut r).unwrap();
    assert!(
        o.query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::Capabilities,
            &current,
            &facts(),
            &mut r
        )
        .is_ok()
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
}

#[test]
fn origin_references_are_context_local_and_cross_owner_target_or_context_cannot_poison() {
    let (mut o, t, mut r) = setup(limits());
    let (mut peer, peer_t, mut pr) = setup(limits());
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let p = cut(&mut o, &t, &s, &mut r);
    let b = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let p = cut(&mut peer, &peer_t, &s, &mut pr);
    let foreign = peer
        .acquire_settings_read_v1(&p, &s, &facts(), &mut pr)
        .unwrap();
    let current = cut(&mut o, &t, &s, &mut r);
    let mut loss = facts();
    loss.authorized = Some(false);
    for (context, target) in [
        (&foreign.context, &a.target),
        (&a.context, &foreign.target),
        (&a.context, &b.target),
    ] {
        assert!(
            o.query_settings_read_v1(
                context,
                target,
                &s,
                SettingsReadOperationV1::Capabilities,
                &current,
                &loss,
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
    }
    let SettingsReadResultV1::Origins(SettingsReadOriginsV1::Whole(a_ids)) = query(
        &mut o,
        &a,
        &t,
        &s,
        SettingsReadOperationV1::PermittedOrigin { key: "flag" },
        &facts(),
        &mut r,
    )
    .unwrap() else {
        panic!()
    };
    let SettingsReadResultV1::Origins(SettingsReadOriginsV1::Whole(b_ids)) = query(
        &mut o,
        &b,
        &t,
        &s,
        SettingsReadOperationV1::PermittedOrigin { key: "flag" },
        &facts(),
        &mut r,
    )
    .unwrap() else {
        panic!()
    };
    assert_ne!(a_ids, b_ids);
}

#[test]
fn actual_inventory_reconciles_funding_and_protected_cleanup_survives_fault_and_fence() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read, install};
    let l = limits();
    let fence_bytes = crate::coordinator::shutdown::tests::limits()
        .required_capture_bytes()
        .unwrap();
    let (mut o, t, mut r) = setup_with_parent(l, l.max_pool_bytes + fence_bytes);
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let pending = cut(&mut o, &t, &s, &mut r);
    let inventory = inspect(&o, &mut r);
    let view = inspect_read(&o, &inventory, &mut r);
    assert!(view.coverage.all_owner_obligations);
    assert_eq!(view.funding.unrepresented_parent_bytes, 0);
    let row = view
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::SettingsReader)
        .unwrap();
    assert_eq!(row.retained_records, Some(2));
    assert_eq!(row.reserved_bytes, Some(l.max_pool_bytes));
    install(&mut o, &mut r);
    assert!(prepare(&mut o, &t, &s, true, true, true, &mut r).is_err());
    assert!(
        o.acquire_settings_read_v1(&pending, &s, &facts(), &mut r)
            .is_err()
    );
    o.faulted = true;
    o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
        .unwrap();
    o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
        .unwrap();
    o.discard_settings_read_cut_v1(&pending, &control(&s), &mut r)
        .unwrap();
    o.retire_settings_reader_v1(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, fence_bytes);
    assert!(
        o.configure_settings_reader_v1("engine", "local", l, &mut r)
            .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().issued_lifetime, 5);
    assert!(!o.retire_settings_reader_v1(&mut r).unwrap());
    assert!(o.faulted);
}

#[test]
fn capture_off_on_timed_and_saturated_preserve_reader_behavior() {
    use crate::instrumentation::NativeTimingProfileV1;
    for mode in 0..4 {
        let (mut o, t, _) = setup(limits());
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let s = scope();
        let p = cut(&mut o, &t, &s, &mut r);
        let a = o
            .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .unwrap();
        let SettingsReadResultV1::Value(v) = query(
            &mut o,
            &a,
            &t,
            &s,
            SettingsReadOperationV1::EffectiveConfiguration { key: "flag" },
            &facts(),
            &mut r,
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(v.read().boolean(), Some(false));
        o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
            .unwrap();
        o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
            .unwrap();
        o.retire_settings_reader_v1(&mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
        if mode == 1 || mode == 2 {
            let mut boundaries = Vec::new();
            while let Some(record) = r.pop_front() {
                assert_eq!(record.service_time.is_some(), mode == 2);
                boundaries.push(record.boundary);
            }
            assert!(boundaries.contains(&Boundary::SettingsReadRelease));
            assert!(boundaries.contains(&Boundary::SettingsReadStop));
            assert!(boundaries.contains(&Boundary::SettingsReadRetirement));
        }
    }
}

#[test]
fn packet_and_context_capacity_release_is_separate_from_lifetime_issuance() {
    let mut l = limits();
    l.max_packets = 1;
    l.max_contexts = 1;
    l.max_pool_bytes = l.required_pool_bytes().unwrap();
    let (mut o, t, mut r) = setup(l);
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    assert!(prepare(&mut o, &t, &s, true, true, true, &mut r).is_err());
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let next = cut(&mut o, &t, &s, &mut r);
    let usage = o.settings_reader_usage_v1();
    assert!(
        o.acquire_settings_read_v1(&next, &s, &facts(), &mut r)
            .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1(), usage);
    o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
        .unwrap();
    assert!(
        o.acquire_settings_read_v1(&next, &s, &facts(), &mut r)
            .is_err()
    );
    o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
        .unwrap();
    let b = o
        .acquire_settings_read_v1(&next, &s, &facts(), &mut r)
        .unwrap();
    assert_ne!(a.context, b.context);
    assert_ne!(a.target, b.target);
    assert!(o.settings_reader_usage_v1().issued_lifetime > usage.issued_lifetime);
    o.stop_settings_read_v1(&b.context, &control(&s), &mut r)
        .unwrap();
    let mut missing = control(&s);
    missing.delivery_drained = None;
    assert!(
        o.retire_settings_read_v1(&b.context, &missing, &mut r)
            .is_err()
    );
    assert_eq!(o.settings_reader_usage_v1().contexts, 1);
    o.retire_settings_read_v1(&b.context, &control(&s), &mut r)
        .unwrap();
    o.retire_settings_reader_v1(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn unrelated_target_publication_does_not_invalidate_a_captured_projection() {
    // Use the already compiled multi-target owner fixture; no synthetic root stamp.
    let mut o = super::super::tests::owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(super::super::tests::multi_target(&mut r), &mut r)
        .unwrap();
    let l = limits();
    o.configure_retention_budget(l.max_pool_bytes, &mut r)
        .unwrap();
    o.configure_settings_reader_v1("engine", "local", l, &mut r)
        .unwrap();
    enable_changes(&mut o, &mut r);
    let a_id = SettingIdentityV1 {
        authority: "engine".into(),
        id: "A".into(),
        incarnation: 1,
    };
    let b_id = SettingIdentityV1 {
        authority: "engine".into(),
        id: "B".into(),
        incarnation: 1,
    };
    let t = o.settings_target(&a_id, &mut r).unwrap();
    let s = scope();
    let p = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    change(&mut o, b_id, 1, false, &mut r);
    let current = prepare(&mut o, &t, &s, true, false, true, &mut r).unwrap();
    let SettingsReadResultV1::Value(value) = o
        .query_settings_read_v1(
            &a.context,
            &a.target,
            &s,
            SettingsReadOperationV1::EffectiveConfiguration { key: "flag" },
            &current,
            &facts(),
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(value.read().boolean(), Some(false));
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
}
