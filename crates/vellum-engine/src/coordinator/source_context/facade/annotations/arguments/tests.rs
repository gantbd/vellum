use super::super::tests::{
    Fixture, Query, Target, counts, enumerate, limits, owner, query, record_queries, retire,
};
use super::*;
use crate::coordinator::source_query::result_contract::tests::member_facts;
use AnnotationEnvelopeValueV1::{Data, Document, Object, Snapshot};
struct Input<'a, 'h> {
    target: AnnotationArgumentTargetV1<'h>,
    document: &'h DocumentId,
    route: &'a str,
    selector: InertValueV1<'a>,
    version: f64,
    variant: &'a str,
    operation: &'a str,
    contract: &'a str,
    profile: &'a str,
    argument: AnnotationEnvelopeValueV1<'a, 'h>,
    limit: f64,
    extra: bool,
    duplicate: bool,
}
fn target(f: &Fixture) -> AnnotationArgumentTargetV1<'_> {
    match &f.target {
        Target::Single(id) => AnnotationArgumentTargetV1::Single(id),
        Target::VersionSet(a) => {
            AnnotationArgumentTargetV1::VersionSet(&a.participants[0].snapshot)
        }
    }
}
impl<'a, 'h> Input<'a, 'h> {
    fn new(f: &'h Fixture, route: &'a str) -> Self
    where
        'h: 'a,
    {
        Self::bound(
            target(f),
            &f.scope.document,
            &f.scope.context.profile,
            route,
        )
    }
    fn bound(
        target: AnnotationArgumentTargetV1<'h>,
        document: &'h DocumentId,
        profile: &'a str,
        route: &'a str,
    ) -> Self {
        Self {
            target,
            document,
            route,
            selector: if route == "typed" {
                InertValueV1::Number(2.0)
            } else {
                InertValueV1::Text("read-annotations")
            },
            version: 1.0,
            variant: "enumerate",
            operation: "enumerate",
            contract: "annotation-query/1",
            profile,
            argument: Data(InertValueV1::Null),
            limit: 1.0,
            extra: false,
            duplicate: false,
        }
    }
}
fn fill_query<'a, 'h>(i: &mut Input<'a, 'h>, q: &'h Query) {
    use AnnotationEnvelopeValueV1 as V;
    let (operation, argument, limit) = match q {
        Query::Single(AnnotationQueryV1::Enumerate {
            limit,
            continuation,
        }) => (
            "enumerate",
            continuation
                .as_ref()
                .map_or(Data(InertValueV1::Null), V::Continuation),
            *limit,
        ),
        Query::Single(AnnotationQueryV1::Inspect { annotation }) => {
            ("inspect", V::Record(annotation), 0)
        }
        Query::Single(AnnotationQueryV1::TargetLocation { annotation }) => {
            ("target-location", V::Record(annotation), 0)
        }
        Query::VersionSet(VersionSetAnnotationQueryV1::Enumerate {
            limit,
            continuation,
        }) => (
            "enumerate",
            continuation
                .as_ref()
                .map_or(Data(InertValueV1::Null), V::VersionSetContinuation),
            *limit,
        ),
        Query::VersionSet(VersionSetAnnotationQueryV1::Inspect { annotation }) => {
            ("inspect", V::VersionSetRecord(annotation), 0)
        }
        Query::VersionSet(VersionSetAnnotationQueryV1::TargetLocation { annotation }) => {
            ("target-location", V::VersionSetRecord(annotation), 0)
        }
    };
    i.variant = operation;
    i.operation = operation;
    i.argument = argument;
    i.limit = limit as f64;
}
pub(in crate::coordinator) fn with_decoded_request<T>(
    native: AnnotationFacadeRequestV1<'_>,
    route: &str,
    consume: impl FnOnce(DecodedAnnotationFacadeRequestV1<'_, '_>) -> T,
) -> T {
    let (target, facts, query) = match native.port {
        AnnotationFacadePortV1::Single {
            context,
            query,
            facts,
        } => (
            AnnotationArgumentTargetV1::Single(context),
            AnnotationArgumentFactsV1::Single(facts),
            Query::Single(query.clone()),
        ),
        AnnotationFacadePortV1::VersionSet {
            member,
            query,
            facts,
        } => (
            AnnotationArgumentTargetV1::VersionSet(member),
            AnnotationArgumentFactsV1::VersionSet(facts),
            Query::VersionSet(query.clone()),
        ),
    };
    let mut input = Input::bound(
        target,
        &native.scope.document,
        &native.scope.context.profile,
        route,
    );
    fill_query(&mut input, &query);
    with_input(&input, |envelope| {
        consume(DecodedAnnotationFacadeRequestV1 {
            envelope,
            actor: native.actor,
            scope: native.scope,
            limits: native.limits,
            facts,
        })
    })
}
fn run(
    f: &mut Fixture,
    q: &Query,
    route: &str,
    profile: Option<&str>,
    authorized: Option<bool>,
    peer_authorized: Option<bool>,
    caps: CaptureLimitsV1,
) -> Option<AnnotationFacadeReplyV1> {
    let mut actor = f.scope.context.clone();
    actor.recipient = "agent-not-a-grant".into();
    let target = match &f.target {
        Target::Single(id) => AnnotationArgumentTargetV1::Single(id),
        Target::VersionSet(a) => {
            AnnotationArgumentTargetV1::VersionSet(&a.participants[0].snapshot)
        }
    };
    let mut i = Input::bound(
        target,
        &f.scope.document,
        profile.unwrap_or(&f.scope.context.profile),
        route,
    );
    fill_query(&mut i, q);
    match &f.target {
        Target::Single(id) => {
            let current = TrustedAnnotationContextFactsV1 {
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
                resources_ready: Some(true),
                reply_ready: Some(true),
                transfer_ready: Some(true),
            };
            with_input(&i, |e| {
                f.o.query_decoded_annotation_facade_v1(
                    DecodedAnnotationFacadeRequestV1 {
                        envelope: e,
                        scope: &f.scope,
                        actor: &actor,
                        limits: caps,
                        facts: AnnotationArgumentFactsV1::Single(&current),
                    },
                    &mut f.r,
                )
            })
        }
        Target::VersionSet(a) => {
            let mut selected = member_facts(&f.scope);
            selected.authorized = authorized;
            selected.source_readable = None;
            let mut peer = member_facts(f.peer.as_ref().unwrap());
            peer.authorized = peer_authorized;
            peer.annotation_readable = None;
            let current = TrustedVersionSetContextFactsV1 {
                context: &a.context,
                members: &[selected, peer],
            };
            with_input(&i, |e| {
                f.o.query_decoded_annotation_facade_v1(
                    DecodedAnnotationFacadeRequestV1 {
                        envelope: e,
                        scope: &f.scope,
                        actor: &actor,
                        limits: caps,
                        facts: AnnotationArgumentFactsV1::VersionSet(&current),
                    },
                    &mut f.r,
                )
            })
        }
    }
}
#[test]
fn decoded_annotation_facade_preserves_typed_normalization_and_native_success_on_both_routes() {
    use super::super::tests::invoke_with;
    for coherent in [false, true] {
        for route in ["typed", "dynamic"] {
            let mut f = owner(coherent);
            f.r = Recorder::try_with_capacity(256)
                .unwrap()
                .with_native_timing(
                    crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                );
            crate::coordinator::release_measurements::tests::enable(&mut f.o, &mut f.r);
            let q = enumerate(&f, 1);
            invoke_with(
                &mut f,
                &q,
                Some(true),
                Some(true),
                Some(true),
                limits(),
                |o, native, r| {
                    let (target, facts) = match native.port {
                        AnnotationFacadePortV1::Single { context, facts, .. } => (
                            AnnotationArgumentTargetV1::Single(context),
                            AnnotationArgumentFactsV1::Single(facts),
                        ),
                        AnnotationFacadePortV1::VersionSet { member, facts, .. } => (
                            AnnotationArgumentTargetV1::VersionSet(member),
                            AnnotationArgumentFactsV1::VersionSet(facts),
                        ),
                    };
                    let mut input = Input::bound(
                        target,
                        &native.scope.document,
                        &native.scope.context.profile,
                        route,
                    );
                    input.extra = true;
                    with_input(&input, |envelope| {
                        assert!(matches!(
                            o.try_query_decoded_annotation_facade_v1(
                                DecodedAnnotationFacadeRequestV1 {
                                    envelope,
                                    scope: native.scope,
                                    actor: native.actor,
                                    limits: native.limits,
                                    facts
                                },
                                r
                            ),
                            Err(Error::InvalidRequest)
                        ));
                    });
                    input.extra = false;
                    with_input(&input, |envelope| {
                        let out = o.try_query_decoded_annotation_facade_v1(
                            DecodedAnnotationFacadeRequestV1 {
                                envelope,
                                scope: native.scope,
                                actor: native.actor,
                                limits: native.limits,
                                facts,
                            },
                            r,
                        );
                        assert!(out.is_ok());
                        out.ok()
                    })
                },
            );
            let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            assert!(events.iter().any(
                |e| e.boundary == Boundary::AnnotationFacadeRelease && e.service_time.is_some()
            ));
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
            assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
        }
    }
}
#[test]
fn native_annotation_arguments_dispatch_all_three_variants_on_both_owners_and_routes() {
    for coherent in [false, true] {
        for route in ["typed", "dynamic"] {
            let mut f = owner(coherent);
            let q = enumerate(&f, 1);
            let base = f.o.retention_usage().reserved;
            let page = run(&mut f, &q, route, None, Some(true), Some(true), limits()).unwrap();
            let (inspect, location) = record_queries(page);
            assert!(
                run(
                    &mut f,
                    &inspect,
                    route,
                    None,
                    Some(true),
                    Some(true),
                    limits()
                )
                .is_some()
            );
            assert!(
                run(
                    &mut f,
                    &location,
                    route,
                    None,
                    Some(true),
                    Some(true),
                    limits()
                )
                .is_some()
            );
            assert_eq!(f.o.retention_usage().reserved, base);
            assert!(!f.o.is_faulted());
        }
    }
}
fn with_input<'a, 'h, T>(
    i: &Input<'a, 'h>,
    f: impl for<'v> FnOnce(&'v AnnotationEnvelopeValueV1<'v, 'h>) -> T,
) -> T {
    let mut args = [
        ("contract", Data(InertValueV1::Text(i.contract))),
        ("operation", Data(InertValueV1::Text(i.operation))),
        ("snapshot", Snapshot(i.target)),
        ("document", Document(i.document)),
        ("profile", Data(InertValueV1::Text(i.profile))),
        (
            if i.variant == "enumerate" {
                "continuation"
            } else {
                "annotation"
            },
            i.argument,
        ),
        ("limit", Data(InertValueV1::Number(i.limit))),
        ("extra", Data(InertValueV1::Null)),
    ];
    let mut n = if i.variant == "enumerate" { 7 } else { 6 };
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
    a: &SettingsExecutionContextV1,
) -> Result<NormalizedAnnotationArgumentsV1<'h>, Error> {
    with_input(i, |e| {
        normalize_annotation_envelope_v1(e, a, limits(), 16384, &mut Recorder::disabled())
    })
}
#[test]
fn native_annotation_routes_normalize_to_equal_meaning_with_original_roles() {
    for coherent in [false, true] {
        let f = owner(coherent);
        let a = normalize_input(&Input::new(&f, "typed"), &f.scope.context).unwrap();
        let b = normalize_input(&Input::new(&f, "dynamic"), &f.scope.context).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.target(), target(&f));
        assert!(std::ptr::eq(a.document(), &f.scope.document));
        assert_eq!(
            a.query(),
            if coherent {
                AnnotationArgumentQueryV1::VersionSetEnumerate {
                    limit: 1,
                    continuation: None,
                }
            } else {
                AnnotationArgumentQueryV1::Enumerate {
                    limit: 1,
                    continuation: None,
                }
            }
        );
    }
}
#[test]
fn malformed_annotation_vocabulary_and_bad_paging_limits_are_not_coerced() {
    let f = owner(false);
    for mode in 0..14 {
        let mut i = Input::new(&f, "typed");
        match mode {
            0 => i.route = "unknown",
            1 => i.selector = InertValueV1::Number(1.0),
            2 => i.version = 2.0,
            3 => i.variant = "unknown",
            4 => i.operation = "inspect",
            5 => i.contract = "annotation-query/2",
            6 => i.profile = "",
            7 => i.limit = 0.0,
            8 => i.limit = -1.0,
            9 => i.limit = 0.5,
            10 => i.limit = 9_007_199_254_740_992.0,
            11 => i.limit = f64::NAN,
            12 => i.extra = true,
            _ => i.duplicate = true,
        }
        assert!(
            normalize_input(&i, &f.scope.context).is_err(),
            "mode {mode}"
        );
    }
}

