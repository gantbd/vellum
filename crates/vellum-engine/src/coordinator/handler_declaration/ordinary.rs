//! Fixed F11 outer schema. Native guards remain authoritative for execution.
use crate::capture::{CaptureErrorV1, CapturedValueV1 as C, InertCaptureV1, InertValueV1 as D};
use crate::coordinator::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub(in crate::coordinator) const ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1: &str =
    "engine-ordinary-edit-declaration/1";
type Constraint<'a> = ([(&'a str, D<'a>); 3], usize);
type Pair<'a, 'b> = (&'a C, D<'b>);
// Closed graph frontier: four branches, four branch members, thirteen fields,
// at most three property keywords. Refuse rather than grow this fixed workspace.
const FRONTIER: usize = 32;
pub(in crate::coordinator) fn ordinary_workspace_bytes_v1() -> usize {
    // Source graph and simultaneous exact-recognition graph, plus descriptor and
    // matcher frontier. Logical staging only, not physical stack/RSS accounting.
    let schema = [
        size_of::<[(&NativeArgumentDeclarationV1, &str); 4]>(),
        size_of::<[[Constraint<'_>; 13]; 4]>(),
        size_of::<[[(&str, D<'_>); 13]; 4]>(),
        size_of::<[[D<'_>; 13]; 4]>(),
        size_of::<[[(&str, D<'_>); 4]; 4]>(),
        size_of::<[D<'_>; 4]>(),
        size_of::<[(&str, D<'_>); 2]>(),
    ]
    .into_iter()
    .sum::<usize>();
    schema * 2
        + size_of::<[(&str, &str); 5]>()
        + size_of::<[Option<Pair<'_, '_>>; FRONTIER]>()
        + size_of::<[(&str, D<'_>); 4]>()
        + size_of::<[[(&str, D<'_>); 2]; 5]>()
        + size_of::<[D<'_>; 5]>() * 2
        + size_of::<[(&str, D<'_>); 3]>() * 2
        + size_of::<[(&str, D<'_>); 1]>()
        + size_of::<[(&str, D<'_>); 6]>() * 2
        + size_of::<[D<'_>; 1]>()
}
fn with_schema<T>(consume: impl FnOnce(D<'_>) -> T) -> Result<T, CaptureErrorV1> {
    let forms = [
        (
            NormalizedLiteralEditArgumentsV1::ARGUMENT_DECLARATION,
            EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1,
        ),
        (
            NormalizedLineEditArgumentsV1::ARGUMENT_DECLARATION,
            EXPLICIT_PREFIX_LINE_EDIT_PROFILE_V1,
        ),
        (
            NormalizedBoundaryEditArgumentsV1::ARGUMENT_DECLARATION,
            UNICODE_LOGICAL_EDIT_PROFILE_V1,
        ),
        (
            NormalizedConfiguredLineArgumentsV1::ARGUMENT_DECLARATION,
            CONFIGURED_PREFIX_LINE_PROFILE_V1,
        ),
    ];
    let constraints: [[Constraint<'_>; 13]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let (d, profile) = forms[i];
            let Some((key, _)) = d.variant.fields.get(j) else {
                return ([("", D::Null); 3], 0);
            };
            let value = match *key {
                "contract" => Some(d.contract),
                "operation" => Some(d.variant.operation),
                "profile" => Some(profile),
                _ => None,
            };
            if let Some(value) = value {
                return ([("const", D::Text(value)), ("", D::Null), ("", D::Null)], 1);
            }
            match *key {
                "invocation" | "action" | "selectionAfter" | "unit" | "direction" => (
                    [
                        ("type", D::Text("string")),
                        ("minLength", D::Number(1.0)),
                        ("", D::Null),
                    ],
                    2,
                ),
                "text" => (
                    [("type", D::Text("string")), ("", D::Null), ("", D::Null)],
                    1,
                ),
                "view" | "document" | "settings" | "boundary" | "configuration" => (
                    [("type", D::Text("object")), ("", D::Null), ("", D::Null)],
                    1,
                ),
                "sourceRevision" | "expectedSelection" | "count" => (
                    [
                        ("type", D::Text("integer")),
                        (
                            "minimum",
                            D::Number(if *key == "count" { 1.0 } else { 0.0 }),
                        ),
                        ("maximum", D::Number(9_007_199_254_740_991.0)),
                    ],
                    3,
                ),
                "extend" => (
                    [("type", D::Text("boolean")), ("", D::Null), ("", D::Null)],
                    1,
                ),
                _ => ([("", D::Null); 3], 0),
            }
        })
    });
    if forms.iter().enumerate().any(|(i, (d, _))| {
        d.variant.operation != "ordinary-edit"
            || d.variant.fields.len() > 13
            || d.variant
                .fields
                .iter()
                .enumerate()
                .any(|(j, _)| constraints[i][j].1 == 0)
    }) {
        return Err(CaptureErrorV1::Limit);
    }
    let properties: [[(&str, D<'_>); 13]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (
                forms[i].0.variant.fields.get(j).map_or("", |(k, _)| *k),
                D::Object(&constraints[i][j].0[..constraints[i][j].1]),
            )
        })
    });
    let required: [[D<'_>; 13]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            forms[i]
                .0
                .variant
                .fields
                .get(j)
                .map_or(D::Null, |(k, _)| D::Text(k))
        })
    });
    let branches: [[(&str, D<'_>); 4]; 4] = std::array::from_fn(|i| {
        let n = forms[i].0.variant.fields.len();
        [
            ("type", D::Text("object")),
            ("properties", D::Object(&properties[i][..n])),
            ("required", D::Array(&required[i][..n])),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let values: [D<'_>; 4] = std::array::from_fn(|i| D::Object(&branches[i]));
    Ok(consume(D::Object(&[
        (
            "$schema",
            D::Text("https://json-schema.org/draft/2020-12/schema"),
        ),
        ("oneOf", D::Array(&values)),
    ])))
}
pub(in crate::coordinator) fn with_ordinary_descriptor_v1<T>(
    consume: impl FnOnce(D<'_>) -> T,
) -> Result<T, CaptureErrorV1> {
    with_schema(|schema| {
        let input = [
            ("contract", D::Text(ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1)),
            ("variant", D::Text("ordinary-edit")),
            ("coverage", D::Text("outer-structure-only")),
            ("schema", schema),
        ];
        let pairs = [
            ("applied", "done"),
            ("unchanged", "done"),
            ("notApplied", "rejected"),
            ("notApplied", "stopped"),
            ("unknown", "faulted"),
        ];
        let terminals: [[(&str, D<'_>); 2]; 5] = std::array::from_fn(|i| {
            [
                ("publication", D::Text(pairs[i].0)),
                ("control", D::Text(pairs[i].1)),
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
            D::Text("nested-value-codec"),
            D::Text("boundary-reference-authenticity"),
            D::Text("semantic-enablement"),
            D::Text("complete-family-dispatch"),
            D::Text("public-wire"),
        ];
        let requirements = [
            (
                "currentPredicates",
                D::Text("owning-profile-target-source-selection-authority-and-companions"),
            ),
            ("inputCoverage", D::Text("outer-structure-only")),
            ("notEstablished", D::Array(&not_established)),
        ];
        let limits = [("values", D::Text("owning-request-and-preparation-profiles"))];
        let variant = [
            ("id", D::Text("ordinary-edit")),
            ("input", D::Object(&input)),
            ("outcomes", D::Object(&outcomes)),
            ("requirements", D::Object(&requirements)),
            ("limits", D::Object(&limits)),
            (
                "cancellation",
                D::Text("owned-prepublication-stop; accepted-publication-is-not-retracted"),
            ),
        ];
        let variants = [D::Object(&variant)];
        consume(D::Object(&[
            ("family", D::Text("F11")),
            ("contract", D::Text(ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1)),
            ("version", D::Number(1.0)),
            ("kind", D::Text("localCommand")),
            ("variants", D::Array(&variants)),
            ("presentation", D::Object(&[])),
        ]))
    })
}
fn push<'a, 'b>(
    work: &mut [Option<Pair<'a, 'b>>; FRONTIER],
    len: &mut usize,
    value: Option<&'a C>,
    expected: D<'b>,
) -> bool {
    let Some(value) = value else { return false };
    if *len == FRONTIER {
        return false;
    }
    work[*len] = Some((value, expected));
    *len += 1;
    true
}
// Exact finite descriptor matching, not execution/validation of caller schemas.
fn same(c: &InertCaptureV1, value: &C, expected: D<'_>) -> bool {
    let mut work = [None; FRONTIER];
    let mut len = 0;
    if !push(&mut work, &mut len, Some(value), expected) {
        return false;
    }
    while len > 0 {
        len -= 1;
        let Some((value, expected)) = work[len].take() else {
            return false;
        };
        match (value, expected) {
            (C::Null, D::Null) => {}
            (C::Boolean(a), D::Boolean(b)) if *a == b => {}
            (C::Number(a), D::Number(b)) if a.to_bits() == b.to_bits() => {}
            (C::Text(a), D::Text(b)) if a == b => {}
            (C::Array(a), D::Array(b)) if a.len() == b.len() => {
                for (index, expected) in a.iter().zip(b) {
                    if !push(&mut work, &mut len, c.node(*index), *expected) {
                        return false;
                    }
                }
            }
            (C::Object(a), D::Object(b)) if a.len() == b.len() => {
                for (key, expected) in b {
                    let value = a
                        .iter()
                        .find(|(k, _)| k == key)
                        .and_then(|(_, i)| c.node(*i));
                    if !push(&mut work, &mut len, value, *expected) {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }
    true
}
pub(in crate::coordinator) fn ordinary_schema_matches_v1(c: &InertCaptureV1, value: &C) -> bool {
    with_schema(|expected| same(c, value, expected)).unwrap_or(false)
}
