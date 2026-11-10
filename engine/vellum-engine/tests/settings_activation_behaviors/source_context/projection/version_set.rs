use super::*;
#[path = "version_set/installed.rs"]
mod installed;
fn limits() -> VersionSetPreparationLimitsV1 {
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
        max_pool_bytes: 4194304,
        source: profile().source,
    }
}
fn base() -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourceReadScopeV1,
) {
    let (mut o, mut r) = crate::setup_with_root_budget(crate::profile(), false, Some(262144));
    o.configure_retention_budget(2097152, &mut r).unwrap();
    let a = o.create(b"first", &mut r).unwrap();
    let b = o.create(b"second", &mut r).unwrap();
    (o, r, scope(&a.document), scope(&b.document))
}
fn current_member(s: &SourceReadScopeV1) -> TrustedVersionSetMemberFactsV1<'_> {
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
#[test]
fn later_member_refusal_never_exposes_a_partial_preparation_or_spends_issuance() {
    for failure in 0..4 {
        let (mut o, mut r, a, b) = base();
        o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
            .unwrap();
        let mut bad = b.clone();
        if failure == 1 {
            bad.context.recipient.push('x');
        }
        let second = if failure == 2 { &a } else { &bad };
        let participants = [
            VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Source,
            },
            VersionSetParticipantV1 {
                scope: second,
                data: AnnotationProjectionClassesV1::Combined,
            },
        ];
        let mut facts = [capture_facts(&a, &[]), capture_facts(second, &[])];
        if failure == 0 {
            facts[1].source_readable = Some(false);
        } else if failure == 3 {
            facts[1].resources_ready = None;
        }
        let before = o.retention_usage().reserved;
        assert!(
            o.prepare_version_set(&participants, &facts, &mut r)
                .is_none()
        );
        assert_eq!(o.version_set_preparation_usage().prepared, 0);
        assert_eq!(o.version_set_preparation_usage().issued_lifetime, 0);
        assert_eq!(o.retention_usage().reserved, before);
        assert!(!o.is_faulted());
    }
}
#[test]
fn whole_set_handoff_observes_later_peer_loss_despite_earlier_unknown_or_pressure() {
    for index in 0..12 {
        for value in [None, Some(false)] {
            let (mut o, mut r, a, b) = base();
            let mut p = limits();
            p.max_issued = 1;
            p.max_prepared = 1;
            o.configure_version_set_preparation("engine", "local", p, &pool_control(), &mut r)
                .unwrap();
            let participants = [
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Combined,
                },
            ];
            let cut = o
                .prepare_version_set(
                    &participants,
                    &[capture_facts(&a, &[]), capture_facts(&b, &[])],
                    &mut r,
                )
                .unwrap();
            let mut members = [current_member(&a), current_member(&b)];
            members[0].reply_ready = None;
            let f = &mut members[1];
            match index {
                0 => f.authorized = value,
                1 => f.scope_allowed = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.not_stopped = value,
                5 => f.projection_current = value,
                6 => f.retained = value,
                7 => f.source_readable = value,
                8 => f.annotation_readable = value,
                9 => f.resources_ready = value,
                10 => f.reply_ready = value,
                _ => f.transfer_ready = value,
            }
            assert!(!o.validate_prepared_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &members
                },
                &mut r
            ));
            assert_eq!(
                o.version_set_preparation_usage().invalidated,
                usize::from(index < 9 && value == Some(false))
            );
            assert_eq!(o.version_set_preparation_usage().issued_lifetime, 1);
        }
    }
}
#[test]
fn class_exclusions_and_original_binding_cannot_poison_an_unrelated_set() {
    let (mut o, mut r, a, b) = base();
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
    let participants = [
        VersionSetParticipantV1 {
            scope: &a,
            data: AnnotationProjectionClassesV1::Source,
        },
        VersionSetParticipantV1 {
            scope: &b,
            data: AnnotationProjectionClassesV1::Annotations,
        },
    ];
    let facts = [capture_facts(&a, &[]), capture_facts(&b, &[])];
    let cut = o
        .prepare_version_set(&participants, &facts, &mut r)
        .unwrap();
    let peer = o
        .prepare_version_set(&participants, &facts, &mut r)
        .unwrap();
    let mut members = [current_member(&a), current_member(&b)];
    members[0].annotation_readable = Some(false);
    members[1].source_readable = Some(false);
    assert!(!o.validate_prepared_version_set(
        &cut,
        &TrustedPreparedVersionSetFactsV1 {
            projection: &peer,
            members: &members
        },
        &mut r
    ));
    assert!(o.validate_prepared_version_set(
        &cut,
        &TrustedPreparedVersionSetFactsV1 {
            projection: &cut,
            members: &members
        },
        &mut r
    ));
    assert_eq!(o.version_set_preparation_usage().invalidated, 0);
    assert!(!o.discard_prepared_version_set(&cut, &[controls(&b), controls(&a)], &mut r));
    assert!(o.discard_prepared_version_set(&cut, &[controls(&a), controls(&b)], &mut r));
    assert_eq!(o.version_set_preparation_usage().prepared, 1);
    assert_eq!(o.version_set_preparation_usage().issued_lifetime, 2);
}
#[test]
fn coherent_preparation_signals_are_optional_bounded_redacted_and_not_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, mut r, a, b) = base();
        o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
            .unwrap();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let cut = o
            .prepare_version_set(
                &[
                    VersionSetParticipantV1 {
                        scope: &a,
                        data: AnnotationProjectionClassesV1::Source,
                    },
                    VersionSetParticipantV1 {
                        scope: &b,
                        data: AnnotationProjectionClassesV1::Annotations,
                    },
                ],
                &[capture_facts(&a, &[]), capture_facts(&b, &[])],
                &mut r,
            )
            .unwrap();
        assert!(o.validate_prepared_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[current_member(&a), current_member(&b)]
            },
            &mut r
        ));
        assert_eq!(o.generation(), generation);
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::VersionSetPreparation)
                    .count(),
                1
            );
            assert!(
                events
                    .iter()
                    .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
            );
            assert!(!format!("{events:?}").contains("first"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
#[test]
fn coherent_preparation_is_one_private_cut_not_a_sequence_of_public_child_captures() {
    let (mut o, mut r) = crate::setup_with_root_budget(crate::profile(), false, Some(262144));
    o.configure_retention_budget(2097152, &mut r).unwrap();
    let d = o.create(b"first", &mut r).unwrap();
    let s = scope(&d.document);
    let peer = o.create(b"second", &mut r).unwrap();
    let mut second = s.clone();
    second.document = peer.document;
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
    let participants = [
        VersionSetParticipantV1 {
            scope: &s,
            data: AnnotationProjectionClassesV1::Source,
        },
        VersionSetParticipantV1 {
            scope: &second,
            data: AnnotationProjectionClassesV1::Annotations,
        },
    ];
    let facts = [capture_facts(&s, &[]), capture_facts(&second, &[])];
    let generation = o.generation();
    let reserved = o.retention_usage().reserved;
    let cut = o
        .prepare_version_set(&participants, &facts, &mut r)
        .unwrap();
    assert_eq!(format!("{cut:?}"), "PreparedVersionSetId(<opaque>)");
    assert_eq!(o.version_set_preparation_usage().prepared, 1);
    assert_eq!(o.version_set_preparation_usage().source_bytes, 5);
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 0);
    assert_eq!(o.source_read_context_usage().issued_lifetime, 0);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage().reserved, reserved);
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "first");
    assert!(o.discard_prepared_version_set(&cut, &[controls(&s), controls(&second)], &mut r));
    assert!(!o.discard_prepared_version_set(&cut, &[controls(&s), controls(&second)], &mut r));
    assert_eq!(o.version_set_preparation_usage().issued_lifetime, 1);
    assert_eq!(
        o.retire_version_set_preparation(&pool_control(), &mut r),
        Ok(true)
    );
}
