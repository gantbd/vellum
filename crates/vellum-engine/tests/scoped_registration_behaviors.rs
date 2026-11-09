//! Scoped mutation uses original opaque bases, not raw native identities/stamps.
#[path = "scoped_registration_behaviors/context_dispatch.rs"]
mod context_dispatch;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/invocation_observation_fixture.rs"]
mod observation_support;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder, measurement as m};
use vellum_engine::source::Limits;
#[path = "support/scoped_registration_envelope.rs"]
mod envelope;
#[path = "support/resource_catalog_fixture.rs"]
mod support;

#[test]
fn f06_receipt_assembly_reauthorizes_scoped_registration_without_resampling_historical_bases() {
    let (mut o, mut r, s, c) = setup();
    let b = basis_facts(&s);
    let native = support::facts();
    let captured = o
        .capture_registration_basis_v1(
            &s,
            &support::address(ResourceCatalogTableV1::Read, "PRIVATE_ROW"),
            &b,
            &mut r,
        )
        .unwrap();
    let refs = [captured.basis];
    let changes = [ScopedResourceRegistrationChangeV1 {
        basis: 0,
        action: ResourceCatalogActionV1::Install(support::configuration(
            ResourceCatalogTableV1::Read,
        )),
    }];
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = observation_support::Profile {
        epoch: 7,
        maximum: 8192,
        capabilities: InvocationReceiptCapabilitiesV1 {
            scoped_registration: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    let f = host(&c, &s, &b, &native);
    o.submit_scoped_resource_registration_v1(&id, &changes, &refs, &c, &f, &mut r)
        .unwrap();
    let mut observer = profile.open(&mut o, &id, &c, &mut r);
    o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
        .unwrap();
    let f = release(&c, &s, &b);
    let InvocationReceiptV1::ScopedRegistration(out) = profile
        .query(
            &o,
            &mut observer,
            InvocationReceiptRequestV1::ScopedRegistration(&f),
            &c,
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(out.invocation, id);
    assert_eq!(out.registrations.len(), 1);
}
fn scope() -> ResourceRegistrationBasisScopeV1 {
    ResourceRegistrationBasisScopeV1 {
        authority: "a".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "register".into(),
        destination: "local".into(),
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
        destination: "local".into(),
        continuity: "session".into(),
        profile: SCOPED_RESOURCE_REGISTRATION_PROFILE_V1.into(),
    }
}
fn basis_facts(
    c: &ResourceRegistrationBasisScopeV1,
) -> TrustedResourceRegistrationBasisFactsV1<'_> {
    TrustedResourceRegistrationBasisFactsV1 {
        scope: c,
        native_profile: support::limits(),
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
fn setup() -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistrationBasisScopeV1,
    SettingsExecutionContextV1,
) {
    setup_with_headroom(0)
}
fn setup_with_headroom(
    extra: usize,
) -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistrationBasisScopeV1,
    SettingsExecutionContextV1,
) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 16,
            max_intent_bytes: 65536,
        },
    );
    let mut r = Recorder::disabled();
    let s = scope();
    let c = context();
    o.configure_retention_budget(1_000_000usize.checked_add(extra).unwrap(), &mut r)
        .unwrap();
    o.configure_resource_catalog_v1(&[], support::limits(), &support::facts(), &mut r)
        .unwrap();
    o.configure_registration_bases_v1(
        &s,
        support::limits(),
        ResourceRegistrationBasisLimitsV1 {
            max_bases: 16,
            max_issued: 32,
            max_root_bytes: 1_000_000,
            max_profile_bytes: 4096,
            max_scope_bytes: 4096,
            max_tag_bytes: 128,
            max_slot_bytes: 4096,
            max_basis_bytes: 8192,
            max_use_bytes: 8192,
            max_reply_bytes: 1024,
            max_held_bytes: 700_000,
        },
        &TrustedResourceRegistrationBasisSetupV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            preparation_reserved: Some(true),
        },
        &mut r,
    )
    .unwrap();
    (o, r, s, c)
}
fn host<'a>(
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

