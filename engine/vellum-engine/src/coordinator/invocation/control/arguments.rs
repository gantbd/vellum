//! Fixed passive ingress; original opaque tickets are never parsed from data.
use super::*;
use crate::capture::InertValueV1;

pub const INVOCATION_CONTROL_PROFILE_V1: &str = "invocation-control/1";

#[derive(Clone, Copy)]
pub enum InvocationCancellationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, InvocationCancellationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Invocation(&'h InvocationId),
}
impl std::fmt::Debug for InvocationCancellationEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<invocation-cancellation-input:redacted>")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationCancellationIngressErrorV1 {
    InvalidRequest,
    Execution(InvocationError),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedInvocationCancellationV1<'h> {
    invocation: &'h InvocationId,
}
impl<'h> NormalizedInvocationCancellationV1<'h> {
    pub fn invocation(self) -> &'h InvocationId {
        self.invocation
    }
}
impl std::fmt::Debug for NormalizedInvocationCancellationV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<invocation-cancellation-arguments:redacted>")
    }
}
type E = InvocationCancellationIngressErrorV1;
type V<'a, 'h> = InvocationCancellationEnvelopeValueV1<'a, 'h>;
fn fields<'a, 'h>(v: &V<'a, 'h>, keys: &[&str]) -> Result<&'a [(&'a str, V<'a, 'h>)], E> {
    let V::Object(rows) = v else {
        return Err(E::InvalidRequest);
    };
    // Reject the size before touching any rows. No recursive walk or identity decoding.
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(p, _)| p == k))
    {
        return Err(E::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(rows: &'a [(&str, V<'a, 'h>)], key: &str) -> Result<&'a V<'a, 'h>, E> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(E::InvalidRequest)
}
fn text<'a>(v: &V<'a, '_>) -> Result<&'a str, E> {
    match v {
        V::Data(InertValueV1::Text(s)) => Ok(s),
        _ => Err(E::InvalidRequest),
    }
}
fn normalize<'h>(envelope: &V<'_, 'h>) -> Result<NormalizedInvocationCancellationV1<'h>, E> {
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(E::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !matches!(
        (route, field(header, keys[0])?),
        ("typed", V::Data(InertValueV1::Number(17.0)))
            | ("dynamic", V::Data(InertValueV1::Text("control-execution")))
    ) || !matches!(
        field(header, "version")?,
        V::Data(InertValueV1::Number(1.0))
    ) || text(field(header, "variant")?)? != "request-cancellation"
    {
        return Err(E::InvalidRequest);
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "invocation"],
    )?;
    if text(field(args, "contract")?)? != INVOCATION_CONTROL_PROFILE_V1
        || text(field(args, "operation")?)? != "request-cancellation"
    {
        return Err(E::InvalidRequest);
    }
    let V::Invocation(invocation) = field(args, "invocation")? else {
        return Err(E::InvalidRequest);
    };
    Ok(NormalizedInvocationCancellationV1 { invocation })
}
/// Exact 2/4/3-field grammar; bounded metadata only, without grants or owner lookup.
pub fn normalize_invocation_cancellation_v1<'h>(
    envelope: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedInvocationCancellationV1<'h>, E> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope);
    r.record(Observation {
        boundary: Boundary::InvocationCancellationArgumentNormalization,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: timer.finish(),
    });
    result
}
impl SourceCoordinator {
    /// Protected direct ingress does not require discovery, observer capacity or
    /// content access. Trusted current control facts remain separate from input.
    pub fn request_decoded_invocation_cancellation_v1(
        &mut self,
        envelope: &V<'_, '_>,
        f: &TrustedInvocationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), E> {
        let normalized = normalize_invocation_cancellation_v1(envelope, r)?;
        self.request_invocation_stop(normalized.invocation(), f, r)
            .map_err(E::Execution)
    }
}

#[cfg(test)]
pub(in crate::coordinator) mod tests;
