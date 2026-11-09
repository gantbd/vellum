use super::*;
use crate::coordinator::annotation::projection::tests::base;
use crate::coordinator::shutdown::tests::install;
#[test]
fn shutdown_inventory_coherent_owner_keeps_both_member_dependencies_and_alias_hold_records() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r, a, b, first) = ready(profile());
    let ctx = o
        .install_version_set(
            &first,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &first,
                members: &[current(&a), current(&b)],
            },
            &mut r,
        )
        .unwrap();
    let members = [current(&a), current(&b)];
    let host_facts = TrustedVersionSetContextFactsV1 {
        context: &ctx.context,
        members: &members,
    };
    let _hold = o
        .hold_version_search(&ctx.participants[0].snapshot, &a, &host_facts, &mut r)
        .unwrap();
    let _pending = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Source,
                },
            ],
            &[facts(&a, &[]), facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let before = (
        o.version_set_preparation_usage(),
        o.version_set_context_usage(),
        o.retention_usage(),
    );
    let cut = inspect(&o, &mut r);
    let view = inspect_read(&o, &cut, &mut r);
    for (role, retained, charge) in [
        (
            ShutdownOwnerRoleV1::VersionSetPreparations,
            1,
            before.0.reserved_pool_bytes,
        ),
        (
            ShutdownOwnerRoleV1::VersionSetContexts,
            4,
            before.1.reserved_pool_bytes,
        ),
    ] {
        let row = view.owners.iter().find(|r| r.owner.role() == role).unwrap();
        assert_eq!(row.state, ShutdownOwnerStateV1::Active);
        assert_eq!(row.retained_records, Some(retained));
        assert_eq!(row.reserved_bytes, Some(charge));
        for document in [&a.document, &b.document] {
            assert!(view.dependencies.iter().any(|e| e.dependent
                == ShutdownInventoryTargetV1::Owner(row.owner.clone())
                && e.required == ShutdownInventoryTargetV1::Document(document.clone())));
        }
    }
    assert_eq!(
        (
            o.version_set_preparation_usage(),
            o.version_set_context_usage(),
            o.retention_usage()
        ),
        before
    );
}

#[test]
fn shutdown_blocks_coherent_capture_install_copy_and_hold_but_preserves_original_service_and_drain()
{
    let (mut o, mut r, a, b, first) = ready(profile());
    let original = o
        .install_version_set(
            &first,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &first,
                members: &[current(&a), current(&b)],
            },
            &mut r,
        )
        .unwrap();
    let source = &original.participants[0].snapshot;
    let members = [current(&a), current(&b)];
    let f = TrustedVersionSetContextFactsV1 {
        context: &original.context,
        members: &members,
    };
    let hold = o.hold_version_search(source, &a, &f, &mut r).unwrap();
    let participants = [VersionSetParticipantV1 {
        scope: &a,
        data: AnnotationProjectionClassesV1::Source,
    }];
    let pending = o
        .prepare_version_set(&participants, &[facts(&a, &[])], &mut r)
        .unwrap();
    install(&mut o, &mut r);
    let before = (
        o.version_set_preparation_usage(),
        o.version_set_context_usage(),
        o.retention_usage(),
    );
    assert!(
        o.prepare_version_set(&participants, &[facts(&a, &[])], &mut r)
            .is_none()
    );
    assert!(
        o.install_version_set(
            &pending,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &pending,
                members: &[current(&a)]
            },
            &mut r
        )
        .is_none()
    );
    assert!(
        o.capture_retained_version_set(&original.context, &participants, &f, Some(true), &mut r)
            .is_none()
    );
    assert!(matches!(
        o.hold_version_search(source, &a, &f, &mut r),
        Err(Error::Unavailable)
    ));
    assert_eq!(
        (
            o.version_set_preparation_usage(),
            o.version_set_context_usage(),
            o.retention_usage()
        ),
        before
    );
    assert!(o.prepared_set(&pending).is_ok());
    assert!(
        o.query_version_set_source(
            source,
            SourceContextQueryV1::Range {
                start: crate::coordinates::BytePosition(0),
                end: crate::coordinates::BytePosition(1)
            },
            &f,
            &mut r
        )
        .is_some()
    );
    assert_eq!(
        o.configure_version_set_contexts(profile(), &control(), &mut r),
        Ok(false)
    );
    assert!(o.discard_prepared_version_set(&pending, &[cleanup(&a)], &mut r));
    let target = VersionSetControlTargetV1::Context(&original.context);
    assert!(o.stop_version_set_context(target, &[cleanup(&a), cleanup(&b)], &mut r));
    o.release_version_search_hold(&hold, &[cleanup(&a), cleanup(&b)])
        .unwrap();
    assert!(o.retire_version_set_context(target, &[cleanup(&a), cleanup(&b)], &mut r));
    assert_eq!(o.retire_version_set_contexts(&control(), &mut r), Ok(true));
    assert_eq!(
        o.retire_version_set_preparation(&control(), &mut r),
        Ok(true)
    );
    assert!(o.retention_usage().reserved > 0);
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_refuses_new_coherent_preparation_and_installed_pools_without_funding_changes() {
    for prepared in [false, true] {
        let (mut o, mut r, ..) = base();
        o.configure_retention_budget(usize::MAX, &mut r).unwrap();
        o.configure_semantic_root_budget(1048576, &mut r).unwrap();
        let prep = super::super::super::tests::profile();
        if prepared {
            o.configure_version_set_preparation("engine", "local", prep, &control(), &mut r)
                .unwrap();
        }
        install(&mut o, &mut r);
        let before = (
            o.version_set_preparation_usage(),
            o.version_set_context_usage(),
            o.retention_usage(),
        );
        assert_eq!(
            o.configure_version_set_preparation("engine", "local", prep, &control(), &mut r),
            if prepared {
                Ok(false)
            } else {
                Err(Error::Unavailable)
            }
        );
        assert_eq!(
            o.configure_version_set_contexts(profile(), &control(), &mut r),
            Err(Error::Unavailable)
        );
        assert_eq!(
            (
                o.version_set_preparation_usage(),
                o.version_set_context_usage(),
                o.retention_usage()
            ),
            before
        );
    }
}

#[test]
fn shutdown_admission_refusal_cannot_hide_loss_in_an_omitted_coherent_peer() {
    let (mut o, mut r, a, b, cut) = ready(profile());
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
    install(&mut o, &mut r);
    let before = o.version_set_context_usage();
    let funding = o.retention_usage();
    let mut members = [current(&a), current(&b)];
    members[1].continuous = Some(false);
    assert!(
        o.capture_retained_version_set(
            &original.context,
            &[VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Source
            }],
            &TrustedVersionSetContextFactsV1 {
                context: &original.context,
                members: &members
            },
            Some(true),
            &mut r
        )
        .is_none()
    );
    let after = o.version_set_context_usage();
    assert_eq!(after.issued_lifetime, before.issued_lifetime);
    assert_eq!(after.stopped, before.stopped + 1);
    assert_eq!(o.retention_usage(), funding);
    assert!(!o.validate_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &TrustedVersionSetContextFactsV1 {
            context: &original.context,
            members: &[current(&a), current(&b)]
        },
        &mut r
    ));
    assert!(!o.is_faulted());
}
