use super::*;
use crate::selection_discovery::Route;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 128,
        max_depth: 8,
    }
}

#[test]
fn decoded_and_installed_replacement_preserve_original_result_across_both_routes() {
    use ReplacementEnvelopeValueV1 as V;
    for installed in [false, true] {
        for dynamic in [false, true] {
            for current in [false, true] {
                for mode in 0..4 {
                    let mut f = fixture(0);
                    let q = request("a");
                    f.o.release_source_search(&f.result, &f.s, &facts(&f.s), &qfacts(&q), &mut f.r)
                        .unwrap();
                    let route = installed.then(|| Route::install_replacement(&mut f.o, &mut f.r));
                    f.r = match mode {
                    0 => Recorder::disabled(),
                    1 => Recorder::try_with_capacity(512).unwrap(),
                    2 => Recorder::try_with_capacity(512).unwrap().with_native_timing(vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1),
                    _ => Recorder::try_with_capacity(0).unwrap().with_native_timing(vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1),
                };
                    let baseline = f.o.retention_usage().reserved;
                    let id = f.o.issue_invocation(&mut f.r).unwrap();
                    let op = if current {
                        "replace-current"
                    } else {
                        "replace-all"
                    };
                    let fields = [
                        ("operation", V::Data(D::Text(op))),
                        (
                            "index",
                            V::Data(if current { D::Number(0.0) } else { D::Null }),
                        ),
                        ("replacement", V::Data(D::Text("λ\n"))),
                        ("invocation", V::Invocation(&id)),
                    ];
                    let header = [
                        (
                            if dynamic { "name" } else { "family" },
                            V::Data(if dynamic {
                                D::Text("change-source")
                            } else {
                                D::Number(10.0)
                            }),
                        ),
                        ("variant", V::Data(D::Text(op))),
                        ("version", V::Data(D::Number(1.0))),
                        ("arguments", V::Object(&fields)),
                    ];
                    let envelope = [
                        (
                            "route",
                            V::Data(D::Text(if dynamic { "dynamic" } else { "typed" })),
                        ),
                        ("request", V::Object(&header)),
                    ];
                    let n = normalize_replacement_envelope_v1(
                        V::Object(&envelope),
                        &f.result,
                        &f.s.context,
                        limits(),
                        &mut f.r,
                    )
                    .unwrap();
                    let scope = f.s.clone();
                    let native = consumer_facts(n.command(), &scope);
                    let submit = |f: &mut Fixture| match &route {
                        Some(route) => route.submit_replacement(
                            &mut f.o,
                            &n,
                            &f.s,
                            SearchReadFactsV1::Source(&facts(&f.s)),
                            &qfacts(&q),
                            &native,
                            &mut f.r,
                        ),
                        None => {
                            f.o.submit_replacement_arguments_v1(
                                &n,
                                &f.s,
                                SearchReadFactsV1::Source(&facts(&f.s)),
                                &qfacts(&q),
                                &native,
                                &mut f.r,
                            )
                            .map_err(DiscoveredNativeCommandErrorV1::Invocation)
                        }
                    };
                    assert_eq!(submit(&mut f), Ok(InvocationState::Pending));
                    let funded = f.o.retention_usage().reserved;
                    assert_eq!(submit(&mut f), Ok(InvocationState::Pending));
                    assert_eq!(f.o.retention_usage().reserved, funded);
                    f.o.run_source_search_consumer(
                        &id,
                        &f.s,
                        &facts(&f.s),
                        &qfacts(&q),
                        &native,
                        &mut f.r,
                    )
                    .unwrap();
                    assert_eq!(
                        f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
                        if current { "λ\nba" } else { "λ\nbλ\n" }
                    );
                    assert_eq!(submit(&mut f), Ok(InvocationState::Finished));
                    f.o.retire_returned_search_replacement_v1(
                        &id,
                        &InvocationObservationPolicyControlV1 {
                            context: &f.s.context,
                            owner_control_allowed: Some(true),
                            retention_released: Some(true),
                            delivery_drained: Some(true),
                        },
                        &mut f.r,
                    )
                    .unwrap();
                    f.o.retire_invocation(&id, &mut f.r).unwrap();
                    assert_eq!(f.o.retention_usage().reserved, baseline);
                    if let Some(route) = route {
                        route.retire(&mut f.o, &mut f.r);
                    }
                    let mut normalization = 0;
                    let mut dispatch = 0;
                    while let Some(record) = f.r.pop_front() {
                        use vellum_engine::instrumentation::Boundary;
                        match record.boundary {
                            Boundary::ReplacementArgumentNormalization => normalization += 1,
                            Boundary::DiscoveryReplacementDispatch => dispatch += 1,
                            _ => continue,
                        }
                        assert_eq!(record.service_time.is_some(), mode == 2);
                    }
                    if mode == 1 || mode == 2 {
                        assert_eq!(normalization, 1);
                        assert_eq!(dispatch, if installed { 3 } else { 0 });
                    }
                }
            }
        }
    }
}

