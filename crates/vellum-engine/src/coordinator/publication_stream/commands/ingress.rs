//! Fixed passive native roles. No wire codec, policy evaluation or callbacks.
use super::*;
use crate::capture::InertValueV1;
#[cfg(test)]
pub(super) mod tests;
pub const SOURCE_PUBLICATION_COMMAND_PROFILE_V1: &str = "source-publication-command/1";
pub const SOURCE_PUBLICATION_RECEIPT_PROFILE_V1: &str = "source-publication-receipt/1";
#[derive(Clone, Copy)]
pub enum SourcePublicationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, SourcePublicationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Command(&'h SourcePublicationCommandIdV1),
    Stream(&'h SourcePublicationStreamIdV1),
    ReceiptContext(&'h SourcePublicationReceiptContextIdV1),
}
pub struct SourcePublicationCommandPortV1<'a> {
    pub command: &'a SourcePublicationCommandIdV1,
    pub request: &'a SourcePublicationCommandRequestV1<'a>,
}
pub struct SourcePublicationReceiptPortV1<'a> {
    pub command: &'a SourcePublicationCommandIdV1,
    pub context: &'a SourcePublicationReceiptContextIdV1,
    pub scope: &'a SourceReadScopeV1,
    pub current: &'a TrustedSourcePublicationReceiptHistoricalFactsV1<'a>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePublicationIngressErrorV1 {
    InvalidRequest,
    Native(SourcePublicationErrorV1),
}
#[derive(Clone, Copy)]
pub struct NormalizedSourcePublicationCommandV1<'h> {
    command: &'h SourcePublicationCommandIdV1,
    stream: Option<&'h SourcePublicationStreamIdV1>,
}
impl<'h> NormalizedSourcePublicationCommandV1<'h> {
    pub fn command(self) -> &'h SourcePublicationCommandIdV1 {
        self.command
    }
    pub fn stream(self) -> Option<&'h SourcePublicationStreamIdV1> {
        self.stream
    }
    pub fn operation(self) -> &'static str {
        if self.stream.is_some() {
            "authorized-resynchronization"
        } else {
            "establish-bounded-stream"
        }
    }
    pub(in crate::coordinator) fn matches(self, p: &SourcePublicationCommandPortV1<'_>) -> bool {
        self.command == p.command
            && match (self.stream, p.request.operation) {
                (None, SourcePublicationCommandOperationV1::Establish) => true,
                (Some(a), SourcePublicationCommandOperationV1::Resynchronize { stream }) => {
                    a == stream
                }
                _ => false,
            }
    }
}
#[derive(Clone, Copy)]
pub struct NormalizedSourcePublicationReceiptV1<'h> {
    command: &'h SourcePublicationCommandIdV1,
    context: &'h SourcePublicationReceiptContextIdV1,
}
impl<'h> NormalizedSourcePublicationReceiptV1<'h> {
    pub fn command(self) -> &'h SourcePublicationCommandIdV1 {
        self.command
    }
    pub fn context(self) -> &'h SourcePublicationReceiptContextIdV1 {
        self.context
    }
    pub(in crate::coordinator) fn matches(self, p: &SourcePublicationReceiptPortV1<'_>) -> bool {
        self.command == p.command && self.context == p.context
    }
}
redacted!(
    SourcePublicationEnvelopeValueV1<'_, '_>,
    SourcePublicationCommandPortV1<'_>,
    SourcePublicationReceiptPortV1<'_>,
    NormalizedSourcePublicationCommandV1<'_>,
    NormalizedSourcePublicationReceiptV1<'_>
);
type V<'a, 'h> = SourcePublicationEnvelopeValueV1<'a, 'h>;
type E = SourcePublicationIngressErrorV1;
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
fn command<'h>(v: &V<'_, 'h>) -> Result<NormalizedSourcePublicationCommandV1<'h>, E> {
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
        SOURCE_PUBLICATION_COMMAND_PROFILE_V1,
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
    Ok(NormalizedSourcePublicationCommandV1 { command, stream })
}
fn receipt<'h>(v: &V<'_, 'h>) -> Result<NormalizedSourcePublicationReceiptV1<'h>, E> {
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
        SOURCE_PUBLICATION_RECEIPT_PROFILE_V1,
    ) || !text(field(args, "operation")?, "retained-outcome")
    {
        return Err(E::InvalidRequest);
    }
    let facet = fields(field(args, "facet")?, &["kind"])?;
    if !text(field(facet, "kind")?, "source-baseline") {
        return Err(E::InvalidRequest);
    }
    let V::Command(command) = field(args, "command")? else {
        return Err(E::InvalidRequest);
    };
    let V::ReceiptContext(context) = field(args, "context")? else {
        return Err(E::InvalidRequest);
    };
    Ok(NormalizedSourcePublicationReceiptV1 { command, context })
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
/// Fixed 2/4/4 fields; source snapshots need no caller-supplied preparation object.
pub fn normalize_source_publication_command_v1<'h>(
    v: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedSourcePublicationCommandV1<'h>, E> {
    let timing = r.begin_service_timing();
    let result = command(v);
    report(
        r,
        Boundary::SourcePublicationCommandArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
/// Fixed 2/4/5/1 fields. Original context and command are engine-issued roles.
pub fn normalize_source_publication_receipt_v1<'h>(
    v: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedSourcePublicationReceiptV1<'h>, E> {
    let timing = r.begin_service_timing();
    let result = receipt(v);
    report(
        r,
        Boundary::SourcePublicationReceiptArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// One original request or protected identity-only lookup, not baseline delivery.
    pub fn request_decoded_source_publication_command_v1(
        &mut self,
        v: &V<'_, '_>,
        port: &SourcePublicationCommandPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationCommandReplyV1, E> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_source_publication_command_v1(v, r)?;
            if !n.matches(port) {
                return Err(E::Native(Error::Unavailable));
            }
            let reply = self
                .request_source_publication_command_v1(port.command, port.request, r)
                .map_err(E::Native)?;
            let (SourcePublicationCommandReplyV1::Settled { command }
            | SourcePublicationCommandReplyV1::ObserveExisting { command }) = &reply;
            if command != n.command {
                return Err(E::Native(Error::Faulted));
            }
            Ok(reply)
        }));
        match caught {
            Ok(result) => {
                report(
                    r,
                    Boundary::SourcePublicationCommandFacade,
                    &result,
                    timing.finish(),
                );
                result
            }
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::SourcePublicationCommandFacade,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        }
    }
    /// Separate current historical authorization remains wholly out of band.
    pub fn release_decoded_source_publication_receipt_v1(
        &mut self,
        v: &V<'_, '_>,
        port: &SourcePublicationReceiptPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationReceiptReplyV1, E> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let n = normalize_source_publication_receipt_v1(v, r)?;
            if !n.matches(port) {
                return Err(E::Native(Error::Unavailable));
            }
            let reply = self
                .release_source_publication_receipt_v1(
                    port.context,
                    port.command,
                    port.scope,
                    port.current,
                    r,
                )
                .map_err(E::Native)?;
            self.validate_source_publication_receipt_reply_v1(
                port.context,
                port.command,
                port.scope,
                &reply,
                r,
            )
            .map_err(E::Native)?;
            Ok(reply)
        })();
        report(
            r,
            Boundary::SourcePublicationReceiptFacade,
            &result,
            timing.finish(),
        );
        result
    }
}
