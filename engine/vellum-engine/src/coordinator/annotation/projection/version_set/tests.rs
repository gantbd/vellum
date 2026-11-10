use super::*;
use crate::coordinator::annotation::projection::tests::{base, cleanup, control, facts};
pub(super) fn profile() -> VersionSetPreparationLimitsV1 {
    VersionSetPreparationLimitsV1 {
        max_prepared: 4,
        max_issued: 8,
        max_members: 8,
        max_rows: 16,
        max_root_bytes: 1048576,
        max_capture_bytes: 16384,
        max_context_bytes: 8192,
        max_request_bytes: 16384,
        max_selector_bytes: 128,
        max_label_bytes: 128,
        max_pool_bytes: 8388608,
        source: SourceProjectionLimitsV1 {
            max_source_bytes: 4096,
            max_request_bytes: 4096,
            max_reply_bytes: 4096,
            max_scan_bytes: 4096,
        },
    }
}
pub(super) fn ready(
    p: VersionSetPreparationLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    Vec<AnnotationFacetGrantV1>,
) {
    let (mut o, mut r, _, s, g) = base();
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_version_set_preparation("engine", "local", p, &control(), &mut r)
        .unwrap();
    (o, r, s, g)
}
#[test]
fn captured_values_share_one_actual_cut_and_never_refresh_or_retain_excluded_classes() {
    let (mut o, mut r, a, g) = ready(profile());
    let peer = o.create(b"peer", &mut r).unwrap();
    let mut b = a.clone();
    b.document = peer.document;
    let generation = o.generation();
    let id = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Combined,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &[facts(&a, &g), facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let before = o.prepared_set(&id).unwrap();
    let version = before.members[0].value.source.as_ref().unwrap().1.clone();
    assert_eq!(before.cut, generation);
    assert!(before.members[1].value.source.is_none());
    assert_eq!(before.members[0].value.rows.len(), 1);
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: version,
                edits: vec![crate::source::Replacement {
                    start: 0,
                    end: 2,
                    expected: "ab".into(),
                    inserted: "zz".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let cut = o.prepared_set(&id).unwrap();
    assert_eq!(cut.cut, generation);
    assert_eq!(cut.members[0].value.source.as_ref().unwrap().0.text(), "ab");
    let only = o
        .prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Source,
            }],
            &[facts(&a, &[])],
            &mut r,
        )
        .unwrap();
    assert!(
        o.prepared_set(&only).unwrap().members[0]
            .value
            .rows
            .is_empty()
    );
}
#[test]
fn local_and_parent_funding_use_the_actual_parent_ceiling_even_for_a_smaller_input_limit() {
    let mut p = profile();
    p.max_root_bytes = 1;
    let charge = p.required_pool_bytes(1048576).unwrap() + "engine".len() + "local".len();
    assert!(charge > 2 * 1048576);
    for local in [false, true] {
        for delta in [-1isize, 0, 1] {
            let (mut o, mut r, _, s, g) = base();
            let n = charge.checked_add_signed(delta).unwrap();
            let mut limits = p;
            if local {
                limits.max_pool_bytes = n;
            }
            o.configure_retention_budget(if local { usize::MAX } else { n }, &mut r)
                .unwrap();
            o.configure_semantic_root_budget(1048576, &mut r).unwrap();
            assert_eq!(
                o.configure_version_set_preparation("engine", "local", limits, &control(), &mut r)
                    .is_ok(),
                delta >= 0
            );
            if delta >= 0 {
                assert_eq!(
                    o.version_set_preparation_usage().reserved_pool_bytes,
                    charge
                );
                assert!(
                    o.prepare_version_set(
                        &[VersionSetParticipantV1 {
                            scope: &s,
                            data: AnnotationProjectionClassesV1::Combined
                        }],
                        &[facts(&s, &g)],
                        &mut r
                    )
                    .is_none()
                );
                assert_eq!(o.version_set_preparation_usage().issued_lifetime, 0);
            } else {
                assert_eq!(o.retention_usage().reserved, 0);
            }
        }
    }
}
#[test]
fn native_request_and_aggregate_context_boundaries_precede_staging() {
    let (_, _, s, g) = ready(profile());
    let scope = scope_bytes(&s).unwrap();
    let request = std::mem::size_of::<PreparedVersionSetId>()
        + scope
        + std::mem::size_of::<AnnotationProjectionClassesV1>()
        + g.len() * std::mem::size_of::<AnnotationFacetGrantV1>();
    for coordinate in 0..2 {
        for delta in [-1isize, 0, 1] {
            let mut p = profile();
            let n = if coordinate == 0 { request } else { scope }
                .checked_add_signed(delta)
                .unwrap();
            if coordinate == 0 {
                p.max_request_bytes = n;
            } else {
                p.max_context_bytes = n;
            }
            let (mut o, mut r, s, g) = ready(p);
            assert_eq!(
                o.prepare_version_set(
                    &[VersionSetParticipantV1 {
                        scope: &s,
                        data: AnnotationProjectionClassesV1::Combined
                    }],
                    &[facts(&s, &g)],
                    &mut r
                )
                .is_some(),
                delta >= 0
            );
            assert_eq!(
                o.version_set_preparation_usage().issued_lifetime,
                usize::from(delta >= 0)
            );
        }
    }
}
#[test]
fn zero_capacity_still_funds_workspace_and_staging_fault_installs_no_partial_cut() {
    let mut p = profile();
    p.max_prepared = 0;
    p.max_issued = 0;
    let (mut o, mut r, s, g) = ready(p);
    assert!(o.version_set_preparation_usage().reserved_pool_bytes > 2 * 1048576);
    assert!(
        o.prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &s,
                data: AnnotationProjectionClassesV1::Combined
            }],
            &[facts(&s, &g)],
            &mut r
        )
        .is_none()
    );
    let (mut o, mut r, s, g) = ready(profile());
    let reserved = o.retention_usage().reserved;
    let generation = o.generation();
    o.version_set_preparation_fault = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        o.prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &s,
                data: AnnotationProjectionClassesV1::Combined,
            }],
            &[facts(&s, &g)],
            &mut r,
        )
    }));
    assert!(result.is_err());
    assert!(o.is_faulted());
    assert_eq!(o.version_set_preparation_usage().prepared, 0);
    assert_eq!(o.version_set_preparation_usage().issued_lifetime, 0);
    assert_eq!(o.retention_usage().reserved, reserved);
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.retire_version_set_preparation(&control(), &mut r),
        Ok(true)
    );
    assert!(o.is_faulted());
}
#[test]
fn private_coherence_drift_faults_and_protected_cleanup_does_not_heal_or_reopen() {
    let (mut o, mut r, s, g) = ready(profile());
    let id = o
        .prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &s,
                data: AnnotationProjectionClassesV1::Combined,
            }],
            &[facts(&s, &g)],
            &mut r,
        )
        .unwrap();
    let VersionSetPreparationState::Active(p) = &mut o.version_set_preparations else {
        unreachable!()
    };
    p.cuts[id.slot].as_mut().unwrap().cut = u64::MAX;
    let f = TrustedVersionSetMemberFactsV1 {
        scope: &s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    assert!(!o.validate_prepared_version_set(
        &id,
        &TrustedPreparedVersionSetFactsV1 {
            projection: &id,
            members: &[f]
        },
        &mut r
    ));
    assert!(o.is_faulted());
    assert!(o.discard_prepared_version_set(&id, &[cleanup(&s)], &mut r));
    assert_eq!(
        o.retire_version_set_preparation(&control(), &mut r),
        Ok(true)
    );
    assert_eq!(o.version_set_preparation_usage().issued_lifetime, 1);
    assert!(
        o.configure_version_set_preparation("engine", "local", profile(), &control(), &mut r)
            .is_err()
    );
    assert!(o.is_faulted());
}
