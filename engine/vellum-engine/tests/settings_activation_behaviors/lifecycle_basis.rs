use super::scoped_basis::{context, facts};
use super::*;
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn ready(issued: usize, retained: usize) -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = super::scoped_basis::ready(issued, retained);
    o.configure_settings_layers(limits(), &mut r).unwrap();
    (o, r)
}
fn basis<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsLifecycleBasisFactsV1<'a> {
    TrustedSettingsLifecycleBasisFactsV1 {
        common,
        lifecycle: limits(),
        basis_allowed: Some(true),
    }
}
fn control(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn observation_role_is_independent_from_layer_grants_and_shares_actual_pool_capacity() {
    let (mut o, mut r) = ready(4, 2);
    let c = context();
    let mut common = facts(&c, &[]);
    common.basis_allowed = None;
    common.writable_layers = None;
    let f = basis(&common);
    let before = o.generation();
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let lease = o.acquire_settings_interpretation(&mut r).unwrap();
    assert_eq!(o.settings_retention_usage().retained, 2);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 2);
    assert!(o.capture_settings_lifecycle_basis(&c, &f, &mut r).is_err());
    assert_eq!(o.generation(), before);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    o.release_settings_interpretation(&lease, &mut r).unwrap();
    assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    for _ in 0..2 {
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    }
    assert!(o.capture_settings_lifecycle_basis(&c, &f, &mut r).is_err());
    assert_eq!(o.settings_retention_usage().issued_lifetime, 4);
    assert_eq!(o.settings_retention_usage().retained, 0);
}
#[test]
fn all_context_fields_and_native_profiles_bind_before_recognized_loss() {
    for field in 0..9 {
        let (mut o, mut r) = ready(4, 4);
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let mut wrong = c.clone();
        match field {
            0 => wrong.authority = "foreign".into(),
            1 => wrong.engine = "foreign".into(),
            2 => wrong.recipient = "foreign".into(),
            3 => wrong.purpose = "foreign".into(),
            4 => wrong.destination = "foreign".into(),
            5 => wrong.profile = "foreign".into(),
            6 => wrong.continuity = "foreign".into(),
            _ => {}
        }
        let mut common = facts(&wrong, &[]);
        if field == 7 {
            common.activation.max_work += 1;
        }
        let mut bad = basis(&common);
        if field == 8 {
            bad.lifecycle.max_changes += 1;
        }
        bad.basis_allowed = Some(false);
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &wrong, &bad, &mut r),
            SettingsBasisAvailabilityV1::Unavailable
        );
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert!(!o.is_faulted());
    }
}
#[test]
fn recognized_permission_loss_is_absorbing_but_unknown_and_transient_facts_do_not_poison() {
    for field in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = ready(4, 4);
            let c = context();
            let mut common = facts(&c, &[]);
            let b = o
                .capture_settings_lifecycle_basis(&c, &basis(&common), &mut r)
                .unwrap();
            match field {
                0 => common.authorized = value,
                1 => common.scope = value,
                2 => common.continuous = value,
                3 => common.live = value,
                4 => common.interpretation_current = value,
                5 => common.settings_writable = value,
                6 => common.not_stopped = value,
                7 => common.resources_ready = value,
                8 => common.owner_control_allowed = value,
                _ => {}
            }
            let mut f = basis(&common);
            if field == 9 {
                f.basis_allowed = value;
            }
            assert_eq!(
                o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
                SettingsBasisAvailabilityV1::Unavailable
            );
            let common = facts(&c, &[]);
            let lost = value == Some(false) && (field <= 5 || field == 9);
            assert_eq!(
                o.inspect_settings_lifecycle_basis(&b, &c, &basis(&common), &mut r),
                if lost {
                    SettingsBasisAvailabilityV1::Unavailable
                } else {
                    SettingsBasisAvailabilityV1::Current
                }
            );
        }
    }
}
#[test]
fn source_publication_commutes_but_actual_settings_progress_invalidates_the_pair() {
    let (mut o, mut r) = ready(4, 4);
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let d = o.create(b"A", &mut r).unwrap();
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    set(&mut o, "left", 2.0, &mut r);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "B");
    let fresh = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    assert_ne!(fresh, b);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&fresh, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
}
#[test]
fn foreign_owner_and_retired_handles_refuse_without_poisoning_and_stop_keeps_charge() {
    let (mut o, mut r) = ready(4, 4);
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let (mut other, mut rr) = ready(4, 4);
    let mut bad = basis(&common);
    bad.basis_allowed = Some(false);
    assert_eq!(
        other.inspect_settings_lifecycle_basis(&b, &c, &bad, &mut rr),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(!other.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut rr));
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let held = o.settings_retention_usage();
    assert!(o.stop_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage(), held);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    assert!(!o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
}
#[test]
fn lifecycle_basis_signals_are_bounded_optional_and_do_not_disclose_native_conditions() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, _) = ready(4, 4);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        assert_eq!(format!("{b:?}"), "SettingsLifecycleBasisId(<opaque>)");
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert!(o.stop_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
        assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            for boundary in [
                Boundary::SettingsLifecycleBasisCapture,
                Boundary::SettingsLifecycleBasisInspection,
                Boundary::SettingsLifecycleBasisStop,
                Boundary::SettingsLifecycleBasisRetirement,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary));
            }
        }
        if capacity.is_none() {
            assert!(events.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        assert_eq!(o.settings_retention_usage().retained, 0);
    }
}
