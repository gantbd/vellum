//! Fixed passive native roles. No wire codec, policy evaluation or callbacks.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
pub(super) mod tests;
pub const FACET_PUBLICATION_COMMAND_PROFILE_V1: &str = "named-facet-publication-command/1";
pub const FACET_PUBLICATION_RECEIPT_PROFILE_V1: &str = "named-facet-publication-receipt/1";
#[derive(Clone, Copy)]
pub enum FacetPublicationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, FacetPublicationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Command(&'h FacetPublicationCommandIdV1),
    Stream(&'h FacetPublicationStreamIdV1),
    ReceiptContext(&'h FacetPublicationReceiptContextIdV1),
}
pub struct FacetPublicationCommandPortV1<'a> {
    pub command: &'a FacetPublicationCommandIdV1,
    pub request: &'a FacetPublicationCommandRequestV1<'a>,
}
pub struct FacetPublicationReceiptPortV1<'a> {
    pub command: &'a FacetPublicationCommandIdV1,
    pub context: &'a FacetPublicationReceiptContextIdV1,
    pub scope: &'a FacetPublicationScopeV1,
    pub current: &'a TrustedFacetPublicationReceiptHistoricalFactsV1<'a>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetPublicationIngressErrorV1 {
    InvalidRequest,
    Native(FacetPublicationErrorV1),
}
#[derive(Clone, Copy)]
pub struct NormalizedFacetPublicationCommandV1<'h> {
    command: &'h FacetPublicationCommandIdV1,
    stream: Option<&'h FacetPublicationStreamIdV1>,
}
impl<'h> NormalizedFacetPublicationCommandV1<'h> {
    pub fn command(self) -> &'h FacetPublicationCommandIdV1 {
        self.command
    }
    pub fn stream(self) -> Option<&'h FacetPublicationStreamIdV1> {
        self.stream
    }
    pub fn operation(self) -> &'static str {
        if self.stream.is_some() {
            "authorized-resynchronization"
        } else {
            "establish-bounded-stream"
        }
    }
    pub(in crate::coordinator) fn matches(self, p: &FacetPublicationCommandPortV1<'_>) -> bool {
        self.command == p.command
            && match (self.stream, p.request.operation) {
                (None, FacetPublicationCommandOperationV1::Establish) => true,
                (Some(a), FacetPublicationCommandOperationV1::Resynchronize { stream }) => {
                    a == stream
                }
                _ => false,
            }
    }
}
#[derive(Clone, Copy)]
pub struct NormalizedFacetPublicationReceiptV1<'h> {
    command: &'h FacetPublicationCommandIdV1,
    context: &'h FacetPublicationReceiptContextIdV1,
}
impl<'h> NormalizedFacetPublicationReceiptV1<'h> {
    pub fn command(self) -> &'h FacetPublicationCommandIdV1 {
        self.command
    }
    pub fn context(self) -> &'h FacetPublicationReceiptContextIdV1 {
        self.context
    }
    pub(in crate::coordinator) fn matches(self, p: &FacetPublicationReceiptPortV1<'_>) -> bool {
        self.command == p.command && self.context == p.context
    }
}
redacted!(
    FacetPublicationEnvelopeValueV1<'_, '_>,
    FacetPublicationCommandPortV1<'_>,
    FacetPublicationReceiptPortV1<'_>,
    NormalizedFacetPublicationCommandV1<'_>,
    NormalizedFacetPublicationReceiptV1<'_>
);
type V<'a, 'h> = FacetPublicationEnvelopeValueV1<'a, 'h>;
type E = FacetPublicationIngressErrorV1;
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
fn field<'a, 'h>(rows: &'a [(&'a str, V<'a, 'h>)], key: &str) -> Result<&'a V<'a, 'h>, E> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(E::InvalidRequest)
}
fn text(v: &V<'_, '_>, value: &str) -> bool {
    matches!(v, V::Data(InertValueV1::Text(t)) if *t == value)
}
fn header<'a, 'h>(v: &V<'a, 'h>, family: f64, name: &str) -> Result<&'a [(&'a str, V<'a, 'h>)], E> {
    let outer = fields(v, &["route", "request"])?;
    let route = field(outer, "route")?;
    let keys = if text(route, "typed") {
        ["family", "variant", "version", "arguments"]
    } else if text(route, "dynamic") {
        ["name", "variant", "version", "arguments"]
    } else {
        return Err(E::InvalidRequest);
    };
    let h = fields(field(outer, "request")?, &keys)?;
    if !(if keys[0] == "family" {
        matches!(field(h, "family")?, V::Data(InertValueV1::Number(n)) if *n == family)
    } else {
        text(field(h, "name")?, name)
    }) || !matches!(field(h, "version")?, V::Data(InertValueV1::Number(1.0)))
    {
        return Err(E::InvalidRequest);
    }
    Ok(h)
}
fn command<'h>(v: &V<'_, 'h>) -> Result<NormalizedFacetPublicationCommandV1<'h>, E> {
    let h = header(v, 18.0, "control-observation")?;
    let variant = field(h, "variant")?;
    let resync = if text(variant, "establish-bounded-stream") {
        false
    } else if text(variant, "authorized-resynchronization") {
        true
    } else {
        return Err(E::InvalidRequest);
    };
    let args = fields(
        field(h, "arguments")?,
        &["contract", "operation", "command", "stream"],
    )?;
    if !text(
        field(args, "contract")?,
        FACET_PUBLICATION_COMMAND_PROFILE_V1,
    ) || !text(
        field(args, "operation")?,
        if resync {
            "authorized-resynchronization"
        } else {
            "establish-bounded-stream"
        },
    ) {
        return Err(E::InvalidRequest);
    }
    let V::Command(command) = field(args, "command")? else {
        return Err(E::InvalidRequest);
    };
    let stream = match (resync, field(args, "stream")?) {
        (false, V::Data(InertValueV1::Null)) => None,
        (true, V::Stream(s)) => Some(*s),
        _ => return Err(E::InvalidRequest),
    };
    Ok(NormalizedFacetPublicationCommandV1 { command, stream })
}
fn receipt<'h>(v: &V<'_, 'h>) -> Result<NormalizedFacetPublicationReceiptV1<'h>, E> {
    let h = header(v, 5.0, "inspect-execution")?;
    if !text(field(h, "variant")?, "retained-outcome") {
        return Err(E::InvalidRequest);
    }
    let args = fields(
        field(h, "arguments")?,
        &["contract", "operation", "command", "context", "facet"],
    )?;
    if !text(
        field(args, "contract")?,
        FACET_PUBLICATION_RECEIPT_PROFILE_V1,
    ) || !text(field(args, "operation")?, "retained-outcome")
    {
        return Err(E::InvalidRequest);
    }
    let facet = fields(field(args, "facet")?, &["kind"])?;
    if !text(field(facet, "kind")?, "named-facet-baseline") {
        return Err(E::InvalidRequest);
    }
    let V::Command(command) = field(args, "command")? else {
        return Err(E::InvalidRequest);
    };
    let V::ReceiptContext(context) = field(args, "context")? else {
        return Err(E::InvalidRequest);
    };
    Ok(NormalizedFacetPublicationReceiptV1 { command, context })
}
fn report<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, E>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(E::Native(Error::Faulted)) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
/// Fixed 2/4/4 fields; named projections need no caller-supplied preparation object.
pub fn normalize_facet_publication_command_v1<'h>(
    v: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedFacetPublicationCommandV1<'h>, E> {
    let timing = r.begin_service_timing();
    let result = command(v);
    report(
        r,
        Boundary::FacetPublicationCommandArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
/// Fixed 2/4/5/1 fields. Original context and command are engine-issued roles.
pub fn normalize_facet_publication_receipt_v1<'h>(
    v: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedFacetPublicationReceiptV1<'h>, E> {
    let timing = r.begin_service_timing();
    let result = receipt(v);
    report(
        r,
        Boundary::FacetPublicationReceiptArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// One original request or protected identity-only lookup, not baseline delivery.
    pub fn request_decoded_facet_publication_command_v1(
        &mut self,
        v: &V<'_, '_>,
        port: &FacetPublicationCommandPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationCommandReplyV1, E> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_facet_publication_command_v1(v, r)?;
            if !n.matches(port) {
                return Err(E::Native(Error::Unavailable));
            }
            let reply = self
                .request_facet_publication_command_v1(port.command, port.request, r)
                .map_err(E::Native)?;
            let (FacetPublicationCommandReplyV1::Settled { command }
            | FacetPublicationCommandReplyV1::ObserveExisting { command }) = &reply;
            if command != n.command {
                return Err(E::Native(Error::Faulted));
            }
            Ok(reply)
        }));
        match caught {
            Ok(result) => {
                report(
                    r,
                    Boundary::FacetPublicationCommandFacade,
                    &result,
                    timing.finish(),
                );
                result
            }
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::FacetPublicationCommandFacade,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        }
    }
    /// Separate current historical authorization remains wholly out of band.
    pub fn release_decoded_facet_publication_receipt_v1(
        &mut self,
        v: &V<'_, '_>,
        port: &FacetPublicationReceiptPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationReceiptReplyV1, E> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let n = normalize_facet_publication_receipt_v1(v, r)?;
            if !n.matches(port) {
                return Err(E::Native(Error::Unavailable));
            }
            let reply = self
                .release_facet_publication_receipt_v1(
                    port.context,
                    port.command,
                    port.scope,
                    port.current,
                    r,
                )
                .map_err(E::Native)?;
            // The native release already validates the complete original reply.
            Ok(reply)
        })();
        report(
            r,
            Boundary::FacetPublicationReceiptFacade,
            &result,
            timing.finish(),
        );
        result
    }
}