#[test]
fn annotation_record_selectors_retain_original_references_for_both_routes_and_variants() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let page = enumerate(&f, 1);
        let (inspect, location) = record_queries(query(&mut f, &page).unwrap());
        for q in [&inspect, &location] {
            let mut a = Input::new(&f, "typed");
            fill_query(&mut a, q);
            let mut b = Input::new(&f, "dynamic");
            fill_query(&mut b, q);
            let n = normalize_input(&a, &f.scope.context).unwrap();
            assert_eq!(n, normalize_input(&b, &f.scope.context).unwrap());
            match (q, n.query()) {
                (
                    Query::Single(AnnotationQueryV1::Inspect { annotation: a }),
                    AnnotationArgumentQueryV1::Inspect { annotation: b },
                )
                | (
                    Query::Single(AnnotationQueryV1::TargetLocation { annotation: a }),
                    AnnotationArgumentQueryV1::TargetLocation { annotation: b },
                ) => assert!(std::ptr::eq(a, b)),
                (
                    Query::VersionSet(VersionSetAnnotationQueryV1::Inspect { annotation: a }),
                    AnnotationArgumentQueryV1::VersionSetInspect { annotation: b },
                )
                | (
                    Query::VersionSet(VersionSetAnnotationQueryV1::TargetLocation {
                        annotation: a,
                    }),
                    AnnotationArgumentQueryV1::VersionSetTargetLocation { annotation: b },
                ) => assert!(std::ptr::eq(a, b)),
                _ => panic!("selector role changed"),
            }
        }
    }
}

