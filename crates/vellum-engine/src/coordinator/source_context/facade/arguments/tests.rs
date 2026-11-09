use super::*;
use crate::coordinator::source_query::result_contract::tests::{
    Fixture, member_facts, source_facts,
};
use SourceEnvelopeValueV1::{Data, Document, Object, Snapshot};
fn owner() -> Fixture {
    let mut f = crate::coordinator::source_query::result_contract::tests::owner();
    f.o.set_settings_parent_limit_for_test(16384);
    f
}
#[test]
fn decoded_source_facade_exposes_typed_errors_without_repair_or_loss_of_original_bindings() {
    use crate::coordinator::source_context::search::result_contract::tests::with_facts;
    for kind in 0..3 {
        for route in ["typed", "dynamic"] {
            let mut f = owner();
            f.r = Recorder::try_with_capacity(256)
                .unwrap()
                .with_native_timing(
                    crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                );
            crate::coordinator::release_measurements::tests::enable(&mut f.o, &mut f.r);
            let base = f.o.retention_usage().reserved;
            with_facts(&mut f, kind, |o, source, scope, facts, r| {
                let mut input = Input::new(source, &scope.document, route, true);
                input.profile = &scope.context.profile;
                input.lo = 999.0;
                with_input(&input, |envelope| {
                    assert!(matches!(
                        o.try_query_decoded_source_facade_v1(
                            DecodedSourceFacadeRequestV1 {
                                envelope,
                                actor: &scope.context,
                                scope,
                                facts,
                                limits: limits()
                            },
                            r
                        ),
                        Err(Error::InvalidRequest)
                    ));
                });
                input.lo = 0.0;
                with_input(&input, |envelope| {
                    assert!(
                        o.try_query_decoded_source_facade_v1(
                            DecodedSourceFacadeRequestV1 {
                                envelope,
                                actor: &scope.context,
                                scope,
                                facts,
                                limits: limits()
                            },
                            r
                        )
                        .is_ok()
                    );
                });
            });
            assert_eq!(f.o.retention_usage().reserved, base);
            let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            assert!(
                events.iter().any(
                    |e| e.boundary == Boundary::SourceFacadeRelease && e.service_time.is_some()
                )
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
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 512,
        max_depth: 16,
    }
}
struct Input<'a, 'h> {
    route: &'a str,
    selector: InertValueV1<'a>,
    version: f64,
    variant: &'a str,
    operation: &'a str,
    contract: &'a str,
    snapshot: SourceEnvelopeValueV1<'a, 'h>,
    document: SourceEnvelopeValueV1<'a, 'h>,
    profile: &'a str,
    lo: f64,
    hi: f64,
    duplicate: bool,
    extra: bool,
}
impl<'a, 'h> Input<'a, 'h> {
    fn new(
        source: SearchReadContextV1<'h>,
        document: &'h DocumentId,
        route: &'a str,
        line: bool,
    ) -> Self {
        let op = if line { "line" } else { "bounded-range" };
        Self {
            route,
            selector: if route == "typed" {
                InertValueV1::Number(1.0)
            } else {
                InertValueV1::Text("read-document")
            },
            version: 1.0,
            variant: op,
            operation: op,
            contract: "source-query/1",
            snapshot: Snapshot(source),
            document: Document(document),
            profile: "profile",
            lo: 0.0,
            hi: 4.0,
            duplicate: false,
            extra: false,
        }
    }
}
fn with_input<'a, 'h, T>(
    i: &Input<'a, 'h>,
    f: impl for<'v> FnOnce(&'v SourceEnvelopeValueV1<'v, 'h>) -> T,
) -> T {
    let mut args = [
        ("contract", Data(InertValueV1::Text(i.contract))),
        ("operation", Data(InertValueV1::Text(i.operation))),
        ("snapshot", i.snapshot),
        ("document", i.document),
        ("profile", Data(InertValueV1::Text(i.profile))),
        (
            if i.variant == "line" { "line" } else { "lo" },
            Data(InertValueV1::Number(i.lo)),
        ),
        ("hi", Data(InertValueV1::Number(i.hi))),
        ("extra", Data(InertValueV1::Null)),
    ];
    let mut n = if i.variant == "line" { 6 } else { 7 };
    if i.extra {
        args[n] = args[7];
        n += 1;
    }
    if i.duplicate {
        args[n - 1].0 = "contract";
    }
    let header = [
        (
            if i.route == "typed" { "family" } else { "name" },
            Data(i.selector),
        ),
        ("variant", Data(InertValueV1::Text(i.variant))),
        ("version", Data(InertValueV1::Number(i.version))),
        ("arguments", Object(&args[..n])),
    ];
    let outer = [
        ("route", Data(InertValueV1::Text(i.route))),
        ("request", Object(&header)),
    ];
    f(&Object(&outer))
}
fn normalize_input<'h>(
    i: &Input<'_, 'h>,
    actor: &SettingsExecutionContextV1,
) -> Result<NormalizedSourceArgumentsV1<'h>, Error> {
    with_input(i, |e| {
        normalize_source_envelope_v1(
            e,
            actor,
            limits(),
            limits().max_bytes,
            &mut Recorder::disabled(),
        )
    })
}
fn invoke(
    f: &mut Fixture,
    kind: usize,
    route: &str,
    line: bool,
    profile: &str,
    authorization: Option<bool>,
) -> Option<SourceFacadeReplyV1> {
    let mut actor = f.s.context.clone();
    actor.recipient = "separate-actor".into();
    let mut sf = source_facts(&f.s);
    sf.authorized = authorization;
    let cf = TrustedAnnotationContextFactsV1 {
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
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    let mut mf = member_facts(&f.s);
    mf.authorized = authorization;
    let parent = TrustedVersionSetContextFactsV1 {
        context: &f.set.context,
        members: &[mf],
    };
    let vf = TrustedVersionSetSearchFactsV1 {
        member: &f.set.participants[0].snapshot,
        scope: &f.s,
        parent: &parent,
    };
    let (source, facts) = match kind {
        0 => (
            SearchReadContextV1::Source(&f.source),
            SearchReadFactsV1::Source(&sf),
        ),
        1 => (
            SearchReadContextV1::Combined(&f.combined),
            SearchReadFactsV1::Combined(&cf),
        ),
        _ => (
            SearchReadContextV1::VersionSet(&f.set.participants[0].snapshot),
            SearchReadFactsV1::VersionSet(&vf),
        ),
    };
    let mut i = Input::new(source, &f.s.document, route, line);
    i.profile = profile;
    with_input(&i, |e| {
        f.o.query_decoded_source_facade_v1(
            DecodedSourceFacadeRequestV1 {
                envelope: e,
                actor: &actor,
                scope: &f.s,
                limits: limits(),
                facts,
            },
            &mut f.r,
        )
    })
}
#[test]
fn native_source_routes_normalize_to_equal_owned_meaning_and_preserve_both_capabilities() {
    let f = owner();
    for source in [
        SearchReadContextV1::Source(&f.source),
        SearchReadContextV1::Combined(&f.combined),
        SearchReadContextV1::VersionSet(&f.set.participants[0].snapshot),
    ] {
        for line in [false, true] {
            let n = normalize_input(
                &Input::new(source, &f.s.document, "typed", line),
                &f.s.context,
            )
            .unwrap();
            let m = normalize_input(
                &Input::new(source, &f.s.document, "dynamic", line),
                &f.s.context,
            )
            .unwrap();
            assert_eq!(n, m);
            assert!(std::ptr::eq(n.document(), &f.s.document));
            match (source, n.source()) {
                (SearchReadContextV1::Source(a), SearchReadContextV1::Source(b)) => {
                    assert!(std::ptr::eq(a, b))
                }
                (SearchReadContextV1::Combined(a), SearchReadContextV1::Combined(b)) => {
                    assert!(std::ptr::eq(a, b))
                }
                (SearchReadContextV1::VersionSet(a), SearchReadContextV1::VersionSet(b)) => {
                    assert!(std::ptr::eq(a, b))
                }
                _ => panic!("changed role"),
            }
        }
    }
}
#[test]
fn normalized_source_data_outlives_packet_and_strings_but_not_native_handles() {
    let f = owner();
    let n = {
        let p = String::from("temporary-profile");
        let mut actor = f.s.context.clone();
        actor.purpose = "temporary-purpose".into();
        let mut i = Input::new(
            SearchReadContextV1::Source(&f.source),
            &f.s.document,
            "typed",
            false,
        );
        i.profile = &p;
        normalize_input(&i, &actor).unwrap()
    };
    assert_eq!(n.profile(), "temporary-profile");
    assert_eq!(n.actor().purpose, "temporary-purpose");
    assert_eq!(
        n.query(),
        SourceContextQueryV1::Range {
            start: BytePosition(0),
            end: BytePosition(4)
        }
    );
    assert_eq!(n.limits(), limits());
    assert!(n.owned_bytes() <= limits().max_bytes);
}
#[test]
fn malformed_source_vocabulary_and_opaque_role_substitution_refuse_without_coercion() {
    let f = owner();
    for mode in 0..18 {
        let mut i = Input::new(
            SearchReadContextV1::Source(&f.source),
            &f.s.document,
            "typed",
            false,
        );
        match mode {
            0 => i.route = "unknown",
            1 => i.selector = InertValueV1::Number(2.0),
            2 => i.version = 2.0,
            3 => i.variant = "unknown",
            4 => i.operation = "line",
            5 => i.contract = "source-query/2",
            6 => i.profile = "",
            7 => i.lo = -1.0,
            8 => i.lo = 0.5,
            9 => i.hi = f64::NAN,
            10 => i.hi = 9_007_199_254_740_992.0,
            11 => i.lo = 5.0,
            12 => i.duplicate = true,
            13 => i.extra = true,
            14 => i.snapshot = Document(&f.s.document),
            15 => i.document = Snapshot(SearchReadContextV1::Source(&f.source)),
            16 => i.document = Data(InertValueV1::Null),
            _ => i.selector = InertValueV1::Text("1"),
        }
        assert!(normalize_input(&i, &f.s.context).is_err(), "mode {mode}");
    }
}
#[test]
fn source_captures_have_independent_byte_node_depth_and_fixed_template_limits() {
    let f = owner();
    let i = Input::new(
        SearchReadContextV1::Source(&f.source),
        &f.s.document,
        "typed",
        false,
    );
    for l in [
        CaptureLimitsV1 {
            max_bytes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 0,
            ..limits()
        },
    ] {
        assert!(
            with_input(&i, |e| normalize_source_envelope_v1(
                e,
                &f.s.context,
                l,
                l.max_bytes,
                &mut Recorder::disabled()
            ))
            .is_err()
        );
    }
    assert!(
        with_input(&i, |e| normalize_source_envelope_v1(
            e,
            &f.s.context,
            limits(),
            1,
            &mut Recorder::disabled()
        ))
        .is_err()
    );
    let mut actor = f.s.context.clone();
    actor.purpose.clear();
    assert!(normalize_input(&i, &actor).is_err());
}
#[test]
fn native_source_decoded_dispatch_reads_all_three_original_owners_through_both_routes() {
    for kind in 0..3 {
        for route in ["typed", "dynamic"] {
            for line in [false, true] {
                let mut f = owner();
                let base = f.o.retention_usage().reserved;
                let profile = f.s.context.profile.clone();
                let reply = invoke(&mut f, kind, route, line, &profile, Some(true)).unwrap();
                let content = match reply {
                    SourceFacadeReplyV1::Source(v) => v.content,
                    SourceFacadeReplyV1::Combined(v) => v.content,
                    SourceFacadeReplyV1::VersionSet(v) => v.content,
                };
                assert!(matches!(
                    (line, content),
                    (true, SourceContextContentV1::Line(_))
                        | (false, SourceContextContentV1::Range(_))
                ));
                assert_eq!(f.o.retention_usage().reserved, base);
                assert_eq!(
                    f.o.read_facade_usage(),
                    ReadFacadeUsageV1 {
                        busy: false,
                        workspace_bytes: 0
                    }
                );
                assert!(!f.o.is_faulted());
            }
        }
    }
}
#[test]
fn unsupported_source_profile_cannot_poison_a_valid_context_with_current_loss() {
    for kind in 0..3 {
        let mut f = owner();
        let profile = f.s.context.profile.clone();
        assert!(invoke(&mut f, kind, "typed", false, "unsupported", Some(false)).is_none());
        assert!(invoke(&mut f, kind, "dynamic", false, &profile, Some(true)).is_some());
    }
}
#[test]
fn accepted_source_authorization_loss_remains_absorbing_after_normalized_dispatch() {
    for kind in 0..3 {
        let mut f = owner();
        let profile = f.s.context.profile.clone();
        assert!(invoke(&mut f, kind, "typed", false, &profile, Some(false)).is_none());
        assert!(invoke(&mut f, kind, "dynamic", false, &profile, Some(true)).is_none());
        assert!(!f.o.is_faulted());
    }
}
#[test]
fn decoded_source_faults_keep_original_obligations_and_protected_cleanup() {
    for fault in [
        Fault::Classification,
        Fault::Workspace,
        Fault::BeforeNative,
        Fault::AfterNative,
        Fault::BadResult,
        Fault::RefundDrift,
    ] {
        for kind in 0..3 {
            let mut f = owner();
            let base = f.o.retention_usage().reserved;
            let profile = f.s.context.profile.clone();
            f.o.source_facade_fault = Some(fault);
            assert!(invoke(&mut f, kind, "typed", false, &profile, Some(true)).is_none());
            assert!(f.o.is_faulted());
            assert!(!f.o.read_facade_usage().busy);
            if fault != Fault::RefundDrift {
                assert_eq!(f.o.retention_usage().reserved, base);
                assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
            }
            f.o.source_facade_fault = None;
            super::super::tests::cleanup_port(&mut f, kind);
        }
    }
}
#[test]
fn decoded_source_reentry_and_lowered_owner_ceiling_use_one_guard() {
    let mut f = owner();
    let profile = f.s.context.profile.clone();
    let base = f.o.retention_usage().reserved;
    f.o.source_facade_fault = Some(Fault::Reentry);
    assert!(invoke(&mut f, 0, "typed", false, &profile, Some(true)).is_some());
    f.o.source_facade_fault = None;
    f.o.set_settings_parent_limit_for_test(1);
    assert!(invoke(&mut f, 0, "typed", false, &profile, Some(false)).is_none());
    f.o.set_settings_parent_limit_for_test(16384);
    assert!(invoke(&mut f, 0, "dynamic", false, &profile, Some(true)).is_some());
    assert_eq!(f.o.retention_usage().reserved, base);
}
#[test]
fn source_normalization_observations_are_optional_bounded_and_redacted() {
    let f = owner();
    let i = Input::new(
        SearchReadContextV1::Source(&f.source),
        &f.s.document,
        "typed",
        false,
    );
    let mut off = Recorder::disabled();
    let mut on = Recorder::try_with_capacity(1).unwrap();
    let a = with_input(&i, |e| {
        normalize_source_envelope_v1(e, &f.s.context, limits(), 16384, &mut off)
    })
    .unwrap();
    let b = with_input(&i, |e| {
        normalize_source_envelope_v1(e, &f.s.context, limits(), 16384, &mut on)
    })
    .unwrap();
    assert_eq!(a, b);
    assert_eq!(format!("{a:?}"), "NormalizedSourceArgumentsV1(<redacted>)");
    assert_eq!(off.queued(), 0);
    assert_eq!(on.queued(), 1);
    assert!(on.dropped() > 0);
}

#[test]
fn foreign_native_document_refuses_before_current_loss_is_applied() {
    let mut f = owner();
    let other = f.o.create(b"other", &mut f.r).unwrap();
    let mut i = Input::new(
        SearchReadContextV1::Source(&f.source),
        &other.document,
        "typed",
        false,
    );
    i.profile = &f.s.context.profile;
    let mut lost = source_facts(&f.s);
    lost.authorized = Some(false);
    assert!(
        with_input(&i, |e| f.o.query_decoded_source_facade_v1(
            DecodedSourceFacadeRequestV1 {
                envelope: e,
                actor: &f.s.context,
                scope: &f.s,
                limits: limits(),
                facts: SearchReadFactsV1::Source(&lost),
            },
            &mut f.r
        ))
        .is_none()
    );
    let profile = f.s.context.profile.clone();
    assert!(invoke(&mut f, 0, "dynamic", false, &profile, Some(true)).is_some());
}

#[test]
fn decoded_source_admission_uses_exact_shared_parent_headroom_without_double_admission() {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 7 * limits().max_bytes;
    for available in [fixed - 1, fixed, total - 1, total, total + 1] {
        for kind in 0..3 {
            let mut f = owner();
            let original = f.o.retention_usage().reserved;
            let pressure = f.o.retention_usage().maximum.unwrap() - original - available;
            let mut held = f.o.retention.reserve(pressure).unwrap();
            let before = f.o.retention_usage().reserved;
            let profile = f.s.context.profile.clone();
            assert_eq!(
                invoke(&mut f, kind, "typed", false, &profile, Some(true)).is_some(),
                available >= total
            );
            assert_eq!(f.o.retention_usage().reserved, before);
            assert!(!f.o.is_faulted());
            assert_eq!(
                f.o.read_facade_usage(),
                ReadFacadeUsageV1 {
                    busy: false,
                    workspace_bytes: 0
                }
            );
            f.o.retention.release(&mut held).unwrap();
            assert_eq!(f.o.retention_usage().reserved, original);
        }
    }
}

#[test]
fn source_capture_exact_node_and_depth_boundaries_are_inclusive() {
    let f = owner();
    let i = Input::new(
        SearchReadContextV1::Source(&f.source),
        &f.s.document,
        "typed",
        false,
    );
    let n = normalize_input(&i, &f.s.context).unwrap();
    let nodes = n.capture_usage().iter().map(|u| u.nodes).max().unwrap();
    let depth = n.capture_usage().iter().map(|u| u.depth).max().unwrap();
    for (l, expected) in [
        (
            CaptureLimitsV1 {
                max_nodes: nodes,
                ..limits()
            },
            true,
        ),
        (
            CaptureLimitsV1 {
                max_nodes: nodes - 1,
                ..limits()
            },
            false,
        ),
        (
            CaptureLimitsV1 {
                max_depth: depth,
                ..limits()
            },
            true,
        ),
        (
            CaptureLimitsV1 {
                max_depth: depth - 1,
                ..limits()
            },
            false,
        ),
    ] {
        assert_eq!(
            with_input(&i, |e| normalize_source_envelope_v1(
                e,
                &f.s.context,
                l,
                16384,
                &mut Recorder::disabled()
            ))
            .is_ok(),
            expected
        );
    }
}
