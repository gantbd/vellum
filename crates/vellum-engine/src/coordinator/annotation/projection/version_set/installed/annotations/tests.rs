use super::*;

#[test]
fn native_annotation_validation_capture_is_bounded_optional_and_not_fault_authority() {
    use crate::coordinator::source_query::result_contract::AnnotationResultFault as F;
    for capacity in [None, Some(64), Some(1)] {
        for malformed in [false, true] {
            let mut f = owner(profile(), true);
            f.r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            f.o.annotation_read_result_fault = malformed.then_some(F::RecordAlias);
            let q = VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            };
            assert_eq!(query(&mut f, &q).is_none(), malformed);
            assert_eq!(f.o.is_faulted(), malformed);
            assert_eq!(f.o.read_result_invariant().is_some(), malformed);
            if capacity == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                let checks: Vec<_> = events
                    .iter()
                    .filter(|e| e.boundary == Boundary::AnnotationReadResultValidation)
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
                if malformed {
                    assert!(
                        events.iter().all(|e| e.boundary
                            != Boundary::AnnotationSelectorRegistration
                            && e.boundary != Boundary::VersionSetAnnotationSelectorRegistration)
                    );
                }
            } else if capacity == Some(1) {
                assert!(f.r.dropped() > 0);
            } else {
                assert_eq!(f.r.queued(), 0);
            }
        }
    }
}

