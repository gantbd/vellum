use super::*;
#[test]
fn semantic_root_counts_current_settings_once_not_held_interpretations() {
    let (mut o, mut r) = setup();
    let before = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(
        before.settings_bytes,
        o.settings_usage().unwrap().logical_bytes
    );
    let p = limits(&o, 4, 4);
    fund(&mut o, p, &mut r);
    let old = o.acquire_settings_interpretation(&mut r).unwrap();
    let alias = old.clone();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    set(&mut o, "left", 2.0, &mut r);
    let current = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(
        current.settings_bytes,
        o.settings_usage().unwrap().logical_bytes
    );
    o.release_settings_interpretation(&alias, &mut r).unwrap();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), current);
    drop(old);
}
fn limits(o: &SourceCoordinator, issued: usize, retained: usize) -> SettingsRetentionLimitsV1 {
    let mut p = SettingsRetentionLimitsV1 {
        max_issued: issued,
        max_retained: retained,
        max_snapshot_bytes: o.settings_usage().unwrap().logical_bytes + 4096,
        max_pool_bytes: usize::MAX,
    };
    p.max_pool_bytes = p.required_pool_bytes().unwrap();
    p
}
fn fund(o: &mut SourceCoordinator, p: SettingsRetentionLimitsV1, r: &mut Recorder) {
    o.configure_retention_budget(p.required_pool_bytes().unwrap(), r)
        .unwrap();
    o.configure_settings_retention(p, r).unwrap();
}
#[test]
fn historical_interpretations_keep_actual_values_origins_and_basis_after_activation() {
    let (mut o, mut r) = setup();
    let p = limits(&o, 4, 4);
    fund(&mut o, p, &mut r);
    let source = o.create(b"untouched", &mut r).unwrap();
    let old = o.acquire_settings_interpretation(&mut r).unwrap();
    let alias = old.clone();
    let before = o.settings_interpretation_basis(&old, &mut r).unwrap();
    set(&mut o, "left", 2.0, &mut r);
    let fresh = o.acquire_settings_interpretation(&mut r).unwrap();
    assert_eq!(
        o.settings_interpretation_basis(&alias, &mut r).unwrap(),
        before
    );
    assert_eq!(
        o.settings_interpretation_read(&old, &id("A"), "unit", &mut r)
            .unwrap()
            .value
            .number(),
        Some(1.0)
    );
    assert_eq!(
        o.settings_interpretation_read(&fresh, &id("A"), "unit", &mut r)
            .unwrap()
            .value
            .number(),
        Some(2.0)
    );
    assert_eq!(
        o.settings_interpretation_basis(&fresh, &mut r)
            .unwrap()
            .configuration_generation,
        2
    );
    assert_eq!(o.settings_retention_usage().retained, 2);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "untouched"
    );
    o.release_settings_interpretation(&alias, &mut r).unwrap();
    assert_eq!(
        o.settings_interpretation_basis(&old, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(
        o.release_settings_interpretation(&old, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(o.settings_retention_usage().retained, 1);
}
#[test]
fn pool_funding_uses_exact_parent_headroom_and_failed_configuration_leaves_no_prefix() {
    for extra in [-1isize, 0, 1] {
        let (mut o, mut r) = setup();
        let p = limits(&o, 2, 2);
        let bytes = p.required_pool_bytes().unwrap();
        assert_eq!(
            o.configure_settings_retention(p, &mut r),
            Err(SettingsError::Unavailable)
        );
        o.configure_retention_budget(bytes.checked_add_signed(extra).unwrap(), &mut r)
            .unwrap();
        let generation = o.generation();
        if extra < 0 {
            assert_eq!(
                o.configure_settings_retention(p, &mut r),
                Err(SettingsError::Limit)
            );
            assert_eq!(o.retention_usage().reserved, 0);
            assert_eq!(
                o.acquire_settings_interpretation(&mut r),
                Err(SettingsError::Unavailable)
            );
        } else {
            assert_eq!(o.configure_settings_retention(p, &mut r), Ok(true));
            assert_eq!(o.configure_settings_retention(p, &mut r), Ok(false));
            assert_eq!(o.retention_usage().reserved, bytes);
            assert_eq!(o.settings_retention_usage().reserved_pool_bytes, bytes);
        }
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
    }
    let (mut o, mut r) = setup();
    let mut p = limits(&o, 2, 2);
    p.max_pool_bytes -= 1;
    assert_eq!(p.required_pool_bytes(), Err(SettingsError::Limit));
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    assert_eq!(
        o.configure_settings_retention(p, &mut r),
        Err(SettingsError::Limit)
    );
    p.max_issued = usize::MAX;
    p.max_pool_bytes = usize::MAX;
    assert_eq!(p.required_pool_bytes(), Err(SettingsError::Limit));
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn release_frees_live_capacity_not_lifetime_or_pool_funding_and_retirement_is_spent() {
    let (mut o, mut r) = setup();
    let p = limits(&o, 2, 1);
    fund(&mut o, p, &mut r);
    let bytes = o.retention_usage().reserved;
    let first = o.acquire_settings_interpretation(&mut r).unwrap();
    assert_eq!(
        o.acquire_settings_interpretation(&mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(
        o.retire_settings_retention(&mut r),
        Err(SettingsError::Unavailable)
    );
    o.release_settings_interpretation(&first, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, bytes);
    let second = o.acquire_settings_interpretation(&mut r).unwrap();
    assert_ne!(first, second);
    o.release_settings_interpretation(&second, &mut r).unwrap();
    assert_eq!(
        o.acquire_settings_interpretation(&mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(o.settings_retention_usage().issued_lifetime, 2);
    assert_eq!(o.retire_settings_retention(&mut r), Ok(true));
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.settings_retention_usage().closed);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 2);
    assert_eq!(o.retire_settings_retention(&mut r), Ok(false));
    assert_eq!(
        o.configure_settings_retention(p, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(
        o.prepare_settings_change(&[], &mut r).err(),
        Some(SettingsError::Unavailable)
    );
    assert_eq!(read(&o, "A", &mut r), 1.0);
}
#[test]
fn all_real_preparations_block_retirement_including_ones_created_before_pool_configuration() {
    let (mut o, mut r) = setup();
    let early = o.prepare_settings_change(&[], &mut r).unwrap();
    let p = limits(&o, 4, 2);
    fund(&mut o, p, &mut r);
    let invocation = o.issue_invocation(&mut r).unwrap();
    let changes = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    o.submit_settings_change(&invocation, &changes, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 2);
    assert_eq!(
        o.retire_settings_retention(&mut r),
        Err(SettingsError::Unavailable)
    );
    drop(early);
    assert_eq!(o.settings_retention_usage().open_preparations, 1);
    o.stop_invocation(&invocation, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.retire_settings_retention(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn snapshot_size_limits_do_not_issue_or_reclaim_live_history_and_foreign_handles_refuse() {
    let (mut o, mut r) = setup();
    let mut p = limits(&o, 4, 2);
    p.max_snapshot_bytes = o.settings_usage().unwrap().logical_bytes - 1;
    fund(&mut o, p, &mut r);
    assert_eq!(
        o.acquire_settings_interpretation(&mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    let (mut foreign, mut fr) = setup();
    let p = limits(&foreign, 4, 2);
    fund(&mut foreign, p, &mut fr);
    let handle = foreign.acquire_settings_interpretation(&mut fr).unwrap();
    assert_eq!(
        o.settings_interpretation_basis(&handle, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(
        o.release_settings_interpretation(&handle, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(foreign.settings_retention_usage().retained, 1);
    assert_eq!(
        o.configure_retention_budget(0, &mut r),
        Err(RetentionErrorV1::Unavailable)
    );
}
#[test]
fn retained_settings_capacity_and_control_have_identical_outcomes_with_optional_capture() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let p = limits(&o, 2, 1);
        fund(&mut o, p, &mut r);
        let handle = o.acquire_settings_interpretation(&mut r).unwrap();
        assert_eq!(
            o.acquire_settings_interpretation(&mut r),
            Err(SettingsError::Limit)
        );
        assert_eq!(
            o.settings_interpretation_read(&handle, &id("A"), "unit", &mut r)
                .unwrap()
                .value
                .number(),
            Some(1.0)
        );
        o.release_settings_interpretation(&handle, &mut r).unwrap();
        o.retire_settings_retention(&mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::RetentionConfiguration,
                Boundary::SettingsRetentionConfiguration,
                Boundary::SettingsInterpretationAcquisition,
                Boundary::SettingsInterpretationQuery,
                Boundary::SettingsInterpretationRelease,
                Boundary::SettingsPoolRetirement,
            ] {
                assert!(records.iter().any(|record| record.boundary == boundary));
            }
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
    }
}
