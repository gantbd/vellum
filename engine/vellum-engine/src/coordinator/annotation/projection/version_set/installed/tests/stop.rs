use super::*;
fn installed() -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourceReadScopeV1,
    VersionSetAcquisitionV1,
) {
    let (mut o, mut r, a, b, cut) = ready(profile());
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[current(&a), current(&b)],
            },
            &mut r,
        )
        .unwrap();
    (o, r, a, b, acq)
}
fn value(n: usize) -> Option<bool> {
    [None, Some(false), Some(true)][n]
}
fn target(acq: &VersionSetAcquisitionV1, n: usize) -> VersionSetControlTargetV1<'_> {
    if n == 0 {
        VersionSetControlTargetV1::Context(&acq.context)
    } else {
        VersionSetControlTargetV1::Member(&acq.participants[n - 1].snapshot)
    }
}
#[test]
fn stop_is_independent_of_every_drain_and_retention_fact_for_parent_and_member_targets() {
    for selector in 0..3 {
        for facts in 0..81 {
            let (mut o, mut r, a, b, acq) = installed();
            let before = o.version_set_context_usage();
            let base = o.retention_usage().reserved;
            let generation = o.generation();
            let controls = [
                SourceReadContextControlV1 {
                    retention_released: value(facts % 3),
                    delivery_drained: value((facts / 3) % 3),
                    ..cleanup(&a)
                },
                SourceReadContextControlV1 {
                    retention_released: value((facts / 9) % 3),
                    delivery_drained: value((facts / 27) % 3),
                    ..cleanup(&b)
                },
            ];
            assert!(
                o.stop_version_set_context(target(&acq, selector), &controls, &mut r),
                "selector {selector}, discharge vector {facts}"
            );
            assert!(o.stop_version_set_context(target(&acq, selector), &controls, &mut r));
            assert_eq!(
                o.version_set_context_usage(),
                VersionSetContextUsageV1 {
                    stopped: 1,
                    ..before
                }
            );
            assert_eq!(o.retention_usage().reserved, base);
            assert_eq!(o.generation(), generation);
            if facts != 80 {
                assert!(!o.retire_version_set_context(target(&acq, selector), &controls, &mut r));
            }
            let current = TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &[current(&a), current(&b)],
            };
            assert!(
                o.query_version_set_source(
                    &acq.participants[0].snapshot,
                    SourceContextQueryV1::Line { index: 0 },
                    &current,
                    &mut r
                )
                .is_none()
            );
            assert!(
                o.query_version_set_annotations(
                    &acq.participants[1].snapshot,
                    &VersionSetAnnotationQueryV1::Enumerate {
                        limit: 1,
                        continuation: None
                    },
                    &current,
                    &mut r
                )
                .is_none()
            );
            assert!(o.retire_version_set_context(
                target(&acq, selector),
                &[cleanup(&a), cleanup(&b)],
                &mut r
            ));
            assert_eq!(
                o.version_set_context_usage().issued_lifetime,
                before.issued_lifetime
            );
            assert_eq!(o.version_set_context_usage().contexts, 0);
            assert!(!o.is_faulted());
        }
    }
}
#[test]
fn stopping_does_not_require_headroom_delivery_drain_or_a_healthy_editing_owner() {
    for faulted in [false, true] {
        for cap in [None, Some(0), Some(1), Some(64)] {
            let (mut o, _, a, b, acq) = installed();
            let mut r = cap.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            let remaining = o.retention_usage().maximum.unwrap() - o.retention_usage().reserved;
            let mut pressure = o.retention.reserve(remaining).unwrap();
            let base = o.retention_usage().reserved;
            o.faulted = faulted;
            let controls = [
                SourceReadContextControlV1 {
                    retention_released: None,
                    delivery_drained: Some(false),
                    ..cleanup(&a)
                },
                SourceReadContextControlV1 {
                    retention_released: Some(false),
                    delivery_drained: None,
                    ..cleanup(&b)
                },
            ];
            assert!(o.stop_version_set_context(target(&acq, 2), &controls, &mut r));
            assert_eq!(o.retention_usage().reserved, base);
            assert_eq!(o.is_faulted(), faulted);
            if cap == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert_eq!(events.len(), 1);
                assert_eq!(events[0].boundary, Boundary::VersionSetContextStop);
                assert_eq!(events[0].disposition, Disposition::Completed);
                assert!(events[0].service_time.is_none());
            }
            o.retention.release(&mut pressure).unwrap();
            assert!(o.retire_version_set_context(
                target(&acq, 0),
                &[cleanup(&a), cleanup(&b)],
                &mut r
            ));
            assert_eq!(o.is_faulted(), faulted);
        }
    }
}
#[test]
fn live_whole_parent_holds_still_prevent_retirement_after_stop_until_protected_discharge() {
    let (mut o, mut r, a, b, acq) = installed();
    let current = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &[current(&a), current(&b)],
    };
    let hold = o
        .hold_version_search(&acq.participants[0].snapshot, &a, &current, &mut r)
        .unwrap();
    let before = o.version_set_context_usage();
    let controls = [
        SourceReadContextControlV1 {
            retention_released: None,
            delivery_drained: None,
            ..cleanup(&a)
        },
        SourceReadContextControlV1 {
            retention_released: None,
            delivery_drained: None,
            ..cleanup(&b)
        },
    ];
    assert!(o.stop_version_set_context(target(&acq, 1), &controls, &mut r));
    assert_eq!(
        o.version_set_context_usage(),
        VersionSetContextUsageV1 {
            stopped: 1,
            ..before
        }
    );
    assert!(!o.retire_version_set_context(target(&acq, 0), &[cleanup(&a), cleanup(&b)], &mut r));
    assert!(o.release_version_search_hold(&hold, &controls).is_err());
    assert_eq!(o.version_set_context_usage().holds, 1);
    assert!(
        o.release_version_search_hold(&hold, &[cleanup(&a), cleanup(&b)])
            .is_ok()
    );
    assert!(
        o.release_version_search_hold(&hold, &[cleanup(&a), cleanup(&b)])
            .is_err()
    );
    assert!(o.retire_version_set_context(target(&acq, 0), &[cleanup(&a), cleanup(&b)], &mut r));
    assert_eq!(
        o.version_set_context_usage().issued_lifetime,
        before.issued_lifetime
    );
    assert_eq!(o.version_set_context_usage().source_bytes, 0);
    assert_eq!(o.version_set_context_usage().body_bytes, 0);
    assert!(!o.is_faulted());
}
#[test]
fn stop_still_requires_every_original_owner_scope_and_exact_ordered_roster() {
    let (mut o, mut r, a, b, acq) = installed();
    let before = o.version_set_context_usage();
    let generation = o.generation();
    for member in 0..2 {
        for allowed in [None, Some(false)] {
            let mut controls = [cleanup(&a), cleanup(&b)];
            controls[member].owner_control_allowed = allowed;
            assert!(!o.stop_version_set_context(target(&acq, 2), &controls, &mut r));
            assert_eq!(o.version_set_context_usage(), before);
        }
        for field in 0..9 {
            let mut wrong = if member == 0 { a.clone() } else { b.clone() };
            match field {
                0 => {
                    wrong.document = if member == 0 {
                        b.document.clone()
                    } else {
                        a.document.clone()
                    }
                }
                1 => wrong.context.authority.push('x'),
                2 => wrong.context.engine.push('x'),
                3 => wrong.context.recipient.push('x'),
                4 => wrong.context.purpose.push('x'),
                5 => wrong.context.destination.push('x'),
                6 => wrong.context.profile.push('x'),
                7 => wrong.context.continuity.push('x'),
                _ => wrong.policy.push('x'),
            }
            let mut controls = [cleanup(&a), cleanup(&b)];
            controls[member].scope = &wrong;
            assert!(!o.stop_version_set_context(target(&acq, 1), &controls, &mut r));
            assert_eq!(o.version_set_context_usage(), before);
        }
    }
    for controls in [
        vec![],
        vec![cleanup(&a)],
        vec![cleanup(&b), cleanup(&a)],
        vec![cleanup(&a), cleanup(&a)],
        vec![cleanup(&a), cleanup(&b), cleanup(&b)],
    ] {
        assert!(!o.stop_version_set_context(target(&acq, 0), &controls, &mut r));
    }
    let (_other, _, _, _, foreign) = installed();
    for selector in 0..3 {
        assert!(!o.stop_version_set_context(
            target(&foreign, selector),
            &[cleanup(&a), cleanup(&b)],
            &mut r
        ));
    }
    assert_eq!(o.version_set_context_usage(), before);
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
    let current = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &[current(&a), current(&b)],
    };
    assert!(
        o.query_version_set_source(
            &acq.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &current,
            &mut r
        )
        .is_some()
    );
    assert!(o.stop_version_set_context(target(&acq, 2), &[cleanup(&a), cleanup(&b)], &mut r));
}
#[test]
fn stopping_one_coherent_parent_does_not_stop_or_retire_an_independent_retained_copy() {
    let (mut o, mut r, a, b, acq) = installed();
    let current = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &[current(&a), current(&b)],
    };
    let copy = o
        .capture_retained_version_set(
            &acq.context,
            &[VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Source,
            }],
            &current,
            Some(true),
            &mut r,
        )
        .unwrap();
    let controls = [
        SourceReadContextControlV1 {
            retention_released: None,
            delivery_drained: None,
            ..cleanup(&a)
        },
        SourceReadContextControlV1 {
            retention_released: None,
            delivery_drained: None,
            ..cleanup(&b)
        },
    ];
    let before = o.version_set_context_usage();
    assert!(o.stop_version_set_context(target(&acq, 2), &controls, &mut r));
    assert_eq!(o.version_set_context_usage().contexts, 2);
    assert_eq!(o.version_set_context_usage().stopped, 1);
    assert!(o.retire_version_set_context(target(&acq, 0), &[cleanup(&a), cleanup(&b)], &mut r));
    let current = TrustedVersionSetContextFactsV1 {
        context: &copy.context,
        members: &[super::current(&a)],
    };
    assert!(
        o.query_version_set_source(
            &copy.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &current,
            &mut r
        )
        .is_some()
    );
    assert!(o.stop_version_set_context(target(&copy, 0), &[cleanup(&a)], &mut r));
    assert!(o.retire_version_set_context(target(&copy, 0), &[cleanup(&a)], &mut r));
    assert_eq!(
        o.version_set_context_usage().issued_lifetime,
        before.issued_lifetime
    );
    assert_eq!(o.version_set_context_usage().contexts, 0);
    assert!(!o.is_faulted());
}