#[test]
fn given_a_basis_permutation_when_service_runs_then_one_scoped_receipt_preserves_order_without_native_fields()
 {
    let (mut o, mut r, s, c) = setup();
    let b = basis_facts(&s);
    let n = support::facts();
    let a = o
        .capture_registration_basis_v1(
            &s,
            &support::address(ResourceCatalogTableV1::Read, "PRIVATE_A"),
            &b,
            &mut r,
        )
        .unwrap();
    let z = o
        .capture_registration_basis_v1(
            &s,
            &support::address(ResourceCatalogTableV1::Write, "PRIVATE_Z"),
            &b,
            &mut r,
        )
        .unwrap();
    let refs = [a.basis, z.basis];
    let changes = [
        ScopedResourceRegistrationChangeV1 {
            basis: 1,
            action: ResourceCatalogActionV1::Install(support::configuration(
                ResourceCatalogTableV1::Write,
            )),
        },
        ScopedResourceRegistrationChangeV1 {
            basis: 0,
            action: ResourceCatalogActionV1::Install(support::configuration(
                ResourceCatalogTableV1::Read,
            )),
        },
    ];
    let id = o.issue_invocation(&mut r).unwrap();
    let f = host(&c, &s, &b, &n);
    assert_eq!(
        o.submit_scoped_resource_registration_v1(&id, &changes, &refs, &c, &f, &mut r),
        Ok(InvocationState::Pending)
    );
    o.create(b"prose", &mut r).unwrap();
    let p = o
        .prepare_resource_catalog_v1(
            &[support::install(ResourceCatalogTableV1::Read, "other")],
            &n,
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &n, &mut r).unwrap();
    let before = o.generation();
    o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
        .unwrap();
    let out = o
        .retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r)
        .unwrap();
    assert_eq!(out.invocation, id);
    assert_eq!(out.status, ResourceCatalogStatusV1::Applied);
    assert_eq!(
        out.registrations,
        vec![
            ScopedResourceRegistrationRowV1 {
                basis: 1,
                state: ResourceRegistrationConditionV1::Present
            },
            ScopedResourceRegistrationRowV1 {
                basis: 0,
                state: ResourceRegistrationConditionV1::Present
            }
        ]
    );
    assert!(!format!("{out:?}").contains("PRIVATE"));
    assert_eq!(
        o.inspect_registration_basis_v1(&refs[0], &s, &b, &mut r),
        Err(vellum_engine::resource::ResourceError::Unavailable)
    );
    o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.generation(), before + 1);
}

#[test]
fn given_optional_or_saturated_diagnostics_when_scoped_jobs_settle_then_real_cohorts_and_results_are_once_only()
 {
    for capacity in [0, 1, 256] {
        let (mut o, _, s, c) = setup();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        o.configure_invocation_measurements_v1(
            InvocationMeasurementLimitsV1 {
                values: m::ValueLimitsV1 {
                    max_integer: 100,
                    max_seconds: 0.0,
                },
                max_pool_bytes: InvocationMeasurementLimitsV1::required_pool_bytes(32).unwrap(),
            },
            Some(true),
            &mut r,
        )
        .unwrap();
        let b = basis_facts(&s);
        let n = support::facts();
        let f = host(&c, &s, &b, &n);
        let first = o
            .capture_registration_basis_v1(
                &s,
                &support::address(ResourceCatalogTableV1::Read, "PRIVATE_TARGET"),
                &b,
                &mut r,
            )
            .unwrap();
        let v = [ScopedResourceRegistrationChangeV1 {
            basis: 0,
            action: ResourceCatalogActionV1::Install(support::configuration(
                ResourceCatalogTableV1::Read,
            )),
        }];
        let id = o.issue_invocation(&mut r).unwrap();
        let ResourceCatalogActionV1::Install(config) = &v[0].action else {
            unreachable!()
        };
        let normalized = envelope::with_install(&id, config, 0.0, |value| {
            normalize_scoped_registration_envelope_v1(
                value,
                1,
                support::limits(),
                vellum_engine::capture::CaptureLimitsV1 {
                    max_bytes: 32768,
                    max_nodes: 128,
                    max_depth: 8,
                },
                65536,
                &mut r,
            )
            .unwrap()
        });
        assert_eq!(
            o.submit_normalized_scoped_registration_v1(&normalized, &[], &c, &f, &mut r),
            Err(InvocationError::MalformedIntent)
        );
        o.submit_normalized_scoped_registration_v1(
            &normalized,
            std::slice::from_ref(&first.basis),
            &c,
            &f,
            &mut r,
        )
        .unwrap();
        o.submit_scoped_resource_registration_v1(&id, &v, &[first.basis], &c, &f, &mut r)
            .unwrap();
        o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
            .unwrap();
        let noop = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_resource_registration_v1(&noop, &[], &[], &c, &f, &mut r)
            .unwrap();
        o.run_scoped_resource_registration_v1(&noop, &c, &f, &mut r)
            .unwrap();
        let second = o
            .capture_registration_basis_v1(
                &s,
                &support::address(ResourceCatalogTableV1::Read, "PRIVATE_OTHER"),
                &b,
                &mut r,
            )
            .unwrap();
        let refused = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_resource_registration_v1(
            &refused,
            &v,
            std::slice::from_ref(&second.basis),
            &c,
            &f,
            &mut r,
        )
        .unwrap();
        let mut denied = host(&c, &s, &b, &n);
        denied.basis_scope_current = None;
        o.run_scoped_resource_registration_v1(&refused, &c, &denied, &mut r)
            .unwrap();
        let stopped = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_resource_registration_v1(&stopped, &v, &[second.basis], &c, &f, &mut r)
            .unwrap();
        o.request_invocation_stop(
            &stopped,
            &TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
        o.run_scoped_resource_registration_v1(&stopped, &c, &denied, &mut r)
            .unwrap();
        let before = o.generation();
        for ticket in [&id, &noop, &refused, &stopped] {
            o.run_scoped_resource_registration_v1(ticket, &c, &f, &mut r)
                .unwrap();
        }
        let receipt = o
            .retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r)
            .unwrap();
        assert_eq!(receipt.status, ResourceCatalogStatusV1::Applied);
        assert_eq!(receipt.registrations.len(), 1);
        assert_eq!(o.generation(), before);
        let points = o.invocation_measurements_v1(&mut r).unwrap().unwrap();
        let total = |point: &m::MeasurementPointV1| {
            let m::PointStateV1::Observed {
                value: m::ValueV1::Counter(v),
                ..
            } = point.state()
            else {
                panic!("counter")
            };
            v.total
        };
        assert_eq!(total(&points.admissions), 4);
        assert_eq!(
            points.dispositions.each_ref().map(total),
            [1, 1, 1, 1, 0, 0]
        );
        assert!(r.queued() <= capacity);
        if capacity == 256 {
            let mut seen = Vec::new();
            let mut publications = 0;
            while let Some(e) = r.pop_front() {
                assert!(!format!("{e:?}").contains("PRIVATE"));
                publications += usize::from(e.boundary == Boundary::ResourceCatalogPublication);
                if matches!(
                    e.boundary,
                    Boundary::ScopedRegistrationPreparation
                        | Boundary::ScopedRegistrationValidation
                        | Boundary::ScopedRegistrationResult
                        | Boundary::ScopedRegistrationArgumentNormalization
                        | Boundary::ScopedRegistrationDecodedAdmission
                ) {
                    assert!(e.service_time.is_some());
                    seen.push(e.boundary);
                }
            }
            assert_eq!(publications, 1);
            for boundary in [
                Boundary::ScopedRegistrationPreparation,
                Boundary::ScopedRegistrationValidation,
                Boundary::ScopedRegistrationResult,
                Boundary::ScopedRegistrationArgumentNormalization,
                Boundary::ScopedRegistrationDecodedAdmission,
            ] {
                assert!(seen.contains(&boundary));
            }
        }
    }
}

