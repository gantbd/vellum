use super::*;
use crate::capture::{CaptureLimitsV1, InertValueV1 as D};
use crate::instrumentation::Recorder;

fn get<'a>(c: &'a InertCaptureV1, v: &'a C, name: &str) -> &'a C {
    let C::Object(rows) = v else { panic!() };
    c.node(rows.iter().find(|(k, _)| k == name).unwrap().1)
        .unwrap()
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 100_000,
        max_nodes: 2048,
        max_depth: 16,
    }
}
#[test]
fn actual_context_source_inputs_are_recognized_only_by_the_explicit_schema_profile() {
    let c = crate::coordinator::capture_native_context_declaration_v1(
        limits(),
        &mut Recorder::disabled(),
    )
    .unwrap();
    let C::Array(variants) = get(&c, c.root(), "variants") else {
        panic!()
    };
    for variant in variants {
        let v = c.node(*variant).unwrap();
        let C::Text(id) = get(&c, v, "id") else {
            panic!()
        };
        let input = get(&c, v, "input");
        assert!(recognize(
            &c,
            input,
            "engine-context-declaration/1",
            id,
            true
        ));
        assert!(!recognize(
            &c,
            input,
            "engine-context-declaration/1",
            id,
            false
        ));
        assert!(!recognize(&c, input, "foreign-contract", id, true));
        assert!(!recognize(
            &c,
            input,
            "engine-context-declaration/1",
            "unknown",
            true
        ));
    }
}

fn fixture(mode: usize) -> InertCaptureV1 {
    let contract = [("const", D::Text("request/1"))];
    let operation = [("const", D::Text("perform"))];
    let profile = [
        ("type", D::Text("string")),
        ("minLength", D::Number(if mode == 1 { 0.0 } else { 1.0 })),
    ];
    let epoch = [
        ("type", D::Text("integer")),
        ("minimum", D::Number(1.0)),
        (
            "maximum",
            D::Number(if mode == 2 {
                1e20
            } else {
                9_007_199_254_740_991.0
            }),
        ),
    ];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        (
            if mode == 3 { "keymap" } else { "profile" },
            D::Object(&profile),
        ),
        ("expectedSerialization", D::Object(&epoch)),
    ];
    let required = [
        D::Text("contract"),
        D::Text(if mode == 4 { "contract" } else { "operation" }),
        D::Text(if mode == 3 { "keymap" } else { "profile" }),
        D::Text("expectedSerialization"),
    ];
    let branch = [
        ("type", D::Text("object")),
        ("properties", D::Object(&properties)),
        (
            "required",
            D::Array(&required[..if mode == 5 { 3 } else { 4 }]),
        ),
        ("additionalProperties", D::Boolean(mode == 6)),
    ];
    let branches = [D::Object(&branch), D::Object(&branch)];
    let schema = [
        (
            "$schema",
            D::Text(if mode == 7 {
                "unknown"
            } else {
                "https://json-schema.org/draft/2020-12/schema"
            }),
        ),
        (
            "oneOf",
            D::Array(
                &branches[..if mode == 8 {
                    0
                } else if mode == 9 {
                    2
                } else {
                    1
                }],
            ),
        ),
        ("$ref", D::Text("https://not-loaded.invalid/schema")),
    ];
    let input = [
        ("contract", D::Text("metadata/1")),
        ("variant", D::Text("perform")),
        (
            "coverage",
            D::Text(if mode == 10 {
                "complete-codec"
            } else {
                "outer-structure-only"
            }),
        ),
        (
            "schema",
            D::Object(&schema[..if mode == 11 { 3 } else { 2 }]),
        ),
    ];
    InertCaptureV1::copy(&D::Object(&input), limits(), &mut Recorder::disabled()).unwrap()
}
#[test]
fn finite_profile_refuses_unknown_keywords_non_disjoint_or_inexact_branches_and_guessed_coverage() {
    let valid = fixture(0);
    assert!(recognize(
        &valid,
        valid.root(),
        "metadata/1",
        "perform",
        true
    ));
    for mode in 1..=11 {
        let c = fixture(mode);
        assert!(
            !recognize(&c, c.root(), "metadata/1", "perform", true),
            "mutation {mode}"
        );
    }
}

