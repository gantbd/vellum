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
    dependencies: &'h ReadDependencyRequestV3,
    execution: &str,
    r: &mut Recorder,
) -> NormalizedScopedLifecycleArgumentsV1<'h> {
    decode_value(job, dependencies, execution, 3.0, r)
}
fn decode_value<'h>(
    job: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    execution: &str,
    value: f64,
    r: &mut Recorder,
) -> NormalizedScopedLifecycleArgumentsV1<'h> {
    use ScopedLifecycleEnvelopeValueV1 as V;
    let set = [("kind", D::Text("set")), ("value", D::Number(value))];
    let values = [("unit", D::Object(&set))];
    let create = [
        ("id", D::Text("new")),
        ("kind", D::Text("workspace")),
        ("values", D::Object(&values)),
    ];
    let creates = [D::Object(&create)];
    let left = identity("left");
    let host = identity("host");
    let right = identity("right");
    let a = identity("A");
    let b = identity("B");
    let removes = [D::Object(&left)];
    let created = [("create", D::Number(0.0))];
    let a_layers = [D::Object(&host), D::Object(&created)];
    let b_layers = [D::Object(&host), D::Object(&right)];
    let ta = [("target", D::Object(&a)), ("layers", D::Array(&a_layers))];
    let tb = [("target", D::Object(&b)), ("layers", D::Array(&b_layers))];
    let targets = [D::Object(&ta), D::Object(&tb)];
    let rows = [
        (
            "contract",
            V::Data(D::Text("scoped-engine-settings-layers/1")),
        ),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(job)),
        ("profile", V::Data(D::Text(execution))),
        ("create", V::Data(D::Array(&creates))),
        ("remove", V::Data(D::Array(&removes))),
        ("targets", V::Data(D::Array(&targets))),
        ("dependencies", V::Dependencies(dependencies)),
    ];
    normalize_scoped_lifecycle_envelope_v1(
        &V::Object(&rows),
        profile(),
        limits(),
        CaptureLimitsV1 {
            max_bytes: 65536,
            max_nodes: 256,
            max_depth: 16,
        },
        128,
        65536,
        r,
    )
    .unwrap()
}
#[test]
fn passive_lifecycle_retains_original_condition_and_reuses_once_only_allocation() {
    let (mut o, mut r) = ready();
    let source = o.create(b"A", &mut r).unwrap();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let basis = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let q = decode(&job, &reads, &c.profile, &mut r);
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &reads));
    let grants = [new_id(), id("left")];
    o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &gate(&f, &grants), &mut r)
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
    o.run_scoped_settings_lifecycle(&job, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
    assert_eq!(read(&o, "A", &mut r), 3.0);
    let issued = o.settings_layer_basis(&mut r).unwrap();
    assert_eq!(issued.layer_incarnation, 2);
    let generation = o.generation();
    assert_eq!(
        o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &gate(&f, &grants), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.settings_layer_basis(&mut r).unwrap(), issued);
    assert_eq!(o.generation(), generation);
}

#[test]
fn decoded_lifecycle_cannot_grant_writes_or_observe_basis_loss_before_native_validation() {
    let (mut o, mut r) = ready();
    let c = context();
    let mut common = facts(&c, &[]);
    let mut f = basis(&common);
    let basis = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let job = o.issue_invocation(&mut r).unwrap();
    let reads = reads();
    let grants = [new_id(), id("left")];
    let q = decode(&job, &reads, &c.profile, &mut r);
    let wrong = decode(&job, &reads, "other", &mut r);
    let before = o.generation();
    assert_eq!(
        o.submit_normalized_scoped_lifecycle_v1(&wrong, &basis, &c, &gate(&f, &grants), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.lifecycle.max_work -= 1;
    assert_eq!(
        o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &gate(&f, &grants), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.lifecycle = limits();
    let denied = gate(&f, &[]);
    assert!(
        o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &denied, &mut r)
            .is_err()
    );
    // Structurally valid but schema-invalid created values must not consume an
    // otherwise valid basis even if a current packet carries recognized loss.
    let invalid = decode_value(&job, &reads, &c.profile, 99.0, &mut r);
    f.basis_allowed = Some(false);
    assert!(
        o.submit_normalized_scoped_lifecycle_v1(&invalid, &basis, &c, &gate(&f, &grants), &mut r)
            .is_err()
    );
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&basis, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    common.authorized = Some(false);
    let f = basis_facts(&common);
    o.run_scoped_settings_lifecycle(&job, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
    assert_eq!(read(&o, "A", &mut r), 1.0);
}
// Avoid shadowing the original retained `basis` role in the scenario above.
fn basis_facts<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsLifecycleBasisFactsV1<'a> {
    super::basis(common)
}

#[test]
fn lifecycle_normalization_and_admission_signals_are_optional_timed_and_content_free() {
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
        let f = basis(&common);
        let basis = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let job = o.issue_invocation(&mut r).unwrap();
        let reads = reads();
        let q = decode(&job, &reads, &c.profile, &mut r);
        let grants = [new_id(), id("left")];
        o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &gate(&f, &grants), &mut r)
            .unwrap();
        o.run_scoped_settings_lifecycle(&job, &c, &gate(&f, &grants), &mut r)
            .unwrap();
        let generation = o.generation();
        assert_eq!(
            o.submit_normalized_scoped_lifecycle_v1(&q, &basis, &c, &gate(&f, &grants), &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
        assert_eq!(read(&o, "A", &mut r), 3.0);
        assert!(!format!("{q:?}").contains("new"));
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 0 {
            assert!(records.is_empty());
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 128 {
            for (boundary, count) in [
                (Boundary::ScopedLifecycleArgumentNormalization, 1),
                (Boundary::ScopedLifecycleDecodedAdmission, 2),
            ] {
                let selected: Vec<_> = records.iter().filter(|v| v.boundary == boundary).collect();
                assert_eq!(selected.len(), count);
                assert!(
                    selected.iter().all(
                        |v| v.disposition == Disposition::Completed && v.service_time.is_some()
                    )
                );
            }
        }
    }
}
