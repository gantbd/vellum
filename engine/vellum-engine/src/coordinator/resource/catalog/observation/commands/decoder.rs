//! Exact passive F19 grammar over native opaque roles, not a wire codec.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub enum ResourceRegistryCommandEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ResourceRegistryCommandEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Command(&'h ResourceRegistryCommandIdV1),
    Stream(&'h ResourceRegistryStreamIdV1),
    Preparation(&'h ResourceRegistryPreparationIdV1),
}
pub struct ResourceRegistryCommandPortV1<'a> {
    pub command: &'a ResourceRegistryCommandIdV1,
    pub request: &'a ResourceRegistryCommandRequestV1<'a>,
}
#[derive(Clone, Copy)]
pub struct NormalizedResourceRegistryCommandArgumentsV1<'h> {
    command: &'h ResourceRegistryCommandIdV1,
    stream: Option<&'h ResourceRegistryStreamIdV1>,
    preparation: Option<&'h ResourceRegistryPreparationIdV1>,
}
impl<'h> NormalizedResourceRegistryCommandArgumentsV1<'h> {
    pub fn command(self) -> &'h ResourceRegistryCommandIdV1 {
        self.command
    }
    pub fn stream(self) -> Option<&'h ResourceRegistryStreamIdV1> {
        self.stream
    }
    pub fn preparation(self) -> Option<&'h ResourceRegistryPreparationIdV1> {
        self.preparation
    }
    pub fn operation(self) -> &'static str {
        if self.stream.is_some() {
            "authorized-resynchronization"
        } else {
            "establish-bounded-stream"
        }
    }
    pub(in crate::coordinator) fn matches(&self, port: &ResourceRegistryCommandPortV1<'_>) -> bool {
        self.command == port.command
            && self.preparation == port.request.preparation
            && match (self.stream, port.request.operation) {
                (None, ResourceRegistryCommandOperationV1::Establish) => true,
                (Some(a), ResourceRegistryCommandOperationV1::Resynchronize { stream }) => {
                    a == stream
                }
                _ => false,
            }
    }
}
redacted!(
    ResourceRegistryCommandEnvelopeValueV1<'_, '_>,
    ResourceRegistryCommandPortV1<'_>,
    NormalizedResourceRegistryCommandArgumentsV1<'_>
);
type Value<'a, 'h> = ResourceRegistryCommandEnvelopeValueV1<'a, 'h>;
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
fn normalize<'h>(v: &Value<'_, 'h>) -> Result<NormalizedResourceRegistryCommandArgumentsV1<'h>> {
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
            Value::Data(InertValueV1::Number(18.0))
        )
    } else {
        text(field(header, "name")?, "control-observation")
    }) || !matches!(
        field(header, "version")?,
        Value::Data(InertValueV1::Number(1.0))
    ) {
        return Err(ResourceError::Malformed);
    }
    let variant = field(header, "variant")?;
    let resync = if text(variant, "establish-bounded-stream") {
        false
    } else if text(variant, "authorized-resynchronization") {
        true
    } else {
        return Err(ResourceError::Malformed);
    };
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "command", "stream", "preparation"],
    )?;
    if !text(
        field(args, "contract")?,
        RESOURCE_REGISTRY_COMMAND_PROFILE_V1,
    ) || !text(
        field(args, "operation")?,
        if resync {
            "authorized-resynchronization"
        } else {
            "establish-bounded-stream"
        },
    ) {
        return Err(ResourceError::Malformed);
    }
    let Value::Command(command) = field(args, "command")? else {
        return Err(ResourceError::Malformed);
    };
    let stream = match (resync, field(args, "stream")?) {
        (false, Value::Data(InertValueV1::Null)) => None,
        (true, Value::Stream(s)) => Some(*s),
        _ => return Err(ResourceError::Malformed),
    };
    let preparation = match field(args, "preparation")? {
        Value::Data(InertValueV1::Null) => None,
        Value::Preparation(p) => Some(*p),
        _ => return Err(ResourceError::Malformed),
    };
    Ok(NormalizedResourceRegistryCommandArgumentsV1 {
        command,
        stream,
        preparation,
    })
}
/// Fixed 2/4/5-field grammar with no ordinary free text, allocation or callback.
pub fn normalize_resource_registry_command_envelope_v1<'h>(
    envelope: &Value<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedResourceRegistryCommandArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope);
    report(
        r,
        Boundary::RegistryCommandArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// One existing native request or retained lookup; never returns baseline data.
    pub fn request_decoded_resource_registry_command_v1(
        &mut self,
        envelope: &Value<'_, '_>,
        port: &ResourceRegistryCommandPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryCommandReplyV1> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_resource_registry_command_envelope_v1(envelope, r)?;
            if !n.matches(port) {
                return Err(ResourceError::Unavailable);
            }
            let reply = self.request_resource_registry_command_v1(port.command, port.request, r)?;
            let (ResourceRegistryCommandReplyV1::Settled { command }
            | ResourceRegistryCommandReplyV1::ObserveExisting { command }) = &reply;
            if command != n.command {
                self.registry_command_mut_v1(port.command).phase =
                    ResourceRegistryCommandPhaseV1::Faulted;
                return Err(ResourceError::Faulted);
            }
            Ok(reply)
        }));
        match caught {
            Ok(result) => {
                report(r, Boundary::RegistryCommandFacade, &result, timing.finish());
                result
            }
            Err(cause) => {
                r.record(Observation {
                    boundary: Boundary::RegistryCommandFacade,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                // Native execution already converts private causes to its closed marker.
                std::panic::resume_unwind(cause)
            }
        }
    }
}