#[test]
fn nonnull_annotation_continuations_resume_the_original_disclosure_population() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 1);
        let first = run(&mut f, &q, "typed", None, Some(true), Some(true), limits()).unwrap();
        let next = match first {
            AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page {
                continuation: Some(c),
                ..
            }) => Query::Single(AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: Some(c),
            }),
            AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
                continuation: Some(c),
                ..
            }) => Query::VersionSet(VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: Some(c),
            }),
            _ => panic!("expected continuation"),
        };
        let mut i = Input::new(&f, "typed");
        fill_query(&mut i, &next);
        let n = normalize_input(&i, &f.scope.context).unwrap();
        assert!(matches!(
            n.query(),
            AnnotationArgumentQueryV1::Enumerate {
                continuation: Some(_),
                ..
            } | AnnotationArgumentQueryV1::VersionSetEnumerate {
                continuation: Some(_),
                ..
            }
        ));
        let page = run(
            &mut f,
            &next,
            "dynamic",
            None,
            Some(true),
            Some(true),
            limits(),
        )
        .unwrap();
        match page {
            AnnotationFacadeReplyV1::Single(AnnotationQueryReplyV1::Page {
                records,
                continuation,
                ..
            }) => {
                assert_eq!(records.len(), 1);
                assert!(continuation.is_none());
            }
            AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
                records,
                continuation,
                ..
            }) => {
                assert_eq!(records.len(), 1);
                assert!(continuation.is_none());
            }
            _ => panic!("page"),
        }
    }
}

