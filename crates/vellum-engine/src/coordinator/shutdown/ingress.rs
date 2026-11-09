//! Passive, fixed native header. Opaque IDs and current authority are not data codecs.
use super::*;
use crate::capture::InertValueV1;
pub const SHUTDOWN_DISPOSITION_PROFILE_V1: &str = "shutdown-disposition/1";

#[derive(Clone, Copy)]
pub enum ShutdownDispositionEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ShutdownDispositionEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Fence(&'h ShutdownFenceIdV1),
    Document(&'h DocumentId),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownDispositionIngressErrorV1 {
    InvalidRequest,
    Execution(CoordinatorError),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedShutdownDispositionV1<'h> {
    fence: &'h ShutdownFenceIdV1,
    document: &'h DocumentId,
}
impl<'h> NormalizedShutdownDispositionV1<'h> {
    pub fn fence(self) -> &'h ShutdownFenceIdV1 {
        self.fence
    }
    pub fn document(self) -> &'h DocumentId {
        self.document
    }
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<shutdown-disposition:redacted>")
    }
})+ }; }
redacted!(
    ShutdownDispositionEnvelopeValueV1<'_, '_>,
    NormalizedShutdownDispositionV1<'_>
);
type E = ShutdownDispositionIngressErrorV1;
type V<'a, 'h> = ShutdownDispositionEnvelopeValueV1<'a, 'h>;
fn fields<'a, 'h>(v: &V<'a, 'h>, keys: &[&str]) -> Result<&'a [(&'a str, V<'a, 'h>)], E> {
    let V::Object(rows) = v else {
        return Err(E::InvalidRequest);
    };
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
fn normalize<'h>(envelope: &V<'_, 'h>) -> Result<NormalizedShutdownDispositionV1<'h>, E> {
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
    ) || text(field(header, "variant")?)? != "shutdown-disposition"
    {
        return Err(E::InvalidRequest);
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "fence", "document"],
    )?;
    if text(field(args, "contract")?)? != SHUTDOWN_DISPOSITION_PROFILE_V1
        || text(field(args, "operation")?)? != "shutdown-disposition"
    {
        return Err(E::InvalidRequest);
    }
    let (V::Fence(fence), V::Document(document)) =
        (field(args, "fence")?, field(args, "document")?)
    else {
        return Err(E::InvalidRequest);
    };
    Ok(NormalizedShutdownDispositionV1 { fence, document })
}
/// Exact 2/4/4-field borrowed grammar: no recursion, payload allocation or authority.
pub fn normalize_shutdown_disposition_v1<'h>(
    envelope: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedShutdownDispositionV1<'h>, E> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope);
    r.record(Observation {
        boundary: Boundary::ShutdownDispositionArgumentNormalization,
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
    /// Execute only this original engine-retained consent plan. Discovery is not
    /// required for protected direct progress; current native guards are unchanged.
    pub fn request_decoded_shutdown_disposition_v1(
        &mut self,
        envelope: &V<'_, '_>,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<LifecycleState, E> {
        let n = normalize_shutdown_disposition_v1(envelope, r)?;
        self.dispose_accepted_shutdown_document_v1(n.fence(), n.document(), facts, coverage, r)
            .map_err(E::Execution)
    }
}
