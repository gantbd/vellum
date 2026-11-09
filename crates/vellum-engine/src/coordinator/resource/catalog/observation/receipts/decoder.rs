//! Fixed passive native grammar; no wire, script execution or policy suppliers.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub enum ResourceRegistryReceiptEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ResourceRegistryReceiptEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Command(&'h ResourceRegistryCommandIdV1),
    Context(&'h ResourceRegistryReceiptContextIdV1),
}
pub struct ResourceRegistryReceiptPortV1<'a> {
    pub command: &'a ResourceRegistryCommandIdV1,
    pub context: &'a ResourceRegistryReceiptContextIdV1,
    pub scope: &'a ResourceRegistryObservationScopeV1,
    pub preparation: &'a ResourceRegistryPreparationIdV1,
    pub current: &'a TrustedResourceRegistryReceiptFactsV1<'a>,
}
#[derive(Clone, Copy)]
pub struct NormalizedResourceRegistryReceiptArgumentsV1<'h> {
    command: &'h ResourceRegistryCommandIdV1,
    context: &'h ResourceRegistryReceiptContextIdV1,
}
impl<'h> NormalizedResourceRegistryReceiptArgumentsV1<'h> {
    pub fn command(self) -> &'h ResourceRegistryCommandIdV1 {
        self.command
    }
    pub fn context(self) -> &'h ResourceRegistryReceiptContextIdV1 {
        self.context
    }
}
redacted!(
    ResourceRegistryReceiptEnvelopeValueV1<'_, '_>,
    ResourceRegistryReceiptPortV1<'_>,
    NormalizedResourceRegistryReceiptArgumentsV1<'_>
);
type Value<'a, 'h> = ResourceRegistryReceiptEnvelopeValueV1<'a, 'h>;
fn fields<'a, 'h>(v: &Value<'a, 'h>, keys: &[&str]) -> Result<&'a [(&'a str, Value<'a, 'h>)]> {
    let Value::Object(rows) = v else {
        return Err(ResourceError::Malformed);
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(p, _)| p == k))
    {
        return Err(ResourceError::Malformed);
    }
    Ok(rows)
}
fn field<'a, 'h>(rows: &'a [(&'a str, Value<'a, 'h>)], key: &str) -> Result<&'a Value<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(ResourceError::Malformed)
}
fn text(v: &Value<'_, '_>, expected: &str) -> bool {
    matches!(v, Value::Data(InertValueV1::Text(s)) if *s == expected)
}
fn normalize<'h>(v: &Value<'_, 'h>) -> Result<NormalizedResourceRegistryReceiptArgumentsV1<'h>> {
    let outer = fields(v, &["route", "request"])?;
    let route = field(outer, "route")?;
    let keys = if text(route, "typed") {
        ["family", "variant", "version", "arguments"]
    } else if text(route, "dynamic") {
        ["name", "variant", "version", "arguments"]
    } else {
        return Err(ResourceError::Malformed);
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !(if keys[0] == "family" {
        matches!(
            field(header, "family")?,
            Value::Data(InertValueV1::Number(5.0))
        )
    } else {
        text(field(header, "name")?, "inspect-execution")
    }) || !text(field(header, "variant")?, "retained-outcome")
        || !matches!(
            field(header, "version")?,
            Value::Data(InertValueV1::Number(1.0))
        )
    {
        return Err(ResourceError::Malformed);
    }
    let args = fields(
        field(header, "arguments")?,
        &[
            "contract",
            "operation",
            "command",
            "context",
            "profile",
            "facet",
        ],
    )?;
    if !text(
        field(args, "contract")?,
        RESOURCE_REGISTRY_RECEIPT_PROFILE_V1,
    ) || !text(field(args, "operation")?, "retained-outcome")
        || !text(
            field(args, "profile")?,
            RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1,
        )
    {
        return Err(ResourceError::Malformed);
    }
    let facet = fields(field(args, "facet")?, &["kind"])?;
    if !text(field(facet, "kind")?, "registry-baseline") {
        return Err(ResourceError::Malformed);
    }
    let Value::Command(command) = field(args, "command")? else {
        return Err(ResourceError::Malformed);
    };
    let Value::Context(context) = field(args, "context")? else {
        return Err(ResourceError::Malformed);
    };
    Ok(NormalizedResourceRegistryReceiptArgumentsV1 { command, context })
}
/// Exact 2/4/6/1-field grammar, fixed constant strings, out-of-band native roles.
/// No allocation, arbitrary subtree traversal or effectful input access occurs.
pub fn normalize_resource_registry_receipt_envelope_v1<'h>(
    envelope: &Value<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedResourceRegistryReceiptArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope);
    report(
        r,
        Boundary::RegistryReceiptArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// Both routes lower to one native historical query, never command execution.
    /// Trusted port and immutable facts are native lowering, not envelope fields.
    pub fn release_decoded_resource_registry_receipt_v1(
        &mut self,
        envelope: &Value<'_, '_>,
        port: &ResourceRegistryReceiptPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryReceiptReplyV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let n = normalize_resource_registry_receipt_envelope_v1(envelope, r)?;
            if n.command != port.command
                || n.context != port.context
                || port.scope.context.profile != RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1
            {
                return Err(ResourceError::Unavailable);
            }
            let reply = self.release_resource_registry_receipt_v1(
                port.context,
                port.command,
                port.scope,
                port.preparation,
                port.current,
                r,
            )?;
            self.validate_resource_registry_receipt_reply_v1(
                port.context,
                port.command,
                port.scope,
                &reply,
                r,
            )?;
            Ok(reply)
        })();
        report(r, Boundary::RegistryReceiptFacade, &result, timing.finish());
        result
    }
}
