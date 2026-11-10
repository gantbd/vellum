use super::*;
fn setup() -> (SourceCoordinator, SettingsRetentionLimitsV1, Recorder) {
    let mut o = super::super::tests::owner();
    let mut r = Recorder::disabled();
    o.install_initial_settings(super::super::tests::compiled(&mut r), &mut r)
        .unwrap();
    let p = SettingsRetentionLimitsV1 {
        max_issued: 4,
        max_retained: 2,
        max_snapshot_bytes: o.settings_usage().unwrap().logical_bytes,
        max_pool_bytes: 65536,
    };
    o.configure_retention_budget(p.required_pool_bytes().unwrap(), &mut r)
        .unwrap();
    (o, p, r)
}
#[test]
fn genuine_competing_parent_reservation_is_not_replaced_by_a_synthetic_pool_budget() {
    let (mut o, p, mut r) = setup();
    let mut competing = o.retention.reserve(1).unwrap();
    assert_eq!(
        o.configure_settings_retention(p, &mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(o.retention_usage().reserved, 1);
    assert!(matches!(o.settings_retention, RetentionPool::Unconfigured));
    o.retention.release(&mut competing).unwrap();
    o.configure_settings_retention(p, &mut r).unwrap();
    assert_eq!(
        o.retention_usage().reserved,
        p.required_pool_bytes().unwrap()
    );
}
#[test]
fn forged_handle_cannot_read_and_fault_does_not_prevent_protected_discharge() {
    let (mut o, p, mut r) = setup();
    o.configure_settings_retention(p, &mut r).unwrap();
    let handle = o.acquire_settings_interpretation(&mut r).unwrap();
    let forged = SettingsInterpretationId {
        owner: o.namespace.clone(),
        slot: usize::MAX,
    };
    assert_eq!(
        o.settings_interpretation_basis(&forged, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(
        o.release_settings_interpretation(&forged, &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(o.settings_retention_usage().retained, 1);
    o.faulted = true;
    assert_eq!(
        o.settings_interpretation_basis(&handle, &mut r),
        Err(SettingsError::Faulted)
    );
    assert_eq!(
        o.acquire_settings_interpretation(&mut r),
        Err(SettingsError::Faulted)
    );
    o.release_settings_interpretation(&handle, &mut r).unwrap();
    o.retire_settings_retention(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.is_faulted());
}
#[test]
fn foreign_funding_token_fences_without_issuing_a_lease_or_refunding_the_wrong_parent() {
    let (mut o, p, mut r) = setup();
    o.configure_settings_retention(p, &mut r).unwrap();
    let (mut foreign, _, _) = setup();
    let wrong = foreign
        .retention
        .reserve(p.required_pool_bytes().unwrap())
        .unwrap();
    let RetentionPool::Active(pool) = &mut o.settings_retention else {
        unreachable!()
    };
    let original = std::mem::replace(&mut pool.reservation, wrong);
    let usage = o.retention_usage();
    let generation = o.generation();
    assert_eq!(
        o.acquire_settings_interpretation(&mut r),
        Err(SettingsError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.settings_retention_usage().issued_lifetime, 0);
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.retire_settings_retention(&mut r),
        Err(SettingsError::Faulted)
    );
    assert_eq!(o.retention_usage(), usage);
    assert_eq!(
        foreign.retention_usage().reserved,
        p.required_pool_bytes().unwrap()
    );
    let RetentionPool::Active(pool) = &mut o.settings_retention else {
        unreachable!()
    };
    pool.reservation = original;
    o.retire_settings_retention(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
