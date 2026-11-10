use super::super::tests as native_fixture;
use super::*;
fn scope() -> ResourceRegistrationBasisScopeV1 {
    ResourceRegistrationBasisScopeV1 {
        authority: "a".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "register".into(),
        destination: "host".into(),
        policy: "policy".into(),
        continuity: "session".into(),
        profile: NATIVE_REGISTRATION_BASIS_PROFILE_V1.into(),
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "register".into(),
        destination: "host".into(),
        continuity: "session".into(),
        profile: SCOPED_RESOURCE_REGISTRATION_PROFILE_V1.into(),
    }
}
pub(super) fn basis(
    s: &ResourceRegistrationBasisScopeV1,
) -> TrustedResourceRegistrationBasisFactsV1<'_> {
    TrustedResourceRegistrationBasisFactsV1 {
        scope: s,
        native_profile: native_fixture::profile(),
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        registration_allowed: Some(true),
        identity_allowed: Some(true),
        registry_coverage_complete: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        basis_allowed: Some(true),
        presence_observable: Some(true),
        equality_observable: Some(true),
        slot_allowed: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        interpretation_current: Some(true),
        host_basis_allowed: Some(true),
    }
}
pub(super) fn host<'a>(
    c: &'a SettingsExecutionContextV1,
    s: &'a ResourceRegistrationBasisScopeV1,
    b: &'a TrustedResourceRegistrationBasisFactsV1<'a>,
    n: &'a TrustedResourceCatalogFactsV1,
) -> TrustedScopedResourceRegistrationInvocationV1<'a> {
    TrustedScopedResourceRegistrationInvocationV1 {
        context: c,
        scope: s,
        basis: b,
        native: n,
        max_result_bytes: 8192,
        interpretation_current: Some(true),
        basis_scope_current: Some(true),
        scoped_result_allowed: Some(true),
    }
}
fn release<'a>(
    c: &'a SettingsExecutionContextV1,
    s: &'a ResourceRegistrationBasisScopeV1,
    b: &'a TrustedResourceRegistrationBasisFactsV1<'a>,
) -> TrustedScopedResourceRegistrationResultV1<'a> {
    TrustedScopedResourceRegistrationResultV1 {
        context: c,
        scope: s,
        basis: b,
        max_result_bytes: 8192,
        interpretation_current: Some(true),
        basis_scope_current: Some(true),
        outcome_readable: Some(true),
        scoped_result_readable: Some(true),
        scoped_result_context_current: Some(true),
        scoped_result_allowed: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
pub(super) fn setup() -> (
    SourceCoordinator,
    Recorder,
    SettingsExecutionContextV1,
    ResourceRegistrationBasisScopeV1,
) {
    let (mut o, mut r) = native_fixture::setup();
    let c = context();
    let s = scope();
    o.configure_retention_budget(1_000_000, &mut r).unwrap();
    o.configure_registration_bases_v1(
        &s,
        native_fixture::profile(),
        ResourceRegistrationBasisLimitsV1 {
            max_bases: 8,
            max_issued: 16,
            max_root_bytes: 1_000_000,
            max_profile_bytes: 4096,
            max_scope_bytes: 4096,
            max_tag_bytes: 128,
            max_slot_bytes: 4096,
            max_basis_bytes: 8192,
            max_use_bytes: 8192,
            max_reply_bytes: 1024,
            max_held_bytes: 300_000,
        },
        &TrustedResourceRegistrationBasisSetupV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            preparation_reserved: Some(true),
        },
        &mut r,
    )
    .unwrap();
    (o, r, c, s)
}
pub(super) fn refs(
    o: &mut SourceCoordinator,
    s: &ResourceRegistrationBasisScopeV1,
    r: &mut Recorder,
) -> Vec<ResourceRegistrationBasisIdV1> {
    vec![
        o.capture_registration_basis_v1(
            s,
            &native_fixture::change("target", CatalogExpectationV1::Absent).address,
            &basis(s),
            r,
        )
        .unwrap()
        .basis,
    ]
}
fn changes() -> Vec<ScopedResourceRegistrationChangeV1> {
    vec![ScopedResourceRegistrationChangeV1 {
        basis: 0,
        action: native_fixture::change("target", CatalogExpectationV1::Absent).action,
    }]
}
fn submit(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    c: &SettingsExecutionContextV1,
    s: &ResourceRegistrationBasisScopeV1,
    refs: &[ResourceRegistrationBasisIdV1],
    changes: &[ScopedResourceRegistrationChangeV1],
) -> InvocationId {
    let id = o.issue_invocation(r).unwrap();
    o.submit_scoped_resource_registration_v1(
        &id,
        changes,
        refs,
        c,
        &host(c, s, &basis(s), &native_fixture::native()),
        r,
    )
    .unwrap();
    id
}
fn control() -> TrustedResourceRegistrationBasisControlV1 {
    TrustedResourceRegistrationBasisControlV1 {
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn gate(f: &mut TrustedResourceRegistrationBasisFactsV1<'_>, i: usize, value: Option<bool>) {
    *[
        &mut f.authorized,
        &mut f.scope_current,
        &mut f.continuous,
        &mut f.live,
        &mut f.not_stopped,
        &mut f.registration_allowed,
        &mut f.identity_allowed,
        &mut f.registry_coverage_complete,
        &mut f.resources_ready,
        &mut f.retained,
        &mut f.reply_ready,
        &mut f.basis_allowed,
        &mut f.presence_observable,
        &mut f.equality_observable,
        &mut f.slot_allowed,
        &mut f.owner_control_allowed,
        &mut f.preparation_reserved,
        &mut f.interpretation_current,
        &mut f.host_basis_allowed,
    ][i] = value;
}

#[test]
fn malformed_permutations_duplicate_slots_and_wrong_table_configs_never_admit() {
    for case in 0..5 {
        let (mut o, mut r, c, s) = setup();
        let mut refs = refs(&mut o, &s, &mut r);
        let mut v = changes();
        match case {
            0 => refs.clear(),
            1 => v[0].basis = 1,
            2 => {
                refs.push(refs[0].clone());
                v.push(ScopedResourceRegistrationChangeV1 {
                    basis: 0,
                    action: ResourceCatalogActionV1::Remove,
                });
            }
            3 => {
                refs.push(refs[0].clone());
                v.push(ScopedResourceRegistrationChangeV1 {
                    basis: 1,
                    action: ResourceCatalogActionV1::Remove,
                });
            }
            _ => {
                v[0].action = ResourceCatalogActionV1::Install(ResourceCatalogConfigurationV1 {
                    identity_domain: "d".into(),
                    identity_key: "k".into(),
                    identity_known: true,
                    locator: "l".into(),
                    capabilities: ResourceCatalogCapabilitiesV1::Write {
                        procedure: "p".into(),
                        logical_continuity: false,
                        synchronization_supported: false,
                        conditional_write: false,
                        causal_receipts: false,
                        non_reuse_tokens: false,
                        exclusive_create: false,
                    },
                })
            }
        }
        let id = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        assert_eq!(
            o.submit_scoped_resource_registration_v1(
                &id,
                &v,
                &refs,
                &c,
                &host(&c, &s, &basis(&s), &native_fixture::native()),
                &mut r
            ),
            Err(InvocationError::MalformedIntent)
        );
        assert!(matches!(
            o.invocations.slots[id.slot].as_ref().unwrap().phase,
            Phase::Issued(_)
        ));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.registration_basis_usage_v1().unwrap().live, 1);
    }
}

#[test]
fn every_basis_gate_is_current_at_admission_service_and_historical_release() {
    for i in 0..19 {
        for value in [Some(false), None] {
            let (mut o, mut r, c, s) = setup();
            let refs = refs(&mut o, &s, &mut r);
            let v = changes();
            let unbound = o.issue_invocation(&mut r).unwrap();
            let mut denied = basis(&s);
            gate(&mut denied, i, value);
            assert_eq!(
                o.submit_scoped_resource_registration_v1(
                    &unbound,
                    &v,
                    &refs,
                    &c,
                    &host(&c, &s, &denied, &native_fixture::native()),
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            );
            let id = submit(&mut o, &mut r, &c, &s, &refs, &v);
            let before = o.generation();
            o.run_scoped_resource_registration_v1(
                &id,
                &c,
                &host(&c, &s, &denied, &native_fixture::native()),
                &mut r,
            )
            .unwrap();
            assert!(matches!(
                o.invocation_outcome(&id, &mut r),
                Ok(InvocationOutcome::ResourceCatalogRejected(
                    ResourceError::Unavailable
                ))
            ));
            assert_eq!(o.generation(), before);
            assert!(!o.is_faulted());
            let succeeded = submit(&mut o, &mut r, &c, &s, &refs, &v);
            o.run_scoped_resource_registration_v1(
                &succeeded,
                &c,
                &host(&c, &s, &basis(&s), &native_fixture::native()),
                &mut r,
            )
            .unwrap();
            assert_eq!(
                o.retained_scoped_resource_registration_result_v1(
                    &succeeded,
                    &release(&c, &s, &denied),
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            );
            assert!(
                o.retained_scoped_resource_registration_result_v1(
                    &succeeded,
                    &release(&c, &s, &basis(&s)),
                    &mut r
                )
                .is_ok()
            );
        }
    }
}

#[test]
fn empty_batches_need_actual_current_issuer_and_native_rights_but_valid_noops_do_not_publish() {
    let (mut o, mut r, c, s) = setup();
    let id = submit(&mut o, &mut r, &c, &s, &[], &[]);
    let before = o.generation();
    o.run_scoped_resource_registration_v1(
        &id,
        &c,
        &host(&c, &s, &basis(&s), &native_fixture::native()),
        &mut r,
    )
    .unwrap();
    let out = o
        .retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &basis(&s)), &mut r)
        .unwrap();
    assert_eq!(out.status, ResourceCatalogStatusV1::Unchanged);
    assert!(out.registrations.is_empty());
    assert_eq!(o.generation(), before);
    let id = submit(&mut o, &mut r, &c, &s, &[], &[]);
    o.retire_registration_basis_issuer_v1(&control(), &mut r)
        .unwrap();
    o.run_scoped_resource_registration_v1(
        &id,
        &c,
        &host(&c, &s, &basis(&s), &native_fixture::native()),
        &mut r,
    )
    .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::ResourceCatalogRejected(
            ResourceError::Unavailable
        ))
    ));
    let (mut o, mut r, c, s) = setup();
    let id = submit(&mut o, &mut r, &c, &s, &[], &[]);
    o.run_scoped_resource_registration_v1(
        &id,
        &c,
        &host(
            &c,
            &s,
            &basis(&s),
            &TrustedResourceCatalogFactsV1::default(),
        ),
        &mut r,
    )
    .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::ResourceCatalogRejected(
            ResourceError::Unavailable
        ))
    ));
}