#[test]
fn final_native_annotation_faults_refuse_before_registry_publication() {
    use crate::coordinator::source_query::result_contract::AnnotationResultFault as F;
    for (fault, kind) in [
        (F::Snapshot, 0),
        (F::RecordAlias, 0),
        (F::Duplicate, 0),
        (F::Continuation, 0),
        (F::MissingContent, 0),
        (F::Version, 0),
        (F::Location, 0),
        (F::Truncate, 0),
        (F::Snapshot, 1),
        (F::RecordAlias, 1),
        (F::MissingContent, 1),
        (F::Version, 1),
        (F::Location, 1),
        (F::Snapshot, 2),
        (F::RecordAlias, 2),
        (F::Location, 2),
    ] {
        let mut f = owner(profile(), true);
        let q = VersionSetAnnotationQueryV1::Enumerate {
            limit: if matches!(fault, F::Duplicate) { 2 } else { 1 },
            continuation: None,
        };
        let q = if kind == 0 {
            q
        } else {
            let Some(VersionSetAnnotationReplyV1::Page { records, .. }) = query(&mut f, &q) else {
                panic!("page")
            };
            let annotation = records[0].reference.clone();
            if kind == 1 {
                VersionSetAnnotationQueryV1::Inspect { annotation }
            } else {
                VersionSetAnnotationQueryV1::TargetLocation { annotation }
            }
        };
        let before = f.o.version_set_context_usage();
        let reserved = f.o.retention_usage().reserved;
        let generation = f.o.generation();
        f.o.annotation_read_result_fault = Some(fault);
        assert!(query(&mut f, &q).is_none(), "{fault:?}/{kind}");
        assert!(f.o.is_faulted());
        assert_eq!(
            f.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeAnnotationResultInvariant)
        );
        assert_eq!(f.o.version_set_context_usage(), before);
        assert_eq!(f.o.retention_usage().reserved, reserved);
        assert_eq!(f.o.generation(), generation);
        f.o.annotation_read_result_fault = None;
        assert!(query(&mut f, &q).is_none());
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.acq.context),
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
fn profile() -> VersionSetContextLimitsV1 {
    super::super::tests::profile()
}
pub(in crate::coordinator) struct Fixture {
    pub(in crate::coordinator) o: SourceCoordinator,
    pub(in crate::coordinator) r: Recorder,
    pub(in crate::coordinator) a: SourceReadScopeV1,
    pub(in crate::coordinator) b: SourceReadScopeV1,
    pub(in crate::coordinator) acq: VersionSetAcquisitionV1,
}
pub(in crate::coordinator) fn native_facade_version_fixture() -> Fixture {
    let mut f = owner(profile(), true);
    let copy =
        f.o.capture_retained_version_set(
            &f.acq.context,
            &[
                VersionSetParticipantV1 {
                    scope: &f.a,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
                VersionSetParticipantV1 {
                    scope: &f.b,
                    data: AnnotationProjectionClassesV1::Source,
                },
            ],
            &TrustedVersionSetContextFactsV1 {
                context: &f.acq.context,
                members: &[current(&f.a), current(&f.b)],
            },
            Some(true),
            &mut f.r,
        )
        .unwrap();
    f.acq = copy;
    f
}
fn owner(p: VersionSetContextLimitsV1, two: bool) -> Fixture {
    let (mut o, mut r, a, mut g) =
        super::super::super::tests::ready(super::super::super::tests::profile());
    let d = o.read(&a.document, &mut r).unwrap().version;
    if two {
        let plan = o
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: d,
                        basis: TargetBasis::Before,
                        target: AttachmentTarget::Document,
                    },
                    body: "oversized second body".repeat(2),
                }],
                &mut r,
            )
            .unwrap();
        let id = o.commit(plan, &mut r).unwrap().annotation_changes[0]
            .annotation
            .clone();
        g.push(AnnotationFacetGrantV1 {
            annotation: id,
            visible: true,
            body: true,
            target: true,
        });
    }
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
    Fixture { o, r, a, b, acq }
}
fn query(f: &mut Fixture, q: &VersionSetAnnotationQueryV1) -> Option<VersionSetAnnotationReplyV1> {
    f.o.query_version_set_annotations(
        &f.acq.participants[0].snapshot,
        q,
        &TrustedVersionSetContextFactsV1 {
            context: &f.acq.context,
            members: &[current(&f.a), current(&f.b)],
        },
        &mut f.r,
    )
}
#[test]
fn native_annotation_request_row_page_reply_and_selector_ceilings_admit_one_complete_page() {
    let f = owner(profile(), false);
    let p = f.o.version_context_pool().unwrap();
    let row = p.rows[f.acq.context.slot].as_ref().unwrap();
    let request = row.value.context_facts_request_bytes().unwrap()
        + std::mem::size_of::<VersionSetMemberId>()
        + std::mem::size_of::<VersionSetAnnotationQueryV1>();
    let row = std::mem::size_of::<VersionSetAnnotationRecordV1>() + 4;
    let reply = std::mem::size_of::<VersionSetAnnotationReplyV1>() + row;
    for (coordinate, bound) in [request, row, reply, 1, 4, 4].into_iter().enumerate() {
        for delta in [-1isize, 0, 1] {
            let mut p = profile();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_request_bytes = n,
                1 => p.max_row_bytes = n,
                2 => p.max_reply_bytes = n,
                3 => p.max_page_rows = n,
                4 => p.max_selectors = n,
                _ => p.max_issued = n,
            }
            let mut f = owner(p, false);
            let reserved = f.o.retention_usage().reserved;
            assert_eq!(
                query(
                    &mut f,
                    &VersionSetAnnotationQueryV1::Enumerate {
                        limit: 1,
                        continuation: None
                    }
                )
                .is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                f.o.version_set_context_usage().query_selectors,
                usize::from(delta >= 0)
            );
            assert_eq!(f.o.retention_usage().reserved, reserved);
        }
    }
}
#[test]
fn later_oversized_row_registers_no_prefix_and_cannot_hide_current_loss() {
    let mut p = profile();
    p.max_row_bytes = std::mem::size_of::<VersionSetAnnotationRecordV1>() + 4;
    let mut f = owner(p, true);
    let issued = f.o.version_set_context_usage().issued_lifetime;
    assert!(
        query(
            &mut f,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 2,
                continuation: None
            }
        )
        .is_none()
    );
    assert_eq!(f.o.version_set_context_usage().query_selectors, 0);
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, issued);
    let VersionSetAnnotationReplyV1::Page { continuation, .. } = query(
        &mut f,
        &VersionSetAnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!("wrong product")
    };
    let q = VersionSetAnnotationQueryV1::Enumerate {
        limit: 1,
        continuation,
    };
    assert!(query(&mut f, &q).is_none());
    assert_eq!(f.o.version_set_context_usage().query_selectors, 2);
    let mut peer = current(&f.b);
    peer.source_readable = Some(false);
    assert!(
        f.o.query_version_set_annotations(
            &f.acq.participants[0].snapshot,
            &q,
            &TrustedVersionSetContextFactsV1 {
                context: &f.acq.context,
                members: &[current(&f.a), peer]
            },
            &mut f.r
        )
        .is_none()
    );
    assert_eq!(f.o.version_set_context_usage().stopped, 1);
}
#[test]
fn native_selector_capacity_or_precommit_fault_registers_nothing_and_preserves_owner_charge() {
    for trap in [false, true] {
        let mut f = owner(profile(), false);
        let before = f.o.version_set_context_usage();
        let reserved = f.o.retention_usage().reserved;
        let generation = f.o.generation();
        if trap {
            f.o.version_set_selector_fault = true;
        } else {
            let VersionSetContextState::Active(p) = &mut f.o.version_set_contexts else {
                unreachable!()
            };
            p.selectors = Vec::new();
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            query(
                &mut f,
                &VersionSetAnnotationQueryV1::Enumerate {
                    limit: 1,
                    continuation: None,
                },
            )
        }));
        assert_eq!(result.is_err(), trap);
        assert!(f.o.is_faulted());
        assert_eq!(f.o.version_set_context_usage(), before);
        assert_eq!(f.o.retention_usage().reserved, reserved);
        assert_eq!(f.o.generation(), generation);
        assert!(f.o.stop_version_set_context(
            VersionSetControlTargetV1::Context(&f.acq.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.acq.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert!(f.o.is_faulted());
    }
}
#[test]
fn owned_cursor_position_surprises_fault_but_wrong_kind_packets_do_not_observe_loss() {
    let mut f = owner(profile(), true);
    let VersionSetAnnotationReplyV1::Page {
        records,
        continuation,
        ..
    } = query(
        &mut f,
        &VersionSetAnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        },
    )
    .unwrap()
    else {
        panic!("wrong product")
    };
    let false_cursor = VersionSetAnnotationContinuationId {
        owner: f.o.namespace.clone(),
        slot: records[0].reference.slot,
    };
    let mut peer = current(&f.b);
    peer.authorized = Some(false);
    assert!(
        f.o.query_version_set_annotations(
            &f.acq.participants[0].snapshot,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: Some(false_cursor)
            },
            &TrustedVersionSetContextFactsV1 {
                context: &f.acq.context,
                members: &[current(&f.a), peer]
            },
            &mut f.r
        )
        .is_none()
    );
    assert_eq!(f.o.version_set_context_usage().stopped, 0);
    let cursor = continuation.unwrap();
    let VersionSetContextState::Active(p) = &mut f.o.version_set_contexts else {
        unreachable!()
    };
    let Some(Selector::Continuation { next, .. }) = &mut p.selectors[cursor.slot] else {
        unreachable!()
    };
    *next = 0;
    assert!(
        query(
            &mut f,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: Some(cursor)
            }
        )
        .is_none()
    );
    assert!(f.o.is_faulted());
}
#[test]
fn annotation_handoff_first_observes_actual_peer_disposal_or_fault_and_cleanup_drops_query_lookups()
{
    for fault in [false, true] {
        let mut f = owner(profile(), false);
        let VersionSetAnnotationReplyV1::Page { records, .. } = query(
            &mut f,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
        )
        .unwrap() else {
            panic!("wrong product")
        };
        if fault {
            f.o.faulted = true;
        } else {
            f.o.dispose(&f.b.document, &mut f.r).unwrap();
        }
        assert!(
            query(
                &mut f,
                &VersionSetAnnotationQueryV1::Inspect {
                    annotation: records[0].reference.clone()
                }
            )
            .is_none()
        );
        assert_eq!(f.o.version_set_context_usage().stopped, 1);
        assert!(f.o.retire_version_set_context(
            VersionSetControlTargetV1::Context(&f.acq.context),
            &[cleanup(&f.a), cleanup(&f.b)],
            &mut f.r
        ));
        assert_eq!(f.o.version_set_context_usage().query_selectors, 0);
        assert_eq!(f.o.version_set_context_usage().issued_lifetime, 4);
        assert_eq!(f.o.is_faulted(), fault);
    }
}