#[test]
fn editing_descriptor_cannot_substitute_a_structurally_valid_f16_schema() {
    use crate::coordinator::handler_declaration::with_context_descriptor_v1;
    with_context_descriptor_v1(|descriptor| {
        fn property<'a>(v: D<'a>, key: &str) -> D<'a> {
            let D::Object(rows) = v else { panic!() };
            rows.iter().find(|(k, _)| *k == key).unwrap().1
        }
        let D::Array(variants) = property(descriptor, "variants") else {
            panic!()
        };
        let schema = property(property(variants[0], "input"), "schema");
        let input = [
            ("contract", D::Text("engine-ordinary-edit-declaration/1")),
            ("variant", D::Text("ordinary-edit")),
            ("coverage", D::Text("outer-structure-only")),
            ("schema", schema),
        ];
        let c =
            InertCaptureV1::copy(&D::Object(&input), limits(), &mut Recorder::disabled()).unwrap();
        assert!(!recognize_profile(
            &c,
            c.root(),
            "engine-ordinary-edit-declaration/1",
            "ordinary-edit",
            "owned-contract-discovery/3"
        ));
    })
    .unwrap();
}

#[test]
fn actual_registry_copier_bounds_metadata_before_shape_recognition() {
    use super::super::*;
    let variants = [DiscoveryVariantInputV1 {
        id: "perform",
        input: D::Text("not a descriptor"),
        outcomes: D::Null,
        requirements: D::Null,
        limits: D::Null,
        cancellation: D::Null,
        order: Some(0),
        enabled: None,
    }];
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority: "owner",
            id: "command",
            incarnation: 1,
        },
        contract: "metadata/1",
        version: 1,
        kind: DiscoveryExecutionKindV1::LocalCommand,
        variants: &variants,
        presentation: D::Object(&[]),
        order: Some(0),
        schema_visible: true,
        presentation_visible: false,
        advice_visible: false,
    }];
    let input = DiscoveryCutInputV1 {
        entries: &entries,
        target: None,
        advice_present: false,
    };
    let mut l = super::super::tests::limits();
    l.max_row_bytes = 32768;
    let mut r = Recorder::disabled();
    assert!(matches!(
        rows_copy(&input, l, 65536, &mut r),
        Err(Error::InvalidRequest)
    ));
    for metadata in [
        CaptureLimitsV1 {
            max_bytes: 1,
            ..l.metadata
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..l.metadata
        },
        CaptureLimitsV1 {
            max_depth: 1,
            ..l.metadata
        },
    ] {
        l.metadata = metadata;
        assert!(matches!(
            rows_copy(&input, l, 65536, &mut r),
            Err(Error::Limit)
        ));
    }
}

#[test]
fn actual_registry_recognition_has_optional_timed_dispositions_without_saturation_interference() {
    use super::super::*;
    use crate::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    let valid = [
        ("contract", D::Text("metadata/1")),
        ("variant", D::Text("perform")),
        ("fields", D::Object(&[])),
    ];
    for capacity in [0, 1, 16] {
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        for input in [D::Object(&valid), D::Text("invalid")] {
            let variants = [DiscoveryVariantInputV1 {
                id: "perform",
                input,
                outcomes: D::Null,
                requirements: D::Null,
                limits: D::Null,
                cancellation: D::Null,
                order: Some(0),
                enabled: None,
            }];
            let entries = [DiscoveryEntryInputV1 {
                command: DiscoveryCommandV1 {
                    authority: "owner",
                    id: "command",
                    incarnation: 1,
                },
                contract: "metadata/1",
                version: 1,
                kind: DiscoveryExecutionKindV1::LocalCommand,
                variants: &variants,
                presentation: D::Object(&[]),
                order: Some(0),
                schema_visible: true,
                presentation_visible: false,
                advice_visible: false,
            }];
            let mut l = super::super::tests::limits();
            l.max_row_bytes = 32768;
            let result = rows_copy(
                &DiscoveryCutInputV1 {
                    entries: &entries,
                    target: None,
                    advice_present: false,
                },
                l,
                65536,
                &mut r,
            );
            assert_eq!(result.is_ok(), matches!(input, D::Object(_)));
        }
        if capacity == 16 {
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front())
                .filter(|v| v.boundary == Boundary::DiscoverySchemaRecognition)
                .collect();
            assert_eq!(records.len(), 2);
            assert_eq!(records[0].disposition, Disposition::Completed);
            assert_eq!(records[1].disposition, Disposition::Refused);
            assert!(records.iter().all(|v| v.service_time.is_some()));
            assert!(!format!("{records:?}").contains("metadata/1"));
        } else if capacity == 1 {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
