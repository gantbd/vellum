//! Complete current native observation stop sum; earlier executable enums stay closed.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
pub(in crate::coordinator) mod tests;

/// Adds original publication-command/receipt pairs; V1-V3 stay exhaustive and unchanged.
pub enum ObservationStopPortV4<'a> {
    Existing(ObservationStopPortV3<'a>),
    SourceReceipt {
        command: &'a SourcePublicationCommandIdV1,
        context: &'a SourcePublicationReceiptContextIdV1,
        control: &'a SourceReadContextControlV1<'a>,
    },
    FacetReceipt {
        command: &'a FacetPublicationCommandIdV1,
        context: &'a FacetPublicationReceiptContextIdV1,
        control: &'a FacetPublicationControlV1<'a>,
    },
    CollectionReceipt {
        command: &'a CollectionPublicationCommandIdV1,
        context: &'a CollectionPublicationReceiptContextIdV1,
        control: &'a CollectionPublicationControlV1<'a>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStopErrorV4 {
    Existing(ObservationStopErrorV3),
    SourceReceipt(SourcePublicationErrorV1),
    FacetReceipt(FacetPublicationErrorV1),
    CollectionReceipt(CollectionPublicationErrorV1),
    InvalidRequest,
    Unavailable,
}
impl ObservationStopErrorV4 {
    pub(in crate::coordinator) fn faulted(self) -> bool {
        match self {
            Self::Existing(e) => e.faulted(),
            Self::SourceReceipt(SourcePublicationErrorV1::Faulted) => true,
            Self::FacetReceipt(FacetPublicationErrorV1::Faulted) => true,
            Self::CollectionReceipt(CollectionPublicationErrorV1::Faulted) => true,
            _ => false,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObservationStopTargetV4<'h> {
    Existing(ObservationStopTargetV3<'h>),
    SourceReceipt {
        command: &'h SourcePublicationCommandIdV1,
        context: &'h SourcePublicationReceiptContextIdV1,
    },
    FacetReceipt {
        command: &'h FacetPublicationCommandIdV1,
        context: &'h FacetPublicationReceiptContextIdV1,
    },
    CollectionReceipt {
        command: &'h CollectionPublicationCommandIdV1,
        context: &'h CollectionPublicationReceiptContextIdV1,
    },
}
#[derive(Clone, Copy)]
pub enum ObservationStopEnvelopeValueV4<'a, 'h> {
    Object(&'a [(&'a str, ObservationStopEnvelopeValueV4<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Context(ObservationStopTargetV4<'h>),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedObservationStopArgumentsV4<'h> {
    target: ObservationStopTargetV4<'h>,
}
impl<'h> NormalizedObservationStopArgumentsV4<'h> {
    pub fn target(self) -> ObservationStopTargetV4<'h> {
        self.target
    }
}
pub struct DecodedObservationStopRequestV4<'a, 'h, 'p> {
    pub envelope: &'a ObservationStopEnvelopeValueV4<'a, 'h>,
    /// First-party native lowering, never constructed from envelope data.
    pub native: ObservationStopPortV4<'p>,
}
macro_rules! redact { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<native-observation-stop:redacted>")
    }
})+ }; }
redact!(
    ObservationStopPortV4<'_>,
    ObservationStopTargetV4<'_>,
    ObservationStopEnvelopeValueV4<'_, '_>,
    NormalizedObservationStopArgumentsV4<'_>,
    DecodedObservationStopRequestV4<'_, '_, '_>
);

impl ObservationStopPortV4<'_> {
    pub(in crate::coordinator) fn matches(&self, target: ObservationStopTargetV4<'_>) -> bool {
        match (self, target) {
            (Self::Existing(p), ObservationStopTargetV4::Existing(t)) => p.matches(t),
            (
                Self::SourceReceipt {
                    command, context, ..
                },
                ObservationStopTargetV4::SourceReceipt {
                    command: c,
                    context: t,
                },
            ) => *command == c && *context == t,
            (
                Self::FacetReceipt {
                    command, context, ..
                },
                ObservationStopTargetV4::FacetReceipt {
                    command: c,
                    context: t,
                },
            ) => *command == c && *context == t,
            (
                Self::CollectionReceipt {
                    command, context, ..
                },
                ObservationStopTargetV4::CollectionReceipt {
                    command: c,
                    context: t,
                },
            ) => *command == c && *context == t,
            _ => false,
        }
    }
}
fn fields<'a, 'h>(
    v: &'a ObservationStopEnvelopeValueV4<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, ObservationStopEnvelopeValueV4<'a, 'h>)], ObservationStopErrorV4> {
    let ObservationStopEnvelopeValueV4::Object(rows) = v else {
        return Err(ObservationStopErrorV4::InvalidRequest);
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(p, _)| p == k))
    {
        return Err(ObservationStopErrorV4::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(
    rows: &'a [(&'a str, ObservationStopEnvelopeValueV4<'a, 'h>)],
    key: &str,
) -> Result<&'a ObservationStopEnvelopeValueV4<'a, 'h>, ObservationStopErrorV4> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(ObservationStopErrorV4::InvalidRequest)
}
fn text<'a>(v: &ObservationStopEnvelopeValueV4<'a, '_>) -> Result<&'a str, ObservationStopErrorV4> {
    match v {
        ObservationStopEnvelopeValueV4::Data(InertValueV1::Text(t)) => Ok(t),
        _ => Err(ObservationStopErrorV4::InvalidRequest),
    }
}
fn normalize<'h>(
    envelope: &ObservationStopEnvelopeValueV4<'_, 'h>,
) -> Result<NormalizedObservationStopArgumentsV4<'h>, ObservationStopErrorV4> {
    normalize_context_operation_v4(envelope, "stop-observation")
}
/// Called only with a first-party constant. Public stop profiles stay closed.
pub(in crate::coordinator) fn normalize_context_operation_v4<'h>(
    envelope: &ObservationStopEnvelopeValueV4<'_, 'h>,
    operation: &str,
) -> Result<NormalizedObservationStopArgumentsV4<'h>, ObservationStopErrorV4> {
    use ObservationStopEnvelopeValueV4 as V;
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(ObservationStopErrorV4::InvalidRequest),
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
        return Err(ObservationStopErrorV4::InvalidRequest);
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "context"],
    )?;
    if text(field(args, "contract")?)? != "read-context/1"
        || text(field(args, "operation")?)? != operation
    {
        return Err(ObservationStopErrorV4::InvalidRequest);
    }
    let V::Context(target) = field(args, "context")? else {
        return Err(ObservationStopErrorV4::InvalidRequest);
    };
    Ok(NormalizedObservationStopArgumentsV4 { target: *target })
}
/// Fixed 2/4/3-field passive native grammar. No transport, allocation or owner lookup.
pub fn normalize_observation_stop_envelope_v4<'h>(
    envelope: &ObservationStopEnvelopeValueV4<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedObservationStopArgumentsV4<'h>, ObservationStopErrorV4> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope);
    r.record(Observation {
        boundary: Boundary::ObservationStopArgumentNormalizationV4,
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
    pub fn stop_observation_facade_v4(
        &mut self,
        port: ObservationStopPortV4<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV4> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match port {
            ObservationStopPortV4::Existing(p) => self
                .stop_observation_facade_v3(p, r)
                .map_err(ObservationStopErrorV4::Existing),
            ObservationStopPortV4::SourceReceipt {
                command,
                context,
                control,
            } => self
                .stop_source_publication_receipt_v1(context, command, control, r)
                .map_err(ObservationStopErrorV4::SourceReceipt),
            ObservationStopPortV4::FacetReceipt {
                command,
                context,
                control,
            } => self
                .stop_facet_publication_receipt_v1(context, command, control, r)
                .map_err(ObservationStopErrorV4::FacetReceipt),
            ObservationStopPortV4::CollectionReceipt {
                command,
                context,
                control,
            } => self
                .stop_collection_publication_receipt_v1(context, command, control, r)
                .map_err(ObservationStopErrorV4::CollectionReceipt),
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationStopFacadeV4,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(e) if e.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::ObservationStopFacadeV4,
            disposition,
            service_time: timing.finish(),
        });
        result
    }
    /// Original target-role binding precedes native current-control evaluation.
    pub fn stop_decoded_observation_facade_v4(
        &mut self,
        request: DecodedObservationStopRequestV4<'_, '_, '_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV4> {
        let validated = normalize_observation_stop_envelope_v4(request.envelope, r).and_then(|n| {
            if request.native.matches(n.target()) {
                Ok(())
            } else {
                Err(ObservationStopErrorV4::Unavailable)
            }
        });
        if let Err(e) = validated {
            r.record(Observation {
                boundary: Boundary::ObservationStopFacadeV4,
                disposition: Disposition::Refused,
                service_time: None,
            });
            return Err(e);
        }
        self.stop_observation_facade_v4(request.native, r)
    }
}
