use super::super::tests::{eligible, profile};
use super::*;

#[test]
fn acquisition_validation_capture_never_controls_health_or_issuance() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;

    for capacity in [None, Some(64), Some(1)] {
        for malformed in [false, true] {
            let (mut o, _, s, original) = fixture();
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            o.acquisition_result_fault = malformed.then_some(F::Context);
            let result = o.capture_retained_annotation_context(
                &original,
                AnnotationProjectionClassesV1::Annotations,
                &s,
                &eligible(&original, &s),
                Some(true),
                &mut r,
            );
            assert_eq!(result.is_none(), malformed);
            assert_eq!(o.is_faulted(), malformed);
            assert_eq!(o.read_result_invariant().is_some(), malformed);
            if capacity == Some(64) {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                let checks: Vec<_> = events
                    .iter()
                    .filter(|e| e.boundary == Boundary::ReadAcquisitionResultValidation)
                    .collect();
                assert_eq!(checks.len(), 1);
                assert_eq!(
                    checks[0].disposition,
                    if malformed {
                        Disposition::Unknown
                    } else {
                        Disposition::Completed
                    }
                );
                assert!(
                    events
                        .iter()
                        .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
                );
            } else if capacity == Some(1) {
                assert!(r.dropped() > 0);
            } else {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}

#[test]
fn malformed_acquisition_refuses_before_registry_or_preparation_commit() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
    for fault in [F::Context, F::Classes, F::RetainedParent] {
        let (mut o, mut r, s, original) = fixture();
        let before = o.annotation_context_usage();

        let reserved = o.retention_usage().reserved;
        let generation = o.generation();
        o.acquisition_result_fault = Some(fault);
        assert!(
            o.capture_retained_annotation_context(
                &original,
                AnnotationProjectionClassesV1::Annotations,
                &s,
                &eligible(&original, &s),
                Some(true),
                &mut r
            )
            .is_none(),
            "{fault:?}"
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeReadAcquisitionResultInvariant)
        );
        assert_eq!(o.annotation_context_usage(), before);

        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.generation(), generation);
        assert!(o.stop_annotation_context(
            &original,
            &crate::coordinator::annotation::projection::tests::cleanup(&s),
            &mut r,
        ));
        assert!(o.retire_annotation_context(
            &original,
            &crate::coordinator::annotation::projection::tests::cleanup(&s),
            &mut r
        ));
        assert!(o.is_faulted());
    }
}
use crate::coordinator::annotation::projection::tests::{control, current, facts, limits, ready};
fn fixture() -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    AnnotationReadContextId,
) {
    fixture_profile(profile())
}
fn fixture_profile(
    p: AnnotationContextLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    AnnotationReadContextId,
) {
    let (mut o, mut r, _, s, g) = ready(limits());
    o.configure_annotation_contexts(p, &control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    (o, r, s, id)
}

#[test]
fn retained_native_request_envelope_refuses_before_loss_and_spends_only_on_success() {
    let (_, _, s, _) = fixture();
    let bound = scope_bytes(&s).unwrap()
        + std::mem::size_of::<AnnotationReadContextId>()
        + std::mem::size_of::<AnnotationProjectionClassesV1>()
        + std::mem::size_of::<Option<bool>>();
    for delta in [-1isize, 0, 1] {
        let mut p = profile();
        p.max_request_bytes = bound.checked_add_signed(delta).unwrap();
        let (mut o, mut r, s, id) = fixture_profile(p);
        let reserved = o.retention_usage().reserved;
        assert_eq!(
            o.capture_retained_annotation_context(
                &id,
                AnnotationProjectionClassesV1::Combined,
                &s,
                &eligible(&id, &s),
                Some(true),
                &mut r
            )
            .is_some(),
            delta >= 0
        );
        assert_eq!(
            o.annotation_context_usage().issued_lifetime,
            if delta < 0 { 1 } else { 2 }
        );
        assert_eq!(o.retention_usage().reserved, reserved);
        if delta < 0 {
            let mut loss = eligible(&id, &s);
            loss.authorized = Some(false);
            assert!(
                o.capture_retained_annotation_context(
                    &id,
                    AnnotationProjectionClassesV1::Combined,
                    &s,
                    &loss,
                    Some(true),
                    &mut r
                )
                .is_none()
            );
            assert_eq!(o.annotation_context_usage().stopped, 0);
        }
    }
}
#[test]
fn native_narrowed_projection_units_exclude_values_and_keep_class_invariants() {
    let (o, _, s, id) = fixture();
    let original = &o.annotation_context_row(&id, &s).unwrap().value;
    for class in [
        AnnotationProjectionClassesV1::Source,
        AnnotationProjectionClassesV1::Annotations,
        AnnotationProjectionClassesV1::Combined,
    ] {
        let copy = original.copy_narrowed(class).unwrap();
        assert!(copy.valid(&s, &o.namespace));
        assert_eq!(copy.bytes(), original.narrowed_bytes(class));
        assert_eq!(copy.source.is_some(), class.includes_source());
        assert_eq!(copy.rows.len(), usize::from(class.includes_annotations()));
        let mut bad = copy;
        bad.classes = if class == AnnotationProjectionClassesV1::Annotations {
            AnnotationProjectionClassesV1::Source
        } else {
            AnnotationProjectionClassesV1::Annotations
        };
        assert!(!bad.valid(&s, &o.namespace));
    }
}
#[test]
fn retained_handoff_fault_preserves_original_registry_funding_and_issuance() {
    let (mut o, mut r, s, id) = fixture();
    let before = o.annotation_context_usage();
    let reserved = o.retention_usage().reserved;
    let generation = o.generation();
    o.annotation_context_install_fault = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        o.capture_retained_annotation_context(
            &id,
            AnnotationProjectionClassesV1::Combined,
            &s,
            &eligible(&id, &s),
            Some(true),
            &mut r,
        )
    }));
    assert!(result.is_err());
    assert!(o.is_faulted());
    assert_eq!(o.annotation_context_usage(), before);
    assert_eq!(o.retention_usage().reserved, reserved);
    assert_eq!(o.generation(), generation);
}
#[test]
fn actual_fault_and_private_projection_drift_cannot_install_a_retained_copy() {
    for corrupt in [false, true] {
        let (mut o, mut r, s, id) = fixture();
        if corrupt {
            let InstalledState::Active(p) = &mut o.annotation_contexts else {
                unreachable!()
            };
            p.rows[id.slot].as_mut().unwrap().value.classes =
                AnnotationProjectionClassesV1::Annotations;
        } else {
            o.faulted = true;
        }
        assert!(
            o.capture_retained_annotation_context(
                &id,
                AnnotationProjectionClassesV1::Annotations,
                &s,
                &eligible(&id, &s),
                Some(true),
                &mut r
            )
            .is_none()
        );
        assert!(o.is_faulted());
        assert_eq!(o.annotation_context_usage().contexts, 1);
        assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
    }
}