#[test]
fn given_passive_envelopes_when_normalized_then_every_configuration_replays_the_same_typed_intent()
{
    use vellum_engine::capture::CaptureLimitsV1;
    for shape in 0..72 {
        let (mut o, mut r, s, c) = setup();
        let b = basis_facts(&s);
        let n = support::facts();
        let table = if shape < 64 {
            ResourceCatalogTableV1::Write
        } else {
            ResourceCatalogTableV1::Read
        };
        let mut config = support::configuration(table);
        config.identity_known = shape % 2 == 0;
        config.capabilities = if shape < 64 {
            ResourceCatalogCapabilitiesV1::Write {
                procedure: "PRIVATE_PROCEDURE".into(),
                logical_continuity: shape & 1 != 0,
                synchronization_supported: shape & 2 != 0,
                conditional_write: shape & 4 != 0,
                causal_receipts: shape & 8 != 0,
                non_reuse_tokens: shape & 16 != 0,
                exclusive_create: shape & 32 != 0,
            }
        } else if shape < 70 {
            ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
                atomic_read: shape % 2 == 0,
                insertion_line_break: [
                    CatalogInsertionLineBreakV1::Lf,
                    CatalogInsertionLineBreakV1::CrLf,
                    CatalogInsertionLineBreakV1::Cr,
                ][(shape - 64) / 2],
            }
        } else {
            ResourceCatalogCapabilitiesV1::AnnotationsV1 {
                atomic_read: shape % 2 == 0,
            }
        };
        let refs = [o
            .capture_registration_basis_v1(
                &s,
                &support::address(table, "PRIVATE_TARGET"),
                &b,
                &mut r,
            )
            .unwrap()
            .basis];
        let id = o.issue_invocation(&mut r).unwrap();
        let f = host(&c, &s, &b, &n);
        let normalized = envelope::with_install(&id, &config, 0.0, |value| {
            normalize_scoped_registration_envelope_v1(
                value,
                1,
                support::limits(),
                CaptureLimitsV1 {
                    max_bytes: 32768,
                    max_nodes: 128,
                    max_depth: 8,
                },
                65536,
                &mut r,
            )
            .unwrap()
        });
        assert_eq!(normalized.invocation(), &id);
        assert_eq!(
            normalized.changes(),
            &[ScopedResourceRegistrationChangeV1 {
                basis: 0,
                action: ResourceCatalogActionV1::Install(config)
            }]
        );
        o.submit_scoped_resource_registration_v1(&id, normalized.changes(), &refs, &c, &f, &mut r)
            .unwrap();
        assert_eq!(
            o.submit_normalized_scoped_registration_v1(&normalized, &refs, &c, &f, &mut r),
            Ok(InvocationState::Pending)
        );
        o.run_scoped_resource_registration_v1(&id, &c, &f, &mut r)
            .unwrap();
        let out = o
            .retained_scoped_resource_registration_result_v1(&id, &release(&c, &s, &b), &mut r)
            .unwrap();
        assert_eq!(
            out.registrations[0].state,
            ResourceRegistrationConditionV1::Present
        );
        assert_eq!(o.invocation_usage().retained, 1);
    }
}
