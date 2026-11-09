use super::scoped_basis::{context, facts, ready};
use super::*;
pub(super) fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 16,
        max_created_value_bytes: 65536,
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
pub(super) fn created() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "extra".into(),
        incarnation: 2,
    }
}
fn proposal() -> SettingsLayerChangeV1 {
    SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: created(),
            scope: SettingScopeV1::Workspace,
            values: vec![(
                "unit".into(),
                SettingContributionV1::Set(SettingValueV1::Number(3.0)),
            )],
        }],
        remove: vec![],
        targets: definition::declaration().targets,
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
pub(super) fn gate<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedNativeSettingsLifecycleInvocationV1<'a> {
    TrustedNativeSettingsLifecycleInvocationV1 {
        common,
        profile: limits(),
        layer_registry_writable: Some(true),
        associations_writable: None,
        required_reads: &[],
    }
}
#[test]
fn native_lifecycle_requires_fresh_registry_and_full_identity_grants_without_reserving_incarnations()
 {
    let (mut o, mut r) = ready(16, 8);
    o.configure_settings_layers(limits(), &mut r).unwrap();
    let c = context();
    let grants = [created()];
    let common = facts(&c, &grants);
    let mut f = gate(&common);
    let changes = proposal();
    let reads = reads();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = NativeSettingsLifecycleRequestV1 {
        expected,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let before = o.generation();
    o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.settings_layer_basis(&mut r).unwrap(), expected);
    assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
    f.layer_registry_writable = Some(false);
    o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap(), expected);
}

