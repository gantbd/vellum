//! Closed metadata recognition after bounded capture, never schema execution.
use crate::capture::{CapturedValueV1 as C, InertCaptureV1};
use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};
#[cfg(test)]
mod tests;

fn field<'a>(c: &'a InertCaptureV1, v: &'a C, key: &str) -> Option<&'a C> {
    let C::Object(rows) = v else { return None };
    c.node(rows.iter().find(|(name, _)| name == key)?.1)
}
fn exact(v: &C, keys: &[&str]) -> bool {
    matches!(v, C::Object(rows) if rows.len() == keys.len()
        && keys.iter().all(|key| rows.iter().any(|(name, _)| name == key)))
}
fn text(v: Option<&C>) -> Option<&str> {
    match v {
        Some(C::Text(v)) => Some(v),
        _ => None,
    }
}
fn literal(c: &InertCaptureV1, v: &C, key: &str, expected: &str) -> bool {
    text(field(c, v, key)) == Some(expected)
}
fn number(c: &InertCaptureV1, v: &C, key: &str, expected: f64) -> bool {
    matches!(field(c, v, key), Some(C::Number(n)) if *n == expected)
}
fn constant(c: &InertCaptureV1, v: &C) -> bool {
    exact(v, &["const"]) && text(field(c, v, "const")).is_some_and(|s| !s.is_empty())
}
fn property(c: &InertCaptureV1, key: &str, v: &C) -> bool {
    match key {
        "contract" | "operation" => constant(c, v),
        "profile" if field(c, v, "const").is_some() => constant(c, v),
        "invocation" | "profile" => {
            exact(v, &["type", "minLength"])
                && literal(c, v, "type", "string")
                && number(c, v, "minLength", 1.0)
        }
        "expectedSerialization" => {
            exact(v, &["type", "minimum", "maximum"])
                && literal(c, v, "type", "integer")
                && number(c, v, "minimum", 1.0)
                && number(c, v, "maximum", 9_007_199_254_740_991.0)
        }
        "create" | "remove" | "targets" | "changes" | "dependencies" => {
            exact(v, &["type"]) && literal(c, v, "type", "array")
        }
        "expected" | "definition" | "request" | "document" | "serialization" => {
            exact(v, &["type"]) && literal(c, v, "type", "object")
        }
        _ => false,
    }
}
fn branch(c: &InertCaptureV1, v: &C) -> bool {
    if !exact(
        v,
        &["type", "properties", "required", "additionalProperties"],
    ) || !literal(c, v, "type", "object")
        || !matches!(field(c, v, "additionalProperties"), Some(C::Boolean(false)))
    {
        return false;
    }
    let Some(properties @ C::Object(rows)) = field(c, v, "properties") else {
        return false;
    };
    let Some(C::Array(required)) = field(c, v, "required") else {
        return false;
    };
    if required.len() != rows.len()
        || field(c, properties, "contract").is_none()
        || field(c, properties, "operation").is_none()
    {
        return false;
    }
    for (index, value) in required.iter().enumerate() {
        let Some(key) = text(c.node(*value)) else {
            return false;
        };
        if required[..index]
            .iter()
            .any(|old| text(c.node(*old)) == Some(key))
        {
            return false;
        }
        let Some(value) = field(c, properties, key) else {
            return false;
        };
        if !property(c, key, value) {
            return false;
        }
    }
    true
}
fn branch_contract<'a>(c: &'a InertCaptureV1, v: &'a C) -> Option<&'a str> {
    text(field(c, field(c, v, "properties")?, "contract").and_then(|v| field(c, v, "const")))
}
fn schema(c: &InertCaptureV1, v: &C) -> bool {
    if !exact(v, &["$schema", "oneOf"])
        || !literal(
            c,
            v,
            "$schema",
            "https://json-schema.org/draft/2020-12/schema",
        )
    {
        return false;
    }
    let Some(C::Array(branches)) = field(c, v, "oneOf") else {
        return false;
    };
    if branches.is_empty() {
        return false;
    }
    for (index, value) in branches.iter().enumerate() {
        let Some(value) = c.node(*value) else {
            return false;
        };
        if !branch(c, value)
            || branches[..index].iter().any(|old| {
                c.node(*old)
                    .is_some_and(|old| branch_contract(c, old) == branch_contract(c, value))
            })
        {
            return false;
        }
    }
    true
}

/// The capture owning `input` must already have passed metadata limits. This
/// performs bounded scans only over owned nodes; no recursion, allocations,
/// callbacks, references, keyword extensions or external schema loading.
/// `schema_profile` is selected by the owner, never inferred from input data.
pub(super) fn recognize(
    c: &InertCaptureV1,
    input: &C,
    contract: &str,
    variant: &str,
    schema_profile: bool,
) -> bool {
    if !literal(c, input, "contract", contract) || !literal(c, input, "variant", variant) {
        return false;
    }
    if exact(input, &["contract", "variant", "fields"]) {
        return matches!(field(c, input, "fields"), Some(C::Object(_)));
    }
    schema_profile
        && exact(input, &["contract", "variant", "coverage", "schema"])
        && literal(c, input, "coverage", "outer-structure-only")
        && field(c, input, "schema").is_some_and(|v| schema(c, v))
}

pub(super) fn recognize_facet(
    c: &InertCaptureV1,
    contract: &str,
    variant: &str,
    profile: &str,
    recorder: &mut Recorder,
) -> bool {
    let timing = recorder.begin_service_timing();
    let accepted = field(c, c.root(), "input")
        .is_some_and(|input| recognize_profile(c, input, contract, variant, profile));
    recorder.record(Observation {
        boundary: Boundary::DiscoverySchemaRecognition,
        disposition: if accepted {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: timing.finish(),
    });
    accepted
}
fn recognize_profile(
    c: &InertCaptureV1,
    input: &C,
    contract: &str,
    variant: &str,
    profile: &str,
) -> bool {
    use crate::coordinator::handler_declaration::{
        ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1, ordinary_schema_matches_v1,
    };
    use crate::coordinator::*;
    if ![
        OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V2,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V3,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V4,
    ]
    .contains(&profile)
    {
        return false;
    }
    if [
        OWNED_CONTRACT_DISCOVERY_PROFILE_V3,
        OWNED_CONTRACT_DISCOVERY_PROFILE_V4,
    ]
    .contains(&profile)
        && contract == ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1
    {
        return variant == "ordinary-edit"
            && exact(input, &["contract", "variant", "coverage", "schema"])
            && literal(c, input, "contract", contract)
            && literal(c, input, "variant", variant)
            && literal(c, input, "coverage", "outer-structure-only")
            && field(c, input, "schema")
                .is_some_and(|schema| ordinary_schema_matches_v1(c, schema));
    }
    recognize(
        c,
        input,
        contract,
        variant,
        profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1,
    )
}