#[test]
fn annotation_selectors_cannot_cross_record_continuation_or_owner_role_types() {
    let mut single = owner(false);
    let mut coherent = owner(true);
    let sq = enumerate(&single, 1);
    let vq = enumerate(&coherent, 1);
    let (inspect, _) = record_queries(query(&mut single, &sq).unwrap());
    let (vinspect, _) = record_queries(query(&mut coherent, &vq).unwrap());
    let mut i = Input::new(&single, "typed");
    fill_query(&mut i, &inspect);
    let record = i.argument;
    i.variant = "enumerate";
    i.operation = "enumerate";
    i.limit = 1.0;
    assert!(normalize_input(&i, &single.scope.context).is_err());
    i.variant = "inspect";
    i.operation = "inspect";
    i.argument = Data(InertValueV1::Null);
    assert!(normalize_input(&i, &single.scope.context).is_err());
    let mut v = Input::new(&coherent, "typed");
    fill_query(&mut v, &vinspect);
    i.argument = v.argument;
    assert!(normalize_input(&i, &single.scope.context).is_err());
    v.argument = record;
    assert!(normalize_input(&v, &coherent.scope.context).is_err());
}

#[test]
fn normalized_annotation_ordinary_data_outlives_packet_while_capabilities_remain_borrowed() {
    let f = owner(false);
    let n = {
        let profile = String::from("temporary-profile");
        let mut actor = f.scope.context.clone();
        actor.purpose = "temporary-purpose".into();
        let mut i = Input::new(&f, "dynamic");
        i.profile = &profile;
        normalize_input(&i, &actor).unwrap()
    };
    assert_eq!(n.profile(), "temporary-profile");
    assert_eq!(n.actor().purpose, "temporary-purpose");
    assert_eq!(n.limits(), limits());
    assert!(n.owned_bytes() <= limits().max_bytes);
    assert_eq!(format!("{n:?}"), "<native-annotation-arguments:redacted>");
}