#[test]
fn native_optional_created_results_are_funded_before_admission_and_published_with_the_root() {
    let (mut o, _) = created_results::ready();
    let mut r = Recorder::try_with_capacity(256)
        .unwrap()
        .with_native_timing(
            vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
        );
    let c = context();
    let grants = [created()];
    let common = facts(&c, &grants);
    let f = gate(&common);
    let changes = proposal();
    let reads = reads();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = NativeSettingsLifecycleRequestV1 {
        expected,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let mut result = TrustedCreatedLayerResultAdmissionV1 {
        context: &c,
        max_result_bytes: 0,
        references_allowed: Some(true),
        outcome_allowed: Some(true),
        context_current: Some(true),
    };
    assert_eq!(
        o.submit_native_lifecycle_with_created_results_v1(&job, &q, &c, &f, &result, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(o.settings_layer_basis(&mut r).unwrap(), expected);
    result.max_result_bytes = 65536;
    o.submit_native_lifecycle_with_created_results_v1(&job, &q, &c, &f, &result, &mut r)
        .unwrap();
    let release = TrustedCreatedLayerResultReleaseV1 {
        context: &c,
        outcome_readable: Some(true),
        created_result_readable: Some(true),
        created_result_context_current: Some(true),
        historical_layers_readable: Some(true),
    };
    assert!(o.created_layer_results(&job, &release, &mut r).is_err());
    o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
        .unwrap();
    let references = o.created_layer_results(&job, &release, &mut r).unwrap();
    assert_eq!(references.created.len(), 1);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
    let generation = o.generation();
    assert_eq!(
        o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    use vellum_engine::instrumentation::Boundary;
    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
    for boundary in [
        Boundary::CreatedResultsAdmission,
        Boundary::CreatedResultsStaging,
        Boundary::CreatedResultsPublication,
        Boundary::CreatedResultsRelease,
    ] {
        let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
        assert!(!matching.is_empty());
        assert!(matching.iter().all(|e| e.service_time.is_some()));
    }
}

pub(super) fn decode<'h>(
    job: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    execution: &str,
    expected: SettingsLayerBasisV1,
    r: &mut Recorder,
) -> NormalizedNativeLifecycleArgumentsV1<'h> {
    use NativeLifecycleEnvelopeValueV1 as V;
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    let ident = |name, n| {
        [
            ("authority", D::Text("engine")),
            ("id", D::Text(name)),
            ("incarnation", D::Number(n)),
        ]
    };
    let new = ident("extra", 2.0);
    let a = ident("A", 1.0);
    let b = ident("B", 1.0);
    let host = ident("host", 1.0);
    let left = ident("left", 1.0);
    let right = ident("right", 1.0);
    let set = [("kind", D::Text("set")), ("value", D::Number(3.0))];
    let values = [("unit", D::Object(&set))];
    let create = [
        ("layer", D::Object(&new)),
        ("kind", D::Text("workspace")),
        ("values", D::Object(&values)),
    ];
    let creates = [D::Object(&create)];
    let la = [D::Object(&host), D::Object(&left)];
    let lb = [D::Object(&host), D::Object(&right)];
    let ta = [("target", D::Object(&a)), ("layers", D::Array(&la))];
    let tb = [("target", D::Object(&b)), ("layers", D::Array(&lb))];
    let targets = [D::Object(&ta), D::Object(&tb)];
    let expected = [
        (
            "generation",
            D::Number(expected.configuration_generation as f64),
        ),
        (
            "layerIncarnation",
            D::Number(expected.layer_incarnation as f64),
        ),
    ];
    let rows = [
        ("contract", V::Data(D::Text("engine-settings-layers/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(job)),
        ("profile", V::Data(D::Text(execution))),
        ("expected", V::Data(D::Object(&expected))),
        ("create", V::Data(D::Array(&creates))),
        ("remove", V::Data(D::Array(&[]))),
        ("targets", V::Data(D::Array(&targets))),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_native_lifecycle_envelope_v1(
        &V::Object(&rows),
        profile(),
        limits(),
        CaptureLimitsV1 {
            max_bytes: 65536,
            max_nodes: 512,
            max_depth: 32,
        },
        128,
        65536,
        r,
    )
    .unwrap()
}
#[test]
fn native_lifecycle_envelope_enters_sealed_service_and_replay_never_allocates_again() {
    let (mut o, mut r) = ready(16, 8);
    o.configure_settings_layers(limits(), &mut r).unwrap();
    let c = context();
    let grants = [created()];
    let common = facts(&c, &grants);
    let f = gate(&common);
    let reads = reads();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let job = o.issue_invocation(&mut r).unwrap();
    let q = decode(&job, &reads, &c.profile, expected, &mut r);
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    o.submit_normalized_native_lifecycle_v1(&q, &c, &f, &mut r)
        .unwrap();
    o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
    let generation = o.generation();
    assert_eq!(
        o.submit_normalized_native_lifecycle_v1(&q, &c, &f, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
}

#[test]
fn native_lifecycle_requires_exact_affected_identities_and_association_grants_without_touching_source()
 {
    let (mut o, mut r) = ready(16, 8);
    o.configure_settings_layers(limits(), &mut r).unwrap();
    let c = context();
    let mut changes = proposal();
    changes.remove.push(SettingLayerRemoveV1 {
        identity: id("left"),
        expected_generation: 1,
    });
    changes.targets[0].layers = vec![id("host"), created()];
    let source = o.create(b"A", &mut r).unwrap();
    let reads = reads();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let q = NativeSettingsLifecycleRequestV1 {
        expected,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let wrong = SettingIdentityV1 {
        incarnation: 2,
        ..id("left")
    };
    let bad_grants = [created(), wrong];
    let bad_common = facts(&c, &bad_grants);
    let mut bad = gate(&bad_common);
    bad.associations_writable = Some(true);
    assert_eq!(
        o.submit_native_settings_lifecycle_v1(&job, &q, &c, &bad, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(o.settings_layer_basis(&mut r).unwrap(), expected);
    let grants = [created(), id("left")];
    let common = facts(&c, &grants);
    let mut f = gate(&common);
    assert_eq!(
        o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r),
        Err(InvocationError::Unavailable)
    );
    f.associations_writable = Some(true);
    o.submit_native_settings_lifecycle_v1(&job, &q, &c, &f, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_settings_layers(&job, expected, &changes, &reads, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
        .unwrap();
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
    assert_eq!(read(&o, "A", &mut r), 3.0);
    assert_eq!(read(&o, "B", &mut r), 1.0);
}

#[test]
fn native_lifecycle_required_reads_stop_and_timed_diagnostics_are_behavior_independent() {
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        for mode in 0..3 {
            let (mut o, _) = ready(16, 8);
            let mut r = if capacity == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(capacity).unwrap()
            }
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            o.configure_settings_layers(limits(), &mut r).unwrap();
            let c = context();
            let grants = [created()];
            let common = facts(&c, &grants);
            let mut f = gate(&common);
            let source = o.create(b"A", &mut r).unwrap();
            let required = [ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(source),
            ))];
            f.required_reads = &required;
            let expected = o.settings_layer_basis(&mut r).unwrap();
            let job = o.issue_invocation(&mut r).unwrap();
            let reads = reads();
            assert_eq!(
                normalize_native_lifecycle_envelope_v1(
                    &NativeLifecycleEnvelopeValueV1::Data(D::Null),
                    profile(),
                    limits(),
                    CaptureLimitsV1 {
                        max_bytes: 65536,
                        max_nodes: 512,
                        max_depth: 32
                    },
                    128,
                    65536,
                    &mut r
                )
                .unwrap_err(),
                InvocationError::MalformedIntent
            );
            let q = decode(&job, &reads, &c.profile, expected, &mut r);
            f.profile.max_work -= 1;
            assert_eq!(
                o.submit_normalized_native_lifecycle_v1(&q, &c, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            f.profile = limits();
            o.submit_normalized_native_lifecycle_v1(&q, &c, &f, &mut r)
                .unwrap();
            if mode == 1 {
                f.required_reads = &[];
            }
            if mode == 2 {
                o.request_invocation_stop(
                    &job,
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
                f.layer_registry_writable = Some(false);
            }
            o.run_native_settings_lifecycle_v1(&job, &c, &f, &mut r)
                .unwrap();
            assert_eq!(
                o.settings_layer_basis(&mut r).unwrap().layer_incarnation,
                expected.layer_incarnation + u64::from(mode == 0)
            );
            if mode == 2 {
                assert!(matches!(
                    o.invocation_outcome(&job, &mut r),
                    Ok(InvocationOutcome::StoppedBeforePublication)
                ));
            }
            let generation = o.generation();
            assert_eq!(
                o.submit_normalized_native_lifecycle_v1(&q, &c, &f, &mut r),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
            assert!(!format!("{q:?}").contains("extra"));
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == 0 {
                assert!(records.is_empty());
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
            if capacity == 128 {
                for (boundary, expected) in [
                    (
                        Boundary::NativeLifecycleArgumentNormalization,
                        vec![Disposition::Refused, Disposition::Completed],
                    ),
                    (
                        Boundary::NativeLifecycleDecodedAdmission,
                        vec![
                            Disposition::Refused,
                            Disposition::Completed,
                            Disposition::Completed,
                        ],
                    ),
                ] {
                    let selected: Vec<_> =
                        records.iter().filter(|v| v.boundary == boundary).collect();
                    assert_eq!(
                        selected.iter().map(|v| v.disposition).collect::<Vec<_>>(),
                        expected
                    );
                    assert!(selected.iter().all(|v| v.service_time.is_some()));
                }
            }
        }
    }
}
