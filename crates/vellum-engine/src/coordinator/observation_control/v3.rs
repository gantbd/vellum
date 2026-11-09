//! Complete current native observation stop sum; earlier executable enums stay closed.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

/// Additive Rust profile; the exhaustive V2 executable vocabulary is unchanged.
pub enum ObservationStopPortV3<'a> {
    Existing(ObservationStopPortV2<'a>),
    FacetPublication {
        stream: &'a FacetPublicationStreamIdV1,
        control: &'a FacetPublicationControlV1<'a>,
    },
    CollectionPublication {
        stream: &'a CollectionPublicationStreamIdV1,
        control: &'a CollectionPublicationControlV1<'a>,
    },
    ResourceRegistry {
        stream: &'a ResourceRegistryStreamIdV1,
        control: &'a ResourceRegistryObservationControlV1<'a>,
    },
    RegistryReceipt {
        context: &'a ResourceRegistryReceiptContextIdV1,
        control: &'a ResourceRegistryObservationControlV1<'a>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStopErrorV3 {
    Existing(ObservationStopErrorV2),
    FacetPublication(FacetPublicationErrorV1),
    CollectionPublication(CollectionPublicationErrorV1),
    ResourceRegistry(crate::resource::ResourceError),
    InvalidRequest,
    Unavailable,
}
impl ObservationStopErrorV3 {
    pub(in crate::coordinator) fn faulted(self) -> bool {
        match self {
            Self::Existing(e) => e.faulted(),
            Self::FacetPublication(FacetPublicationErrorV1::Faulted)
            | Self::CollectionPublication(CollectionPublicationErrorV1::Faulted)
            | Self::ResourceRegistry(crate::resource::ResourceError::Faulted) => true,
            _ => false,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObservationStopTargetV3<'h> {
    Existing(ObservationStopTargetV2<'h>),
    FacetPublication(&'h FacetPublicationStreamIdV1),
    CollectionPublication(&'h CollectionPublicationStreamIdV1),
    ResourceRegistry(&'h ResourceRegistryStreamIdV1),
    RegistryReceipt(&'h ResourceRegistryReceiptContextIdV1),
}
#[derive(Clone, Copy)]
pub enum ObservationStopEnvelopeValueV3<'a, 'h> {
    Object(&'a [(&'a str, ObservationStopEnvelopeValueV3<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Context(ObservationStopTargetV3<'h>),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedObservationStopArgumentsV3<'h> {
    target: ObservationStopTargetV3<'h>,
}
impl<'h> NormalizedObservationStopArgumentsV3<'h> {
    pub fn target(self) -> ObservationStopTargetV3<'h> {
        self.target
    }
}
pub struct DecodedObservationStopRequestV3<'a, 'h, 'p> {
    pub envelope: &'a ObservationStopEnvelopeValueV3<'a, 'h>,
    /// First-party native lowering, never constructed from envelope data.
    pub native: ObservationStopPortV3<'p>,
}
macro_rules! redact { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<native-observation-stop:redacted>")
    }
})+ }; }
redact!(
    ObservationStopPortV3<'_>,
    ObservationStopTargetV3<'_>,
    ObservationStopEnvelopeValueV3<'_, '_>,
    NormalizedObservationStopArgumentsV3<'_>,
    DecodedObservationStopRequestV3<'_, '_, '_>
);

impl ObservationStopPortV3<'_> {
    pub(in crate::coordinator) fn matches(&self, target: ObservationStopTargetV3<'_>) -> bool {
        match (self, target) {
            (Self::Existing(p), ObservationStopTargetV3::Existing(t)) => p.matches(t),
            (
                Self::FacetPublication { stream, .. },
                ObservationStopTargetV3::FacetPublication(t),
            ) => *stream == t,
            (
                Self::CollectionPublication { stream, .. },
                ObservationStopTargetV3::CollectionPublication(t),
            ) => *stream == t,
            (
                Self::ResourceRegistry { stream, .. },
                ObservationStopTargetV3::ResourceRegistry(t),
            ) => *stream == t,
            (
                Self::RegistryReceipt { context, .. },
                ObservationStopTargetV3::RegistryReceipt(t),
            ) => *context == t,
            _ => false,
        }
    }
}
fn fields<'a, 'h>(
    v: &'a ObservationStopEnvelopeValueV3<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, ObservationStopEnvelopeValueV3<'a, 'h>)], ObservationStopErrorV3> {
    let ObservationStopEnvelopeValueV3::Object(rows) = v else {
        return Err(ObservationStopErrorV3::InvalidRequest);
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(p, _)| p == k))
    {
        return Err(ObservationStopErrorV3::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(
    rows: &'a [(&'a str, ObservationStopEnvelopeValueV3<'a, 'h>)],
    key: &str,
) -> Result<&'a ObservationStopEnvelopeValueV3<'a, 'h>, ObservationStopErrorV3> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(ObservationStopErrorV3::InvalidRequest)
}
fn text<'a>(v: &ObservationStopEnvelopeValueV3<'a, '_>) -> Result<&'a str, ObservationStopErrorV3> {
    match v {
        ObservationStopEnvelopeValueV3::Data(InertValueV1::Text(t)) => Ok(t),
        _ => Err(ObservationStopErrorV3::InvalidRequest),
    }
}
fn normalize<'h>(
    envelope: &ObservationStopEnvelopeValueV3<'_, 'h>,
) -> Result<NormalizedObservationStopArgumentsV3<'h>, ObservationStopErrorV3> {
    normalize_context_operation(envelope, "stop-observation")
}
/// Called only with a first-party constant. Public stop profiles stay closed.
pub(in crate::coordinator) fn normalize_context_operation<'h>(
    envelope: &ObservationStopEnvelopeValueV3<'_, 'h>,
    operation: &str,
) -> Result<NormalizedObservationStopArgumentsV3<'h>, ObservationStopErrorV3> {
    use ObservationStopEnvelopeValueV3 as V;
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(ObservationStopErrorV3::InvalidRequest),
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
    ) || text(field(header, "variant")?)? != operation
    {
        return Err(ObservationStopErrorV3::InvalidRequest);
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "context"],
    )?;
    if text(field(args, "contract")?)? != "read-context/1"
        || text(field(args, "operation")?)? != operation
    {
        return Err(ObservationStopErrorV3::InvalidRequest);
    }
    let V::Context(target) = field(args, "context")? else {
        return Err(ObservationStopErrorV3::InvalidRequest);
    };
    Ok(NormalizedObservationStopArgumentsV3 { target: *target })
}
/// Fixed 2/4/3-field passive native grammar. No transport, allocation or owner lookup.
pub fn normalize_observation_stop_envelope_v3<'h>(
    envelope: &ObservationStopEnvelopeValueV3<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedObservationStopArgumentsV3<'h>, ObservationStopErrorV3> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope);
    r.record(Observation {
        boundary: Boundary::ObservationStopArgumentNormalizationV3,
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
    pub fn stop_observation_facade_v3(
        &mut self,
        port: ObservationStopPortV3<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV3> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match port {
            ObservationStopPortV3::Existing(p) => self
                .stop_observation_facade_v2(p, r)
                .map_err(ObservationStopErrorV3::Existing),
            ObservationStopPortV3::FacetPublication { stream, control } => self
                .stop_facet_publication_stream_v1(stream, control, r)
                .map_err(ObservationStopErrorV3::FacetPublication),
            ObservationStopPortV3::CollectionPublication { stream, control } => self
                .stop_collection_publication_stream_v1(stream, control, r)
                .map_err(ObservationStopErrorV3::CollectionPublication),
            ObservationStopPortV3::ResourceRegistry { stream, control } => self
                .stop_resource_registry_observation_v1(stream, control, r)
                .map_err(ObservationStopErrorV3::ResourceRegistry),
            ObservationStopPortV3::RegistryReceipt { context, control } => self
                .stop_resource_registry_receipt_v1(context, control, r)
                .map_err(ObservationStopErrorV3::ResourceRegistry),
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationStopFacadeV3,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(ObservationStopErrorV3::Existing(e)) if e.faulted() => Disposition::Unknown,
            Err(
                ObservationStopErrorV3::FacetPublication(FacetPublicationErrorV1::Faulted)
                | ObservationStopErrorV3::CollectionPublication(
                    CollectionPublicationErrorV1::Faulted,
                )
                | ObservationStopErrorV3::ResourceRegistry(crate::resource::ResourceError::Faulted),
            ) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::ObservationStopFacadeV3,
            disposition,
            service_time: timing.finish(),
        });
        result
    }
    /// Original target-role binding precedes native current-control evaluation.
    pub fn stop_decoded_observation_facade_v3(
        &mut self,
        request: DecodedObservationStopRequestV3<'_, '_, '_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV3> {
        let validated = normalize_observation_stop_envelope_v3(request.envelope, r).and_then(|n| {
            if request.native.matches(n.target()) {
                Ok(())
            } else {
                Err(ObservationStopErrorV3::Unavailable)
            }
        });
        if let Err(e) = validated {
            r.record(Observation {
                boundary: Boundary::ObservationStopFacadeV3,
                disposition: Disposition::Refused,
                service_time: None,
            });
            return Err(e);
        }
        self.stop_observation_facade_v3(request.native, r)
    }
}
