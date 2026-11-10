//! Descriptive outer schemas are not executable codecs or authority.
use vellum_engine::capture::{CaptureLimitsV1, CapturedValueV1 as C, InertCaptureV1};
use vellum_engine::coordinator::capture_native_context_declaration_v1;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Recorder};

fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 100_000,
        max_nodes: 2048,
        max_depth: 16,
    }
}
fn field<'a>(c: &'a InertCaptureV1, v: &'a C, key: &str) -> &'a C {
    let C::Object(rows) = v else {
        panic!("object required")
    };
    c.node(rows.iter().find(|(k, _)| k == key).unwrap().1)
        .unwrap()
}
fn array<'a>(c: &'a InertCaptureV1, v: &'a C) -> Vec<&'a C> {
    let C::Array(rows) = v else {
        panic!("array required")
    };
    rows.iter().map(|i| c.node(*i).unwrap()).collect()
}
fn text(v: &C) -> &str {
    let C::Text(v) = v else {
        panic!("text required")
    };
    v
}
fn signature(c: &InertCaptureV1, v: &C) -> String {
    match v {
        C::Null => "null".into(),
        C::Boolean(v) => format!("{v}"),
        C::Number(v) => format!("{v}"),
        C::Text(v) => format!("{v:?}"),
        C::Array(rows) => format!(
            "[{}]",
            rows.iter()
                .map(|i| signature(c, c.node(*i).unwrap()))
                .collect::<Vec<_>>()
                .join(",")
        ),
        C::Object(rows) => format!(
            "{{{}}}",
            rows.iter()
                .map(|(k, i)| format!("{k:?}:{}", signature(c, c.node(*i).unwrap())))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

#[test]
fn fixed_context_source_has_two_variants_ten_disjoint_exact_outer_forms_and_five_facets() {
    let mut r = Recorder::disabled();
    let c = capture_native_context_declaration_v1(limits(), &mut r).unwrap();
    assert_eq!(
        text(field(&c, c.root(), "contract")),
        "engine-context-declaration/1"
    );
    let variants = array(&c, field(&c, c.root(), "variants"));
    assert_eq!(variants.len(), 2);
    let mut contracts = Vec::new();
    for (variant, expected_count) in variants.into_iter().zip([6, 4]) {
        for name in [
            "input",
            "outcomes",
            "requirements",
            "limits",
            "cancellation",
        ] {
            field(&c, variant, name);
        }
        let input = field(&c, variant, "input");
        assert_eq!(text(field(&c, input, "coverage")), "outer-structure-only");
        let schema = field(&c, input, "schema");
        assert_eq!(
            text(field(&c, schema, "$schema")),
            "https://json-schema.org/draft/2020-12/schema"
        );
        let branches = array(&c, field(&c, schema, "oneOf"));
        assert_eq!(branches.len(), expected_count);
        for branch in branches {
            assert!(matches!(
                field(&c, branch, "additionalProperties"),
                C::Boolean(false)
            ));
            let props = field(&c, branch, "properties");
            let C::Object(properties) = props else {
                panic!()
            };
            let required = array(&c, field(&c, branch, "required"));
            assert_eq!(required.len(), properties.len());
            for name in required {
                field(&c, props, text(name));
            }
            for (name, index) in properties {
                let property = c.node(*index).unwrap();
                let expected = match name.as_str() {
                    "contract" | "operation" => continue,
                    "profile" if properties.iter().any(|(k, _)| k == "request") => continue,
                    "invocation" | "profile" => "string",
                    "expectedSerialization" => "integer",
                    "create" | "remove" | "targets" | "changes" | "dependencies" => "array",
                    "expected" | "definition" | "request" | "document" | "serialization" => {
                        "object"
                    }
                    _ => panic!("unrecognized field"),
                };
                assert_eq!(text(field(&c, property, "type")), expected);
                if expected == "string" {
                    assert!(matches!(field(&c, property, "minLength"), C::Number(1.0)));
                }
            }
            let contract = text(field(&c, field(&c, props, "contract"), "const"));
            assert!(!contracts.contains(&contract));
            contracts.push(contract);
            let operation = text(field(&c, field(&c, props, "operation"), "const"));
            if contract.contains("registration-invocation") {
                assert_eq!(operation, "activate-context");
                let expected = if contract.starts_with("scoped-") {
                    "scoped-resource-registration/1"
                } else {
                    "native-resource-registration/1"
                };
                assert_eq!(
                    text(field(&c, field(&c, props, "profile"), "const")),
                    expected
                );
            } else {
                assert_eq!(operation, text(field(&c, variant, "id")));
            }
            if contract == "source-serialization/1" {
                let revision = field(&c, props, "expectedSerialization");
                assert!(matches!(field(&c, revision, "minimum"), C::Number(1.0)));
                assert!(matches!(
                    field(&c, revision, "maximum"),
                    C::Number(9_007_199_254_740_991.0)
                ));
            }
        }
    }
    assert_eq!(contracts.len(), 10);
    contracts.sort_unstable();
    let mut expected = [
        "semantic-settings-activation/1",
        "scoped-semantic-settings/1",
        "engine-settings-layers/1",
        "scoped-engine-settings-layers/1",
        "source-serialization/1",
        "scoped-source-serialization/1",
        "engine-settings-definition/1",
        "scoped-engine-settings-definition/1",
        "registration-invocation/1",
        "scoped-registration-invocation/1",
    ];
    expected.sort_unstable();
    assert_eq!(contracts, expected);
}

#[test]
fn source_capture_bounds_and_diagnostic_saturation_do_not_change_the_product() {
    let mut off = Recorder::disabled();
    let c = capture_native_context_declaration_v1(limits(), &mut off).unwrap();
    let usage = c.usage();
    let expected = signature(&c, c.root());
    for capacity in [0, 1, 64] {
        let mut r = Recorder::try_with_capacity(capacity).unwrap();
        for _ in 0..3 {
            let out = capture_native_context_declaration_v1(limits(), &mut r).unwrap();
            assert_eq!(out.usage(), usage);
            assert_eq!(signature(&out, out.root()), expected);
            assert_eq!(array(&out, field(&out, out.root(), "variants")).len(), 2);
        }
    }
    for l in [
        CaptureLimitsV1 {
            max_bytes: 0,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: 1,
            ..limits()
        },
    ] {
        assert!(capture_native_context_declaration_v1(l, &mut off).is_err());
    }
    let exact = CaptureLimitsV1 {
        max_bytes: usage.peak_bytes,
        max_nodes: usage.nodes,
        max_depth: usage.depth,
    };
    assert!(capture_native_context_declaration_v1(exact, &mut off).is_ok());
    for l in [
        CaptureLimitsV1 {
            max_bytes: exact.max_bytes - 1,
            ..exact
        },
        CaptureLimitsV1 {
            max_nodes: exact.max_nodes - 1,
            ..exact
        },
        CaptureLimitsV1 {
            max_depth: exact.max_depth - 1,
            ..exact
        },
    ] {
        assert!(capture_native_context_declaration_v1(l, &mut off).is_err());
    }
}

#[test]
fn descriptor_carries_qualified_terminal_pairs_and_explicit_non_authority_limits() {
    let c = capture_native_context_declaration_v1(limits(), &mut Recorder::disabled()).unwrap();
    for v in array(&c, field(&c, c.root(), "variants")) {
        let outcomes = field(&c, v, "outcomes");
        let pairs = array(&c, field(&c, outcomes, "qualifiedLocalTerminalForms"));
        assert_eq!(pairs.len(), 5);
        for (pair, (publication, control)) in pairs.into_iter().zip([
            ("applied", "done"),
            ("unchanged", "done"),
            ("notApplied", "rejected"),
            ("notApplied", "stopped"),
            ("unknown", "faulted"),
        ]) {
            assert_eq!(text(field(&c, pair, "publication")), publication);
            assert_eq!(text(field(&c, pair, "control")), control);
        }
        assert_eq!(text(field(&c, outcomes, "effect")), "none");
        assert_eq!(
            text(field(&c, outcomes, "release")),
            "captured-current-recipient-intersection; not guaranteed disclosure"
        );
        let requirements = field(&c, v, "requirements");
        let exclusions = array(&c, field(&c, requirements, "notEstablished"));
        assert_eq!(
            exclusions.into_iter().map(text).collect::<Vec<_>>(),
            [
                "semantic-enablement",
                "complete-value-codec",
                "public-wire",
                "installed-all-family-dispatch"
            ]
        );
        assert_eq!(
            text(field(&c, field(&c, v, "limits"), "values")),
            "current-owning-request-preparation-and-execution-profiles"
        );
        assert_eq!(
            text(field(&c, v, "cancellation")),
            "owned-prepublication-stop; accepted-publication-is-not-retracted"
        );
    }
}

#[test]
fn actual_descriptor_path_times_success_and_refusal_without_metadata_labels() {
    let mut r = Recorder::try_with_capacity(8)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    capture_native_context_declaration_v1(limits(), &mut r).unwrap();
    assert!(
        capture_native_context_declaration_v1(
            CaptureLimitsV1 {
                max_nodes: 0,
                ..limits()
            },
            &mut r
        )
        .is_err()
    );
    let observations = std::iter::from_fn(|| r.pop_front())
        .filter(|o| o.boundary == Boundary::ContextDeclarationCapture)
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].disposition, Disposition::Completed);
    assert_eq!(observations[1].disposition, Disposition::Refused);
    assert!(observations.iter().all(|o| o.service_time.is_some()));
    assert!(!format!("{observations:?}").contains("semantic-settings"));
}
