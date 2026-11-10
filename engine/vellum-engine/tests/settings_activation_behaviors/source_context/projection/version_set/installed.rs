use super::*;
#[test]
fn given_coherent_read_contexts_when_delivery_is_not_drained_then_stop_closes_the_parent_without_premature_retirement()
 {
    let (mut o, mut r, a, b, prepared) = fixture(owner_limits());
    let acq = o
        .install_version_set(
            &prepared,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &prepared,
                members: &[current_member(&a), current_member(&b)],
            },
            &mut r,
        )
        .unwrap();
    let facts = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &[current_member(&a), current_member(&b)],
    };
    let delivered = o
        .query_version_set_source(
            &acq.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &facts,
            &mut r,
        )
        .unwrap();
    let before = o.version_set_context_usage();
    let reserved = o.retention_usage().reserved;
    let controls = [
        SourceReadContextControlV1 {
            scope: &a,
            owner_control_allowed: Some(true),
            retention_released: None,
            delivery_drained: Some(false),
        },
        SourceReadContextControlV1 {
            scope: &b,
            owner_control_allowed: Some(true),
            retention_released: Some(false),
            delivery_drained: None,
        },
    ];
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Member(&acq.participants[1].snapshot),
        &controls,
        &mut r
    ));
    assert_eq!(
        o.version_set_context_usage(),
        VersionSetContextUsageV1 {
            stopped: 1,
            ..before
        }
    );
    assert_eq!(o.retention_usage().reserved, reserved);
    assert!(
        o.query_version_set_source(
            &acq.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &facts,
            &mut r
        )
        .is_none()
    );
    assert!(!o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &controls,
        &mut r
    ));
    let SourceContextContentV1::Line(line) = &delivered.content else {
        panic!("line");
    };
    assert_eq!(line.body(), b"first"); // stopping cannot recall already delivered bytes
    drop(delivered);
    let drained = [
        SourceReadContextControlV1 {
            retention_released: Some(true),
            delivery_drained: Some(true),
            ..controls[0]
        },
        SourceReadContextControlV1 {
            retention_released: Some(true),
            delivery_drained: Some(true),
            ..controls[1]
        },
    ];
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &drained,
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().contexts, 0);
    assert_eq!(
        o.version_set_context_usage().issued_lifetime,
        before.issued_lifetime
    );
    assert!(!o.is_faulted());
}
#[test]
fn native_coherent_acquisition_facade_preserves_order_and_independent_retained_ownership() {
    let (mut o, mut r, a, b, prepared) = fixture(owner_limits());
    let limits = vellum_engine::capture::CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 256,
        max_depth: 16,
    };
    let roster = [
        VersionSetParticipantV1 {
            scope: &a,
            data: AnnotationProjectionClassesV1::Source,
        },
        VersionSetParticipantV1 {
            scope: &b,
            data: AnnotationProjectionClassesV1::Combined,
        },
    ];
    let wrong = [
        VersionSetParticipantV1 {
            scope: &b,
            data: AnnotationProjectionClassesV1::Combined,
        },
        VersionSetParticipantV1 {
            scope: &a,
            data: AnnotationProjectionClassesV1::Source,
        },
    ];
    let mut lost = current_member(&b);
    lost.authorized = Some(false);
    let false_facts = TrustedPreparedVersionSetFactsV1 {
        projection: &prepared,
        members: &[current_member(&a), lost],
    };
    let before = o.version_set_preparation_usage();
    let reserved = o.retention_usage().reserved;
    assert!(
        o.acquire_read_facade_v1(
            ReadAcquisitionFacadeRequestV1 {
                port: ReadAcquisitionFacadePortV1::VersionSetCurrent {
                    participants: &wrong,
                    prepared: &prepared,
                    facts: &false_facts
                },
                actor: &a.context,
                limits,
            },
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_preparation_usage(), before);
    let facts = TrustedPreparedVersionSetFactsV1 {
        projection: &prepared,
        members: &[current_member(&a), current_member(&b)],
    };
    let Some(ReadAcquisitionFacadeReplyV1::VersionSet(original)) = o.acquire_read_facade_v1(
        ReadAcquisitionFacadeRequestV1 {
            port: ReadAcquisitionFacadePortV1::VersionSetCurrent {
                participants: &roster,
                prepared: &prepared,
                facts: &facts,
            },
            actor: &a.context,
            limits,
        },
        &mut r,
    ) else {
        panic!("ordered original")
    };
    assert_eq!(original.participants[0].document, a.document);
    assert_eq!(original.participants[1].document, b.document);
    let parent = TrustedVersionSetContextFactsV1 {
        context: &original.context,
        members: &[current_member(&a), current_member(&b)],
    };
    let Some(ReadAcquisitionFacadeReplyV1::VersionSet(copy)) = o.acquire_read_facade_v1(
        ReadAcquisitionFacadeRequestV1 {
            port: ReadAcquisitionFacadePortV1::VersionSetRetained {
                original: &original.context,
                participants: &[VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Source,
                }],
                facts: &parent,
                preparation_ready: Some(true),
            },
            actor: &b.context,
            limits,
        },
        &mut r,
    ) else {
        panic!("retained subset")
    };
    assert_ne!(copy.context, original.context);
    assert_eq!(copy.participants.len(), 1);
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(
        o.query_version_set_source(
            &copy.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &TrustedVersionSetContextFactsV1 {
                context: &copy.context,
                members: &[current_member(&b)]
            },
            &mut r
        )
        .is_some()
    );
    assert!(!o.is_faulted());
    assert_eq!(o.retention_usage().reserved, reserved);
    assert!(!o.read_facade_usage().busy);
}
#[path = "installed/annotations.rs"]
mod annotations;
#[path = "installed/retained.rs"]
mod retained;
#[path = "installed/search.rs"]
mod search;
#[path = "installed/source.rs"]
mod source;
fn fixture(
    p: VersionSetContextLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourceReadScopeV1,
    PreparedVersionSetId,
) {
    let (mut o, mut r, a, b) = base();
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
    o.configure_version_set_contexts(p, &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Combined,
                },
            ],
            &[capture_facts(&a, &[]), capture_facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    (o, r, a, b, cut)
}
#[test]
fn full_installed_pool_cannot_conceal_prepared_peer_loss_or_consume_a_refused_cut() {
    let mut p = owner_limits();
    p.max_contexts = 0;
    let (mut o, mut r, a, b, cut) = fixture(p);
    let mut members = [current_member(&a), current_member(&b)];
    let reserved = o.retention_usage().reserved;
    assert!(
        o.install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members
            },
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_preparation_usage().prepared, 1);
    members[1].annotation_readable = Some(false);
    assert!(
        o.install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members
            },
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_preparation_usage().invalidated, 1);
    assert_eq!(o.version_set_context_usage().issued_lifetime, 0);
    assert_eq!(o.retention_usage().reserved, reserved);
}
#[test]
fn installed_whole_parent_loss_is_absorbing_but_unknown_and_pressure_are_not() {
    for index in 0..12 {
        for value in [None, Some(false)] {
            let (mut o, mut r, a, b, cut) = fixture(owner_limits());
            let mut members = [current_member(&a), current_member(&b)];
            let acquired = o
                .install_version_set(
                    &cut,
                    &TrustedPreparedVersionSetFactsV1 {
                        projection: &cut,
                        members: &members,
                    },
                    &mut r,
                )
                .unwrap();
            members[0].resources_ready = None;
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
            assert!(!o.validate_version_set_context(
                VersionSetControlTargetV1::Member(&acquired.participants[0].snapshot),
                &TrustedVersionSetContextFactsV1 {
                    context: &acquired.context,
                    members: &members
                },
                &mut r
            ));
            assert_eq!(
                o.version_set_context_usage().stopped,
                usize::from(index < 9 && value == Some(false))
            );
            assert_eq!(o.version_set_context_usage().aliases, 2);
        }
    }
}
#[test]
fn aliases_and_current_facts_cannot_cross_parents_and_cleanup_preserves_other_groups() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let first = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Combined,
                },
            ],
            &[capture_facts(&a, &[]), capture_facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let second = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let mut loss = [current_member(&a), current_member(&b)];
    loss[1].authorized = Some(false);
    assert!(!o.validate_version_set_context(
        VersionSetControlTargetV1::Member(&first.participants[0].snapshot),
        &TrustedVersionSetContextFactsV1 {
            context: &second.context,
            members: &loss
        },
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().stopped, 0);
    assert!(!o.stop_version_set_context(
        VersionSetControlTargetV1::Member(&first.participants[0].snapshot),
        &[controls(&a)],
        &mut r
    ));
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Member(&first.participants[1].snapshot),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Member(&first.participants[0].snapshot),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().contexts, 1);
    assert_eq!(o.version_set_context_usage().aliases, 2);
    assert_eq!(o.version_set_context_usage().issued_lifetime, 6);
    assert!(o.validate_version_set_context(
        VersionSetControlTargetV1::Context(&second.context),
        &TrustedVersionSetContextFactsV1 {
            context: &second.context,
            members: &members
        },
        &mut r
    ));
}
#[test]
fn installation_telemetry_is_optional_bounded_redacted_and_never_authored_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, _, a, b, cut) = fixture(owner_limits());
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let members = [current_member(&a), current_member(&b)];
        let acquired = o
            .install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &members,
                },
                &mut r,
            )
            .unwrap();
        assert_eq!(
            format!("{acquired:?}"),
            "VersionSetAcquisitionV1(<redacted>)"
        );
        assert_eq!(
            format!("{:?}", acquired.participants[0].snapshot),
            "VersionSetMemberId(<opaque>)"
        );
        assert!(o.validate_version_set_context(
            VersionSetControlTargetV1::Context(&acquired.context),
            &TrustedVersionSetContextFactsV1 {
                context: &acquired.context,
                members: &members
            },
            &mut r
        ));
        assert_eq!(o.generation(), generation);
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::VersionSetInstallation)
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
fn owner_limits() -> VersionSetContextLimitsV1 {
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
#[test]
fn installation_transfers_the_original_cut_once_and_member_stop_closes_the_whole_parent() {
    let (mut o, mut r, a, b) = base();
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
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
    o.configure_version_set_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let members = [current_member(&a), current_member(&b)];
    let f = TrustedPreparedVersionSetFactsV1 {
        projection: &cut,
        members: &members,
    };
    let acquired = o.install_version_set(&cut, &f, &mut r).unwrap();
    assert_eq!(acquired.participants.len(), 2);
    assert_eq!(acquired.participants[0].document, a.document);
    assert_eq!(acquired.participants[1].document, b.document);
    assert_eq!(
        acquired.participants[0].data,
        AnnotationProjectionClassesV1::Source
    );
    assert_eq!(o.version_set_context_usage().contexts, 1);
    assert_eq!(o.version_set_context_usage().aliases, 2);
    assert_eq!(o.version_set_context_usage().issued_lifetime, 3);
    assert_eq!(o.version_set_preparation_usage().prepared, 0);
    assert!(o.install_version_set(&cut, &f, &mut r).is_none());
    assert_eq!(
        o.retire_version_set_preparation(&pool_control(), &mut r),
        Ok(true)
    );
    let facts = TrustedVersionSetContextFactsV1 {
        context: &acquired.context,
        members: &members,
    };
    assert!(o.validate_version_set_context(
        VersionSetControlTargetV1::Member(&acquired.participants[0].snapshot),
        &facts,
        &mut r
    ));
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Member(&acquired.participants[1].snapshot),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(!o.validate_version_set_context(
        VersionSetControlTargetV1::Context(&acquired.context),
        &facts,
        &mut r
    ));
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&acquired.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().aliases, 0);
    assert_eq!(o.version_set_context_usage().issued_lifetime, 3);
    assert_eq!(
        o.retire_version_set_contexts(&pool_control(), &mut r),
        Ok(true)
    );
}
