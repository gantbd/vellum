use super::*;
use vellum_engine::capture::InertValueV1 as D;
#[path = "../support/discovered_context_fixture.rs"]
mod discovery_support;
#[path = "../support/invocation_observation_fixture.rs"]
mod observation;

fn scope() -> DiscoveryScopeV1 {
    let mut context = context();
    context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context,
        policy_epoch: 7,
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "authority".into(),
        engine: "engine".into(),
        recipient: "reader".into(),
        purpose: "containers".into(),
        destination: "host".into(),
        profile: NATIVE_RELATION_LIFECYCLE_PROFILE_V1.into(),
        continuity: "one-session".into(),
    }
}
fn visible() -> NativeBuiltinEntryVisibilityV1<2> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
            order: Some(i),
            enabled: None,
        }),
    }
}
fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_container_discovery_cut_v1(
        &scope(),
        &visible(),
        &discovery_support::facts(),
        r,
    )
    .unwrap()
}
fn selectors(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    Vec<DiscoveryVariantDescriptorV1>,
) {
    let cut = prepare(o, r);
    let context = o
        .acquire_discovery_v1(&scope(), &cut, &discovery_support::facts(), r)
        .unwrap()
        .context;
    let cut = prepare(o, r);
    let DiscoveryReplyV1::Page { entries, .. } = o
        .query_discovery_v1(
            &context,
            &scope(),
            &cut,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &discovery_support::facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    let entry = entries[0].reference.clone();
    let cut = prepare(o, r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
        .query_discovery_v1(
            &context,
            &scope(),
            &cut,
            DiscoveryQueryV1::Inspect { entry: &entry },
            &discovery_support::facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(descriptor.variants.len(), 2);
    (context, entry, descriptor.variants)
}
fn normalize<'h>(
    id: &'h InvocationId,
    command: &'h RelationCommandV1,
    r: &mut Recorder,
) -> NormalizedRelationArgumentsV1<'h> {
    use RelationEnvelopeValueV1 as V;
    let (operation, action) = match command {
        RelationCommandV1::CreateView => ("view-container", "create"),
        RelationCommandV1::RetireView { .. } => ("view-container", "retire"),
        RelationCommandV1::CreateWorkspace => ("workspace-container", "create"),
        RelationCommandV1::RetireWorkspace { .. } => ("workspace-container", "retire"),
    };
    let mut input = vec![
        ("contract", V::Data(D::Text("relation-invocation/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("action", V::Data(D::Text(action))),
        (
            "profile",
            V::Data(D::Text(NATIVE_RELATION_LIFECYCLE_PROFILE_V1)),
        ),
        ("invocation", V::Invocation(id)),
    ];
    match command {
        RelationCommandV1::RetireView { view, expected } => input.extend([
            ("target", V::View(view)),
            ("expected", V::ViewStamp(expected)),
        ]),
        RelationCommandV1::RetireWorkspace { expected } => input.extend([
            ("target", V::Workspace(&expected.workspace)),
            ("expected", V::WorkspaceContext(expected)),
        ]),
        _ => {}
    }
    normalize_relation_arguments_v1(
        V::Object(&input),
        RelationArgumentLimitsV1 {
            max_owned_bytes: NormalizedRelationArgumentsV1::required_owned_bytes(),
        },
        r,
    )
    .unwrap()
}
#[test]
fn all_four_installed_container_forms_preserve_original_execution_and_f06_results() {
    use vellum_engine::instrumentation::NativeTimingProfileV1;
    for capacity in [0, 1, 512] {
        for workspace in [false, true] {
            let (mut o, _) = owner();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            o.configure_retention_budget(33554432, &mut r).unwrap();
            o.configure_discovery_v1(
                &scope(),
                discovery_support::limits(),
                &discovery_support::facts(),
                &mut r,
            )
            .unwrap();
            let (dc, entry, variants) = selectors(&mut o, &mut r);
            let c = context();
            let admission = TrustedRelationResultAdmissionV1 {
                context: &c,
                max_result_bytes: RelationResultV1::required_bytes(),
                result_admission_allowed: Some(true),
                context_current: Some(true),
                relation_result_identifiers_allowed: Some(true),
            };
            let release = TrustedRelationResultReleaseV1 {
                context: &c,
                max_result_bytes: RelationResultV1::required_bytes(),
                outcome_readable: Some(true),
                relation_result_readable: Some(true),
                relation_result_context_current: Some(true),
                historical_target_readable: Some(true),
                reply_ready: Some(true),
                resources_ready: Some(true),
            };
            let profile = observation::Profile {
                epoch: 7,
                maximum: 4096,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
            };
            let mut command = if workspace {
                RelationCommandV1::CreateWorkspace
            } else {
                RelationCommandV1::CreateView
            };
            let variant = &variants[usize::from(workspace)].reference;
            for retiring in [false, true] {
                let id = o.issue_invocation(&mut r).unwrap();
                profile.bind(&mut o, &id, &c, &mut r);
                let q = normalize(&id, &command, &mut r);
                let cut = prepare(&mut o, &mut r);
                let route = NativeDiscoveredCommandRouteV1 {
                    context: &dc,
                    scope: &scope(),
                    cut: &cut,
                    entry: &entry,
                    variant,
                    facts: &discovery_support::facts(),
                };
                o.submit_discovered_native_container_v1(route, &q, Some(&admission), &mut r)
                    .unwrap();
                o.run(&id, &mut r).unwrap();
                let generation = o.generation();
                let cut = prepare(&mut o, &mut r);
                o.submit_discovered_native_container_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &cut,
                        entry: &entry,
                        variant,
                        facts: &discovery_support::facts(),
                    },
                    &q,
                    Some(&admission),
                    &mut r,
                )
                .unwrap();
                o.run(&id, &mut r).unwrap();
                assert_eq!(o.generation(), generation);
                let mut observer = profile.open(&mut o, &id, &c, &mut r);
                let denied = TrustedRelationResultReleaseV1 {
                    relation_result_readable: None,
                    ..release
                };
                assert!(
                    profile
                        .query(
                            &o,
                            &mut observer,
                            InvocationReceiptRequestV1::Relation(&denied),
                            &c,
                            &mut r
                        )
                        .is_err()
                );
                let InvocationReceiptV1::Relation(result) = profile
                    .query(
                        &o,
                        &mut observer,
                        InvocationReceiptRequestV1::Relation(&release),
                        &c,
                        &mut r,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                assert_eq!(result.invocation, id);
                if !retiring {
                    command = match result.receipt {
                        RelationReceiptV1::ViewCreated(view) => RelationCommandV1::RetireView {
                            expected: o.logical_view(&view, &mut r).unwrap().stamp,
                            view,
                        },
                        RelationReceiptV1::WorkspaceCreated(ref workspace) => {
                            RelationCommandV1::RetireWorkspace {
                                expected: o.workspace(workspace, &mut r).unwrap().context,
                            }
                        }
                        _ => panic!(),
                    };
                } else {
                    assert!(matches!(
                        result.receipt,
                        RelationReceiptV1::ViewRetired(_) | RelationReceiptV1::WorkspaceRetired(_)
                    ));
                }
                o.forget_invocation_outcome(&id, &mut r).unwrap();
                assert!(
                    profile
                        .query(
                            &o,
                            &mut observer,
                            InvocationReceiptRequestV1::Relation(&release),
                            &c,
                            &mut r
                        )
                        .is_err()
                );
                assert_eq!(
                    o.retire_invocation(&id, &mut r),
                    Err(InvocationError::Unavailable)
                );
                let control = InvocationObservationPolicyControlV1 {
                    context: &c,
                    owner_control_allowed: Some(true),
                    retention_released: Some(true),
                    delivery_drained: Some(true),
                };
                o.retire_invocation_observation_v1(&mut observer, &control, &mut r)
                    .unwrap();
                o.retire_invocation_observation_policy_v1(&id, &control, &mut r)
                    .unwrap();
                o.retire_invocation(&id, &mut r).unwrap();
                let released = o.retention_usage();
                assert!(o.retire_invocation(&id, &mut r).is_err());
                assert_eq!(o.retention_usage(), released);
            }
            assert_eq!(o.logical_view_usage().live, 0);
            assert_eq!(o.workspace_usage().live, 0);
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == 512 {
                for boundary in [
                    Boundary::DiscoveryBuiltinPreparation,
                    Boundary::RelationDecodedAdmission,
                    Boundary::RelationResultAdmission,
                    Boundary::RelationResultRelease,
                    Boundary::InvocationReceiptRelease,
                ] {
                    assert!(
                        events
                            .iter()
                            .any(|e| e.boundary == boundary && e.service_time.is_some()),
                        "{boundary:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn wrong_container_variant_cannot_apply_loss_and_result_admission_cannot_upgrade_intent() {
    let (mut o, mut r) = owner();
    o.configure_retention_budget(33554432, &mut r).unwrap();
    o.configure_discovery_v1(
        &scope(),
        discovery_support::limits(),
        &discovery_support::facts(),
        &mut r,
    )
    .unwrap();
    let (dc, entry, variants) = selectors(&mut o, &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    let command = RelationCommandV1::CreateView;
    let q = normalize(&id, &command, &mut r);
    let c = context();
    let mut admission = TrustedRelationResultAdmissionV1 {
        context: &c,
        max_result_bytes: RelationResultV1::required_bytes(),
        result_admission_allowed: None,
        context_current: Some(true),
        relation_result_identifiers_allowed: Some(true),
    };
    let mut lost = discovery_support::facts();
    lost.authorized = Some(false);
    let cut = prepare(&mut o, &mut r);
    assert_eq!(
        o.submit_discovered_native_container_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &dc,
                scope: &scope(),
                cut: &cut,
                entry: &entry,
                variant: &variants[1].reference,
                facts: &lost,
            },
            &q,
            Some(&admission),
            &mut r
        ),
        Err(DiscoveredNativeCommandErrorV1::Discovery(
            DiscoveryErrorV1::Unavailable
        ))
    );
    o.retire_discovery_cut_v1(&cut, &discovery_support::facts(), &mut r)
        .unwrap();
    let cut = prepare(&mut o, &mut r);
    assert_eq!(
        o.submit_discovered_native_container_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &dc,
                scope: &scope(),
                cut: &cut,
                entry: &entry,
                variant: &variants[0].reference,
                facts: &discovery_support::facts(),
            },
            &q,
            Some(&admission),
            &mut r
        ),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    let cut = prepare(&mut o, &mut r);
    o.submit_discovered_native_container_v1(
        NativeDiscoveredCommandRouteV1 {
            context: &dc,
            scope: &scope(),
            cut: &cut,
            entry: &entry,
            variant: &variants[0].reference,
            facts: &discovery_support::facts(),
        },
        &q,
        None,
        &mut r,
    )
    .unwrap();
    admission.result_admission_allowed = Some(true);
    let cut = prepare(&mut o, &mut r);
    assert_eq!(
        o.submit_discovered_native_container_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &dc,
                scope: &scope(),
                cut: &cut,
                entry: &entry,
                variant: &variants[0].reference,
                facts: &discovery_support::facts(),
            },
            &q,
            Some(&admission),
            &mut r
        ),
        Err(DiscoveredNativeCommandErrorV1::Invocation(
            InvocationError::Unavailable
        ))
    );
    o.run(&id, &mut r).unwrap();
    assert_eq!(o.logical_view_usage().live, 1);
}