#[test]
fn annotation_argument_capture_enforces_independent_bounds_and_inclusive_node_depth_edges() {
    let f = owner(false);
    let i = Input::new(&f, "typed");
    let n = normalize_input(&i, &f.scope.context).unwrap();
    let nodes = n.capture_usage().iter().map(|u| u.nodes).max().unwrap();
    let depth = n.capture_usage().iter().map(|u| u.depth).max().unwrap();
    for (caps, expected) in [
        (
            CaptureLimitsV1 {
                max_bytes: 1,
                ..limits()
            },
            false,
        ),
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
            with_input(&i, |e| normalize_annotation_envelope_v1(
                e,
                &f.scope.context,
                caps,
                16384,
                &mut Recorder::disabled()
            ))
            .is_ok(),
            expected
        );
    }
    assert!(
        with_input(&i, |e| normalize_annotation_envelope_v1(
            e,
            &f.scope.context,
            limits(),
            1,
            &mut Recorder::disabled()
        ))
        .is_err()
    );
    let mut actor = f.scope.context.clone();
    actor.recipient.clear();
    assert!(normalize_input(&i, &actor).is_err());
}

#[test]
fn unsupported_annotation_profile_refuses_without_poisoning_the_original_owner() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 1);
        assert!(
            run(
                &mut f,
                &q,
                "typed",
                Some("unsupported"),
                Some(false),
                Some(false),
                limits()
            )
            .is_none()
        );
        assert!(
            run(
                &mut f,
                &q,
                "dynamic",
                None,
                Some(true),
                Some(true),
                limits()
            )
            .is_some()
        );
    }
}

#[test]
fn foreign_same_role_annotation_selector_cannot_lend_loss_to_a_valid_context() {
    for coherent in [false, true] {
        let mut original = owner(coherent);
        let mut foreign = owner(coherent);
        let page = enumerate(&foreign, 1);
        let (foreign_record, _) = record_queries(query(&mut foreign, &page).unwrap());
        assert!(
            run(
                &mut original,
                &foreign_record,
                "typed",
                None,
                Some(false),
                Some(true),
                limits()
            )
            .is_none()
        );
        let page = enumerate(&original, 1);
        assert!(
            run(
                &mut original,
                &page,
                "dynamic",
                None,
                Some(true),
                Some(true),
                limits()
            )
            .is_some()
        );
        assert!(!original.o.is_faulted());
    }
}

#[test]
fn foreign_annotation_document_binding_refuses_before_native_current_loss() {
    let mut f = owner(false);
    let foreign = f.o.create(b"foreign", &mut f.r).unwrap();
    let Target::Single(id) = &f.target else {
        panic!("single")
    };
    let i = Input::bound(
        AnnotationArgumentTargetV1::Single(id),
        &foreign.document,
        &f.scope.context.profile,
        "typed",
    );
    let current = TrustedAnnotationContextFactsV1 {
        context: id,
        scope: &f.scope,
        authorized: Some(false),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: None,
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    };
    assert!(
        with_input(&i, |e| f.o.query_decoded_annotation_facade_v1(
            DecodedAnnotationFacadeRequestV1 {
                envelope: e,
                actor: &f.scope.context,
                scope: &f.scope,
                limits: limits(),
                facts: AnnotationArgumentFactsV1::Single(&current)
            },
            &mut f.r
        ))
        .is_none()
    );
    let page = enumerate(&f, 1);
    assert!(
        run(
            &mut f,
            &page,
            "dynamic",
            None,
            Some(true),
            Some(true),
            limits()
        )
        .is_some()
    );
}

#[test]
fn decoded_annotation_unknown_is_not_loss_and_coherent_peer_loss_stays_absorbing() {
    for coherent in [false, true] {
        let mut f = owner(coherent);
        let q = enumerate(&f, 1);
        assert!(run(&mut f, &q, "typed", None, None, Some(true), limits()).is_none());
        assert!(
            run(
                &mut f,
                &q,
                "dynamic",
                None,
                Some(true),
                Some(true),
                limits()
            )
            .is_some()
        );
        assert!(run(&mut f, &q, "typed", None, Some(false), Some(true), limits()).is_none());
        assert!(
            run(
                &mut f,
                &q,
                "dynamic",
                None,
                Some(true),
                Some(true),
                limits()
            )
            .is_none()
        );
        assert!(!f.o.is_faulted());
    }
    let mut f = owner(true);
    let q = enumerate(&f, 1);
    assert!(run(&mut f, &q, "typed", None, Some(true), Some(false), limits()).is_none());
    assert!(
        run(
            &mut f,
            &q,
            "dynamic",
            None,
            Some(true),
            Some(true),
            limits()
        )
        .is_none()
    );
}

