use super::super::tests::{eligible, profile};
use super::*;
pub(in crate::coordinator) fn native_facade_single_fixture() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
    AnnotationReadContextId,
) {
    owner(profile(), true)
}

#[test]
fn annotation_guard_preserves_absent_facets_and_does_not_claim_a_body_oracle() {
    let (mut o, mut r, _, s, id) = owner(profile(), false);
    let q = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let Some(AnnotationQueryReplyV1::Page { mut records, .. }) =
        o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
    else {
        panic!("page")
    };
    let mut record = records.remove(0);
    let p = o.annotation_context_pool().unwrap();
    let rows = &o.annotation_context_row(&id, &s).unwrap().value.rows;
    let binding = |v: &AnnotationRecordId| {
        p.selectors
            .get(v.slot)
            .copied()
            .flatten()
            .map(|key| (v.slot, key))
    };
    let resolved = Resolved::Record {
        index: 0,
        slot: record.reference.slot,
    };
    record
        .content
        .as_mut()
        .unwrap()
        .body
        .replace_range(..1, "X");
    assert!(kernel::result::valid(
        resolved,
        rows,
        id.slot,
        true,
        kernel::result::View::Record(&record),
        binding
    ));
    let absent = Row {
        id: rows[0].id.clone(),
        body: None,
        location: None,
    };
    record.content = None;
    record.location = None;
    assert!(kernel::result::valid(
        resolved,
        &[absent],
        id.slot,
        true,
        kernel::result::View::Record(&record),
        binding
    ));
    assert!(!kernel::result::valid(
        resolved,
        rows,
        id.slot,
        true,
        kernel::result::View::Record(&record),
        binding
    ));
    assert!(!kernel::result::valid(
        Resolved::Page {
            start: usize::MAX,
            end: 0,
            next: None,
            limit: 1
        },
        rows,
        id.slot,
        true,
        kernel::result::View::<AnnotationRecordV1>::Page(&[], None),
        binding,
    ));
}

