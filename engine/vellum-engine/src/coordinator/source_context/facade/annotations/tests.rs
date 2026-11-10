use super::*;
use crate::coordinator::source_query::result_contract::tests::{cleanup, member_facts};
pub(in crate::coordinator) enum Target {
    Single(AnnotationReadContextId),
    VersionSet(VersionSetAcquisitionV1),
}
pub(in crate::coordinator) struct Fixture {
    pub(in crate::coordinator) o: SourceCoordinator,
    pub(in crate::coordinator) r: Recorder,
    pub(in crate::coordinator) scope: SourceReadScopeV1,
    pub(super) peer: Option<SourceReadScopeV1>,
    pub(super) target: Target,
}
pub(in crate::coordinator) enum Query {
    Single(AnnotationQueryV1),
    VersionSet(VersionSetAnnotationQueryV1),
}
pub(in crate::coordinator) fn owner(coherent: bool) -> Fixture {
    let mut f = if coherent {
        use crate::coordinator::annotation::projection::version_set::{
            AnnotationFacadeFixture, native_facade_version_fixture,
        };
        let AnnotationFacadeFixture { o, r, a, b, acq } = native_facade_version_fixture();
        Fixture {
            o,
            r,
            scope: a,
            peer: Some(b),
            target: Target::VersionSet(acq),
        }
    } else {
        let (o, r, _, scope, id) =
            crate::coordinator::annotation::projection::native_facade_single_fixture();
        Fixture {
            o,
            r,
            scope,
            peer: None,
            target: Target::Single(id),
        }
    };
    f.o.set_settings_parent_limit_for_test(16384);
    f
}
pub(in crate::coordinator) fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 512,
        max_depth: 16,
    }
}
pub(in crate::coordinator) fn enumerate(f: &Fixture, limit: usize) -> Query {
    match f.target {
        Target::Single(_) => Query::Single(AnnotationQueryV1::Enumerate {
            limit,
            continuation: None,
        }),
        Target::VersionSet(_) => Query::VersionSet(VersionSetAnnotationQueryV1::Enumerate {
            limit,
            continuation: None,
        }),
    }
}
fn invoke(
    f: &mut Fixture,
    q: &Query,
    authorized: Option<bool>,
    resources: Option<bool>,
    peer_authorized: Option<bool>,
    limits: CaptureLimitsV1,
) -> Option<AnnotationFacadeReplyV1> {
    invoke_with(
        f,
        q,
        authorized,
        resources,
        peer_authorized,
        limits,
        |o, request, r| o.query_annotation_facade_v1(request, r),
    )
}
pub(in crate::coordinator) fn invoke_with(
    f: &mut Fixture,
    q: &Query,
    authorized: Option<bool>,
    resources: Option<bool>,
    peer_authorized: Option<bool>,
    limits: CaptureLimitsV1,
    run: impl for<'a> FnOnce(
        &mut SourceCoordinator,
        AnnotationFacadeRequestV1<'a>,
        &mut Recorder,
    ) -> Option<AnnotationFacadeReplyV1>,
) -> Option<AnnotationFacadeReplyV1> {
    let mut actor = f.scope.context.clone();
    actor.recipient = "agent-not-recipient-grant".into();
    match (&f.target, q) {
        (Target::Single(id), Query::Single(query)) => {
            let facts = TrustedAnnotationContextFactsV1 {
                context: id,
                scope: &f.scope,
                authorized,
                scope_allowed: Some(true),
                continuous: Some(true),
                live: Some(true),
                not_stopped: Some(true),
                annotation_readable: Some(true),
                source_readable: None,
                projection_current: Some(true),
                retained: Some(true),
                resources_ready: resources,
                reply_ready: Some(true),
                transfer_ready: Some(true),
            };
            run(
                &mut f.o,
                AnnotationFacadeRequestV1 {
                    port: AnnotationFacadePortV1::Single {
                        context: id,
                        query,
                        facts: &facts,
                    },
                    scope: &f.scope,
                    actor: &actor,
                    limits,
                },
                &mut f.r,
            )
        }
        (Target::VersionSet(acq), Query::VersionSet(query)) => {
            let mut selected = member_facts(&f.scope);
            selected.authorized = authorized;
            selected.resources_ready = resources;
            selected.source_readable = None;
            let mut peer = member_facts(f.peer.as_ref().unwrap());
            peer.authorized = peer_authorized;
            peer.annotation_readable = None;
            let facts = TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &[selected, peer],
            };
            run(
                &mut f.o,
                AnnotationFacadeRequestV1 {
                    port: AnnotationFacadePortV1::VersionSet {
                        member: &acq.participants[0].snapshot,
                        query,
                        facts: &facts,
                    },
                    scope: &f.scope,
                    actor: &actor,
                    limits,
                },
                &mut f.r,
            )
        }
        _ => None,
    }
}
pub(super) fn query(f: &mut Fixture, q: &Query) -> Option<AnnotationFacadeReplyV1> {
    invoke(f, q, Some(true), Some(true), Some(true), limits())
}
#[test]
fn typed_annotation_facade_preserves_invalid_queries_and_both_original_issuers() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        f.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        crate::coordinator::release_measurements::tests::enable(&mut f.o, &mut f.r);
        let base = f.o.retention_usage().reserved;
        for limit in [0, 1] {
            let q = enumerate(&f, limit);
            invoke_with(
                &mut f,
                &q,
                Some(true),
                Some(true),
                Some(true),
                limits(),
                |o, req, r| {
                    let out = o.try_query_annotation_facade_v1(req, r);
                    if limit == 0 {
                        assert!(matches!(out, Err(Error::InvalidRequest)));
                    } else {
                        assert!(out.is_ok());
                    }
                    out.ok()
                },
            );
        }
        assert_eq!(f.o.retention_usage().reserved, base);
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        assert!(
            events.iter().any(
                |e| e.boundary == Boundary::AnnotationFacadeRelease && e.service_time.is_some()
            )
        );
        assert!(
            events
                .iter()
                .filter(|e| e.boundary != Boundary::AnnotationFacadeRelease)
                .all(|e| e.service_time.is_none())
        );
        assert!(!f.o.is_faulted());
        assert_eq!(
            crate::coordinator::release_measurements::tests::totals(&mut f.o, &mut f.r),
            [1, 0, 1, 0, 0, 0]
        );
    }
}
// Structural checking does not authenticate or consult current host facts.
fn shape(f: &Fixture, q: &Query, reply: &AnnotationFacadeReplyV1) -> bool {
    match (&f.target, q) {
        (Target::Single(context), Query::Single(query)) => {
            let facts = TrustedAnnotationContextFactsV1 {
                context,
                scope: &f.scope,
                authorized: None,
                scope_allowed: None,
                continuous: None,
                live: None,
                not_stopped: None,
                annotation_readable: None,
                source_readable: None,
                projection_current: None,
                retained: None,
                resources_ready: None,
                reply_ready: None,
                transfer_ready: None,
            };
            result::valid(
                reply,
                AnnotationFacadePortV1::Single {
                    context,
                    query,
                    facts: &facts,
                },
            )
        }
        (Target::VersionSet(acq), Query::VersionSet(query)) => {
            let facts = TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &[],
            };
            result::valid(
                reply,
                AnnotationFacadePortV1::VersionSet {
                    member: &acq.participants[0].snapshot,
                    query,
                    facts: &facts,
                },
            )
        }
        _ => false,
    }
}
#[test]
fn facade_page_shape_keeps_optional_facets_but_rejects_duplicate_native_aliases() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 2);
        let mut reply = query(&mut f, &q).unwrap();
        assert!(shape(&f, &q, &reply));
        match &mut reply {
            AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page { records, .. }) => {
                records[0].content = None;
                records[0].location = None;
            }
            AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
                records,
                ..
            }) => {
                records[0].content = None;
                records[0].location = None;
            }
            _ => panic!("page"),
        }
        assert!(shape(&f, &q, &reply));
        match &mut reply {
            AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page { records, .. }) => {
                records[1].reference = records[0].reference.clone()
            }
            AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
                records,
                ..
            }) => records[1].reference = records[0].reference.clone(),
            _ => panic!("page"),
        }
        assert!(!shape(&f, &q, &reply));
    }
}
#[test]
fn facade_record_request_identity_cannot_be_replaced_by_another_valid_alias_from_the_same_page() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 2);
        let page = query(&mut f, &q).unwrap();
        let mut second = match &page {
            AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page { records, .. }) => {
                Query::Single(AnnotationQueryV1::Inspect {
                    annotation: records[1].reference.clone(),
                })
            }
            AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
                records,
                ..
            }) => Query::VersionSet(VersionSetAnnotationQueryV1::Inspect {
                annotation: records[1].reference.clone(),
            }),
            _ => panic!("page"),
        };
        let (inspect, _) = record_queries(page);
        let mut reply = query(&mut f, &inspect).unwrap();
        assert!(shape(&f, &inspect, &reply));
        match (&mut reply, &mut second) {
            (
                AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Record { record, .. }),
                Query::Single(AnnotationQueryV1::Inspect { annotation }),
            ) => record.reference = annotation.clone(),
            (
                AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Record {
                    record,
                    ..
                }),
                Query::VersionSet(VersionSetAnnotationQueryV1::Inspect { annotation }),
            ) => record.reference = annotation.clone(),
            _ => panic!("record"),
        }
        assert!(!shape(&f, &inspect, &reply));
    }
}
pub(in crate::coordinator) fn record_queries(reply: AnnotationFacadeReplyV1) -> (Query, Query) {
    match reply {
        AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page { records, .. }) => {
            let id = records[0].reference.clone();
            (
                Query::Single(AnnotationQueryV1::Inspect {
                    annotation: id.clone(),
                }),
                Query::Single(AnnotationQueryV1::TargetLocation { annotation: id }),
            )
        }
        AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
            records, ..
        }) => {
            let id = records[0].reference.clone();
            (
                Query::VersionSet(VersionSetAnnotationQueryV1::Inspect {
                    annotation: id.clone(),
                }),
                Query::VersionSet(VersionSetAnnotationQueryV1::TargetLocation { annotation: id }),
            )
        }
        _ => panic!("page"),
    }
}
pub(super) fn counts(f: &Fixture) -> (usize, usize) {
    match f.target {
        Target::Single(_) => {
            let u = f.o.annotation_context_usage();
            (u.issued_lifetime, u.selectors)
        }
        Target::VersionSet(_) => {
            let u = f.o.version_set_context_usage();
            (u.issued_lifetime, u.query_selectors)
        }
    }
}
fn idle(f: &Fixture, reserved: usize) {
    assert_eq!(f.o.retention_usage().reserved, reserved);
    assert_eq!(
        f.o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
}
pub(in crate::coordinator) fn retire(f: &mut Fixture) {
    match &f.target {
        Target::Single(id) => {
            assert!(f.o.stop_annotation_context(id, &cleanup(&f.scope), &mut f.r));
            assert!(f.o.retire_annotation_context(id, &cleanup(&f.scope), &mut f.r));
        }
        Target::VersionSet(acq) => {
            let controls = [cleanup(&f.scope), cleanup(f.peer.as_ref().unwrap())];
            assert!(f.o.stop_version_set_context(
                VersionSetControlTargetV1::Context(&acq.context),
                &controls,
                &mut f.r
            ));
            assert!(f.o.retire_version_set_context(
                VersionSetControlTargetV1::Context(&acq.context),
                &controls,
                &mut f.r
            ));
        }
    }
}
#[test]
fn shared_busy_fence_refuses_cross_family_reentry_without_treating_owned_workspace_as_a_surprise() {
    use crate::coordinator::source_query::result_contract::tests::{
        owner as source_owner, source_facts,
    };
    let foreign = source_owner();
    let mut f = owner(false);
    let reserved = f.o.retention_usage().reserved;
    {
        let mut guard = Gateway::for_annotation(&mut f.o, &mut f.r);
        guard.admit(limits()).unwrap();
        let before = guard.owner.read_facade_usage();
        assert!(
            guard
                .owner
                .query_source_facade_v1(
                    SourceFacadeRequestV1 {
                        source: SearchReadContextV1::Source(&foreign.source),
                        scope: &foreign.s,
                        actor: &foreign.s.context,
                        query: SourceContextQueryV1::Line { index: 0 },
                        limits: limits(),
                        facts: SearchReadFactsV1::Source(&source_facts(&foreign.s)),
                    },
                    guard.recorder
                )
                .is_none()
        );
        assert_eq!(guard.owner.read_facade_usage(), before);
        assert!(!guard.owner.is_faulted());
        guard.finish(&Err::<(), _>(Error::Unavailable));
    }
    idle(&f, reserved);
    assert!(!f.o.is_faulted());
}
#[test]
fn source_only_projection_cannot_be_promoted_into_annotation_disclosure_or_poisoned_by_that_attempt()
 {
    let mut f = crate::coordinator::source_query::result_contract::tests::owner();
    f.o.set_settings_parent_limit_for_test(16384);
    fn facts<'a>(
        id: &'a AnnotationReadContextId,
        scope: &'a SourceReadScopeV1,
    ) -> TrustedAnnotationContextFactsV1<'a> {
        TrustedAnnotationContextFactsV1 {
            context: id,
            scope,
            authorized: Some(true),
            scope_allowed: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            annotation_readable: Some(true),
            source_readable: Some(true),
            projection_current: Some(true),
            retained: Some(true),
            resources_ready: Some(true),
            reply_ready: Some(true),
            transfer_ready: Some(true),
        }
    }
    let id =
        f.o.capture_retained_annotation_context(
            &f.combined,
            AnnotationProjectionClassesV1::Source,
            &f.s,
            &facts(&f.combined, &f.s),
            Some(true),
            &mut f.r,
        )
        .unwrap()
        .context;
    let mut denied = facts(&id, &f.s);
    denied.authorized = Some(false);
    let before = f.o.annotation_context_usage();
    assert!(
        f.o.query_annotation_facade_v1(
            AnnotationFacadeRequestV1 {
                port: AnnotationFacadePortV1::Single {
                    context: &id,
                    query: &AnnotationQueryV1::Enumerate {
                        limit: 1,
                        continuation: None
                    },
                    facts: &denied
                },
                scope: &f.s,
                actor: &f.s.context,
                limits: limits(),
            },
            &mut f.r
        )
        .is_none()
    );
    assert_eq!(f.o.annotation_context_usage(), before);
    assert!(!f.o.is_faulted());
    assert!(
        f.o.query_annotation_context_source(
            &id,
            &f.s,
            SourceContextQueryV1::Line { index: 0 },
            &facts(&id, &f.s),
            &mut f.r
        )
        .is_some()
    );
}
#[test]
fn late_facade_faults_preserve_committed_selectors_while_pre_handoff_faults_install_nothing() {
    for coherent in [false, true] {
        for operation in 0..3 {
            for fault in [
                Fault::Classification,
                Fault::Workspace,
                Fault::BeforeNative,
                Fault::AfterNative,
                Fault::BadResult,
                Fault::RefundDrift,
            ] {
                let mut f = owner(coherent);
                let page = enumerate(&f, 1);
                let q = if operation == 0 {
                    page
                } else {
                    let (inspect, location) = record_queries(query(&mut f, &page).unwrap());
                    if operation == 1 { inspect } else { location }
                };
                let before = counts(&f);
                let reserved = f.o.retention_usage().reserved;
                let generation = f.o.generation();
                f.o.source_facade_fault = Some(fault);
                assert!(
                    query(&mut f, &q).is_none(),
                    "{coherent}/{operation}/{fault:?}"
                );
                assert!(f.o.is_faulted());
                idle(&f, reserved);
                assert_eq!(f.o.generation(), generation);
                let delta = usize::from(
                    operation == 0
                        && matches!(
                            fault,
                            Fault::AfterNative | Fault::BadResult | Fault::RefundDrift
                        ),
                ) * 2;
                assert_eq!(counts(&f), (before.0 + delta, before.1 + delta));
                if fault == Fault::BadResult {
                    assert_eq!(
                        f.o.read_result_invariant(),
                        Some(ReadResultInvariantV1::NativeAnnotationFacadeResultInvariant)
                    );
                }
                f.o.source_facade_fault = None;
                assert!(query(&mut f, &q).is_none());
                let issued = counts(&f).0;
                retire(&mut f);
                assert_eq!(counts(&f).0, issued);
                assert!(f.o.is_faulted());
            }
        }
    }
}
#[test]
fn native_prepublication_annotation_fault_keeps_its_cause_and_installs_no_selectors() {
    use crate::coordinator::source_query::result_contract::AnnotationResultFault;
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 1);
        let before = counts(&f);
        let reserved = f.o.retention_usage().reserved;
        f.o.annotation_read_result_fault = Some(AnnotationResultFault::RecordAlias);
        assert!(query(&mut f, &q).is_none());
        assert_eq!(counts(&f), before);
        assert!(f.o.is_faulted());
        assert_eq!(
            f.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeAnnotationResultInvariant)
        );
        idle(&f, reserved);
    }
}
#[test]
fn temporary_annotation_admission_uses_actual_parent_headroom_without_spending_selector_capacity() {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 7 * limits().max_bytes;
    for coherent in [false, true] {
        for available in [fixed - 1, fixed, total - 1, total, total + 1] {
            let mut f = owner(coherent);
            let q = enumerate(&f, 1);
            let original = f.o.retention_usage().reserved;
            let pressure = f.o.retention_usage().maximum.unwrap() - original - available;
            let mut held = f.o.retention.reserve(pressure).unwrap();
            let before = f.o.retention_usage().reserved;
            let issued = counts(&f).0;
            assert_eq!(query(&mut f, &q).is_some(), available >= total);
            assert_eq!(counts(&f).0, issued + usize::from(available >= total) * 2);
            assert!(!f.o.is_faulted());
            idle(&f, before);
            f.o.retention.release(&mut held).unwrap();
            idle(&f, original);
        }
    }
}
#[test]
fn false_and_unknown_copied_facts_remain_distinct_and_peer_loss_closes_the_whole_parent() {
    for coherent in [false, true] {
        for (authorized, peer, stopped) in [
            (None, Some(true), false),
            (Some(true), Some(true), false),
            (Some(false), Some(true), true),
        ] {
            let mut f = owner(coherent);
            let q = enumerate(&f, 1);
            let reserved = f.o.retention_usage().reserved;
            assert!(invoke(&mut f, &q, authorized, Some(false), peer, limits()).is_none());
            assert_eq!(query(&mut f, &q).is_none(), stopped);
            assert!(!f.o.is_faulted());
            idle(&f, reserved);
        }
    }
    let mut f = owner(true);
    let q = enumerate(&f, 1);
    assert!(invoke(&mut f, &q, Some(true), Some(false), Some(false), limits()).is_none());
    assert!(query(&mut f, &q).is_none());
    assert!(!f.o.is_faulted());
}
#[test]
fn bounded_argument_refusal_does_not_lend_loss_and_later_valid_queries_still_work() {
    for coherent in [false, true] {
        for coordinate in 0..4 {
            let mut f = owner(coherent);
            let mut limits = limits();
            let q = enumerate(&f, usize::from(coordinate != 0));
            match coordinate {
                1 => limits.max_bytes = 0,
                2 => limits.max_nodes = 0,
                3 => limits.max_depth = 1,
                _ => (),
            }
            let before = counts(&f);
            let reserved = f.o.retention_usage().reserved;
            assert!(invoke(&mut f, &q, Some(false), Some(true), Some(true), limits).is_none());
            assert_eq!(counts(&f), before);
            assert!(!f.o.is_faulted());
            idle(&f, reserved);
            let valid = enumerate(&f, 1);
            assert!(query(&mut f, &valid).is_some());
        }
    }
}
#[test]
fn annotation_reentry_is_refused_without_poisoning_the_original_call() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 1);
        let reserved = f.o.retention_usage().reserved;
        f.o.source_facade_fault = Some(Fault::Reentry);
        assert!(query(&mut f, &q).is_some());
        assert!(!f.o.is_faulted());
        idle(&f, reserved);
    }
}
#[test]
fn annotation_capture_off_available_and_full_never_change_data_fault_or_refund_outcomes() {
    for coherent in [false, true] {
        for capacity in [None, Some(128), Some(1)] {
            for malformed in [false, true] {
                let mut f = owner(coherent);
                let q = enumerate(&f, 1);
                f.r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                let reserved = f.o.retention_usage().reserved;
                f.o.source_facade_fault = malformed.then_some(Fault::BadResult);
                assert_eq!(query(&mut f, &q).is_none(), malformed);
                assert_eq!(f.o.is_faulted(), malformed);
                idle(&f, reserved);
                if capacity == Some(128) {
                    let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                    let refund = events
                        .iter()
                        .position(|e| e.boundary == Boundary::AnnotationFacadeRefund)
                        .unwrap();
                    let release = events
                        .iter()
                        .position(|e| e.boundary == Boundary::AnnotationFacadeRelease)
                        .unwrap();
                    assert!(refund < release);
                    assert_eq!(
                        events
                            .iter()
                            .filter(|e| e.boundary == Boundary::AnnotationFacadeRefund)
                            .count(),
                        1
                    );
                    assert!(events.iter().all(|e| e.service_time.is_none()
                        && !matches!(
                            e.boundary,
                            Boundary::Publication
                                | Boundary::SourceFacadeRelease
                                | Boundary::SourceFacadeRefund
                                | Boundary::SourceFacadeNativeHandoff
                        )));
                    assert!(!format!("{events:?}").contains("body"));
                } else if capacity == Some(1) {
                    assert!(f.r.dropped() > 0);
                } else {
                    assert_eq!(f.r.queued(), 0);
                }
            }
        }
    }
}
#[test]
fn native_annotation_only_ports_round_trip_fresh_records_locations_and_continuations_without_source_promotion()
 {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let reserved = f.o.retention_usage().reserved;
        let q = enumerate(&f, 1);
        let first = query(&mut f, &q).unwrap();
        let (inspect, location) = record_queries(first);
        let issued = counts(&f);
        let again = query(&mut f, &q).unwrap();
        assert_eq!(counts(&f), issued);
        let next = match again {
            AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page {
                records,
                continuation,
                ..
            }) => {
                assert_eq!(records.len(), 1);
                assert_eq!(records[0].content.as_ref().unwrap().body, "body");
                Query::Single(AnnotationQueryV1::Enumerate {
                    limit: 1,
                    continuation: Some(continuation.unwrap()),
                })
            }
            AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
                records,
                continuation,
                ..
            }) => {
                assert_eq!(records.len(), 1);
                assert_eq!(records[0].content.as_ref().unwrap().body, "body");
                Query::VersionSet(VersionSetAnnotationQueryV1::Enumerate {
                    limit: 1,
                    continuation: Some(continuation.unwrap()),
                })
            }
            _ => panic!("page"),
        };
        assert!(query(&mut f, &inspect).is_some());
        assert!(query(&mut f, &location).is_some());
        assert_eq!(counts(&f), issued);
        assert!(query(&mut f, &next).is_some());
        idle(&f, reserved);
        assert!(!f.o.is_faulted());
    }
}
