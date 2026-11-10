//! Exact passive-native F19 envelope. All ordinary accepted fields are closed
//! constants, so normalization owns no caller strings and allocates nothing.
//! This is not a wire parser, arbitrary-data copier or selector identity codec.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub enum ReadObservationStopTargetV1<'h> {
    Source(&'h SourceReadContextId),
    Annotation(&'h AnnotationReadContextId),
    VersionSet(VersionSetControlTargetV1<'h>),
    History(&'h HistoryInspectionIdV1),
}
impl PartialEq for ReadObservationStopTargetV1<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Source(a), Self::Source(b)) => a == b,
            (Self::Annotation(a), Self::Annotation(b)) => a == b,
            (Self::History(a), Self::History(b)) => a == b,
            (
                Self::VersionSet(VersionSetControlTargetV1::Context(a)),
                Self::VersionSet(VersionSetControlTargetV1::Context(b)),
            ) => a == b,
            (
                Self::VersionSet(VersionSetControlTargetV1::Member(a)),
                Self::VersionSet(VersionSetControlTargetV1::Member(b)),
            ) => a == b,
            _ => false,
        }
    }
}
impl Eq for ReadObservationStopTargetV1<'_> {}
#[derive(Clone, Copy)]
pub enum ReadObservationStopEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ReadObservationStopEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Context(ReadObservationStopTargetV1<'h>),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedReadObservationStopArgumentsV1<'h> {
    target: ReadObservationStopTargetV1<'h>,
}
impl<'h> NormalizedReadObservationStopArgumentsV1<'h> {
    pub fn target(self) -> ReadObservationStopTargetV1<'h> {
        self.target
    }
}
#[derive(Clone, Copy)]
pub struct DecodedReadObservationStopRequestV1<'a, 'h> {
    pub envelope: &'a ReadObservationStopEnvelopeValueV1<'a, 'h>,
    /// Trusted first-party lowering, never obtained from envelope fields.
    pub native: ReadObservationStopPortV1<'a>,
}
macro_rules! redact {($($t:ty),+)=>{$(impl std::fmt::Debug for $t {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str("<native-stop-arguments:redacted>") }
})+};}
redact!(
    ReadObservationStopTargetV1<'_>,
    ReadObservationStopEnvelopeValueV1<'_, '_>,
    NormalizedReadObservationStopArgumentsV1<'_>,
    DecodedReadObservationStopRequestV1<'_, '_>
);

impl<'a> ReadObservationStopPortV1<'a> {
    pub fn target(self) -> ReadObservationStopTargetV1<'a> {
        match self {
            Self::Source { context, .. } => ReadObservationStopTargetV1::Source(context),
            Self::Annotation { context, .. } => ReadObservationStopTargetV1::Annotation(context),
            Self::VersionSet { target, .. } => ReadObservationStopTargetV1::VersionSet(target),
            Self::History { context, .. } => ReadObservationStopTargetV1::History(context),
        }
    }
}
/// Pure canonicalization, not owner lookup, current control or a stop grant.
/// Exact records are bounded to 2/4/3 fields; accepted scalar spellings are fixed.
/// Unknown data is rejected without recursive traversal or capability inspection.
pub fn normalize_read_observation_stop_envelope_v1<'h>(
    envelope: &ReadObservationStopEnvelopeValueV1<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedReadObservationStopArgumentsV1<'h>, Error> {
    let timing = r.begin_service_timing();
    let out = normalize(envelope);
    timed_observation(
        r,
        Boundary::ReadObservationStopArgumentNormalization,
        if out.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        timing.finish(),
    );
    out
}
fn fields<'a, 'h>(
    value: &'a ReadObservationStopEnvelopeValueV1<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, ReadObservationStopEnvelopeValueV1<'a, 'h>)], Error> {
    let ReadObservationStopEnvelopeValueV1::Object(rows) = value else {
        return Err(Error::InvalidRequest);
    };
    // Test the fixed schema width before inspecting any caller-sized sequence.
    if rows.len() != keys.len()
        || rows.iter().any(|(key, _)| !keys.contains(key))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (key, _))| rows[..i].iter().any(|(prior, _)| prior == key))
    {
        return Err(Error::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(
    rows: &'a [(&'a str, ReadObservationStopEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a ReadObservationStopEnvelopeValueV1<'a, 'h>, Error> {
    rows.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value)
        .ok_or(Error::InvalidRequest)
}
fn text<'a>(value: &ReadObservationStopEnvelopeValueV1<'a, '_>) -> Result<&'a str, Error> {
    match value {
        ReadObservationStopEnvelopeValueV1::Data(InertValueV1::Text(s)) => Ok(s),
        _ => Err(Error::InvalidRequest),
    }
}
fn normalize<'h>(
    envelope: &ReadObservationStopEnvelopeValueV1<'_, 'h>,
) -> Result<NormalizedReadObservationStopArgumentsV1<'h>, Error> {
    use ReadObservationStopEnvelopeValueV1 as V;
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let header_keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &header_keys)?;
    if !matches!(
        (route, field(header, header_keys[0])?),
        ("typed", V::Data(InertValueV1::Number(18.0)))
            | (
                "dynamic",
                V::Data(InertValueV1::Text("control-observation"))
            )
    ) || !matches!(
        field(header, "version")?,
        V::Data(InertValueV1::Number(1.0))
    ) || text(field(header, "variant")?)? != "stop-observation"
    {
        return Err(Error::InvalidRequest);
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "context"],
    )?;
    if text(field(args, "contract")?)? != "read-context/1"
        || text(field(args, "operation")?)? != "stop-observation"
    {
        return Err(Error::InvalidRequest);
    }
    let V::Context(target) = field(args, "context")? else {
        return Err(Error::InvalidRequest);
    };
    Ok(NormalizedReadObservationStopArgumentsV1 { target: *target })
}
impl SourceCoordinator {
    /// No ordinary read admission. This passive native envelope is not a hosted
    /// wire decoder or a claim of protected ingress capacity outside the engine.
    pub fn stop_decoded_read_observation_facade_v1(
        &mut self,
        request: DecodedReadObservationStopRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<(), SourceReadContextErrorV1> {
        let normalized = match normalize_read_observation_stop_envelope_v1(request.envelope, r) {
            Ok(value) => value,
            Err(error) => {
                observation(r, Boundary::ReadObservationStopFacade, Disposition::Refused);
                return Err(error);
            }
        };
        if normalized.target != request.native.target() {
            observation(r, Boundary::ReadObservationStopFacade, Disposition::Refused);
            return Err(Error::Unavailable);
        }
        self.stop_read_observation_facade_v1(request.native, r)
    }
}
