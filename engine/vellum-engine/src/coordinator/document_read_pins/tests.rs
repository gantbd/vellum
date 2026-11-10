use super::*;
use crate::coordinator::source_query::result_contract::tests::{cleanup, owner};
use crate::instrumentation::NativeTimingProfileV1;
#[test]
fn read_contexts_and_holds_are_precise_and_stop_does_not_retire_them() {
    let mut f = owner();
    let unrelated = f.o.create(b"unrelated", &mut f.r).unwrap();
    let capture = TrustedAnnotationCaptureFactsV1 {
        scope: &f.s,
        grants: &[],
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        visibility_complete: Some(true),
        projection_order_attested: Some(true),
        resources_ready: Some(true),
    };
    let annotation_cut =
        f.o.prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &f.s,
            &capture,
            &mut f.r,
        )
        .unwrap();
    let version_cut =
        f.o.prepare_version_set(
            &[VersionSetParticipantV1 {
                scope: &f.s,
                data: AnnotationProjectionClassesV1::Combined,
            }],
            &[capture],
            &mut f.r,
        )
        .unwrap();
    assert_eq!(
        f.o.document_read_pins_v1(&unrelated.document, &mut f.r)
            .unwrap()
            .total(),
        Ok(0)
    );
    let initial = f.o.document_read_pins_v1(&f.s.document, &mut f.r).unwrap();
    assert_eq!(
        (
            initial.source_contexts,
            initial.annotation_contexts,
            initial.version_set_contexts
        ),
        (1, 1, 1)
    );
    assert_eq!(
        (
            initial.annotation_preparations,
            initial.version_set_preparations
        ),
        (1, 1)
    );
    let hold =
        f.o.hold_source_read_context(
            &f.source,
            &f.s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut f.r,
        )
        .unwrap();
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r)
            .unwrap()
            .source_holds,
        1
    );
    assert!(f.o.stop_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
    assert!(!f.o.retire_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r)
            .unwrap()
            .source_holds,
        1
    );
    assert!(f.o.release_source_read_hold(&hold, &cleanup(&f.s), &mut f.r));
    assert!(f.o.retire_source_read_context(&f.source, &cleanup(&f.s), &mut f.r));
    assert!(f.o.stop_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
    assert!(f.o.retire_annotation_context(&f.combined, &cleanup(&f.s), &mut f.r));
    let controls = [cleanup(&f.s)];
    assert!(f.o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&f.set.context),
        &controls,
        &mut f.r
    ));
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r)
            .unwrap()
            .version_set_contexts,
        1
    );
    assert!(f.o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&f.set.context),
        &controls,
        &mut f.r
    ));
    let final_pins = f.o.document_read_pins_v1(&f.s.document, &mut f.r).unwrap();
    assert_eq!(
        (
            final_pins.source_contexts,
            final_pins.source_holds,
            final_pins.annotation_contexts,
            final_pins.version_set_contexts
        ),
        (0, 0, 0, 0)
    );
    assert_eq!(
        (
            final_pins.annotation_preparations,
            final_pins.version_set_preparations
        ),
        (1, 1)
    );
    assert!(f.o.discard_prepared_annotation_projection(&annotation_cut, &cleanup(&f.s), &mut f.r));
    assert!(f.o.discard_prepared_version_set(&version_cut, &controls, &mut f.r));
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r)
            .unwrap()
            .total(),
        Ok(0)
    );
}
#[test]
fn read_pin_diagnostics_preserve_funding_and_counts_with_capture_off_on_and_saturated() {
    for capacity in [0, 1, 16] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            let mut f = owner();
            let expected = f.o.document_read_pins_v1(&f.s.document, &mut f.r).unwrap();
            let debt = f.o.retention_usage();
            let generation = f.o.generation();
            f.r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            for _ in 0..3 {
                assert_eq!(
                    f.o.document_read_pins_v1(&f.s.document, &mut f.r).unwrap(),
                    expected
                );
            }
            assert_eq!(f.o.retention_usage(), debt);
            assert_eq!(f.o.generation(), generation);
            let records: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            assert!(records.len() <= capacity);
            assert!(
                records
                    .iter()
                    .all(|v| v.boundary == Boundary::DocumentReadPins
                        && v.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
            );
            if capacity == 16 {
                assert_eq!(records.len(), 3);
            }
        }
    }
}
#[test]
fn foreign_fault_and_overflow_do_not_become_zero_read_pins() {
    let mut f = owner();
    let foreign = owner();
    assert_eq!(
        f.o.document_read_pins_v1(&foreign.s.document, &mut f.r),
        Err(CoordinatorError::Unavailable)
    );
    f.o.faulted = true;
    assert_eq!(
        f.o.document_read_pins_v1(&f.s.document, &mut f.r),
        Err(CoordinatorError::Faulted)
    );
    assert_eq!(
        DocumentReadPinsV1 {
            source_contexts: usize::MAX,
            history_groups: 1,
            ..Default::default()
        }
        .total(),
        Err(CoordinatorError::Faulted)
    );
}
