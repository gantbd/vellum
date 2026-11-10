use super::*;
use crate::coordinator::source_query::result_contract::tests::{
    Fixture as Base, member_facts, source_facts,
};
use ReadAcquisitionFacadePortV1 as P;
use ReadAcquisitionFacadeReplyV1 as R;
pub(in crate::coordinator) struct Fixture {
    pub(in crate::coordinator) base: Base,
    pub(super) single: PreparedAnnotationProjectionId,
    pub(super) set: PreparedVersionSetId,
}
pub(in crate::coordinator) struct Params {
    pub(super) classes: Option<AnnotationProjectionClassesV1>,
    pub(super) authorized: Option<bool>,
    pub(super) resources: Option<bool>,
    pub(super) ready: Option<bool>,
    pub(super) wrong_scope: bool,
    pub(super) limits: CaptureLimitsV1,
}
pub(in crate::coordinator) fn params() -> Params {
    Params {
        classes: None,
        authorized: Some(true),
        resources: Some(true),
        ready: Some(true),
        wrong_scope: false,
        limits: CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 512,
            max_depth: 16,
        },
    }
}
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
pub(in crate::coordinator) fn owner() -> Fixture {
    let mut base = crate::coordinator::source_query::result_contract::tests::owner();
    base.o.set_settings_parent_limit_for_test(16384);
    let single = base
        .o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &base.s,
            &capture(&base.s),
            &mut base.r,
        )
        .unwrap();
    let set = base
        .o
        .prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &base.s,
                data: AnnotationProjectionClassesV1::Combined,
            }],
            &[capture(&base.s)],
            &mut base.r,
        )
        .unwrap();
    Fixture { base, single, set }
}
fn invoke(f: &mut Fixture, kind: usize, p: Params) -> Option<R> {
    invoke_with(f, kind, p, |o, request, r| {
        o.acquire_read_facade_v1(request, r)
    })
}
#[test]
fn typed_acquisition_facade_refuses_invalid_capture_without_consuming_any_of_the_six_original_ports()
 {
    for kind in 0..6 {
        let mut f = owner();
        f.base.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        crate::coordinator::release_measurements::tests::enable(&mut f.base.o, &mut f.base.r);
        invoke_with(&mut f, kind, params(), |o, req, r| {
            let before = o.retention_usage().reserved;
            assert!(matches!(
                o.try_acquire_read_facade_v1(
                    ReadAcquisitionFacadeRequestV1 {
                        limits: CaptureLimitsV1 {
                            max_nodes: 0,
                            ..req.limits
                        },
                        ..req
                    },
                    r
                ),
                Err(Error::InvalidRequest)
            ));
            assert_eq!(o.retention_usage().reserved, before);
            let out = o.try_acquire_read_facade_v1(req, r);
            assert!(out.is_ok(), "port {kind}");
            out.ok()
        });
        let events: Vec<_> = std::iter::from_fn(|| f.base.r.pop_front()).collect();
        assert!(
            events
                .iter()
                .any(|e| e.boundary == Boundary::AcquisitionFacadeRelease
                    && e.service_time.is_some())
        );
        assert!(
            events
                .iter()
                .filter(|e| e.boundary != Boundary::AcquisitionFacadeRelease)
                .all(|e| e.service_time.is_none())
        );
        assert!(!f.base.o.is_faulted());
        assert_eq!(
            crate::coordinator::release_measurements::tests::totals(&mut f.base.o, &mut f.base.r),
            [1, 0, 1, 0, 0, 0]
        );
        assert_eq!(f.base.o.read_facade_usage().workspace_bytes, 0);
    }
}
#[test]
fn typed_acquisition_facade_reports_the_native_source_context_ceiling_without_spending_another_identity()
 {
    let mut f = owner();
    let mut reached_limit = false;
    for _ in 0..32 {
        invoke_with(&mut f, 0, params(), |o, req, r| {
            let before = o.source_read_context_usage();
            match o.try_acquire_read_facade_v1(req, r) {
                Ok(value) => Some(value),
                Err(Error::Limit) => {
                    assert_eq!(o.source_read_context_usage(), before);
                    reached_limit = true;
                    None
                }
                Err(e) => panic!("native ceiling collapsed or misclassified: {e:?}"),
            }
        });
        if reached_limit {
            break;
        }
    }
    assert!(reached_limit);
    assert!(!f.base.o.is_faulted());
    assert_eq!(f.base.o.read_facade_usage().workspace_bytes, 0);
}
pub(in crate::coordinator) fn invoke_with(
    f: &mut Fixture,
    kind: usize,
    p: Params,
    run: impl for<'a> FnOnce(
        &mut SourceCoordinator,
        ReadAcquisitionFacadeRequestV1<'a>,
        &mut Recorder,
    ) -> Option<R>,
) -> Option<R> {
    let mut requested = f.base.s.clone();
    if p.wrong_scope {
        requested.policy.push_str("-other");
    }
    let mut actor = f.base.s.context.clone();
    actor.recipient = "separate-agent".into();
    let classes = p.classes.unwrap_or(if kind == 2 || kind == 4 {
        AnnotationProjectionClassesV1::Combined
    } else {
        AnnotationProjectionClassesV1::Source
    });
    let mut source = source_facts(&f.base.s);
    source.authorized = p.authorized;
    source.resources_ready = p.resources;
    let single = TrustedPreparedAnnotationFactsV1 {
        projection: &f.single,
        scope: &f.base.s,
        authorized: p.authorized,
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: p.resources,
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let installed = TrustedAnnotationContextFactsV1 {
        context: &f.base.combined,
        scope: &f.base.s,
        authorized: p.authorized,
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: p.resources,
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let mut member = member_facts(&f.base.s);
    member.authorized = p.authorized;
    member.resources_ready = p.resources;
    let members = [member];
    let prepared = TrustedPreparedVersionSetFactsV1 {
        projection: &f.set,
        members: &members,
    };
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.base.set.context,
        members: &members,
    };
    let participants = [VersionSetParticipantV1 {
        scope: &requested,
        data: classes,
    }];
    let port = match kind {
        0 => P::SourceCurrent {
            scope: &requested,
            facts: &source,
        },
        1 => P::SourceRetained {
            scope: &requested,
            original: &f.base.source,
            facts: &source,
        },
        2 => P::AnnotationCurrent {
            scope: &requested,
            classes,
            prepared: &f.single,
            facts: &single,
        },
        3 => P::AnnotationRetained {
            scope: &requested,
            classes,
            original: &f.base.combined,
            facts: &installed,
            preparation_ready: p.ready,
        },
        4 => P::VersionSetCurrent {
            participants: &participants,
            prepared: &f.set,
            facts: &prepared,
        },
        _ => P::VersionSetRetained {
            participants: &participants,
            original: &f.base.set.context,
            facts: &parent,
            preparation_ready: p.ready,
        },
    };
    run(
        &mut f.base.o,
        ReadAcquisitionFacadeRequestV1 {
            port,
            actor: &actor,
            limits: p.limits,
        },
        &mut f.base.r,
    )
}
pub(super) fn counters(f: &Fixture) -> [usize; 5] {
    [
        f.base.o.source_read_context_usage().issued_lifetime,
        f.base.o.annotation_context_usage().issued_lifetime,
        f.base.o.version_set_context_usage().issued_lifetime,
        f.base.o.annotation_preparation_usage().prepared,
        f.base.o.version_set_preparation_usage().prepared,
    ]
}
#[test]
fn ordered_cross_document_preparation_binding_refuses_reordered_meaning_before_accepting_loss() {
    let mut f = owner();
    let d = f.base.o.create(b"peer", &mut f.base.r).unwrap();
    let mut peer = f.base.s.clone();
    peer.document = d.document;
    let correct = [
        VersionSetParticipantV1 {
            scope: &f.base.s,
            data: AnnotationProjectionClassesV1::Combined,
        },
        VersionSetParticipantV1 {
            scope: &peer,
            data: AnnotationProjectionClassesV1::Source,
        },
    ];
    let prepared = f
        .base
        .o
        .prepare_version_set(
            &correct,
            &[capture(&f.base.s), capture(&peer)],
            &mut f.base.r,
        )
        .unwrap();
    let wrong = [
        VersionSetParticipantV1 {
            scope: &peer,
            data: AnnotationProjectionClassesV1::Source,
        },
        VersionSetParticipantV1 {
            scope: &f.base.s,
            data: AnnotationProjectionClassesV1::Combined,
        },
    ];
    let mut loss = member_facts(&f.base.s);
    loss.authorized = Some(false);
    let supplied = TrustedPreparedVersionSetFactsV1 {
        projection: &prepared,
        members: &[loss, member_facts(&peer)],
    };
    let before = f.base.o.version_set_preparation_usage();
    let reserved = f.base.o.retention_usage().reserved;
    assert!(
        f.base
            .o
            .acquire_read_facade_v1(
                ReadAcquisitionFacadeRequestV1 {
                    port: P::VersionSetCurrent {
                        participants: &wrong,
                        prepared: &prepared,
                        facts: &supplied
                    },
                    actor: &f.base.s.context,
                    limits: params().limits,
                },
                &mut f.base.r
            )
            .is_none()
    );
    assert_eq!(f.base.o.version_set_preparation_usage(), before);
    assert!(!f.base.o.is_faulted());
    let facts = TrustedPreparedVersionSetFactsV1 {
        projection: &prepared,
        members: &[member_facts(&f.base.s), member_facts(&peer)],
    };
    let Some(R::VersionSet(reply)) = f.base.o.acquire_read_facade_v1(
        ReadAcquisitionFacadeRequestV1 {
            port: P::VersionSetCurrent {
                participants: &correct,
                prepared: &prepared,
                facts: &facts,
            },
            actor: &f.base.s.context,
            limits: params().limits,
        },
        &mut f.base.r,
    ) else {
        panic!("correct ordered cut")
    };
    assert_eq!(reply.participants[0].document, f.base.s.document);
    assert_eq!(reply.participants[1].document, peer.document);
    idle(&f, reserved);
}
#[test]
fn facade_retained_source_copy_survives_retirement_of_its_original() {
    use crate::coordinator::source_query::result_contract::tests::cleanup;
    let mut f = owner();
    let copy = invoke(&mut f, 1, params()).unwrap();
    assert!(
        f.base
            .o
            .stop_source_read_context(&f.base.source, &cleanup(&f.base.s), &mut f.base.r)
    );
    assert!(f.base.o.retire_source_read_context(
        &f.base.source,
        &cleanup(&f.base.s),
        &mut f.base.r
    ));
    assert_eq!(read(&mut f, copy).body(), b"body");
    assert!(!f.base.o.is_faulted());
}
fn idle(f: &Fixture, reserved: usize) {
    assert_eq!(f.base.o.retention_usage().reserved, reserved);
    assert_eq!(
        f.base.o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
}
#[test]
fn acquisition_fault_frames_preserve_native_installations_and_cut_consumption_after_handoff() {
    for kind in 0..6 {
        for fault in [
            Fault::Classification,
            Fault::Workspace,
            Fault::BeforeNative,
            Fault::AfterNative,
            Fault::BadResult,
            Fault::RefundDrift,
        ] {
            let mut f = owner();
            let before = counters(&f);
            let reserved = f.base.o.retention_usage().reserved;
            let generation = f.base.o.generation();
            f.base.o.source_facade_fault = Some(fault);
            assert!(invoke(&mut f, kind, params()).is_none(), "{kind}/{fault:?}");
            assert!(f.base.o.is_faulted());
            idle(&f, reserved);
            assert_eq!(f.base.o.generation(), generation);
            let mut expected = before;
            if matches!(
                fault,
                Fault::AfterNative | Fault::BadResult | Fault::RefundDrift
            ) {
                expected[if kind < 2 {
                    0
                } else if kind < 4 {
                    1
                } else {
                    2
                }] += if kind < 4 { 1 } else { 2 };
                if kind == 2 {
                    expected[3] -= 1;
                }
                if kind == 4 {
                    expected[4] -= 1;
                }
            }
            assert_eq!(counters(&f), expected, "{kind}/{fault:?}");
            if fault == Fault::BadResult && kind != 0 {
                assert_eq!(
                    f.base.o.read_result_invariant(),
                    Some(ReadResultInvariantV1::NativeAcquisitionFacadeResultInvariant)
                );
            }
            f.base.o.source_facade_fault = None;
            assert!(invoke(&mut f, kind, params()).is_none());
            assert_eq!(counters(&f), expected);
        }
    }
}
#[test]
fn each_acquisition_uses_eight_frames_of_actual_parent_headroom_and_refunds_only_its_temporary_charge()
 {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 8 * params().limits.max_bytes;
    for kind in 0..6 {
        for available in [fixed - 1, fixed, total - 1, total, total + 1] {
            let mut f = owner();
            let original = f.base.o.retention_usage().reserved;
            let pressure = f.base.o.retention_usage().maximum.unwrap() - original - available;
            let mut held = f.base.o.retention.reserve(pressure).unwrap();
            let before = f.base.o.retention_usage().reserved;
            let counts = counters(&f);
            assert_eq!(invoke(&mut f, kind, params()).is_some(), available >= total);
            idle(&f, before);
            if available < total {
                assert_eq!(counters(&f), counts);
            }
            assert!(!f.base.o.is_faulted());
            f.base.o.retention.release(&mut held).unwrap();
            idle(&f, original);
        }
    }
}
#[test]
fn acquisition_copy_limits_refuse_before_accepting_supplied_loss_and_do_not_consume_preparations() {
    for kind in 0..6 {
        for coordinate in 0..4 {
            let mut f = owner();
            let before = counters(&f);
            let reserved = f.base.o.retention_usage().reserved;
            let mut p = params();
            p.authorized = Some(false);
            match coordinate {
                0 => p.limits.max_bytes = 0,
                1 => p.limits.max_nodes = 0,
                2 => p.limits.max_depth = 1,
                _ => f.base.o.set_settings_parent_limit_for_test(0),
            }
            assert!(invoke(&mut f, kind, p).is_none());
            assert_eq!(counters(&f), before);
            assert!(!f.base.o.is_faulted());
            idle(&f, reserved);
            f.base.o.set_settings_parent_limit_for_test(16384);
            assert!(invoke(&mut f, kind, params()).is_some());
        }
    }
}
#[test]
fn unknown_host_facts_withhold_but_correctly_bound_known_loss_remains_absorbing() {
    for kind in 0..6 {
        let mut f = owner();
        let mut p = params();
        p.authorized = None;
        assert!(invoke(&mut f, kind, p).is_none());
        assert!(!f.base.o.is_faulted());
        assert!(invoke(&mut f, kind, params()).is_some());
    }
    for kind in 0..6 {
        let mut f = owner();
        let mut p = params();
        p.authorized = Some(false);
        p.resources = Some(false);
        assert!(invoke(&mut f, kind, p).is_none());
        assert!(!f.base.o.is_faulted());
        assert_eq!(invoke(&mut f, kind, params()).is_none(), kind != 0);
    }
}
#[test]
fn acquisition_reentry_is_an_ordinary_refusal_and_the_original_handoff_stays_healthy() {
    for kind in 0..6 {
        let mut f = owner();
        let reserved = f.base.o.retention_usage().reserved;
        f.base.o.source_facade_fault = Some(Fault::Reentry);
        assert!(invoke(&mut f, kind, params()).is_some());
        assert!(!f.base.o.is_faulted());
        idle(&f, reserved);
    }
}
#[test]
fn native_preinstallation_invariant_is_not_relabelled_as_a_late_facade_fault() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault;
    for kind in 0..6 {
        let mut f = owner();
        let before = counters(&f);
        let reserved = f.base.o.retention_usage().reserved;
        f.base.o.acquisition_result_fault = Some(AcquisitionResultFault::Context);
        assert!(invoke(&mut f, kind, params()).is_none());
        assert!(f.base.o.is_faulted());
        assert_eq!(
            f.base.o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeReadAcquisitionResultInvariant)
        );
        assert_eq!(counters(&f), before);
        idle(&f, reserved);
    }
}
#[test]
fn acquisition_diagnostics_are_optional_bounded_content_free_and_follow_protected_refund() {
    for kind in 0..6 {
        for capacity in [None, Some(128), Some(1)] {
            for malformed in [false, true] {
                let mut f = owner();
                f.base.r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                f.base.o.source_facade_fault = malformed.then_some(Fault::RefundDrift);
                let reserved = f.base.o.retention_usage().reserved;
                assert_eq!(invoke(&mut f, kind, params()).is_none(), malformed);
                assert_eq!(f.base.o.is_faulted(), malformed);
                idle(&f, reserved);
                if capacity == Some(128) {
                    let events: Vec<_> = std::iter::from_fn(|| f.base.r.pop_front()).collect();
                    let refund = events
                        .iter()
                        .position(|e| e.boundary == Boundary::AcquisitionFacadeRefund)
                        .unwrap();
                    let release = events
                        .iter()
                        .position(|e| e.boundary == Boundary::AcquisitionFacadeRelease)
                        .unwrap();
                    assert!(refund < release);
                    assert_eq!(
                        events
                            .iter()
                            .filter(|e| e.boundary == Boundary::AcquisitionFacadeRefund)
                            .count(),
                        1
                    );
                    assert!(events.iter().all(|e| e.service_time.is_none()
                        && !matches!(
                            e.boundary,
                            Boundary::Publication
                                | Boundary::SourceFacadeRefund
                                | Boundary::AnnotationFacadeRefund
                        )));
                    assert!(!format!("{events:?}").contains("body"));
                } else if capacity == Some(1) {
                    assert!(f.base.r.dropped() > 0);
                } else {
                    assert_eq!(f.base.r.queued(), 0);
                }
            }
        }
    }
}
fn read(f: &mut Fixture, reply: R) -> SourceLineReply {
    let q = SourceContextQueryV1::Line { index: 0 };
    let content = match reply {
        R::Source(id) => {
            f.base
                .o
                .query_source_read_context(
                    &id,
                    &f.base.s,
                    q,
                    &source_facts(&f.base.s),
                    &mut f.base.r,
                )
                .unwrap()
                .content
        }
        R::Annotation(acq) => {
            let p = params();
            let facts = TrustedAnnotationContextFactsV1 {
                context: &acq.context,
                scope: &f.base.s,
                authorized: p.authorized,
                scope_allowed: Some(true),
                continuous: Some(true),
                live: Some(true),
                not_stopped: Some(true),
                annotation_readable: Some(true),
                source_readable: Some(true),
                projection_current: Some(true),
                retained: Some(true),
                resources_ready: p.resources,
                reply_ready: Some(true),
                transfer_ready: Some(true),
            };
            f.base
                .o
                .query_annotation_context_source(&acq.context, &f.base.s, q, &facts, &mut f.base.r)
                .unwrap()
                .content
        }
        R::VersionSet(acq) => {
            f.base
                .o
                .query_version_set_source(
                    &acq.participants[0].snapshot,
                    q,
                    &TrustedVersionSetContextFactsV1 {
                        context: &acq.context,
                        members: &[member_facts(&f.base.s)],
                    },
                    &mut f.base.r,
                )
                .unwrap()
                .content
        }
    };
    let SourceContextContentV1::Line(line) = content else {
        panic!("line")
    };
    line
}
#[test]
fn all_six_fixed_acquisition_routes_preserve_prepared_and_retained_roots_without_refresh() {
    for kind in 0..6 {
        let mut f = owner();
        let plan = f
            .base
            .o
            .prepare(
                &[DocumentEdits {
                    expected: f.base.version.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 4,
                        expected: "body".into(),
                        inserted: "new".into(),
                    }],
                }],
                &[],
                &mut f.base.r,
            )
            .unwrap();
        f.base.o.commit(plan, &mut f.base.r).unwrap();
        let reserved = f.base.o.retention_usage().reserved;
        let reply = invoke(&mut f, kind, params()).unwrap();
        idle(&f, reserved);
        let line = read(&mut f, reply);
        assert_eq!(
            line.body(),
            if kind == 0 {
                b"new".as_slice()
            } else {
                b"body".as_slice()
            }
        );
        assert_eq!(line.delimiter(), b"\r\n");
        if kind != 0 {
            assert_eq!(line.source, f.base.version);
        }
        assert!(!f.base.o.is_faulted());
    }
}
#[test]
fn class_and_scope_mismatch_cannot_lend_false_facts_to_a_held_preparation_or_consume_it() {
    for kind in [2, 4] {
        for wrong_scope in [false, true] {
            let mut f = owner();
            let mut p = params();
            p.authorized = Some(false);
            p.wrong_scope = wrong_scope;
            if !wrong_scope {
                p.classes = Some(AnnotationProjectionClassesV1::Annotations);
            }
            let before = counters(&f);
            let reserved = f.base.o.retention_usage().reserved;
            assert!(invoke(&mut f, kind, p).is_none());
            assert_eq!(counters(&f), before);
            assert_eq!(f.base.o.annotation_preparation_usage().invalidated, 0);
            assert_eq!(f.base.o.version_set_preparation_usage().invalidated, 0);
            idle(&f, reserved);
            assert!(!f.base.o.is_faulted());
            assert!(invoke(&mut f, kind, params()).is_some());
            assert!(invoke(&mut f, kind, params()).is_none()); // original cut consumed exactly once
        }
    }
}