#[test]
fn native_annotation_validation_capture_is_bounded_optional_and_not_fault_authority() {
    use crate::coordinator::source_query::result_contract::AnnotationResultFault as F;
    for capacity in [None, Some(64), Some(1)] {
        for malformed in [false, true] {
            let (mut o, _, _, s, id) = owner(profile(), true);
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            o.annotation_read_result_fault = malformed.then_some(F::RecordAlias);
            let q = AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            };
            assert_eq!(
                o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
                    .is_none(),
                malformed
            );
            assert_eq!(o.is_faulted(), malformed);
            assert_eq!(o.read_result_invariant().is_some(), malformed);
            if capacity == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
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
                assert!(r.dropped() > 0);
            } else {
                assert_eq!(r.queued(), 0);
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
        let (mut o, mut r, _, s, id) = owner(profile(), true);
        let q = AnnotationQueryV1::Enumerate {
            limit: if matches!(fault, F::Duplicate) { 2 } else { 1 },
            continuation: None,
        };
        let q = if kind == 0 {
            q
        } else {
            let Some(AnnotationQueryReplyV1::Page { records, .. }) =
                o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
            else {
                panic!("page")
            };
            let annotation = records[0].reference.clone();
            if kind == 1 {
                AnnotationQueryV1::Inspect { annotation }
            } else {
                AnnotationQueryV1::TargetLocation { annotation }
            }
        };
        let before = o.annotation_context_usage();
        let reserved = o.retention_usage().reserved;
        let generation = o.generation();
        o.annotation_read_result_fault = Some(fault);
        assert!(
            o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
                .is_none(),
            "{fault:?}/{kind}"
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeAnnotationResultInvariant)
        );
        assert_eq!(o.annotation_context_usage(), before);
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.generation(), generation);
        o.annotation_read_result_fault = None;
        assert!(
            o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
                .is_none()
        );
        assert!(o.retire_annotation_context(&id, &cleanup(&s), &mut r));
        assert!(o.is_faulted());
    }
}
use crate::coordinator::annotation::projection::tests::{
    cleanup, control, current, facts, limits, ready,
};
fn owner(
    p: AnnotationContextLimitsV1,
    two: bool,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
    AnnotationReadContextId,
) {
    let (mut o, mut r, d, s, mut g) = ready(limits());
    if two {
        let plan = o
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: d.clone(),
                        basis: TargetBasis::Before,
                        target: AttachmentTarget::Document,
                    },
                    body: "other".into(),
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
    o.configure_annotation_contexts(p, &control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    (o, r, d, s, id)
}
#[test]
fn f03_request_reply_row_page_selector_and_lifetime_boundaries_reserve_a_whole_page() {
    let (_, _, _, s, _) = owner(profile(), false);
    let request = scope_bytes(&s).unwrap()
        + std::mem::size_of::<AnnotationReadContextId>()
        + std::mem::size_of::<AnnotationQueryV1>();
    let row = std::mem::size_of::<AnnotationRecordV1>() + 4;
    let reply = std::mem::size_of::<AnnotationQueryReplyV1>() + row;
    for (coordinate, bound) in [request, reply, row, 1, 2, 2].into_iter().enumerate() {
        for delta in [-1isize, 0, 1] {
            let n = bound.checked_add_signed(delta).unwrap();
            let mut p = profile();
            match coordinate {
                0 => p.max_request_bytes = n,
                1 => p.max_reply_bytes = n,
                2 => p.max_row_bytes = n,
                3 => p.max_page_rows = n,
                4 => p.max_selectors = n,
                _ => p.max_issued = n,
            }
            let (mut o, mut r, _, s, id) = owner(p, false);
            let q = AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            };
            assert_eq!(
                o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
                    .is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                o.annotation_context_usage().issued_lifetime,
                if delta >= 0 { 2 } else { 1 }
            );
            let mut loss = eligible(&id, &s);
            loss.authorized = Some(false);
            assert!(
                o.query_annotation_context(&id, &s, &q, &loss, &mut r)
                    .is_none()
            );
            assert_eq!(
                o.annotation_context_usage().stopped,
                usize::from(delta >= 0 || (coordinate != 0 && coordinate != 3))
            );
        }
    }
}
#[test]
fn a_later_oversized_row_cannot_register_the_fitting_page_prefix() {
    let mut p = profile();
    p.max_row_bytes = std::mem::size_of::<AnnotationRecordV1>() + 4;
    let (mut o, mut r, _, s, id) = owner(p, true);
    let q = AnnotationQueryV1::Enumerate {
        limit: 2,
        continuation: None,
    };
    assert!(
        o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
            .is_none()
    );
    assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
    assert_eq!(o.annotation_context_usage().selectors, 1);
    assert_eq!(o.annotation_context_usage().stopped, 0);
}
#[test]
fn wrong_selector_kind_and_namespace_do_not_poison_context_but_owned_position_drift_faults() {
    let (mut o, mut r, _, s, id) = owner(profile(), true);
    let q = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    } = o
        .query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
        .unwrap()
    else {
        panic!("not page")
    };
    let record = records[0].reference.clone();
    let cursor = continuation.unwrap();
    let forged = AnnotationRecordId {
        owner: Arc::new(()),
        slot: record.slot,
    };
    assert_ne!(record, forged);
    let wrong_kind = AnnotationRecordId {
        owner: o.namespace.clone(),
        slot: cursor.slot,
    };
    for reference in [forged, wrong_kind] {
        let mut loss = eligible(&id, &s);
        loss.authorized = Some(false);
        assert!(
            o.query_annotation_context(
                &id,
                &s,
                &AnnotationQueryV1::Inspect {
                    annotation: reference
                },
                &loss,
                &mut r
            )
            .is_none()
        );
    }
    assert_eq!(o.annotation_context_usage().stopped, 0);
    let InstalledState::Active(p) = &mut o.annotation_contexts else {
        unreachable!()
    };
    p.selectors[record.slot] = Some(Selector::Record {
        context: id.slot,
        index: usize::MAX,
    });
    let mut loss = eligible(&id, &s);
    loss.authorized = Some(false);
    assert!(
        o.query_annotation_context(
            &id,
            &s,
            &AnnotationQueryV1::Inspect { annotation: record },
            &loss,
            &mut r
        )
        .is_none()
    );
    assert!(o.is_faulted());
    assert!(o.stop_annotation_context(&id, &cleanup(&s), &mut r));
    assert!(o.retire_annotation_context(&id, &cleanup(&s), &mut r));
    assert_eq!(o.retire_annotation_contexts(&control(), &mut r), Ok(true));
    assert!(o.is_faulted());
}
#[test]
fn selector_commit_surprise_or_owned_capacity_drift_cannot_register_a_prefix_or_spend_ids() {
    for trap in [false, true] {
        let (mut o, mut r, d, s, id) = owner(profile(), true);
        let q = AnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        };
        let generation = o.generation();
        let reserved = o.retention_usage().reserved;
        if trap {
            o.annotation_selector_registration_fault = true;
        } else {
            let InstalledState::Active(p) = &mut o.annotation_contexts else {
                unreachable!()
            };
            p.selectors = Vec::new();
        }
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            o.query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
        }));
        assert_eq!(out.is_err(), trap);
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
        assert_eq!(o.annotation_context_usage().selectors, 1);
        assert_eq!(o.document(&d.document).unwrap().source.text(), "ab");
        assert!(o.stop_annotation_context(&id, &cleanup(&s), &mut r));
        assert!(o.retire_annotation_context(&id, &cleanup(&s), &mut r));
        assert_eq!(o.retire_annotation_contexts(&control(), &mut r), Ok(true));
        assert!(o.is_faulted());
    }
}
#[test]
fn finite_cursor_traversal_is_complete_ordered_and_halts_without_raw_index_disclosure() {
    for limit in 1..=4 {
        let (mut o, mut r, _, s, id) = owner(profile(), true);
        let mut continuation = None;
        let mut bodies = Vec::new();
        for _ in 0..3 {
            let q = AnnotationQueryV1::Enumerate {
                limit,
                continuation,
            };
            let AnnotationQueryReplyV1::Page {
                records,
                continuation: next,
                ..
            } = o
                .query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
                .unwrap()
            else {
                panic!("not page")
            };
            bodies.extend(records.into_iter().map(|r| r.content.unwrap().body));
            continuation = next;
            if continuation.is_none() {
                break;
            }
        }
        assert!(continuation.is_none());
        assert_eq!(bodies, vec!["body", "other"]);
    }
}
