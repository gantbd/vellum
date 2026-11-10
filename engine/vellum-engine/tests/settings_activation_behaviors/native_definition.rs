use super::scoped_basis::{context, facts, ready};
use super::*;
pub(super) fn limits() -> SettingsDefinitionLimitsV1 {
    SettingsDefinitionLimitsV1 {
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
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
) -> TrustedNativeSettingsDefinitionInvocationV1<'a> {
    TrustedNativeSettingsDefinitionInvocationV1 {
        common,
        profile: limits(),
        schema_writable: Some(true),
        restrictions_writable: None,
        associations_writable: None,
        required_reads: &[],
    }
}
#[test]
fn native_definition_uses_common_context_ledger_and_fresh_changed_facet_grants() {
    let (mut o, mut r) = ready(16, 8);
    o.configure_settings_definitions(limits(), &mut r).unwrap();
    let c = context();
    let mut common = facts(&c, &[]);
    common.basis_allowed = None;
    common.writable_layers = None;
    let mut declaration = definition::declaration();
    declaration.schemas[0].sensitive = true;
    let reads = reads();
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    let request = NativeSettingsDefinitionRequestV1 {
        expected,
        declaration: &declaration,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let before = o.generation();
    o.submit_native_settings_definition_v1(&job, &request, &c, &gate(&common), &mut r)
        .unwrap();
    assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
    let mut denied = gate(&common);
    denied.schema_writable = Some(false);
    assert_eq!(
        o.run_native_settings_definition_v1(&job, &c, &denied, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_basis(&mut r).unwrap().interpretation(), expected);
}

pub(super) fn decode<'h>(
    job: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    execution: &str,
    expected: SettingsInterpretationEpochsV1,
    r: &mut Recorder,
) -> NormalizedNativeDefinitionArgumentsV1<'h> {
    use NativeDefinitionEnvelopeValueV1 as V;
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    let ty = [
        ("kind", D::Text("integer")),
        ("min", D::Number(0.0)),
        ("max", D::Number(8.0)),
    ];
    let scopes = [D::Text("host"), D::Text("workspace")];
    let schema = [
        ("key", D::Text("unit")),
        ("type", D::Object(&ty)),
        ("default", D::Number(1.0)),
        ("domain", D::Text("source")),
        ("scopes", D::Array(&scopes)),
        ("composition", D::Text("replace")),
        ("sensitivity", D::Boolean(true)),
    ];
    let schemas = [D::Object(&schema)];
    let identity = |name| {
        [
            ("authority", D::Text("engine")),
            ("id", D::Text(name)),
            ("incarnation", D::Number(1.0)),
        ]
    };
    let a = identity("A");
    let b = identity("B");
    let host = identity("host");
    let left = identity("left");
    let right = identity("right");
    let la = [D::Object(&host), D::Object(&left)];
    let lb = [D::Object(&host), D::Object(&right)];
    let ta = [("target", D::Object(&a)), ("layers", D::Array(&la))];
    let tb = [("target", D::Object(&b)), ("layers", D::Array(&lb))];
    let targets = [D::Object(&ta), D::Object(&tb)];
    let definition = [
        ("schemas", D::Array(&schemas)),
        ("restrictions", D::Object(&[])),
        ("targets", D::Array(&targets)),
    ];
    let expected = [
        ("schemaGeneration", D::Number(expected.schema as f64)),
        (
            "restrictionGeneration",
            D::Number(expected.restriction as f64),
        ),
        (
            "associationGeneration",
            D::Number(expected.association as f64),
        ),
    ];
    let rows = [
        ("contract", V::Data(D::Text("engine-settings-definition/1"))),
        ("operation", V::Data(D::Text("supported-registry"))),
        ("invocation", V::Invocation(job)),
        ("profile", V::Data(D::Text(execution))),
        ("expected", V::Data(D::Object(&expected))),
        ("definition", V::Data(D::Object(&definition))),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_native_definition_envelope_v1(
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
fn native_definition_envelope_reuses_declaration_codec_and_cannot_enter_values_or_raw_job_ports() {
    let (mut o, mut r) = ready(16, 8);
    o.configure_settings_definitions(limits(), &mut r).unwrap();
    let c = context();
    let common = facts(&c, &[]);
    let reads = reads();
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    let job = o.issue_invocation(&mut r).unwrap();
    let q = decode(&job, &reads, &c.profile, expected, &mut r);
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    o.submit_normalized_native_definition_v1(&q, &c, &gate(&common), &mut r)
        .unwrap();
    let values = TrustedNativeSettingsInvocationV1 {
        common: &common,
        required_reads: &[],
    };
    assert_eq!(
        o.run_native_settings_change_v1(&job, &c, &values, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.submit_settings_definition(&job, expected, q.declaration(), &reads, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_native_settings_definition_v1(&job, &c, &gate(&common), &mut r)
        .unwrap();
    assert_eq!(
        o.settings_basis(&mut r).unwrap().schema_generation,
        expected.schema + 1
    );
    let generation = o.generation();
    assert_eq!(
        o.submit_normalized_native_definition_v1(&q, &c, &gate(&common), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
}

#[test]
fn each_changed_native_definition_facet_requires_its_own_grant_and_preserves_source_progress() {
    for facet in 0..3 {
        let (mut o, mut r) = ready(16, 8);
        o.configure_settings_definitions(limits(), &mut r).unwrap();
        let c = context();
        let common = facts(&c, &[]);
        let mut f = gate(&common);
        f.schema_writable = None;
        f.restrictions_writable = None;
        f.associations_writable = None;
        let source = o.create(b"A", &mut r).unwrap();
        let mut declaration = definition::declaration();
        match facet {
            0 => declaration.schemas[0].sensitive = true,
            1 => declaration.restrictions.push((
                "unit".into(),
                vec![SettingValueV1::Number(1.0), SettingValueV1::Number(2.0)],
            )),
            _ => declaration.targets[0].layers = vec![id("host"), id("right")],
        }
        let expected = o.settings_basis(&mut r).unwrap().interpretation();
        let reads = reads();
        let q = NativeSettingsDefinitionRequestV1 {
            expected,
            declaration: &declaration,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        let before = o.generation();
        assert_eq!(
            o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&job, &mut r),
            Ok(InvocationState::Issued)
        );
        assert_eq!(o.generation(), before);
        match facet {
            0 => f.schema_writable = Some(true),
            1 => f.restrictions_writable = Some(true),
            _ => f.associations_writable = Some(true),
        }
        o.submit_native_settings_definition_v1(&job, &q, &c, &f, &mut r)
            .unwrap();
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
        o.run_native_settings_definition_v1(&job, &c, &f, &mut r)
            .unwrap();
        assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
        let current = o.settings_basis(&mut r).unwrap().interpretation();
        assert_eq!(current.schema, expected.schema + u64::from(facet == 0));
        assert_eq!(
            current.restriction,
            expected.restriction + u64::from(facet == 1)
        );
        assert_eq!(
            current.association,
            expected.association + u64::from(facet == 2)
        );
    }
}

#[test]
fn native_definition_diagnostics_and_required_reads_stop_and_replay_share_original_ownership() {
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
            o.configure_settings_definitions(limits(), &mut r).unwrap();
            let c = context();
            let common = facts(&c, &[]);
            let mut f = gate(&common);
            let source = o.create(b"A", &mut r).unwrap();
            let required = [ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(source),
            ))];
            f.required_reads = &required;
            let job = o.issue_invocation(&mut r).unwrap();
            let reads = reads();
            let expected = o.settings_basis(&mut r).unwrap().interpretation();
            assert_eq!(
                normalize_native_definition_envelope_v1(
                    &NativeDefinitionEnvelopeValueV1::Data(D::Null),
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
                o.submit_normalized_native_definition_v1(&q, &c, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            f.profile = limits();
            o.submit_normalized_native_definition_v1(&q, &c, &f, &mut r)
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
                f.schema_writable = Some(false);
            }
            o.run_native_settings_definition_v1(&job, &c, &f, &mut r)
                .unwrap();
            assert_eq!(
                o.settings_basis(&mut r).unwrap().schema_generation,
                expected.schema + u64::from(mode == 0)
            );
            if mode == 2 {
                assert!(matches!(
                    o.invocation_outcome(&job, &mut r),
                    Ok(InvocationOutcome::StoppedBeforePublication)
                ));
            }
            let generation = o.generation();
            assert_eq!(
                o.submit_normalized_native_definition_v1(&q, &c, &f, &mut r),
                Ok(InvocationState::Finished)
            );
            assert_eq!(o.generation(), generation);
            assert!(!format!("{q:?}").contains("unit"));
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
                        Boundary::NativeDefinitionArgumentNormalization,
                        vec![Disposition::Refused, Disposition::Completed],
                    ),
                    (
                        Boundary::NativeDefinitionDecodedAdmission,
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
                if mode != 2 {
                    assert!(records.iter().any(|v| v.boundary
                        == Boundary::NativeSettingsPublication
                        && v.disposition
                            == if mode == 0 {
                                Disposition::Completed
                            } else {
                                Disposition::Refused
                            }));
                }
            }
        }
    }
}
