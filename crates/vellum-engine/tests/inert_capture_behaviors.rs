use vellum_engine::capture::{
    CaptureErrorV1, CaptureLimitsV1, CapturedValueV1, InertCaptureV1, InertValueV1,
};
use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};

fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 1_048_576,
        max_nodes: 8192,
        max_depth: 8192,
    }
}
fn copy(input: &InertValueV1<'_>) -> InertCaptureV1 {
    InertCaptureV1::copy(input, limits(), &mut Recorder::disabled()).unwrap()
}

#[test]
fn given_nested_inert_arguments_when_captured_then_output_owns_plain_values() {
    let captured = {
        let text = String::from("prose 🦀");
        let entries = [("body", InertValueV1::Text(&text))];
        copy(&InertValueV1::Object(&entries))
    };
    assert_eq!(captured.usage().nodes, 2);
    assert_eq!(captured.usage().depth, 2);
    let CapturedValueV1::Object(entries) = captured.root() else {
        panic!("object")
    };
    assert_eq!(entries[0].0, "body");
    let Some(CapturedValueV1::Text(body)) = captured.node(entries[0].1) else {
        panic!("body")
    };
    assert_eq!(body, "prose 🦀");
}

#[test]
fn given_the_closed_scalar_vocabulary_when_copied_then_values_are_not_coerced() {
    let values = [
        InertValueV1::Null,
        InertValueV1::Boolean(false),
        InertValueV1::Number(-0.0),
        InertValueV1::Number(f64::MAX),
        InertValueV1::Text(""),
        InertValueV1::Array(&[]),
        InertValueV1::Object(&[]),
    ];
    let c = copy(&InertValueV1::Array(&values));
    let CapturedValueV1::Array(ids) = c.root() else {
        panic!("array")
    };
    assert!(matches!(c.node(ids[0]), Some(CapturedValueV1::Null)));
    assert!(matches!(
        c.node(ids[1]),
        Some(CapturedValueV1::Boolean(false))
    ));
    let Some(CapturedValueV1::Number(zero)) = c.node(ids[2]) else {
        panic!("number")
    };
    assert_eq!(zero.to_bits(), (-0.0_f64).to_bits());
    assert!(matches!(c.node(ids[3]), Some(CapturedValueV1::Number(n)) if *n == f64::MAX));
    assert!(matches!(c.node(ids[4]), Some(CapturedValueV1::Text(v)) if v.is_empty()));
    assert!(matches!(c.node(ids[5]), Some(CapturedValueV1::Array(v)) if v.is_empty()));
    assert!(matches!(c.node(ids[6]), Some(CapturedValueV1::Object(v)) if v.is_empty()));
    assert!(c.node(usize::MAX).is_none());
}

#[test]
fn given_nonfinite_numbers_when_copied_then_the_whole_capture_is_refused() {
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            InertCaptureV1::copy(
                &InertValueV1::Number(n),
                limits(),
                &mut Recorder::disabled()
            )
            .unwrap_err(),
            CaptureErrorV1::NonfiniteNumber
        );
    }
}

#[test]
fn given_duplicate_object_fields_when_copied_then_no_ambiguous_record_is_returned() {
    for key in ["", "profile", "ü"] {
        let entries = [
            (key, InertValueV1::Null),
            (key, InertValueV1::Boolean(true)),
        ];
        assert_eq!(
            InertCaptureV1::copy(
                &InertValueV1::Object(&entries),
                limits(),
                &mut Recorder::disabled()
            )
            .unwrap_err(),
            CaptureErrorV1::DuplicateField
        );
    }
}

#[test]
fn given_independent_byte_node_depth_ceilings_when_exact_then_admit_and_below_then_refuse() {
    let inner = [InertValueV1::Text("é\n")];
    let fields = [("items", InertValueV1::Array(&inner))];
    let input = InertValueV1::Object(&fields);
    let usage = copy(&input).usage();
    for (coordinate, bound) in [usage.peak_bytes, usage.nodes, usage.depth]
        .into_iter()
        .enumerate()
    {
        for delta in [-1isize, 0, 1] {
            let mut p = limits();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_bytes = n,
                1 => p.max_nodes = n,
                _ => p.max_depth = n,
            }
            let result = InertCaptureV1::copy(&input, p, &mut Recorder::disabled());
            assert_eq!(result.is_ok(), delta >= 0, "{coordinate}/{delta}");
            if let Ok(c) = result {
                assert_eq!(c.usage(), usage);
            } else {
                assert_eq!(result.unwrap_err(), CaptureErrorV1::Limit);
            }
        }
    }
    assert!(usage.peak_bytes > usage.retained_bytes);
}

#[test]
fn given_zero_ceilings_when_even_a_scalar_arrives_then_refuse_without_defaults() {
    for coordinate in 0..3 {
        let mut p = limits();
        match coordinate {
            0 => p.max_bytes = 0,
            1 => p.max_nodes = 0,
            _ => p.max_depth = 0,
        }
        assert_eq!(
            InertCaptureV1::copy(&InertValueV1::Null, p, &mut Recorder::disabled()).unwrap_err(),
            CaptureErrorV1::Limit
        );
    }
}

#[test]
fn given_shared_but_acyclic_data_when_copied_then_occurrences_remain_independent_values() {
    let shared = [InertValueV1::Text("same")];
    let input = [InertValueV1::Array(&shared), InertValueV1::Array(&shared)];
    let c = copy(&InertValueV1::Array(&input));
    assert_eq!(c.usage().nodes, 5);
    let CapturedValueV1::Array(ids) = c.root() else {
        panic!("root")
    };
    let Some(CapturedValueV1::Array(a)) = c.node(ids[0]) else {
        panic!("first")
    };
    let Some(CapturedValueV1::Array(b)) = c.node(ids[1]) else {
        panic!("second")
    };
    assert_ne!(a[0], b[0]);
}

#[test]
fn given_a_safe_rust_static_cycle_when_copied_then_refuse_without_recursive_execution() {
    static CYCLE: [InertValueV1<'static>; 1] = [InertValueV1::Array(&CYCLE)];
    assert_eq!(
        InertCaptureV1::copy(&CYCLE[0], limits(), &mut Recorder::disabled()).unwrap_err(),
        CaptureErrorV1::Cycle
    );
}

#[test]
fn given_capture_off_available_or_full_when_copying_then_content_and_refusals_are_identical() {
    for capacity in [None, Some(8), Some(1)] {
        for malformed in [false, true] {
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            if capacity == Some(1) {
                r.record(Observation {
                    boundary: Boundary::InertDataCapture,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let input = if malformed {
                InertValueV1::Number(f64::NAN)
            } else {
                InertValueV1::Text("secret body")
            };
            let result = InertCaptureV1::copy(&input, limits(), &mut r);
            assert_eq!(result.is_err(), malformed);
            if let Ok(c) = result {
                assert!(matches!(c.root(), CapturedValueV1::Text(v) if v == "secret body"));
                assert!(!format!("{c:?}").contains("secret"));
                assert!(!format!("{:?}", c.root()).contains("secret"));
            }
            if capacity == Some(8) {
                let event = r.pop_front().unwrap();
                assert_eq!(event.boundary, Boundary::InertDataCapture);
                assert_eq!(
                    event.disposition,
                    if malformed {
                        Disposition::Refused
                    } else {
                        Disposition::Completed
                    }
                );
                assert!(event.service_time.is_none());
                assert!(r.pop_front().is_none());
            } else if capacity == Some(1) {
                assert_eq!(r.dropped(), 1);
            } else {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}
