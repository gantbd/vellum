//! Fixed passive native roles. No wire codec, policy evaluation or callbacks.
use super::*;
use crate::capture::InertValueV1;
pub const COLLECTION_PUBLICATION_COMMAND_PROFILE_V1: &str = "collection-publication-command/1";
pub const COLLECTION_PUBLICATION_RECEIPT_PROFILE_V1: &str = "collection-publication-receipt/1";
#[derive(Clone, Copy)]
pub enum CollectionPublicationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, CollectionPublicationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Command(&'h CollectionPublicationCommandIdV1),
    Stream(&'h CollectionPublicationStreamIdV1),
    ReceiptContext(&'h CollectionPublicationReceiptContextIdV1),
    Preparation(&'h CollectionVisibilityPreparationIdV1),
}
pub struct CollectionPublicationCommandPortV1<'a> {
    pub command: &'a CollectionPublicationCommandIdV1,
    pub request: &'a CollectionPublicationCommandRequestV1<'a>,
}
pub struct CollectionPublicationReceiptPortV1<'a> {
    pub command: &'a CollectionPublicationCommandIdV1,
    pub context: &'a CollectionPublicationReceiptContextIdV1,
    pub scope: &'a CollectionPublicationScopeV1,
    pub current: &'a TrustedCollectionPublicationReceiptHistoricalFactsV1<'a>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionPublicationIngressErrorV1 {
    InvalidRequest,
    Native(CollectionPublicationErrorV1),
}
#[derive(Clone, Copy)]
pub struct NormalizedCollectionPublicationCommandV1<'h> {
    command: &'h CollectionPublicationCommandIdV1,
    stream: Option<&'h CollectionPublicationStreamIdV1>,
    preparation: &'h CollectionVisibilityPreparationIdV1,
}
impl<'h> NormalizedCollectionPublicationCommandV1<'h> {
    pub fn command(self) -> &'h CollectionPublicationCommandIdV1 {
        self.command
    }
    pub fn preparation(self) -> &'h CollectionVisibilityPreparationIdV1 {
        self.preparation
    }
    pub fn stream(self) -> Option<&'h CollectionPublicationStreamIdV1> {
        self.stream
    }
    pub fn operation(self) -> &'static str {
        if self.stream.is_some() {
            "authorized-resynchronization"
        } else {
            "establish-bounded-stream"
        }
    }
    pub(in crate::coordinator) fn matches(
        self,
        p: &CollectionPublicationCommandPortV1<'_>,
    ) -> bool {
        self.command == p.command
            && self.preparation == p.request.preparation
            && match (self.stream, p.request.operation) {
                (None, CollectionPublicationCommandOperationV1::Establish) => true,
                (Some(a), CollectionPublicationCommandOperationV1::Resynchronize { stream }) => {
                    a == stream
                }
                _ => false,
            }
    }
}
#[derive(Clone, Copy)]
pub struct NormalizedCollectionPublicationReceiptV1<'h> {
    command: &'h CollectionPublicationCommandIdV1,
    context: &'h CollectionPublicationReceiptContextIdV1,
}
impl<'h> NormalizedCollectionPublicationReceiptV1<'h> {
    pub fn command(self) -> &'h CollectionPublicationCommandIdV1 {
        self.command
    }
    pub fn context(self) -> &'h CollectionPublicationReceiptContextIdV1 {
        self.context
    }
    pub(in crate::coordinator) fn matches(
        self,
        p: &CollectionPublicationReceiptPortV1<'_>,
    ) -> bool {
        self.command == p.command && self.context == p.context
    }
}
redacted!(
    CollectionPublicationEnvelopeValueV1<'_, '_>,
    CollectionPublicationCommandPortV1<'_>,
    CollectionPublicationReceiptPortV1<'_>,
    NormalizedCollectionPublicationCommandV1<'_>,
    NormalizedCollectionPublicationReceiptV1<'_>
);
type V<'a, 'h> = CollectionPublicationEnvelopeValueV1<'a, 'h>;
type E = CollectionPublicationIngressErrorV1;
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
fn command<'h>(v: &V<'_, 'h>) -> Result<NormalizedCollectionPublicationCommandV1<'h>, E> {
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
        &["contract", "operation", "command", "stream", "preparation"],
    )?;
    if !text(
        field(args, "contract")?,
        COLLECTION_PUBLICATION_COMMAND_PROFILE_V1,
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
    let V::Preparation(preparation) = field(args, "preparation")? else {
        return Err(E::InvalidRequest);
    };
    Ok(NormalizedCollectionPublicationCommandV1 {
        command,
        stream,
        preparation,
    })
}
fn receipt<'h>(v: &V<'_, 'h>) -> Result<NormalizedCollectionPublicationReceiptV1<'h>, E> {
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
        COLLECTION_PUBLICATION_RECEIPT_PROFILE_V1,
    ) || !text(field(args, "operation")?, "retained-outcome")
    {
        return Err(E::InvalidRequest);
    }
    let facet = fields(field(args, "facet")?, &["kind"])?;
    if !text(field(facet, "kind")?, "collection-baseline") {
        return Err(E::InvalidRequest);
    }
    let V::Command(command) = field(args, "command")? else {
        return Err(E::InvalidRequest);
    };
    let V::ReceiptContext(context) = field(args, "context")? else {
        return Err(E::InvalidRequest);
    };
    Ok(NormalizedCollectionPublicationReceiptV1 { command, context })
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
/// Fixed 2/4/5 fields, including the original opaque visibility preparation.
pub fn normalize_collection_publication_command_v1<'h>(
    v: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedCollectionPublicationCommandV1<'h>, E> {
    let timing = r.begin_service_timing();
    let result = command(v);
    report(
        r,
        Boundary::CollectionPublicationCommandArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
/// Fixed 2/4/5/1 fields. Original context and command are engine-issued roles.
pub fn normalize_collection_publication_receipt_v1<'h>(
    v: &V<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedCollectionPublicationReceiptV1<'h>, E> {
    let timing = r.begin_service_timing();
    let result = receipt(v);
    report(
        r,
        Boundary::CollectionPublicationReceiptArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// One original request or protected identity-only lookup, not baseline delivery.
    pub fn request_decoded_collection_publication_command_v1(
        &mut self,
        v: &V<'_, '_>,
        port: &CollectionPublicationCommandPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationCommandReplyV1, E> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_collection_publication_command_v1(v, r)?;
            if !n.matches(port) {
                return Err(E::Native(Error::Unavailable));
            }
            let reply = self
                .request_collection_publication_command_v1(port.command, port.request, r)
                .map_err(E::Native)?;
            let (CollectionPublicationCommandReplyV1::Settled { command }
            | CollectionPublicationCommandReplyV1::ObserveExisting { command }) = &reply;
            if command != n.command {
                return Err(E::Native(Error::Faulted));
            }
            Ok(reply)
        }));
        match caught {
            Ok(result) => {
                report(
                    r,
                    Boundary::CollectionPublicationCommandFacade,
                    &result,
                    timing.finish(),
                );
                result
            }
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::CollectionPublicationCommandFacade,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        }
    }
    /// Separate current historical authorization remains wholly out of band.
    pub fn release_decoded_collection_publication_receipt_v1(
        &mut self,
        v: &V<'_, '_>,
        port: &CollectionPublicationReceiptPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationReceiptReplyV1, E> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let n = normalize_collection_publication_receipt_v1(v, r)?;
            if !n.matches(port) {
                return Err(E::Native(Error::Unavailable));
            }
            let reply = self
                .release_collection_publication_receipt_v1(
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
            Boundary::CollectionPublicationReceiptFacade,
            &result,
            timing.finish(),
        );
        result
    }
}
