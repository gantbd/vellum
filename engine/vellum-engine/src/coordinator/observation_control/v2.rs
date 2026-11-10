//! Versioned native control sum; V1 stays exhaustive and unchanged.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

/// Additive Rust profile; the exhaustive V1 executable vocabulary is unchanged.
pub enum ObservationStopPortV2<'a> {
    Existing(ObservationStopPortV1<'a>),
    SourcePublication {
        stream: &'a SourcePublicationStreamIdV1,
        control: &'a SourceReadContextControlV1<'a>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStopErrorV2 {
    Existing(ObservationStopErrorV1),
    SourcePublication(SourcePublicationErrorV1),
    InvalidRequest,
    Unavailable,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObservationStopTargetV2<'h> {
    Read(ReadObservationStopTargetV1<'h>),
    Invocation(&'h InvocationObservationContextIdV1),
    NativeOperation(&'h NativeOperationObservationIdV1),
    Discovery(&'h DiscoveryContextIdV1),
    SourcePublication(&'h SourcePublicationStreamIdV1),
}
#[derive(Clone, Copy)]
pub enum ObservationStopEnvelopeValueV2<'a, 'h> {
    Object(&'a [(&'a str, ObservationStopEnvelopeValueV2<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Context(ObservationStopTargetV2<'h>),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedObservationStopArgumentsV2<'h> {
    target: ObservationStopTargetV2<'h>,
}
impl<'h> NormalizedObservationStopArgumentsV2<'h> {
    pub fn target(self) -> ObservationStopTargetV2<'h> {
        self.target
    }
}
pub struct DecodedObservationStopRequestV2<'a, 'h, 'p> {
    pub envelope: &'a ObservationStopEnvelopeValueV2<'a, 'h>,
    /// First-party native lowering, never constructed from envelope data.
    pub native: ObservationStopPortV2<'p>,
}
macro_rules! redact { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<native-observation-stop:redacted>")
    }
})+ }; }
redact!(
    ObservationStopPortV2<'_>,
    ObservationStopTargetV2<'_>,
    ObservationStopEnvelopeValueV2<'_, '_>,
    NormalizedObservationStopArgumentsV2<'_>,
    DecodedObservationStopRequestV2<'_, '_, '_>
);

impl ObservationStopPortV2<'_> {
    pub(super) fn matches(&self, target: ObservationStopTargetV2<'_>) -> bool {
        match (self, target) {
            (Self::Existing(ObservationStopPortV1::Read(p)), ObservationStopTargetV2::Read(t)) => {
                p.target() == t
            }
            (
                Self::Existing(ObservationStopPortV1::Invocation { observer, .. }),
                ObservationStopTargetV2::Invocation(t),
            ) => t.matches_context(observer),
            (
                Self::Existing(ObservationStopPortV1::NativeOperation { observer, .. }),
                ObservationStopTargetV2::NativeOperation(t),
            ) => *observer == t,
            (
                Self::Existing(ObservationStopPortV1::Discovery { context, .. }),
                ObservationStopTargetV2::Discovery(t),
            ) => *context == t,
            (
                Self::SourcePublication { stream, .. },
                ObservationStopTargetV2::SourcePublication(t),
            ) => *stream == t,
            _ => false,
        }
    }
}
impl ObservationStopErrorV2 {
    pub(super) fn faulted(self) -> bool {
        matches!(self, Self::Existing(e) if e.faulted())
            || matches!(
                self,
                Self::SourcePublication(SourcePublicationErrorV1::Faulted)
            )
    }
}
fn fields<'a, 'h>(
    v: &'a ObservationStopEnvelopeValueV2<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, ObservationStopEnvelopeValueV2<'a, 'h>)], ObservationStopErrorV2> {
    let ObservationStopEnvelopeValueV2::Object(rows) = v else {
        return Err(ObservationStopErrorV2::InvalidRequest);
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(p, _)| p == k))
    {
        return Err(ObservationStopErrorV2::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(
    rows: &'a [(&'a str, ObservationStopEnvelopeValueV2<'a, 'h>)],
    key: &str,
) -> Result<&'a ObservationStopEnvelopeValueV2<'a, 'h>, ObservationStopErrorV2> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(ObservationStopErrorV2::InvalidRequest)
}
fn text<'a>(v: &ObservationStopEnvelopeValueV2<'a, '_>) -> Result<&'a str, ObservationStopErrorV2> {
    match v {
        ObservationStopEnvelopeValueV2::Data(InertValueV1::Text(t)) => Ok(t),
        _ => Err(ObservationStopErrorV2::InvalidRequest),
    }
}
fn normalize<'h>(
    envelope: &ObservationStopEnvelopeValueV2<'_, 'h>,
) -> Result<NormalizedObservationStopArgumentsV2<'h>, ObservationStopErrorV2> {
    use ObservationStopEnvelopeValueV2 as V;
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(ObservationStopErrorV2::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !matches!(
        (route, field(header, keys[0])?),
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
        return Err(ObservationStopErrorV2::InvalidRequest);
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "context"],
    )?;
    if text(field(args, "contract")?)? != "read-context/1"
        || text(field(args, "operation")?)? != "stop-observation"
    {
        return Err(ObservationStopErrorV2::InvalidRequest);
    }
    let V::Context(target) = field(args, "context")? else {
        return Err(ObservationStopErrorV2::InvalidRequest);
    };
    Ok(NormalizedObservationStopArgumentsV2 { target: *target })
}
/// Fixed 2/4/3-field passive native grammar. No transport, allocation or owner lookup.
pub fn normalize_observation_stop_envelope_v2<'h>(
    envelope: &ObservationStopEnvelopeValueV2<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedObservationStopArgumentsV2<'h>, ObservationStopErrorV2> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope);
    r.record(Observation {
        boundary: Boundary::ObservationStopArgumentNormalizationV2,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: timing.finish(),
    });
    result
}
impl SourceCoordinator {
    /// One native control; stop grants no disclosure and retires no retention.
    pub fn stop_observation_facade_v2(
        &mut self,
        port: ObservationStopPortV2<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV2> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match port {
            ObservationStopPortV2::Existing(p) => self
                .stop_observation_facade_v1(p, r)
                .map_err(ObservationStopErrorV2::Existing),
            ObservationStopPortV2::SourcePublication { stream, control } => self
                .stop_source_publication_stream_v1(stream, control, r)
                .map_err(ObservationStopErrorV2::SourcePublication),
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationStopFacadeV2,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(ObservationStopErrorV2::Existing(e)) if e.faulted() => Disposition::Unknown,
            Err(ObservationStopErrorV2::SourcePublication(SourcePublicationErrorV1::Faulted)) => {
                Disposition::Unknown
            }
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::ObservationStopFacadeV2,
            disposition,
            service_time: timing.finish(),
        });
        result
    }
    /// Original target-role binding precedes native current-control evaluation.
    pub fn stop_decoded_observation_facade_v2(
        &mut self,
        request: DecodedObservationStopRequestV2<'_, '_, '_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV2> {
        let validated = normalize_observation_stop_envelope_v2(request.envelope, r).and_then(|n| {
            if request.native.matches(n.target()) {
                Ok(())
            } else {
                Err(ObservationStopErrorV2::Unavailable)
            }
        });
        if let Err(e) = validated {
            r.record(Observation {
                boundary: Boundary::ObservationStopFacadeV2,
                disposition: Disposition::Refused,
                service_time: None,
            });
            return Err(e);
        }
        self.stop_observation_facade_v2(request.native, r)
    }
}
