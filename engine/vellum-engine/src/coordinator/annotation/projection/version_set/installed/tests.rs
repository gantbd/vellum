use super::*;
mod shutdown;
mod stop;

#[test]
fn acquisition_validation_capture_never_controls_health_or_issuance() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;

    for capacity in [None, Some(64), Some(1)] {
        for malformed in [false, true] {
            let (mut o, _, a, b, cut) = ready(profile());
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            o.acquisition_result_fault = malformed.then_some(F::Context);
            let result = o.install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &[current(&a), current(&b)],
                },
                &mut r,
            );
            assert_eq!(result.is_none(), malformed);
            assert_eq!(o.is_faulted(), malformed);
            assert_eq!(o.read_result_invariant().is_some(), malformed);
            if capacity == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                let checks: Vec<_> = events
                    .iter()
                    .filter(|e| e.boundary == Boundary::ReadAcquisitionResultValidation)
                    .collect();
                assert_eq!(checks.len(), 1);
                assert_eq!(
                    checks[0].disposition,
                    if malformed {
                        Disposition::Unknown
                    } else {
                        Disposition::Completed
                    }
                );
                assert!(
                    events
                        .iter()
                        .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
                );
            } else if capacity == Some(1) {
                assert!(r.dropped() > 0);
            } else {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}