#[test]
fn retired_or_stale_bases_reject_before_any_catalog_publication_and_absence_can_become_current_again()
 {
    for mode in 0..3 {
        let (mut o, mut r, c, s) = setup();
        let refs = refs(&mut o, &s, &mut r);
        let id = submit(&mut o, &mut r, &c, &s, &refs, &changes());
        if mode == 0 {
            o.retire_registration_basis_v1(&refs[0], &control(), &mut r)
                .unwrap();
        } else {
            let p = o
                .prepare_resource_catalog_v1(
                    &[native_fixture::change(
                        "target",
                        CatalogExpectationV1::Absent,
                    )],
                    &native_fixture::native(),
                    &mut r,
                )
                .unwrap();
            o.commit_resource_catalog_v1(p, &native_fixture::native(), &mut r)
                .unwrap();
            if mode == 2 {
                let mut remove = native_fixture::change(
                    "target",
                    CatalogExpectationV1::Present { generation: 1 },
                );
                remove.action = ResourceCatalogActionV1::Remove;
                let p = o
                    .prepare_resource_catalog_v1(&[remove], &native_fixture::native(), &mut r)
                    .unwrap();
                o.commit_resource_catalog_v1(p, &native_fixture::native(), &mut r)
                    .unwrap();
            }
        }
        let before = o.generation();
        o.run_scoped_resource_registration_v1(
            &id,
            &c,
            &host(&c, &s, &basis(&s), &native_fixture::native()),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), before + u64::from(mode == 2));
        assert_eq!(
            o.retained_scoped_resource_registration_result_v1(
                &id,
                &release(&c, &s, &basis(&s)),
                &mut r
            )
            .is_ok(),
            mode == 2
        );
    }
}

