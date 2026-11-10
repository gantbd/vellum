use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
fn identity(name: &str) -> [(&str, D<'_>); 3] {
    [
        ("authority", D::Text("engine")),
        ("id", D::Text(name)),
        ("incarnation", D::Number(1.0)),
    ]
}
pub(crate) fn decode<'h>(
    job: &'h InvocationId,
    reads: &'h ReadDependencyRequestV3,
    execution: &str,
    default: f64,
    r: &mut Recorder,
) -> NormalizedScopedDefinitionArgumentsV1<'h> {
    use ScopedDefinitionEnvelopeValueV1 as V;
    let ty = [
        ("kind", D::Text("integer")),
        ("min", D::Number(0.0)),
        ("max", D::Number(8.0)),
    ];
    let scopes = [D::Text("host"), D::Text("workspace")];
    let schema = [
        ("key", D::Text("unit")),
        ("type", D::Object(&ty)),
        ("default", D::Number(default)),
        ("domain", D::Text("source")),
        ("scopes", D::Array(&scopes)),
        ("composition", D::Text("replace")),
        ("sensitivity", D::Boolean(true)),
    ];
    let schemas = [D::Object(&schema)];
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
    let declaration = [
        ("schemas", D::Array(&schemas)),
        ("restrictions", D::Object(&[])),
        ("targets", D::Array(&targets)),
    ];
    let rows = [
        (
            "contract",
            V::Data(D::Text("scoped-engine-settings-definition/1")),
        ),
        ("operation", V::Data(D::Text("supported-registry"))),
        ("invocation", V::Invocation(job)),
        ("profile", V::Data(D::Text(execution))),
        ("definition", V::Data(D::Object(&declaration))),
        ("dependencies", V::Dependencies(reads)),
    ];
    normalize_scoped_definition_envelope_v1(
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
fn passive_definition_uses_the_original_basis_and_existing_publication_replay() {
    let (mut o, mut r) = ready();
    let source = o.create(b"A", &mut r).unwrap();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let q = decode(&job, &reads, &c.profile, 1.0, &mut r);
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let before = o.settings_basis(&mut r).unwrap();
    o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r)
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
    o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r)
        .unwrap();
    let published = o.settings_basis(&mut r).unwrap();
    assert_eq!(published.schema_generation, before.schema_generation + 1);
    assert_eq!(
        published.restriction_generation,
        before.restriction_generation
    );
    let generation = o.generation();
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
    assert_eq!(
        o.submit_scoped_settings_definition(&job, &q.request(&b), &c, &gate(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
}

#[test]
fn decoded_definition_validates_native_proposal_before_loss_and_rechecks_service_grants() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let mut f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let q = decode(&job, &reads, &c.profile, 1.0, &mut r);
    let wrong = decode(&job, &reads, "other", 1.0, &mut r);
    let before = o.generation();
    let basis_before = o.settings_basis(&mut r).unwrap();
    assert_eq!(
        o.submit_normalized_scoped_definition_v1(&wrong, &b, &c, &gate(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.definition.max_work -= 1;
    assert_eq!(
        o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.definition = limits();
    let mut denied = gate(&f);
    denied.schema_writable = Some(false);
    assert!(
        o.submit_normalized_scoped_definition_v1(&q, &b, &c, &denied, &mut r)
            .is_err()
    );
    let invalid = decode(&job, &reads, &c.profile, 99.0, &mut r);
    f.basis_allowed = Some(false);
    assert!(
        o.submit_normalized_scoped_definition_v1(&invalid, &b, &c, &gate(&f), &mut r)
            .is_err()
    );
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r)
        .unwrap();
    let mut denied = gate(&f);
    denied.schema_writable = Some(false);
    o.run_scoped_settings_definition(&job, &c, &denied, &mut r)
        .unwrap();
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_basis(&mut r).unwrap(), basis_before);
}

#[test]
fn definition_diagnostics_cover_refusal_and_success_without_changing_publication_or_replay() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        let (mut o, _) = ready();
        let mut r = if capacity == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(capacity).unwrap()
        }
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let c = context();
        let common = facts(&c, &[]);
        let mut f = basis(&common);
        let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
        let job = o.issue_invocation(&mut r).unwrap();
        let reads = reads();
        assert_eq!(
            normalize_scoped_definition_envelope_v1(
                &ScopedDefinitionEnvelopeValueV1::Data(D::Null),
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
        let q = decode(&job, &reads, &c.profile, 1.0, &mut r);
        f.definition.max_work -= 1;
        assert_eq!(
            o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r),
            Err(InvocationError::Unavailable)
        );
        f.definition = limits();
        o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r)
            .unwrap();
        o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r)
            .unwrap();
        let generation = o.generation();
        assert_eq!(
            o.submit_normalized_scoped_definition_v1(&q, &b, &c, &gate(&f), &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.settings_basis(&mut r).unwrap().schema_generation, 2);
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
                    Boundary::ScopedDefinitionArgumentNormalization,
                    vec![Disposition::Refused, Disposition::Completed],
                ),
                (
                    Boundary::ScopedDefinitionDecodedAdmission,
                    vec![
                        Disposition::Refused,
                        Disposition::Completed,
                        Disposition::Completed,
                    ],
                ),
            ] {
                let selected: Vec<_> = records.iter().filter(|v| v.boundary == boundary).collect();
                assert_eq!(
                    selected.iter().map(|v| v.disposition).collect::<Vec<_>>(),
                    expected
                );
                assert!(selected.iter().all(|v| v.service_time.is_some()));
            }
        }
    }
}
