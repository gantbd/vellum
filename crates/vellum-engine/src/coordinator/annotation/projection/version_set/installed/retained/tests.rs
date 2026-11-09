use super::*;

#[test]
fn acquisition_validation_capture_never_controls_health_or_issuance() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;

    for capacity in [None, Some(64), Some(1)] {
        for malformed in [false, true] {
            let mut f = owner(profile());
            f.r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            f.o.acquisition_result_fault = malformed.then_some(F::Context);
            let result = f.o.capture_retained_version_set(
                &f.original.context,
                &[
                    VersionSetParticipantV1 {
                        scope: &f.b,
                        data: AnnotationProjectionClassesV1::Source,
                    },
                    VersionSetParticipantV1 {
                        scope: &f.a,
                        data: AnnotationProjectionClassesV1::Annotations,
                    },
                ],
                &TrustedVersionSetContextFactsV1 {
                    context: &f.original.context,
                    members: &[current(&f.a), current(&f.b)],
                },
                Some(true),
                &mut f.r,
            );
            assert_eq!(result.is_none(), malformed);
            assert_eq!(f.o.is_faulted(), malformed);
            assert_eq!(f.o.read_result_invariant().is_some(), malformed);
            if capacity == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
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
                assert!(f.r.dropped() > 0);
            } else {
                assert_eq!(f.r.queued(), 0);
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
        F::RetainedParent,
    ] {
        let mut f = owner(profile());
        let before = f.o.version_set_context_usage();

        let reserved = f.o.retention_usage().reserved;
        let generation = f.o.generation();
        f.o.acquisition_result_fault = Some(fault);
        assert!(
            f.o.capture_retained_version_set(
                &f.original.context,
                &[
                    VersionSetParticipantV1 {
                        scope: &f.b,
                        data: AnnotationProjectionClassesV1::Source
                    },
                    VersionSetParticipantV1 {
                        scope: &f.a,
                        data: AnnotationProjectionClassesV1::Annotations
                    },
                ],
                &TrustedVersionSetContextFactsV1 {
                    context: &f.original.context,
                    members: &[current(&f.a), current(&f.b)]
                },
                Some(true),
                &mut f.r
            )
            .is_none(),
            "{fault:?}"
        );
        assert!(f.o.is_faulted());
        assert_eq!(
            f.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeReadAcquisitionResultInvariant)
        );
        assert_eq!(f.o.version_set_context_usage(), before);

        assert_eq!(f.o.retention_usage().reserved, reserved);
        assert_eq!(f.o.generation(), generation);
        assert!(f.o.stop_version_set_context(
            VersionSetControlTargetV1::Context(&f.original.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r,
        ));
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.original.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert!(f.o.is_faulted());
    }
}
use crate::coordinator::annotation::projection::tests::{cleanup, control, facts};
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
struct Fixture {
    o: SourceCoordinator,
    r: Recorder,
    a: SourceReadScopeV1,
    b: SourceReadScopeV1,
    original: VersionSetAcquisitionV1,
}
fn profile() -> VersionSetContextLimitsV1 {
    super::super::tests::profile()
}
fn owner(p: VersionSetContextLimitsV1) -> Fixture {
    let (mut o, mut r, a, g) =
        super::super::super::tests::ready(super::super::super::tests::profile());
    let d = o.create(b"peer", &mut r).unwrap();
    let mut b = a.clone();
    b.document = d.document;
    o.configure_version_set_contexts(p, &control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Combined,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Source,
                },
            ],
            &[facts(&a, &g), facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let original = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[current(&a), current(&b)],
            },
            &mut r,
        )
        .unwrap();
    Fixture {
        o,
        r,
        a,
        b,
        original,
    }
}
fn capture(f: &mut Fixture) -> Option<VersionSetAcquisitionV1> {
    f.o.capture_retained_version_set(
        &f.original.context,
        &[VersionSetParticipantV1 {
            scope: &f.a,
            data: AnnotationProjectionClassesV1::Annotations,
        }],
        &TrustedVersionSetContextFactsV1 {
            context: &f.original.context,
            members: &[current(&f.a), current(&f.b)],
        },
        Some(true),
        &mut f.r,
    )
}
#[test]
fn complete_retained_input_and_fresh_parent_alias_issuance_have_exact_native_boundaries() {
    let f = owner(profile());
    let p = f.o.version_context_pool().unwrap();
    let row = p.rows[f.original.context.slot].as_ref().unwrap();
    let request = row.value.context_facts_request_bytes().unwrap()
        + std::mem::size_of::<Option<bool>>()
        + scope_bytes(&f.a).unwrap()
        + std::mem::size_of::<AnnotationProjectionClassesV1>();
    for (coordinate, bound) in [request, 2, 5, 5].into_iter().enumerate() {
        for delta in [-1isize, 0, 1] {
            let mut p = profile();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_request_bytes = n,
                1 => p.max_contexts = n,
                2 => p.max_issued = n,
                _ => p.max_selectors = n,
            }
            let mut f = owner(p);
            let reserved = f.o.retention_usage().reserved;
            assert_eq!(
                capture(&mut f).is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                f.o.version_set_context_usage().contexts,
                if delta < 0 { 1 } else { 2 }
            );
            assert_eq!(
                f.o.version_set_context_usage().issued_lifetime,
                if delta < 0 { 3 } else { 5 }
            );
            assert_eq!(f.o.retention_usage().reserved, reserved);
        }
    }
}
#[test]
fn copied_private_cut_classes_and_holds_are_original_but_independently_owned() {
    let mut f = owner(profile());
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.original.context,
        members: &[current(&f.a), current(&f.b)],
    };
    let hold =
        f.o.hold_version_search(
            &f.original.participants[0].snapshot,
            &f.a,
            &parent,
            &mut f.r,
        )
        .unwrap();
    let old = f.o.version_context_pool().unwrap().rows[f.original.context.slot]
        .as_ref()
        .unwrap()
        .value
        .cut;
    let d = f.o.read(&f.a.document, &mut f.r).unwrap().version;
    let p =
        f.o.prepare(
            &[DocumentEdits {
                expected: d,
                edits: vec![crate::source::Replacement {
                    start: 0,
                    end: 2,
                    expected: "ab".into(),
                    inserted: "zz".into(),
                }],
            }],
            &[],
            &mut f.r,
        )
        .unwrap();
    f.o.commit(p, &mut f.r).unwrap();
    let copy =
        f.o.capture_retained_version_set(
            &f.original.context,
            &[
                VersionSetParticipantV1 {
                    scope: &f.b,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &f.a,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &parent,
            Some(true),
            &mut f.r,
        )
        .unwrap();
    let p = f.o.version_context_pool().unwrap();
    let row = p.rows[copy.context.slot].as_ref().unwrap();
    assert_eq!(row.value.cut, old);
    assert_eq!(row.holds, 0);
    assert_eq!(p.rows[f.original.context.slot].as_ref().unwrap().holds, 1);
    assert!(row.value.members[0].value.rows.is_empty());
    assert_eq!(
        row.value.members[0].value.source.as_ref().unwrap().0.text(),
        "peer"
    );
    assert!(row.value.members[1].value.source.is_none());
    assert_eq!(
        row.value.members[1].value.rows[0].body.as_ref().unwrap().0,
        "body"
    );
    f.o.release_version_search_hold(&hold, &[cleanup(&f.a), cleanup(&f.b)])
        .unwrap();
    assert_eq!(f.o.version_set_context_usage().holds, 0);
}
#[test]
fn retained_registry_staging_fault_or_capacity_surprise_preserves_every_original_and_charge() {
    for trap in [false, true] {
        let mut f = owner(profile());
        let before = f.o.version_set_context_usage();
        let reserved = f.o.retention_usage().reserved;
        let generation = f.o.generation();
        if trap {
            f.o.version_set_install_fault = true;
        } else {
            let VersionSetContextState::Active(p) = &mut f.o.version_set_contexts else {
                unreachable!()
            };
            p.aliases = std::mem::take(&mut p.aliases).into_boxed_slice().into_vec();
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| capture(&mut f)));
        assert_eq!(result.is_err(), trap);
        assert!(f.o.is_faulted());
        assert_eq!(f.o.version_set_context_usage(), before);
        assert_eq!(f.o.retention_usage().reserved, reserved);
        assert_eq!(f.o.generation(), generation);
        assert!(f.o.stop_version_set_context(
            VersionSetControlTargetV1::Context(&f.original.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.original.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert!(f.o.is_faulted());
    }
}
#[test]
fn retained_acquisition_first_observes_actual_omitted_peer_disposal_or_owner_fault() {
    for fault in [false, true] {
        let mut f = owner(profile());
        if fault {
            f.o.faulted = true;
        } else {
            f.o.dispose(&f.b.document, &mut f.r).unwrap();
        }
        assert!(capture(&mut f).is_none());
        assert_eq!(f.o.version_set_context_usage().stopped, 1);
        assert_eq!(f.o.version_set_context_usage().contexts, 1);
        assert_eq!(f.o.version_set_context_usage().issued_lifetime, 3);
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.original.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert_eq!(f.o.is_faulted(), fault);
    }
}
