use super::*;
use crate::coordinator::source_query::result_contract::tests::{
    Fixture, cleanup, member_facts, source_facts,
};
fn owner() -> Fixture {
    let mut f = crate::coordinator::source_query::result_contract::tests::owner();
    f.o.set_settings_parent_limit_for_test(16384);
    f
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 512,
        max_depth: 16,
    }
}
#[test]
fn native_read_timing_covers_refusal_fault_and_early_fences_without_changing_cleanup() {
    use crate::instrumentation::NativeTimingProfileV1;
    for mode in 0..5 {
        let mut f = owner();
        f.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let base = f.o.retention_usage().reserved;
        match mode {
            0 => f.o.source_facade_fault = Some(Fault::AfterNative),
            1 => f.o.source_facade_fault = Some(Fault::RefundDrift),
            2 => f.o.faulted = true,
            3 => f.o.source_facade_busy = true,
            _ => f.o.source_facade_workspace = 7,
        }
        assert!(query(&mut f, 0, false).is_none());
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        let terminal = events.last().unwrap();
        assert!(terminal.service_time.is_some());
        assert_eq!(
            terminal.boundary,
            if mode < 2 {
                Boundary::SourceFacadeRelease
            } else {
                Boundary::SourceFacadeAdmission
            }
        );
        assert_eq!(f.o.retention_usage().reserved, base);
        if mode == 4 {
            assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
        }
        if mode == 3 {
            assert!(!f.o.is_faulted());
            assert!(f.o.read_facade_usage().busy);
        } else {
            assert!(f.o.is_faulted());
        }
    }
}
#[test]
fn typed_source_facade_preserves_native_shape_errors_and_all_issuer_products() {
    use crate::coordinator::source_context::search::result_contract::tests::with_facts;
    for kind in 0..3 {
        let mut f = owner();
        f.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        crate::coordinator::release_measurements::tests::enable(&mut f.o, &mut f.r);
        let base = f.o.retention_usage().reserved;
        with_facts(&mut f, kind, |o, source, scope, facts, r| {
            let request = SourceFacadeRequestV1 {
                source,
                scope,
                actor: &scope.context,
                facts,
                limits: limits(),
                query: SourceContextQueryV1::Line { index: 999 },
            };
            assert!(matches!(
                o.try_query_source_facade_v1(request, r),
                Err(Error::InvalidRequest)
            ));
            assert!(
                o.try_query_source_facade_v1(
                    SourceFacadeRequestV1 {
                        query: SourceContextQueryV1::Line { index: 0 },
                        ..request
                    },
                    r
                )
                .is_ok()
            );
        });
        idle(&f, base);
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        assert!(
            events
                .iter()
                .any(|e| e.boundary == Boundary::SourceFacadeRelease && e.service_time.is_some())
        );
        assert!(
            events
                .iter()
                .filter(|e| e.boundary != Boundary::SourceFacadeRelease)
                .all(|e| e.service_time.is_none())
        );
        assert!(!f.o.is_faulted());
        assert_eq!(
            crate::coordinator::release_measurements::tests::totals(&mut f.o, &mut f.r),
            [1, 0, 1, 0, 0, 0]
        );
    }
}
fn invoke(
    f: &mut Fixture,
    kind: usize,
    line: bool,
    authorization: Option<bool>,
    resources: Option<bool>,
    limits: CaptureLimitsV1,
) -> Option<SourceFacadeReplyV1> {
    let mut actor = f.s.context.clone();
    actor.recipient = "separate-actor-not-a-grant".into();
    let mut source = source_facts(&f.s);
    source.authorized = authorization;
    source.resources_ready = resources;
    let combined = TrustedAnnotationContextFactsV1 {
        context: &f.combined,
        scope: &f.s,
        authorized: authorization,
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: resources,
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let mut member = member_facts(&f.s);
    member.authorized = authorization;
    member.resources_ready = resources;
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.set.context,
        members: &[member],
    };
    let selected = TrustedVersionSetSearchFactsV1 {
        member: &f.set.participants[0].snapshot,
        scope: &f.s,
        parent: &parent,
    };
    let (port, facts) = match kind {
        0 => (
            SearchReadContextV1::Source(&f.source),
            SearchReadFactsV1::Source(&source),
        ),
        1 => (
            SearchReadContextV1::Combined(&f.combined),
            SearchReadFactsV1::Combined(&combined),
        ),
        _ => (
            SearchReadContextV1::VersionSet(&f.set.participants[0].snapshot),
            SearchReadFactsV1::VersionSet(&selected),
        ),
    };
    f.o.query_source_facade_v1(
        SourceFacadeRequestV1 {
            source: port,
            scope: &f.s,
            actor: &actor,
            query: if line {
                SourceContextQueryV1::Line { index: 0 }
            } else {
                SourceContextQueryV1::Range {
                    start: BytePosition(0),
                    end: BytePosition(4),
                }
            },
            limits,
            facts,
        },
        &mut f.r,
    )
}
fn query(f: &mut Fixture, kind: usize, line: bool) -> Option<SourceFacadeReplyV1> {
    invoke(f, kind, line, Some(true), Some(true), limits())
}
fn content(reply: SourceFacadeReplyV1) -> SourceContextContentV1 {
    match reply {
        SourceFacadeReplyV1::Source(v) => v.content,
        SourceFacadeReplyV1::Combined(v) => v.content,
        SourceFacadeReplyV1::VersionSet(v) => v.content,
    }
}
fn idle(f: &Fixture, reserved: usize) {
    assert_eq!(f.o.retention_usage().reserved, reserved);
    assert_eq!(
        f.o.source_facade_usage(),
        SourceFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
}
#[test]
fn an_outstanding_workspace_surprise_is_fenced_without_erasure_or_new_admission() {
    let mut f = owner();
    let reserved = f.o.retention_usage().reserved;
    f.o.source_facade_workspace = 7;
    assert!(query(&mut f, 0, false).is_none());
    assert!(f.o.is_faulted());
    assert_eq!(
        f.o.source_facade_usage(),
        SourceFacadeUsageV1 {
            busy: false,
            workspace_bytes: 7
        }
    );
    assert_eq!(f.o.retention_usage().reserved, reserved);
}
#[test]
fn malformed_native_arguments_or_foreign_actor_domain_do_not_lend_loss_to_a_context() {
    for mode in 0..4 {
        let mut f = owner();
        let mut actor = f.s.context.clone();
        let query = match mode {
            0 => {
                actor.engine = "foreign".into();
                SourceContextQueryV1::Line { index: 0 }
            }
            1 => SourceContextQueryV1::Range {
                start: BytePosition(4),
                end: BytePosition(0),
            },
            2 => SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(usize::MAX),
            },
            _ => SourceContextQueryV1::Line { index: usize::MAX },
        };
        let reserved = f.o.retention_usage().reserved;
        let mut facts = source_facts(&f.s);
        facts.authorized = Some(false);
        assert!(
            f.o.query_source_facade_v1(
                SourceFacadeRequestV1 {
                    source: SearchReadContextV1::Source(&f.source),
                    scope: &f.s,
                    actor: &actor,
                    query,
                    limits: limits(),
                    facts: SearchReadFactsV1::Source(&facts),
                },
                &mut f.r
            )
            .is_none()
        );
        idle(&f, reserved);
        assert!(!f.o.is_faulted());
        assert!(query_source_again(&mut f));
    }
    fn query_source_again(f: &mut Fixture) -> bool {
        query(f, 0, false).is_some()
    }
}
#[test]
fn foreign_port_fact_kind_cannot_poison_or_redirect_the_original_source_owner() {
    let mut f = owner();
    let actor = f.s.context.clone();
    let facts = TrustedAnnotationContextFactsV1 {
        context: &f.combined,
        scope: &f.s,
        authorized: Some(false),
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
    };
    assert!(
        f.o.query_source_facade_v1(
            SourceFacadeRequestV1 {
                source: SearchReadContextV1::Source(&f.source),
                scope: &f.s,
                actor: &actor,
                query: SourceContextQueryV1::Line { index: 0 },
                limits: limits(),
                facts: SearchReadFactsV1::Combined(&facts),
            },
            &mut f.r
        )
        .is_none()
    );
    assert!(!f.o.is_faulted());
    assert!(query(&mut f, 0, false).is_some());
}
#[test]
fn complete_copied_peer_facts_preserve_whole_parent_authority_and_reject_incomplete_loss_packets() {
    fn capture(scope: &SourceReadScopeV1) -> TrustedAnnotationCaptureFactsV1<'_> {
        TrustedAnnotationCaptureFactsV1 {
            scope,
            grants: &[],
            authorized: Some(true),
            scope_allowed: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            source_readable: Some(true),
            annotation_readable: Some(true),
            visibility_complete: Some(true),
            projection_order_attested: Some(true),
            resources_ready: Some(true),
        }
    }
    let mut f = owner();
    let d = f.o.create(b"peer", &mut f.r).unwrap();
    let mut peer = f.s.clone();
    peer.document = d.document;
    let cut =
        f.o.prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &f.s,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &peer,
                    data: AnnotationProjectionClassesV1::Source,
                },
            ],
            &[capture(&f.s), capture(&peer)],
            &mut f.r,
        )
        .unwrap();
    let acquired =
        f.o.install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[member_facts(&f.s), member_facts(&peer)],
            },
            &mut f.r,
        )
        .unwrap();
    let scope = f.s.clone();
    let actor = scope.context.clone();
    let run = |o: &mut SourceCoordinator,
               r: &mut Recorder,
               members: &[TrustedVersionSetMemberFactsV1<'_>]| {
        let parent = TrustedVersionSetContextFactsV1 {
            context: &acquired.context,
            members,
        };
        let facts = TrustedVersionSetSearchFactsV1 {
            member: &acquired.participants[0].snapshot,
            scope: &scope,
            parent: &parent,
        };
        o.query_source_facade_v1(
            SourceFacadeRequestV1 {
                source: SearchReadContextV1::VersionSet(&acquired.participants[0].snapshot),
                scope: &scope,
                actor: &actor,
                query: SourceContextQueryV1::Line { index: 0 },
                limits: limits(),
                facts: SearchReadFactsV1::VersionSet(&facts),
            },
            r,
        )
    };
    let reserved = f.o.retention_usage().reserved;
    let mut loss = member_facts(&scope);
    loss.authorized = Some(false);
    assert!(run(&mut f.o, &mut f.r, &[loss]).is_none());
    assert!(
        run(
            &mut f.o,
            &mut f.r,
            &[member_facts(&scope), member_facts(&peer)]
        )
        .is_some()
    );
    let mut lost_peer = member_facts(&peer);
    lost_peer.authorized = Some(false);
    lost_peer.resources_ready = Some(false);
    assert!(run(&mut f.o, &mut f.r, &[member_facts(&scope), lost_peer]).is_none());
    assert!(
        run(
            &mut f.o,
            &mut f.r,
            &[member_facts(&scope), member_facts(&peer)]
        )
        .is_none()
    );
    assert!(!f.o.is_faulted());
    idle(&f, reserved);
}
pub(super) fn cleanup_port(f: &mut Fixture, kind: usize) {
    match kind {
        0 => {
            assert!(f.o.stop_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
            assert!(f.o.retire_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
        }
        1 => {
            assert!(f.o.stop_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
            assert!(f.o.retire_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
        }
        _ => {
            assert!(f.o.stop_version_set_context(
                VersionSetControlTargetV1::Context(&f.set.context),
                &[cleanup(&f.s)],
                &mut f.r
            ));
            assert!(f.o.retire_version_set_context(
                VersionSetControlTargetV1::Context(&f.set.context),
                &[cleanup(&f.s)],
                &mut f.r
            ));
        }
    }
}
#[test]
fn temporary_frames_require_actual_shared_parent_headroom_and_refund_only_their_charge() {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 7 * limits().max_bytes;
    for available in [fixed - 1, fixed, total - 1, total, total + 1] {
        for kind in 0..3 {
            let mut f = owner();
            let original = f.o.retention_usage().reserved;
            let pressure = f.o.retention_usage().maximum.unwrap() - original - available;
            let mut held = f.o.retention.reserve(pressure).unwrap();
            let before = f.o.retention_usage().reserved;
            assert_eq!(query(&mut f, kind, false).is_some(), available >= total);
            idle(&f, before);
            assert!(!f.o.is_faulted());
            f.o.retention.release(&mut held).unwrap();
            idle(&f, original);
        }
    }
}
#[test]
fn native_fault_and_late_result_frames_refund_temporary_work_without_retiring_native_contexts() {
    for fault in [
        Fault::Classification,
        Fault::Workspace,
        Fault::BeforeNative,
        Fault::AfterNative,
        Fault::BadResult,
        Fault::RefundDrift,
    ] {
        for kind in 0..3 {
            for line in [false, true] {
                let mut f = owner();
                let reserved = f.o.retention_usage().reserved;
                let usage = (
                    f.o.source_read_context_usage(),
                    f.o.annotation_context_usage(),
                    f.o.version_set_context_usage(),
                );
                let generation = f.o.generation();
                f.o.source_facade_fault = Some(fault);
                assert!(
                    query(&mut f, kind, line).is_none(),
                    "{fault:?}/{kind}/{line}"
                );
                assert!(f.o.is_faulted());
                idle(&f, reserved);
                assert_eq!(f.o.generation(), generation);
                assert_eq!(
                    (
                        f.o.source_read_context_usage(),
                        f.o.annotation_context_usage(),
                        f.o.version_set_context_usage()
                    ),
                    usage
                );
                if fault == Fault::BadResult {
                    assert_eq!(
                        f.o.read_result_invariant(),
                        Some(ReadResultInvariantV1::NativeSourceFacadeResultInvariant)
                    );
                }
                f.o.source_facade_fault = None;
                assert!(query(&mut f, kind, line).is_none());
                cleanup_port(&mut f, kind);
                assert!(f.o.is_faulted());
            }
        }
    }
}
#[test]
fn reentry_is_refused_while_original_capture_query_and_release_keep_the_busy_fence() {
    for kind in 0..3 {
        for line in [false, true] {
            let mut f = owner();
            let reserved = f.o.retention_usage().reserved;
            f.o.source_facade_fault = Some(Fault::Reentry);
            assert!(query(&mut f, kind, line).is_some());
            assert!(!f.o.is_faulted());
            idle(&f, reserved);
        }
    }
}
#[test]
fn copied_unknown_and_transient_facts_withhold_but_recognized_loss_survives_resource_refusal() {
    for kind in 0..3 {
        for (authorization, resources, stopped) in [
            (None, Some(false), false),
            (Some(true), Some(false), false),
            (Some(false), Some(false), true),
        ] {
            let mut f = owner();
            let reserved = f.o.retention_usage().reserved;
            assert!(invoke(&mut f, kind, false, authorization, resources, limits()).is_none());
            assert!(!f.o.is_faulted());
            assert_eq!(query(&mut f, kind, false).is_none(), stopped);
            idle(&f, reserved);
        }
    }
}
#[test]
fn explicit_caller_and_parent_copy_limits_refuse_without_fault_and_allow_later_valid_calls() {
    for kind in 0..3 {
        for coordinate in 0..4 {
            let mut f = owner();
            let reserved = f.o.retention_usage().reserved;
            let mut p = limits();
            match coordinate {
                0 => p.max_bytes = 0,
                1 => p.max_nodes = 0,
                2 => p.max_depth = 1,
                _ => f.o.set_settings_parent_limit_for_test(0),
            }
            assert!(invoke(&mut f, kind, false, Some(true), Some(true), p).is_none());
            assert!(!f.o.is_faulted());
            idle(&f, reserved);
            f.o.set_settings_parent_limit_for_test(16384);
            assert!(query(&mut f, kind, false).is_some());
        }
    }
}
#[test]
fn workspace_multiplication_overflow_is_an_ordinary_refusal_not_a_wrapped_reservation() {
    let mut f = owner();
    let reserved = f.o.retention_usage().reserved;
    f.o.set_settings_parent_limit_for_test(usize::MAX);
    let mut p = limits();
    p.max_bytes = usize::MAX;
    assert!(invoke(&mut f, 0, false, Some(true), Some(true), p).is_none());
    assert!(!f.o.is_faulted());
    idle(&f, reserved);
}
#[test]
fn native_producer_invariant_keeps_its_cause_and_is_not_misreported_as_a_facade_shape_failure() {
    for kind in 0..3 {
        let mut f = owner();
        let reserved = f.o.retention_usage().reserved;
        f.o.source_read_result_fault =
            Some(source_query::result_contract::SourceResultFault::Reference);
        assert!(query(&mut f, kind, false).is_none());
        assert!(f.o.is_faulted());
        assert_eq!(
            f.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeSourceResultInvariant)
        );
        idle(&f, reserved);
    }
}
#[test]
fn capture_off_available_and_full_never_change_native_release_or_fault_refund() {
    for capacity in [None, Some(128), Some(1)] {
        for kind in 0..3 {
            for malformed in [false, true] {
                let mut f = owner();
                f.r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                let reserved = f.o.retention_usage().reserved;
                f.o.source_facade_fault = malformed.then_some(Fault::BadResult);
                assert_eq!(query(&mut f, kind, true).is_none(), malformed);
                assert_eq!(f.o.is_faulted(), malformed);
                idle(&f, reserved);
                if capacity == Some(128) {
                    let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                    assert_eq!(
                        events
                            .iter()
                            .filter(|e| e.boundary == Boundary::SourceFacadeRefund)
                            .count(),
                        1
                    );
                    let release = events
                        .iter()
                        .position(|e| e.boundary == Boundary::SourceFacadeRelease)
                        .unwrap();
                    let refund = events
                        .iter()
                        .position(|e| e.boundary == Boundary::SourceFacadeRefund)
                        .unwrap();
                    assert!(refund < release);
                    assert!(
                        events.iter().all(
                            |e| e.service_time.is_none() && e.boundary != Boundary::Publication
                        )
                    );
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
fn actual_fixed_native_ports_release_original_range_and_line_without_acquiring_or_refreshing() {
    for kind in 0..3 {
        for line in [false, true] {
            let mut f = owner();
            let p =
                f.o.prepare(
                    &[DocumentEdits {
                        expected: f.version.clone(),
                        edits: vec![Replacement {
                            start: 0,
                            end: 4,
                            expected: "body".into(),
                            inserted: "later".into(),
                        }],
                    }],
                    &[],
                    &mut f.r,
                )
                .unwrap();
            f.o.commit(p, &mut f.r).unwrap();
            let reserved = f.o.retention_usage().reserved;
            let usage = (
                f.o.source_read_context_usage(),
                f.o.annotation_context_usage(),
                f.o.version_set_context_usage(),
            );
            let reply = content(query(&mut f, kind, line).unwrap());
            match reply {
                SourceContextContentV1::Range(v) => {
                    assert_eq!(v.bytes(), b"body");
                    assert_eq!(v.source, f.version);
                }
                SourceContextContentV1::Line(v) => {
                    assert_eq!(v.body(), b"body");
                    assert_eq!(v.delimiter(), b"\r\n");
                    assert_eq!(v.source, f.version);
                }
            }
            idle(&f, reserved);
            assert_eq!(
                (
                    f.o.source_read_context_usage(),
                    f.o.annotation_context_usage(),
                    f.o.version_set_context_usage()
                ),
                usage
            );
            assert!(!f.o.is_faulted());
        }
    }
}