#[test]
fn decoded_annotation_faults_preserve_native_selector_publication_and_protected_retirement() {
    for coherent in [false, true] {
        for op in 0..3 {
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
                let q = if op == 0 {
                    page
                } else {
                    let (a, b) = record_queries(query(&mut f, &page).unwrap());
                    if op == 1 { a } else { b }
                };
                let before = counts(&f);
                let base = f.o.retention_usage().reserved;
                let generation = f.o.generation();
                f.o.source_facade_fault = Some(fault);
                assert!(run(&mut f, &q, "typed", None, Some(true), Some(true), limits()).is_none());
                assert!(f.o.is_faulted());
                assert_eq!(f.o.retention_usage().reserved, base);
                assert_eq!(f.o.generation(), generation);
                assert_eq!(
                    f.o.read_facade_usage(),
                    ReadFacadeUsageV1 {
                        busy: false,
                        workspace_bytes: 0
                    }
                );
                let delta = usize::from(
                    op == 0
                        && matches!(
                            fault,
                            Fault::AfterNative | Fault::BadResult | Fault::RefundDrift
                        ),
                ) * 2;
                assert_eq!(counts(&f), (before.0 + delta, before.1 + delta));
                f.o.source_facade_fault = None;
                let issued = counts(&f).0;
                retire(&mut f);
                assert_eq!(counts(&f).0, issued);
            }
        }
    }
}

#[test]
fn decoded_annotation_uses_one_exact_parent_reservation_and_shared_reentry_fence() {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 7 * limits().max_bytes;
    for coherent in [false, true] {
        for available in [fixed - 1, fixed, total - 1, total] {
            let mut f = owner(coherent);
            let q = enumerate(&f, 1);
            let base = f.o.retention_usage().reserved;
            let pressure = f.o.retention_usage().maximum.unwrap() - base - available;
            let mut held = f.o.retention.reserve(pressure).unwrap();
            let before = f.o.retention_usage().reserved;
            assert_eq!(
                run(&mut f, &q, "typed", None, Some(true), Some(true), limits()).is_some(),
                available >= total
            );
            assert_eq!(f.o.retention_usage().reserved, before);
            assert!(!f.o.is_faulted());
            f.o.retention.release(&mut held).unwrap();
        }
        let mut f = owner(coherent);
        let q = enumerate(&f, 1);
        f.o.source_facade_fault = Some(Fault::Reentry);
        assert!(run(&mut f, &q, "typed", None, Some(true), Some(true), limits()).is_some());
        f.o.source_facade_fault = None;
        f.o.set_settings_parent_limit_for_test(1);
        assert!(
            run(
                &mut f,
                &q,
                "typed",
                None,
                Some(false),
                Some(false),
                limits()
            )
            .is_none()
        );
        f.o.set_settings_parent_limit_for_test(16384);
        assert!(
            run(
                &mut f,
                &q,
                "dynamic",
                None,
                Some(true),
                Some(true),
                limits()
            )
            .is_some()
        );
    }
}

#[test]
fn decoded_annotation_optional_diagnostics_preserve_data_refunds_and_selector_counts() {
    for coherent in [false, true] {
        for capacity in [None, Some(128), Some(1)] {
            let mut f = owner(coherent);
            let q = enumerate(&f, 1);
            let base = f.o.retention_usage().reserved;
            let before = counts(&f);
            f.r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            assert!(
                run(
                    &mut f,
                    &q,
                    "dynamic",
                    None,
                    Some(true),
                    Some(true),
                    limits()
                )
                .is_some()
            );
            assert_eq!(counts(&f), (before.0 + 2, before.1 + 2));
            assert_eq!(f.o.retention_usage().reserved, base);
            assert!(!f.o.is_faulted());
            match capacity {
                None => assert_eq!(f.r.queued(), 0),
                Some(1) => assert!(f.r.dropped() > 0),
                _ => {
                    let mut seen = false;
                    while let Some(e) = f.r.pop_front() {
                        seen |= e.boundary == Boundary::AnnotationArgumentNormalization;
                        assert!(e.service_time.is_none());
                    }
                    assert!(seen);
                }
            }
        }
    }
}