#[test]
fn malformed_acquisition_refuses_before_registry_or_preparation_commit() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
    for fault in [
        F::Context,
        F::Classes,
        F::Roster,
        F::MemberAlias,
        F::Duplicate,
        F::Document,
        F::StagedBinding,
    ] {
        let (mut o, mut r, a, b, cut) = ready(profile());
        let before = o.version_set_context_usage();
        let prepared = o.version_set_preparation_usage();
        let reserved = o.retention_usage().reserved;
        let generation = o.generation();
        o.acquisition_result_fault = Some(fault);
        assert!(
            o.install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &[current(&a), current(&b)]
                },
                &mut r
            )
            .is_none(),
            "{fault:?}"
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeReadAcquisitionResultInvariant)
        );
        assert_eq!(o.version_set_context_usage(), before);
        assert_eq!(o.version_set_preparation_usage(), prepared);
        assert!(o.prepared_set(&cut).is_ok());
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.generation(), generation);
        assert!(o.discard_prepared_version_set(&cut, &[cleanup(&a), cleanup(&b)], &mut r));
        assert!(o.is_faulted());
    }
}
use crate::coordinator::annotation::projection::tests::{base, cleanup, control, facts};
pub(super) fn profile() -> VersionSetContextLimitsV1 {
    VersionSetContextLimitsV1 {
        max_holds: 8,
        max_page_rows: 16,
        max_row_bytes: 4096,
        max_contexts: 4,
        max_members: 8,
        max_selectors: 32,
        max_issued: 64,
        max_rows: 16,
        max_capture_bytes: 16384,
        max_context_bytes: 8192,
        max_request_bytes: 16384,
        max_reply_bytes: 8192,
        max_selector_bytes: 128,
        max_label_bytes: 128,
        max_pool_bytes: 1048576,
    }
}
fn current(s: &SourceReadScopeV1) -> TrustedVersionSetMemberFactsV1<'_> {
    TrustedVersionSetMemberFactsV1 {
        scope: s,
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
    }
}
fn ready(
    p: VersionSetContextLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourceReadScopeV1,
    PreparedVersionSetId,
) {
    let (mut o, mut r, a, g) = super::super::tests::ready(super::super::tests::profile());
    let bdoc = o.create(b"peer", &mut r).unwrap();
    let mut b = a.clone();
    b.document = bdoc.document;
    o.configure_version_set_contexts(p, &control(), &mut r)
        .unwrap();
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
    (o, r, a, b, id)
}
#[test]
fn installed_native_bounds_cover_the_complete_parent_alias_and_reply_batch() {
    let (o, _, _, _, id) = ready(profile());
    let cut = o.prepared_set(&id).unwrap();
    let request = cut.facts_request_bytes().unwrap();
    let capture = cut.capture_bytes().unwrap();
    let scope = cut.scope_bytes().unwrap();
    let reply = std::mem::size_of::<VersionSetAcquisitionV1>()
        + 2 * std::mem::size_of::<VersionSetMemberAcquisitionV1>();
    for (coordinate, bound) in [
        request,
        capture,
        scope,
        reply,
        std::mem::size_of::<VersionSetContextId>(),
        2,
        1,
        1,
        3,
        3,
    ]
    .into_iter()
    .enumerate()
    {
        for delta in [-1isize, 0, 1] {
            let n = bound.checked_add_signed(delta).unwrap();
            let mut p = profile();
            match coordinate {
                0 => p.max_request_bytes = n,
                1 => p.max_capture_bytes = n,
                2 => p.max_context_bytes = n,
                3 => p.max_reply_bytes = n,
                4 => p.max_selector_bytes = n,
                5 => p.max_members = n,
                6 => p.max_rows = n,
                7 => p.max_contexts = n,
                8 => p.max_selectors = n,
                _ => p.max_issued = n,
            }
            let (mut o, mut r, a, b, id) = ready(p);
            let members = [current(&a), current(&b)];
            let reserved = o.retention_usage().reserved;
            assert_eq!(
                o.install_version_set(
                    &id,
                    &TrustedPreparedVersionSetFactsV1 {
                        projection: &id,
                        members: &members
                    },
                    &mut r
                )
                .is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                o.version_set_context_usage().issued_lifetime,
                if delta < 0 { 0 } else { 3 }
            );
            assert_eq!(
                o.version_set_preparation_usage().prepared,
                usize::from(delta < 0)
            );
            assert_eq!(o.retention_usage().reserved, reserved);
        }
    }
}
#[test]
fn installation_private_fault_and_capacity_surprise_preserve_preparation_and_both_charges() {
    for trap in [false, true] {
        let (mut o, mut r, a, b, id) = ready(profile());
        let reserved = o.retention_usage().reserved;
        let generation = o.generation();
        if trap {
            o.version_set_install_fault = true;
        } else {
            let VersionSetContextState::Active(p) = &mut o.version_set_contexts else {
                unreachable!()
            };
            p.aliases = Vec::new();
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            o.install_version_set(
                &id,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &id,
                    members: &[current(&a), current(&b)],
                },
                &mut r,
            )
        }));
        assert_eq!(result.is_err(), trap);
        assert!(o.is_faulted());
        assert_eq!(o.version_set_context_usage().contexts, 0);
        assert_eq!(o.version_set_context_usage().aliases, 0);
        assert_eq!(o.version_set_context_usage().issued_lifetime, 0);
        assert_eq!(o.version_set_preparation_usage().prepared, 1);
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.generation(), generation);
        assert!(o.discard_prepared_version_set(&id, &[cleanup(&a), cleanup(&b)], &mut r));
        assert_eq!(o.retire_version_set_contexts(&control(), &mut r), Ok(true));
        assert!(o.is_faulted());
    }
}
#[test]
fn installed_source_and_common_cut_are_original_and_private_alias_drift_fences() {
    let (mut o, mut r, a, b, id) = ready(profile());
    let generation = o.prepared_set(&id).unwrap().cut;
    let version = o.prepared_set(&id).unwrap().members[0]
        .value
        .source
        .as_ref()
        .unwrap()
        .1
        .clone();
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
    let acq = o
        .install_version_set(
            &id,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &id,
                members: &[current(&a), current(&b)],
            },
            &mut r,
        )
        .unwrap();
    let p = o.version_context_pool().unwrap();
    let row = p.rows[acq.context.slot].as_ref().unwrap();
    assert_eq!(row.value.cut, generation);
    assert_eq!(
        row.value.members[0].value.source.as_ref().unwrap().0.text(),
        "ab"
    );
    assert!(row.value.members[1].value.source.is_none());
    let VersionSetContextState::Active(p) = &mut o.version_set_contexts else {
        unreachable!()
    };
    p.aliases[acq.participants[1].snapshot.slot]
        .as_mut()
        .unwrap()
        .index = usize::MAX;
    assert!(!o.validate_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &TrustedVersionSetContextFactsV1 {
            context: &acq.context,
            members: &[current(&a), current(&b)]
        },
        &mut r
    ));
    assert!(o.is_faulted());
}
#[test]
fn actual_peer_disposal_or_owner_fault_fences_all_aliases_and_protected_cleanup_remains_available()
{
    for fault in [false, true] {
        let (mut o, mut r, a, b, id) = ready(profile());
        let acq = o
            .install_version_set(
                &id,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &id,
                    members: &[current(&a), current(&b)],
                },
                &mut r,
            )
            .unwrap();
        if fault {
            o.faulted = true;
        } else {
            o.dispose(&b.document, &mut r).unwrap();
        }
        assert!(!o.validate_version_set_context(
            VersionSetControlTargetV1::Member(&acq.participants[0].snapshot),
            &TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &[current(&a), current(&b)]
            },
            &mut r
        ));
        assert_eq!(o.version_set_context_usage().stopped, 1);
        assert!(o.retire_version_set_context(
            VersionSetControlTargetV1::Member(&acq.participants[1].snapshot),
            &[cleanup(&a), cleanup(&b)],
            &mut r
        ));
        assert_eq!(o.version_set_context_usage().issued_lifetime, 3);
        assert_eq!(o.is_faulted(), fault);
    }
}
#[test]
fn installed_pool_local_and_parent_funding_have_exact_native_boundaries() {
    let p = profile();
    let charge = p.required_pool_bytes().unwrap() + "engine".len() + "local".len();
    let prep = super::super::tests::profile();
    let prep_charge = prep.required_pool_bytes(1048576).unwrap() + "engine".len() + "local".len();
    for local in [false, true] {
        for delta in [-1isize, 0, 1] {
            let (mut o, mut r, _, _, _) = base();
            let n = charge.checked_add_signed(delta).unwrap();
            let mut limits = p;
            if local {
                limits.max_pool_bytes = n;
            }
            o.configure_retention_budget(if local { usize::MAX } else { prep_charge + n }, &mut r)
                .unwrap();
            o.configure_semantic_root_budget(1048576, &mut r).unwrap();
            o.configure_version_set_preparation("engine", "local", prep, &control(), &mut r)
                .unwrap();
            assert_eq!(
                o.configure_version_set_contexts(limits, &control(), &mut r)
                    .is_ok(),
                delta >= 0
            );
            assert_eq!(
                o.retention_usage().reserved,
                prep_charge + if delta >= 0 { charge } else { 0 }
            );
        }
    }
}