#[test]
fn replay_binds_original_refs_indices_configuration_context_and_result_ceiling() {
    let (mut o, mut r, c, s) = setup();
    let refs = refs(&mut o, &s, &mut r);
    let v = changes();
    let b = basis(&s);
    let n = native_fixture::native();
    let mut f = host(&c, &s, &b, &n);
    f.scoped_result_allowed = None;
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r)
        .unwrap();
    f.scoped_result_allowed = Some(true);
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r),
        Ok(InvocationState::Pending)
    );
    let changed = [ScopedResourceRegistrationChangeV1 {
        basis: 0,
        action: ResourceCatalogActionV1::Remove,
    }];
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &changed, &refs, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    f.max_result_bytes += 1;
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    f.max_result_bytes -= 1;
    let mut other = s.clone();
    other.policy.push('x');
    let ob = basis(&other);
    let of = host(&c, &other, &ob, &n);
    let other_refs = vec![
        o.capture_registration_basis_v1(
            &other,
            &native_fixture::change("target", CatalogExpectationV1::Absent).address,
            &ob,
            &mut r,
        )
        .unwrap()
        .basis,
    ];
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &v, &other_refs, &c, &of, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
        .unwrap();
    assert_eq!(
        o.retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(!o.is_faulted());
}

#[test]
fn native_and_scoped_routes_cannot_reinterpret_the_same_slot_or_leak_raw_receipts() {
    let (mut o, mut r, c, s) = setup();
    let refs = refs(&mut o, &s, &mut r);
    let id = submit(&mut o, &mut r, &c, &s, &refs, &changes());
    let n = native_fixture::native();
    let raw = TrustedResourceRegistrationInvocationV1 {
        context: &c,
        profile: native_fixture::profile(),
        max_result_bytes: 8192,
        interpretation_current: Some(true),
        host_result_allowed: Some(true),
        native: &n,
    };
    assert_eq!(
        o.run_resource_registration_v1(&id, &c, &raw, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_scoped_resource_registration_v1(&id, &c, &host(&c, &s, &basis(&s), &n), &mut r)
        .unwrap();
    let raw_result = TrustedResourceRegistrationResultV1 {
        context: &c,
        max_result_bytes: 8192,
        outcome_readable: Some(true),
        registration_result_readable: Some(true),
        registration_result_context_current: Some(true),
        host_result_allowed: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    assert!(matches!(
        o.retained_resource_registration_result_v1(&id, &raw_result, &mut r),
        Err(InvocationError::Unavailable)
    ));
    let id = o.issue_invocation(&mut r).unwrap();
    let mut native_c = c.clone();
    native_c.profile = NATIVE_RESOURCE_REGISTRATION_PROFILE_V1.into();
    let raw = TrustedResourceRegistrationInvocationV1 {
        context: &native_c,
        ..raw
    };
    o.submit_resource_registration_v1(&id, &[], &native_c, &raw, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_scoped_resource_registration_v1(
            &id,
            &[],
            &[],
            &c,
            &host(&c, &s, &basis(&s), &n),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    assert!(
        o.retained_scoped_resource_registration_result_v1(
            &id,
            &release(&c, &s, &basis(&s)),
            &mut r
        )
        .is_err()
    );
}

#[test]
fn result_delivery_reauthorizes_without_retesting_rows_and_forget_preserves_shared_bases() {
    let (mut o, mut r, c, s) = setup();
    let refs = refs(&mut o, &s, &mut r);
    let id = submit(&mut o, &mut r, &c, &s, &refs, &changes());
    o.run_scoped_resource_registration_v1(
        &id,
        &c,
        &host(&c, &s, &basis(&s), &native_fixture::native()),
        &mut r,
    )
    .unwrap();
    let before = o.generation();
    let b = basis(&s);
    let live = o.registration_basis_usage_v1().unwrap();
    for i in 0..8 {
        for denied in [Some(false), None] {
            let mut f = release(&c, &s, &b);
            *[
                &mut f.interpretation_current,
                &mut f.basis_scope_current,
                &mut f.outcome_readable,
                &mut f.scoped_result_readable,
                &mut f.scoped_result_context_current,
                &mut f.scoped_result_allowed,
                &mut f.reply_ready,
                &mut f.resources_ready,
            ][i] = denied;
            assert_eq!(
                o.retained_scoped_resource_registration_result_v1(&id, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
    }
    o.retire_registration_basis_v1(&refs[0], &control(), &mut r)
        .unwrap();
    assert!(
        o.retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r)
            .is_ok()
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(
        o.registration_basis_usage_v1().unwrap().issued_lifetime,
        live.issued_lifetime
    );
    assert_eq!(o.generation(), before);
}

#[test]
fn original_protected_stop_survives_lost_basis_and_edit_rights_without_retiring_shared_authority() {
    let (mut o, mut r, c, s) = setup();
    let refs = refs(&mut o, &s, &mut r);
    let id = submit(&mut o, &mut r, &c, &s, &refs, &changes());
    let control = TrustedInvocationControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    };
    o.request_invocation_stop(&id, &control, &mut r).unwrap();
    let before = o.generation();
    let mut b = basis(&s);
    b.authorized = None;
    b.host_basis_allowed = None;
    let mut wrong = s.clone();
    wrong.policy.push('x');
    let unavailable = TrustedResourceCatalogFactsV1::default();
    let f = host(&c, &wrong, &b, &unavailable);
    assert_eq!(
        o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r),
        Err(InvocationError::Unavailable)
    );
    o.run_scoped_resource_registration_v1(
        &id,
        &c,
        &host(&c, &s, &b, &TrustedResourceCatalogFactsV1::default()),
        &mut r,
    )
    .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert_eq!(o.registration_basis_usage_v1().unwrap().live, 1);
    assert_eq!(o.generation(), before);
}

#[test]
fn scoped_jobs_share_native_unknown_fault_semantics_before_and_after_publication() {
    for after in [false, true] {
        let (mut o, mut r, c, s) = setup();
        let refs = refs(&mut o, &s, &mut r);
        let id = submit(&mut o, &mut r, &c, &s, &refs, &changes());
        let before = o.generation();
        native_fixture::inject_fault(after);
        assert_eq!(
            o.run_scoped_resource_registration_v1(
                &id,
                &c,
                &host(&c, &s, &basis(&s), &native_fixture::native()),
                &mut r
            ),
            Err(InvocationError::Faulted)
        );
        assert!(o.is_faulted());
        assert!(matches!(
            o.invocations.slots[id.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
        assert_eq!(o.generation(), before + u64::from(after));
        assert!(o.retire_invocation(&id, &mut r).is_err());
    }
}

#[test]
fn identical_scoped_repreparation_still_requires_fresh_native_companions() {
    let (mut o, mut r, c, s) = setup();
    let refs = refs(&mut o, &s, &mut r);
    let v = changes();
    let id = submit(&mut o, &mut r, &c, &s, &refs, &v);
    let mut n = native_fixture::native();
    n.companions_ready = None;
    assert_eq!(
        o.submit_scoped_resource_registration_v1(
            &id,
            &v,
            &refs,
            &c,
            &host(&c, &s, &basis(&s), &n),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert!(matches!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    ));
}

#[test]
fn complete_native_scoped_reference_and_receipt_envelopes_fit_exactly_or_refuse_without_binding() {
    for short in [false, true] {
        let (mut o, mut r, c, s) = setup();
        let refs = refs(&mut o, &s, &mut r);
        let v = changes();
        let native_bytes = CatalogRequestCapture::bytes(
            &[native_fixture::change(
                "target",
                CatalogExpectationV1::Absent,
            )],
            native_fixture::profile(),
        )
        .unwrap();
        let exact = retained_prefix(&c, &s, refs.len()).unwrap() + native_bytes;
        assert!(exact >= input_bytes(&c, &s, &v).unwrap());
        o.invocations.limits.max_intent_bytes = exact - usize::from(short);
        let id = o.issue_invocation(&mut r).unwrap();
        let b = basis(&s);
        let n = native_fixture::native();
        let mut f = host(&c, &s, &b, &n);
        f.max_result_bytes = receipt_bytes(1).unwrap();
        let outcome = o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r);
        if short {
            assert_eq!(outcome, Err(InvocationError::Limit));
            assert!(matches!(
                o.invocations.slots[id.slot].as_ref().unwrap().phase,
                Phase::Issued(_)
            ));
        } else {
            assert_eq!(outcome, Ok(InvocationState::Pending));
            o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
                .unwrap();
            let before = o.generation();
            let mut g = release(&c, &s, &b);
            g.max_result_bytes = f.max_result_bytes - 1;
            assert_eq!(
                o.retained_scoped_resource_registration_result_v1(&id, &g, &mut r),
                Err(InvocationError::Limit)
            );
            g.max_result_bytes += 1;
            assert!(
                o.retained_scoped_resource_registration_result_v1(&id, &g, &mut r)
                    .is_ok()
            );
            assert_eq!(o.generation(), before);
        }
    }
    let (mut o, mut r, c, s) = setup();
    let refs = refs(&mut o, &s, &mut r);
    let mut v = changes();
    let id = o.issue_invocation(&mut r).unwrap();
    let b = basis(&s);
    let n = native_fixture::native();
    let mut f = host(&c, &s, &b, &n);
    f.max_result_bytes = receipt_bytes(1).unwrap() - 1;
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    f.max_result_bytes += 1;
    let ResourceCatalogActionV1::Install(config) = &mut v[0].action else {
        unreachable!()
    };
    config.locator = "x".repeat(100_000);
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &v, &refs, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert!(matches!(
        o.invocations.slots[id.slot].as_ref().unwrap().phase,
        Phase::Issued(_)
    ));
    assert_eq!(receipt_bytes(usize::MAX), Err(InvocationError::Limit));
}

#[test]
fn privately_corrupted_original_indices_fault_service_and_protected_controls_instead_of_known_rejection()
 {
    for protected in [false, true] {
        let (mut o, mut r, c, s) = setup();
        let refs = refs(&mut o, &s, &mut r);
        let id = submit(&mut o, &mut r, &c, &s, &refs, &changes());
        let slot = o.invocations.slots[id.slot].as_mut().unwrap();
        let Some(BoundIntent::ResourceCatalog(mut original)) = slot.intent.take() else {
            unreachable!()
        };
        slot.phase = Phase::Running; // Drop the other private Arc, not an execution.
        let capture = Arc::get_mut(&mut original).unwrap();
        let scoped = Arc::get_mut(capture.scoped.as_mut().unwrap()).unwrap();
        scoped.indices[0] = usize::MAX;
        slot.intent = Some(BoundIntent::ResourceCatalog(original.clone()));
        slot.phase = Phase::ResourceCatalogPending(original);
        let before = o.generation();
        if protected {
            let f = TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            };
            assert_eq!(
                o.request_invocation_stop(&id, &f, &mut r),
                Err(InvocationError::Faulted)
            );
        } else {
            assert_eq!(
                o.run_scoped_resource_registration_v1(
                    &id,
                    &c,
                    &host(&c, &s, &basis(&s), &native_fixture::native()),
                    &mut r
                ),
                Err(InvocationError::Faulted)
            );
        }
        assert!(o.is_faulted());
        assert_eq!(o.generation(), before);
    }
}

#[test]
fn reordered_reference_arrays_cannot_replay_an_equal_native_batch_with_a_different_scoped_mapping()
{
    let (mut o, mut r, c, s) = setup();
    let mut refs = refs(&mut o, &s, &mut r);
    refs.push(
        o.capture_registration_basis_v1(
            &s,
            &native_fixture::change("other", CatalogExpectationV1::Absent).address,
            &basis(&s),
            &mut r,
        )
        .unwrap()
        .basis,
    );
    let mut v = changes();
    v.push(ScopedResourceRegistrationChangeV1 {
        basis: 1,
        action: native_fixture::change("other", CatalogExpectationV1::Absent).action,
    });
    let id = submit(&mut o, &mut r, &c, &s, &refs, &v);
    refs.swap(0, 1);
    v[0].basis = 1;
    v[1].basis = 0;
    assert_eq!(
        o.submit_scoped_resource_registration_v1(
            &id,
            &v,
            &refs,
            &c,
            &host(&c, &s, &basis(&s), &native_fixture::native()),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
}