#[test]
fn replacement_normalization_refuses_open_shapes_bad_indices_and_independent_input_limits() {
    use ReplacementEnvelopeValueV1 as V;
    let mut f = fixture(0);
    let id = f.o.issue_invocation(&mut f.r).unwrap();
    let before = f.o.retention_usage().reserved;
    let valid = [
        ("operation", V::Data(D::Text("replace-all"))),
        ("index", V::Data(D::Null)),
        ("replacement", V::Data(D::Text(""))),
        ("invocation", V::Invocation(&id)),
    ];
    let norm = |rows: &[(&str, V<'_, '_>)], l, r: &mut Recorder| {
        normalize_replacement_arguments_v1(V::Object(rows), &f.result, &f.s.context, l, r)
            .map(|_| ())
    };
    assert_eq!(norm(&valid, limits(), &mut f.r), Ok(()));
    for omitted in 0..4 {
        let rows: Vec<_> = valid
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (i != omitted).then_some(*v))
            .collect();
        assert_eq!(
            norm(&rows, limits(), &mut f.r),
            Err(InvocationError::MalformedIntent)
        );
    }
    for extra in [("extra", V::Data(D::Null)), valid[0]] {
        let mut rows = valid.to_vec();
        rows.push(extra);
        assert_eq!(
            norm(&rows, limits(), &mut f.r),
            Err(InvocationError::MalformedIntent)
        );
    }
    for (op, index) in [
        ("replace-all", D::Number(0.0)),
        ("replace-current", D::Null),
        ("replace-current", D::Number(-1.0)),
        ("replace-current", D::Number(0.5)),
        ("replace-current", D::Number(f64::NAN)),
        ("replace-current", D::Number(9_007_199_254_740_992.0)),
        ("template-replace", D::Null),
    ] {
        let mut rows = valid;
        rows[0].1 = V::Data(D::Text(op));
        rows[1].1 = V::Data(index);
        assert_eq!(
            norm(&rows, limits(), &mut f.r),
            Err(InvocationError::MalformedIntent)
        );
    }
    for limit in [
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
    ] {
        assert_eq!(norm(&valid, limit, &mut f.r), Err(InvocationError::Limit));
    }
    let header = [
        ("family", V::Data(D::Number(10.0))),
        ("variant", V::Data(D::Text("replace-current"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&valid)),
    ];
    let envelope = [
        ("route", V::Data(D::Text("typed"))),
        ("request", V::Object(&header)),
    ];
    assert!(matches!(
        normalize_replacement_envelope_v1(
            V::Object(&envelope),
            &f.result,
            &f.s.context,
            limits(),
            &mut f.r
        ),
        Err(InvocationError::MalformedIntent)
    ));
    assert_eq!(
        f.o.invocation_state(&id, &mut f.r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(f.o.retention_usage().reserved, before);
}

fn normalized<'h>(
    id: &'h InvocationId,
    result: &SourceSearchResultId,
    actor: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> NormalizedReplacementArgumentsV1<'h> {
    use ReplacementEnvelopeValueV1 as V;
    let fields = [
        ("operation", V::Data(D::Text("replace-all"))),
        ("index", V::Data(D::Null)),
        ("replacement", V::Data(D::Text("Z"))),
        ("invocation", V::Invocation(id)),
    ];
    normalize_replacement_arguments_v1(V::Object(&fields), result, actor, limits(), r).unwrap()
}

#[test]
fn installed_selector_refusal_cannot_apply_supplied_query_loss_but_valid_metadata_cannot_grant_mutation()
 {
    let mut f = fixture(0);
    let q = request("a");
    f.o.release_source_search(&f.result, &f.s, &facts(&f.s), &qfacts(&q), &mut f.r)
        .unwrap();
    let route = Route::install_replacement(&mut f.o, &mut f.r);
    let id = f.o.issue_invocation(&mut f.r).unwrap();
    let n = normalized(&id, &f.result, &f.s.context, &mut f.r);
    let baseline = f.o.retention_usage().reserved;
    let mut lost = qfacts(&q);
    lost.grant = Some(false);
    assert!(matches!(
        route.submit_replacement_as_variant(
            &mut f.o,
            &n,
            0,
            &f.s,
            SearchReadFactsV1::Source(&facts(&f.s)),
            &lost,
            &consumer_facts(n.command(), &f.s),
            &mut f.r
        ),
        Err(DiscoveredNativeCommandErrorV1::Discovery(_))
    ));
    assert_eq!(f.o.source_search_usage().invalidated, 0);
    let mut denied = consumer_facts(n.command(), &f.s);
    denied.mutation_allowed = Some(false);
    assert_eq!(
        route.submit_replacement(
            &mut f.o,
            &n,
            &f.s,
            SearchReadFactsV1::Source(&facts(&f.s)),
            &qfacts(&q),
            &denied,
            &mut f.r
        ),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    assert_eq!(
        f.o.invocation_state(&id, &mut f.r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(f.o.retention_usage().reserved, baseline);
    route
        .submit_replacement(
            &mut f.o,
            &n,
            &f.s,
            SearchReadFactsV1::Source(&facts(&f.s)),
            &qfacts(&q),
            &consumer_facts(n.command(), &f.s),
            &mut f.r,
        )
        .unwrap();
    let scope = f.s.clone();
    f.o.request_invocation_stop(
        &id,
        &TrustedInvocationControlV1 {
            context: &scope.context,
            owner_control_allowed: Some(true),
            control_context_current: Some(true),
            control_scope_valid: Some(true),
            protected_control_ready: Some(true),
        },
        &mut f.r,
    )
    .unwrap();
    f.o.run_source_search_consumer(&id, &f.s, &facts(&f.s), &qfacts(&q), &denied, &mut f.r)
        .unwrap();
    assert_eq!(
        f.o.invocation_outcome(&id, &mut f.r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    assert_eq!(
        f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
        "aba"
    );
    f.o.forget_invocation_outcome(&id, &mut f.r).unwrap();
    assert!(f.o.retention_usage().reserved > baseline);
    f.o.retire_returned_search_replacement_v1(
        &id,
        &InvocationObservationPolicyControlV1 {
            context: &scope.context,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut f.r,
    )
    .unwrap();
    assert_eq!(f.o.retention_usage().reserved, baseline);
    f.o.retire_invocation(&id, &mut f.r).unwrap();
    route.retire(&mut f.o, &mut f.r);
}

#[test]
fn equal_query_result_substitution_and_actor_change_do_not_borrow_original_binding() {
    let mut f = fixture(0);
    let q = request("a");
    let other =
        f.o.query_source_search(&f.source, &f.s, &q, &facts(&f.s), &qfacts(&q), &mut f.r)
            .unwrap();
    for result in [&f.result, &other] {
        f.o.release_source_search(result, &f.s, &facts(&f.s), &qfacts(&q), &mut f.r)
            .unwrap();
    }
    let id = f.o.issue_invocation(&mut f.r).unwrap();
    let n = normalized(&id, &f.result, &f.s.context, &mut f.r);
    let other = normalized(&id, &other, &f.s.context, &mut f.r);
    let mut actor = f.s.context.clone();
    actor.purpose.push_str("-changed");
    let changed = normalized(&id, &f.result, &actor, &mut f.r);
    assert_eq!(
        f.o.submit_replacement_arguments_v1(
            &changed,
            &f.s,
            SearchReadFactsV1::Source(&facts(&f.s)),
            &qfacts(&q),
            &consumer_facts(changed.command(), &f.s),
            &mut f.r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        f.o.invocation_state(&id, &mut f.r),
        Ok(InvocationState::Issued)
    );
    f.o.submit_replacement_arguments_v1(
        &n,
        &f.s,
        SearchReadFactsV1::Source(&facts(&f.s)),
        &qfacts(&q),
        &consumer_facts(n.command(), &f.s),
        &mut f.r,
    )
    .unwrap();
    let funded = f.o.retention_usage().reserved;
    assert_eq!(
        f.o.submit_replacement_arguments_v1(
            &other,
            &f.s,
            SearchReadFactsV1::Source(&facts(&f.s)),
            &qfacts(&q),
            &consumer_facts(other.command(), &f.s),
            &mut f.r
        ),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(f.o.retention_usage().reserved, funded);
}

#[test]
fn replacement_outcome_uses_original_f06_observer_and_independent_current_release() {
    use crate::observation_support::{Profile, facts as observer_facts, policy};
    use InvocationObservationEnvelopeValueV1 as V;
    use std::mem::size_of;
    for dynamic in [false, true] {
        let mut f = fixture(0);
        let scope = f.s.clone();
        let q = request("a");
        f.o.release_source_search(&f.result, &scope, &facts(&scope), &qfacts(&q), &mut f.r)
            .unwrap();
        let baseline = f.o.retention_usage().reserved;
        let id = f.o.issue_invocation(&mut f.r).unwrap();
        let profile = Profile {
            epoch: 7,
            maximum: 1024,
            capabilities: InvocationReceiptCapabilitiesV1::default(),
        };
        profile.bind(&mut f.o, &id, &scope.context, &mut f.r);
        let n = normalized(&id, &f.result, &scope.context, &mut f.r);
        f.o.submit_replacement_arguments_v1(
            &n,
            &scope,
            SearchReadFactsV1::Source(&facts(&scope)),
            &qfacts(&q),
            &consumer_facts(n.command(), &scope),
            &mut f.r,
        )
        .unwrap();
        f.o.run_source_search_consumer(
            &id,
            &scope,
            &facts(&scope),
            &qfacts(&q),
            &consumer_facts(n.command(), &scope),
            &mut f.r,
        )
        .unwrap();
        let current = policy(&scope.context, 7);
        let mut l = InvocationObservationLimitsV1 {
            max_waiters: 0,
            max_issued: 4,
            max_wait_steps: 0,
            max_context_bytes: 2048,
            max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
            max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
                .max(size_of::<InvocationControlReferenceV1>()),
            max_reply_bytes: 4096,
            max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
            max_held_bytes: 0,
        };
        l.max_held_bytes = l.required_hold_bytes().unwrap();
        let mut observer =
            f.o.open_invocation_observation_v1(
                &id,
                &scope.context,
                &current,
                l,
                &observer_facts(&scope.context),
                &mut f.r,
            )
            .unwrap();
        let context = observer.context_id_v1();
        let args = [
            ("contract", V::Data(D::Text("execution-observation/1"))),
            ("operation", V::Data(D::Text("retained-outcome"))),
            ("invocation", V::Invocation(&id)),
            ("context", V::Context(&context)),
            ("profile", V::Data(D::Text(&scope.context.profile))),
        ];
        let header = [
            (
                if dynamic { "name" } else { "family" },
                V::Data(if dynamic {
                    D::Text("inspect-execution")
                } else {
                    D::Number(5.0)
                }),
            ),
            ("variant", V::Data(D::Text("retained-outcome"))),
            ("version", V::Data(D::Number(1.0))),
            ("arguments", V::Object(&args)),
        ];
        let envelope = [
            (
                "route",
                V::Data(D::Text(if dynamic { "dynamic" } else { "typed" })),
            ),
            ("request", V::Object(&header)),
        ];
        let mut release = TrustedInvocationObservationReleaseV1 {
            context: &scope.context,
            authorized: None,
            scope: Some(true),
            continuous: Some(true),
            live: Some(true),
            retained: Some(true),
            transfer_ready: Some(true),
            control_ready: Some(true),
        };
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &release,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        assert_eq!(
            f.o.query_decoded_invocation_observation_v1(
                &mut observer,
                &V::Object(&envelope),
                InvocationObservationReadPortV1::Ordinary {
                    current: &current,
                    facts: &ordinary
                },
                &mut f.r
            ),
            Err(InvocationError::Unavailable)
        );
        release.authorized = Some(true);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &release,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let InvocationObservationReadReplyV1::Ordinary(reply) =
            f.o.query_decoded_invocation_observation_v1(
                &mut observer,
                &V::Object(&envelope),
                InvocationObservationReadPortV1::Ordinary {
                    current: &current,
                    facts: &ordinary,
                },
                &mut f.r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            reply.outcome.unwrap().publication,
            Some(InvocationPublicationV1::Applied)
        );
        let ctl = InvocationObservationPolicyControlV1 {
            context: &scope.context,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        f.o.retire_invocation_observation_v1(&mut observer, &ctl, &mut f.r)
            .unwrap();
        f.o.retire_invocation_observation_policy_v1(&id, &ctl, &mut f.r)
            .unwrap();
        f.o.retire_returned_search_replacement_v1(&id, &ctl, &mut f.r)
            .unwrap();
        f.o.retire_invocation(&id, &mut f.r).unwrap();
        assert_eq!(f.o.retention_usage().reserved, baseline);
    }
}
