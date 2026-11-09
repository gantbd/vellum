use super::*;
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "alice".into(),
        purpose: "edit".into(),
        destination: "client".into(),
        profile: "v1".into(),
        continuity: "session".into(),
    }
}
fn access() -> SettingsBasisLimitsV1 {
    SettingsBasisLimitsV1 {
        max_label_bytes: 128,
        max_context_bytes: 4096,
        max_grants: 16,
        max_work: 65536,
    }
}
pub(super) fn facts<'a>(
    context: &'a SettingsExecutionContextV1,
    layers: &'a [SettingIdentityV1],
) -> TrustedSettingsBasisFactsV1<'a> {
    TrustedSettingsBasisFactsV1 {
        context,
        activation: profile(),
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        settings_writable: Some(true),
        basis_allowed: Some(true),
        writable_layers: Some(layers),
    }
}
pub(super) fn ready(issued: usize, retained: usize) -> (SourceCoordinator, Recorder) {
    ready_with_root_budget(issued, retained, Some(1_048_576))
}
pub(super) fn ready_with_root_budget(
    issued: usize,
    retained: usize,
    ceiling: Option<usize>,
) -> (SourceCoordinator, Recorder) {
    ready_with_headroom(issued, retained, ceiling, 0)
}
pub(super) fn ready_with_headroom(
    issued: usize,
    retained: usize,
    ceiling: Option<usize>,
    extra: usize,
) -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = setup_with_root_budget(profile(), false, ceiling);
    let mut pool = SettingsRetentionLimitsV1 {
        max_issued: issued,
        max_retained: retained,
        max_snapshot_bytes: 65536,
        max_pool_bytes: usize::MAX,
    };
    pool.max_pool_bytes = pool.required_pool_bytes().unwrap();
    o.configure_retention_budget(pool.max_pool_bytes.checked_add(extra).unwrap(), &mut r)
        .unwrap();
    o.configure_settings_retention(pool, &mut r).unwrap();
    o.configure_settings_bases("local", access(), &mut r)
        .unwrap();
    (o, r)
}
#[test]
fn issued_basis_inspection_releases_only_availability_and_shares_interpretation_capacity() {
    let (mut o, mut r) = ready(3, 2);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let generation = o.generation();
    let b = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let interpretation = o.acquire_settings_interpretation(&mut r).unwrap();
    assert_eq!(o.settings_retention_usage().issued_lifetime, 2);
    assert_eq!(o.settings_retention_usage().retained, 2);
    assert_eq!(o.generation(), generation);
    assert!(
        o.capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .is_err()
    );
    o.release_settings_interpretation(&interpretation, &mut r)
        .unwrap();
    let other = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    assert_ne!(b, other);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 3);
}
#[test]
fn recognized_observed_loss_is_absorbing_but_transient_readiness_is_not() {
    let (mut o, mut r) = ready(4, 4);
    let c = context();
    let layers = [id("left")];
    let mut f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    f.resources_ready = Some(false);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    f.resources_ready = Some(true);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    f.authorized = Some(false);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    f.authorized = Some(true);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert_eq!(o.settings_retention_usage().retained, 1);
}
pub(super) fn control(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn all_current_guards_require_true_but_only_explicit_recognized_losses_close_a_basis() {
    for field in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = ready(4, 4);
            let c = context();
            let layers = [id("left")];
            let mut f = facts(&c, &layers);
            let b = o
                .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
                .unwrap();
            match field {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.not_stopped = value,
                5 => f.interpretation_current = value,
                6 => f.resources_ready = value,
                7 => f.owner_control_allowed = value,
                8 => f.settings_writable = value,
                _ => f.basis_allowed = value,
            }
            assert!(
                o.capture_settings_write_basis(&id("left"), &c, &f, &mut r)
                    .is_err()
            );
            assert_eq!(
                o.inspect_settings_write_basis(&b, &c, &f, &mut r),
                SettingsBasisAvailabilityV1::Unavailable
            );
            let f = facts(&c, &layers);
            let closed = value == Some(false) && ![4, 6, 7].contains(&field);
            assert_eq!(
                o.inspect_settings_write_basis(&b, &c, &f, &mut r),
                if closed {
                    SettingsBasisAvailabilityV1::Unavailable
                } else {
                    SettingsBasisAvailabilityV1::Current
                }
            );
        }
    }
}
#[test]
fn all_seven_context_fields_bind_before_loss_observation_and_foreign_facts_cannot_poison() {
    for field in 0..7 {
        let (mut o, mut r) = ready(4, 4);
        let c = context();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        let b = o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap();
        let mut foreign = c.clone();
        match field {
            0 => foreign.authority.push('x'),
            1 => foreign.engine.push('x'),
            2 => foreign.recipient.push('x'),
            3 => foreign.purpose.push('x'),
            4 => foreign.destination.push('x'),
            5 => foreign.profile.push('x'),
            _ => foreign.continuity.push('x'),
        }
        let mut denied = facts(&foreign, &layers);
        denied.authorized = Some(false);
        assert_eq!(
            o.inspect_settings_write_basis(&b, &foreign, &denied, &mut r),
            SettingsBasisAvailabilityV1::Unavailable
        );
        assert_eq!(
            o.inspect_settings_write_basis(&b, &c, &denied, &mut r),
            SettingsBasisAvailabilityV1::Unavailable
        );
        assert!(!o.stop_settings_write_basis(&b, &foreign, &control(&foreign), &mut r));
        assert!(!o.retire_settings_write_basis(&b, &foreign, &control(&foreign), &mut r));
        assert_eq!(
            o.inspect_settings_write_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
}
#[test]
fn one_owner_can_serve_separate_authorized_recipients_without_crossing_contexts_or_owners() {
    let (mut o, mut r) = ready(4, 4);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let a = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let mut second = c.clone();
    second.recipient = "bob".into();
    second.destination = "other".into();
    assert!(
        o.capture_settings_write_basis(&id("left"), &second, &f, &mut r)
            .is_err()
    );
    let b = o
        .capture_settings_write_basis(&id("left"), &second, &facts(&second, &layers), &mut r)
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert_eq!(
        o.inspect_settings_write_basis(&a, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let (mut other, mut rr) = ready(4, 4);
    assert_eq!(
        other.inspect_settings_write_basis(&a, &c, &f, &mut rr),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(!other.retire_settings_write_basis(&a, &c, &control(&c), &mut rr));
    assert_eq!(
        o.inspect_settings_write_basis(&b, &second, &facts(&second, &layers), &mut r),
        SettingsBasisAvailabilityV1::Current
    );
}
#[test]
fn unknown_malformed_or_foreign_profile_packets_do_not_assert_loss_but_known_missing_grants_do() {
    let (mut o, mut r) = ready(4, 4);
    let c = context();
    let layers = [id("left")];
    let mut f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    f.writable_layers = None;
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    f.writable_layers = Some(&layers);
    f.activation.max_work -= 1;
    f.authorized = Some(false);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    let mut malformed = id("left");
    malformed.incarnation = 0;
    let bad = [malformed];
    f.activation = profile();
    f.writable_layers = Some(&bad);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    f = facts(&c, &layers);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    f.writable_layers = Some(&[]);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    f.writable_layers = Some(&layers);
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
}
#[test]
fn actual_contributor_and_interpretation_loss_close_but_disjoint_or_source_progress_survives() {
    for producer in ["source", "right", "left", "host", "aba", "schema"] {
        let (mut o, mut r) = ready(4, 4);
        let c = context();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        let b = o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap();
        match producer {
            "source" => {
                o.create(b"S", &mut r).unwrap();
            }
            "aba" => {
                set(&mut o, "left", 2.0, &mut r);
                set(&mut o, "left", 1.0, &mut r);
            }
            "schema" => {
                o.configure_settings_definitions(
                    SettingsDefinitionLimitsV1 {
                        max_request_bytes: 65536,
                        max_plan_bytes: 1_048_576,
                        max_work: 1_048_576,
                    },
                    &mut r,
                )
                .unwrap();
                let mut d = super::definition::declaration();
                d.schemas[0].sensitive = true;
                let expected = o.settings_basis(&mut r).unwrap().interpretation();
                let p = o
                    .prepare_settings_definition(
                        expected,
                        &d,
                        &ReadDependencyRequestV3 {
                            predicates: vec![],
                            budget: ReadDependencyBudget {
                                max_predicates: 8,
                                max_work: 4096,
                            },
                        },
                        &mut r,
                    )
                    .unwrap();
                o.commit_settings_definition(p, &mut r).unwrap();
            }
            layer => {
                set(&mut o, layer, 1.0, &mut r);
            } // equal effective value still changes authored origin/generation
        }
        let valid = ["source", "right"].contains(&producer);
        assert_eq!(
            o.inspect_settings_write_basis(&b, &c, &f, &mut r),
            if valid {
                SettingsBasisAvailabilityV1::Current
            } else {
                SettingsBasisAvailabilityV1::Unavailable
            },
            "{producer}"
        );
    }
}
#[test]
fn stop_keeps_capacity_and_only_protected_drain_retires_without_refunding_issuance_or_reopening_pool()
 {
    let (mut o, mut r) = ready(1, 1);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let mut denied = control(&c);
    denied.owner_control_allowed = Some(false);
    assert!(!o.stop_settings_write_basis(&b, &c, &denied, &mut r));
    assert_eq!(
        o.inspect_settings_write_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert!(o.stop_settings_write_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().retained, 1);
    let mut ctrl = control(&c);
    ctrl.retention_released = Some(false);
    assert!(!o.retire_settings_write_basis(&b, &c, &ctrl, &mut r));
    ctrl.retention_released = Some(true);
    ctrl.delivery_drained = None;
    assert!(!o.retire_settings_write_basis(&b, &c, &ctrl, &mut r));
    assert!(o.retire_settings_write_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().retained, 0);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 1);
    assert!(
        o.capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
    assert!(
        o.capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .is_err()
    );
    assert!(!o.retire_settings_write_basis(&b, &c, &control(&c), &mut r));
}
#[test]
fn basis_signals_and_debug_are_content_free_with_disabled_available_or_saturated_capture() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready(4, 4);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let mut c = context();
        c.recipient = "private-recipient-label".into();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        let b = o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap();
        assert_eq!(
            o.inspect_settings_write_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert!(!format!("{o:?} {b:?} {c:?}").contains("private-recipient-label"));
        o.stop_settings_write_basis(&b, &c, &control(&c), &mut r);
        o.retire_settings_write_basis(&b, &c, &control(&c), &mut r);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SettingsBasisCapture,
                Boundary::SettingsBasisInspection,
                Boundary::SettingsBasisStop,
                Boundary::SettingsBasisRetirement,
            ] {
                assert!(records.iter().any(|x| x.boundary == boundary));
            }
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        assert_eq!(o.settings_retention_usage().retained, 0);
    }
}
