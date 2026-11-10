//! Fixed F16 metadata source. No caller registration, evaluator or enablement.
use crate::capture::{CaptureErrorV1, CaptureLimitsV1, InertCaptureV1, InertValueV1 as D};
use crate::coordinator::*;
use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};

const CONTRACT: &str = "engine-context-declaration/1";
const COVERAGE: &str = "outer-structure-only";
const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
type PropertyConstraint<'a> = ([(&'a str, D<'a>); 3], usize);

/// Capture the two fixed context variants and their ten handler-owned outer
/// request forms. Bounded, inert descriptive metadata only: not a Rust/wire
/// codec, nested validator, live registry, capability or recipient release.
/// The caller funds the returned capture; no coordinator state is accessed.
/// Existing capture observations report actual copy success/refusal/loss.
pub fn capture_native_context_declaration_v1(
    limits: CaptureLimitsV1,
    recorder: &mut Recorder,
) -> Result<InertCaptureV1, CaptureErrorV1> {
    let timing = recorder.begin_service_timing();
    let result = with_context_descriptor_v1(|value| InertCaptureV1::copy(&value, limits, recorder))
        .and_then(std::convert::identity);
    recorder.record(Observation {
        boundary: Boundary::ContextDeclarationCapture,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: timing.finish(),
    });
    result
}
// All simultaneous fixed source arrays, conservatively prepaid by the discovery
// owner. Logical staging only, not a measured stack/allocator/RSS charge.
pub(in crate::coordinator) fn context_workspace_bytes_v1() -> usize {
    use std::mem::size_of;
    [
        size_of::<[&NativeArgumentDeclarationV1; 10]>(),
        size_of::<[[PropertyConstraint<'_>; 9]; 10]>(),
        size_of::<[[(&str, D<'_>); 9]; 10]>(),
        size_of::<[[D<'_>; 9]; 10]>(),
        size_of::<[[(&str, D<'_>); 4]; 10]>(),
        size_of::<[D<'_>; 10]>(),
        size_of::<[[(&str, D<'_>); 2]; 2]>(),
        size_of::<[&str; 2]>(),
        size_of::<[[(&str, D<'_>); 4]; 2]>(),
        size_of::<[(&str, &str); 5]>(),
        size_of::<[[(&str, D<'_>); 2]; 5]>(),
        size_of::<[D<'_>; 5]>(),
        size_of::<[(&str, D<'_>); 3]>() * 2,
        size_of::<[D<'_>; 4]>(),
        size_of::<[(&str, D<'_>); 1]>(),
        size_of::<[[(&str, D<'_>); 6]; 2]>(),
        size_of::<[D<'_>; 2]>(),
        size_of::<[(&str, D<'_>); 6]>(),
    ]
    .into_iter()
    .sum()
}

// Private, immediate borrowing handoff, not a caller extension or executable
// metadata callback. Avoids allocating/copying an intermediate schema tree.
pub(in crate::coordinator) fn with_context_descriptor_v1<T>(
    consume: impl FnOnce(D<'_>) -> T,
) -> Result<T, CaptureErrorV1> {
    let declarations = [
        NormalizedNativeSettingsArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedSettingsArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedNativeLifecycleArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedLifecycleArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedSourceSerializationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedSerializationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedNativeDefinitionArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedDefinitionArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedResourceRegistrationArgumentsV1::ARGUMENT_DECLARATION,
        NormalizedScopedRegistrationArgumentsV1::ARGUMENT_DECLARATION,
    ];
    // Fixed stack workspace: ten forms, at most nine properties, three keywords
    // per property. No caller-sized traversal or intermediate heap allocation.
    let constraints: [[PropertyConstraint<'_>; 9]; 10] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let d = declarations[i];
            let Some((name, _)) = d.variant.fields.get(j) else {
                return ([("", D::Null); 3], 0);
            };
            let constant = match *name {
                "contract" => Some(d.contract),
                "operation" => Some(d.variant.operation),
                "profile" if i == 8 => Some(NATIVE_RESOURCE_REGISTRATION_PROFILE_V1),
                "profile" if i == 9 => Some(SCOPED_RESOURCE_REGISTRATION_PROFILE_V1),
                _ => None,
            };
            if let Some(value) = constant {
                return ([("const", D::Text(value)), ("", D::Null), ("", D::Null)], 1);
            }
            match *name {
                "invocation" | "profile" => (
                    [
                        ("type", D::Text("string")),
                        ("minLength", D::Number(1.0)),
                        ("", D::Null),
                    ],
                    2,
                ),
                "expectedSerialization" => (
                    [
                        ("type", D::Text("integer")),
                        ("minimum", D::Number(1.0)),
                        ("maximum", D::Number(9_007_199_254_740_991.0)),
                    ],
                    3,
                ),
                "create" | "remove" | "targets" | "changes" | "dependencies" => (
                    [("type", D::Text("array")), ("", D::Null), ("", D::Null)],
                    1,
                ),
                // The remaining fields are named object products. Exhaustively
                // matched so a future handler field cannot acquire a guess.
                "expected" | "definition" | "request" | "document" | "serialization" => (
                    [("type", D::Text("object")), ("", D::Null), ("", D::Null)],
                    1,
                ),
                _ => ([("", D::Null); 3], 0),
            }
        })
    });
    // Fail closed on declaration growth rather than emitting a truncated or
    // guessed schema. The caller's limits still bound all copying below.
    if declarations.iter().enumerate().any(|(i, d)| {
        d.variant.fields.len() > 9
            || d.variant
                .fields
                .iter()
                .enumerate()
                .any(|(j, _)| constraints[i][j].1 == 0)
    }) {
        return Err(CaptureErrorV1::Limit);
    }
    let properties: [[(&str, D<'_>); 9]; 10] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let key = declarations[i]
                .variant
                .fields
                .get(j)
                .map_or("", |(key, _)| *key);
            (key, D::Object(&constraints[i][j].0[..constraints[i][j].1]))
        })
    });
    let required: [[D<'_>; 9]; 10] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            declarations[i]
                .variant
                .fields
                .get(j)
                .map_or(D::Null, |(key, _)| D::Text(key))
        })
    });
    let branches: [[(&str, D<'_>); 4]; 10] = std::array::from_fn(|i| {
        let count = declarations[i].variant.fields.len();
        [
            ("type", D::Text("object")),
            ("properties", D::Object(&properties[i][..count])),
            ("required", D::Array(&required[i][..count])),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let one_of: [D<'_>; 10] = std::array::from_fn(|i| D::Object(&branches[i]));
    let schemas = [
        [
            ("$schema", D::Text(DIALECT)),
            ("oneOf", D::Array(&one_of[..6])),
        ],
        [
            ("$schema", D::Text(DIALECT)),
            ("oneOf", D::Array(&one_of[6..])),
        ],
    ];
    let ids = ["semantic-settings", "supported-registry"];
    let inputs: [[(&str, D<'_>); 4]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(CONTRACT)),
            ("variant", D::Text(ids[i])),
            ("coverage", D::Text(COVERAGE)),
            ("schema", D::Object(&schemas[i])),
        ]
    });
    let terminal_pairs = [
        ("applied", "done"),
        ("unchanged", "done"),
        ("notApplied", "rejected"),
        ("notApplied", "stopped"),
        ("unknown", "faulted"),
    ];
    let terminals: [[(&str, D<'_>); 2]; 5] = std::array::from_fn(|i| {
        [
            ("publication", D::Text(terminal_pairs[i].0)),
            ("control", D::Text(terminal_pairs[i].1)),
        ]
    });
    let terminal_values: [D<'_>; 5] = std::array::from_fn(|i| D::Object(&terminals[i]));
    let outcomes = [
        ("qualifiedLocalTerminalForms", D::Array(&terminal_values)),
        ("effect", D::Text("none")),
        (
            "release",
            D::Text("captured-current-recipient-intersection; not guaranteed disclosure"),
        ),
    ];
    let not_established = [
        D::Text("semantic-enablement"),
        D::Text("complete-value-codec"),
        D::Text("public-wire"),
        D::Text("installed-all-family-dispatch"),
    ];
    let requirements = [
        (
            "currentPredicates",
            D::Text("owning-profile-context-authority-dependencies-and-state-specific"),
        ),
        ("inputCoverage", D::Text(COVERAGE)),
        ("notEstablished", D::Array(&not_established)),
    ];
    let bounds = [(
        "values",
        D::Text("current-owning-request-preparation-and-execution-profiles"),
    )];
    let variants: [[(&str, D<'_>); 6]; 2] = std::array::from_fn(|i| {
        [
            ("id", D::Text(ids[i])),
            ("input", D::Object(&inputs[i])),
            ("outcomes", D::Object(&outcomes)),
            ("requirements", D::Object(&requirements)),
            ("limits", D::Object(&bounds)),
            (
                "cancellation",
                D::Text("owned-prepublication-stop; accepted-publication-is-not-retracted"),
            ),
        ]
    });
    let variant_values = [D::Object(&variants[0]), D::Object(&variants[1])];
    Ok(consume(D::Object(&[
        ("family", D::Text("F16")),
        ("contract", D::Text(CONTRACT)),
        ("version", D::Number(1.0)),
        ("kind", D::Text("localCommand")),
        ("variants", D::Array(&variant_values)),
        ("presentation", D::Object(&[])),
    ])))
}
