use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};

pub(crate) fn decode<'h>(
    job: &'h InvocationId,
    dependencies: &'h ReadDependencyRequestV3,
    execution: &str,
    basis: f64,
    r: &mut Recorder,
) -> NormalizedScopedSettingsArgumentsV1<'h> {
    use ScopedSettingsEnvelopeValueV1 as V;
    let ch = [
        ("basis", D::Number(basis)),
        ("key", D::Text("unit")),
        ("operation", D::Text("set")),
        ("value", D::Number(2.0)),
    ];
    let changes = [D::Object(&ch)];
    let rows = [
        ("contract", V::Data(D::Text("scoped-semantic-settings/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(job)),
        ("profile", V::Data(D::Text(execution))),
        ("changes", V::Data(D::Array(&changes))),
        ("dependencies", V::Dependencies(dependencies)),
    ];
    normalize_scoped_settings_envelope_v1(
        &V::Object(&rows),
        profile(),
        CaptureLimitsV1 {
            max_bytes: 65536,
            max_nodes: 128,
            max_depth: 16,
        },
        128,
        65536,
        r,
    )
    .unwrap()
}

#[test]
fn passive_scoped_settings_reuses_the_original_basis_and_once_only_job() {
    use ScopedSettingsEnvelopeValueV1 as V;
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let bases = [o
        .capture_settings_write_basis(&layers[0], &c, &f, &mut r)
        .unwrap()];
    let job = o.issue_invocation(&mut r).unwrap();
    let dependencies = reads();
    let change = [
        ("basis", D::Number(0.0)),
        ("key", D::Text("unit")),
        ("operation", D::Text("set")),
        ("value", D::Number(2.0)),
    ];
    let changes = [D::Object(&change)];
    let rows = [
        ("contract", V::Data(D::Text("scoped-semantic-settings/1"))),
        ("operation", V::Data(D::Text("semantic-settings"))),
        ("invocation", V::Invocation(&job)),
        ("profile", V::Data(D::Text(&c.profile))),
        ("changes", V::Data(D::Array(&changes))),
        ("dependencies", V::Dependencies(&dependencies)),
    ];
    let q = normalize_scoped_settings_envelope_v1(
        &V::Object(&rows),
        profile(),
        CaptureLimitsV1 {
            max_bytes: 65536,
            max_nodes: 128,
            max_depth: 16,
        },
        128,
        65536,
        &mut r,
    )
    .unwrap();
    assert!(std::ptr::eq(q.invocation(), &job));
    assert!(std::ptr::eq(q.dependencies(), &dependencies));
    let before = o.generation();
    assert_eq!(
        o.submit_normalized_scoped_settings_v1(&q, &bases, &c, &gateway(&f), &mut r),
        Ok(InvocationState::Pending)
    );
    o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
        .unwrap();
    assert_eq!(read(&o, "A", &mut r), 2.0);
    let published = o.generation();
    assert!(published > before);
    assert_eq!(
        o.submit_normalized_scoped_settings_v1(&q, &bases, &c, &gateway(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), published);
}

#[test]
fn decoded_refusals_preserve_issued_job_and_do_not_poison_bases_before_reference_validation() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let layers = [id("left"), id("right")];
    let mut f = facts(&c, &layers);
    let bases = [
        o.capture_settings_write_basis(&layers[0], &c, &f, &mut r)
            .unwrap(),
        o.capture_settings_write_basis(&layers[1], &c, &f, &mut r)
            .unwrap(),
    ];
    let job = o.issue_invocation(&mut r).unwrap();
    let dependencies = reads();
    let q = decode(&job, &dependencies, &c.profile, 0.0, &mut r);
    let before = o.generation();
    // Recognized loss would close a valid basis, but malformed reference vectors
    // must refuse before observing that loss in either basis.
    f.basis_allowed = Some(false);
    for invalid in [&bases[..0], &bases[..2]] {
        assert!(
            o.submit_normalized_scoped_settings_v1(&q, invalid, &c, &gateway(&f), &mut r)
                .is_err()
        );
    }
    let later = decode(&job, &dependencies, &c.profile, 1.0, &mut r);
    assert!(
        o.submit_normalized_scoped_settings_v1(&later, &bases[..1], &c, &gateway(&f), &mut r)
            .is_err()
    );
    f.basis_allowed = Some(true);
    for basis in &bases {
        assert_eq!(
            o.inspect_settings_write_basis(basis, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
    let wrong = decode(&job, &dependencies, "other", 0.0, &mut r);
    assert_eq!(
        o.submit_normalized_scoped_settings_v1(&wrong, &bases[..1], &c, &gateway(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.activation.max_work -= 1;
    assert_eq!(
        o.submit_normalized_scoped_settings_v1(&q, &bases[..1], &c, &gateway(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.activation = profile();
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.submit_normalized_scoped_settings_v1(&q, &bases[..1], &c, &gateway(&f), &mut r)
        .unwrap();
    // Current permission at service is not replaced by successful normalization.
    f.settings_writable = Some(false);
    o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
        .unwrap();
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(o.generation(), before);
}

#[test]
fn passive_settings_capture_is_optional_bounded_timed_and_behavior_preserving() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        let (mut o, _) = ready(8, 8);
        let mut r = if capacity == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(capacity).unwrap()
        }
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let c = context();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        let bases = [o
            .capture_settings_write_basis(&layers[0], &c, &f, &mut r)
            .unwrap()];
        let job = o.issue_invocation(&mut r).unwrap();
        let dependencies = reads();
        let q = decode(&job, &dependencies, &c.profile, 0.0, &mut r);
        o.submit_normalized_scoped_settings_v1(&q, &bases, &c, &gateway(&f), &mut r)
            .unwrap();
        o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
            .unwrap();
        let before = o.generation();
        assert_eq!(
            o.submit_normalized_scoped_settings_v1(&q, &bases, &c, &gateway(&f), &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), before);
        assert_eq!(read(&o, "A", &mut r), 2.0);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 0 {
            assert!(records.is_empty());
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 128 {
            for (boundary, count) in [
                (Boundary::ScopedSettingsArgumentNormalization, 1),
                (Boundary::ScopedSettingsDecodedAdmission, 2),
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
