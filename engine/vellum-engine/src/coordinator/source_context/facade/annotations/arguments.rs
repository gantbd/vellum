//! Passive native F03 arguments. Opaque document/snapshot/selector bindings are
//! not wire identity records; no parser or executable caller object is accepted.
use super::super::arguments::{captured, natural};
use super::*;
use crate::capture::{CaptureUsageV1, CapturedValueV1};
#[cfg(test)]
pub(in crate::coordinator) mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnnotationArgumentTargetV1<'h> {
    Single(&'h AnnotationReadContextId),
    VersionSet(&'h VersionSetMemberId),
}
#[derive(Clone, Copy)]
pub enum AnnotationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, AnnotationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Snapshot(AnnotationArgumentTargetV1<'h>),
    Document(&'h DocumentId),
    Record(&'h AnnotationRecordId),
    Continuation(&'h AnnotationContinuationId),
    VersionSetRecord(&'h VersionSetAnnotationRecordId),
    VersionSetContinuation(&'h VersionSetAnnotationContinuationId),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnnotationArgumentQueryV1<'h> {
    Enumerate {
        limit: usize,
        continuation: Option<&'h AnnotationContinuationId>,
    },
    Inspect {
        annotation: &'h AnnotationRecordId,
    },
    TargetLocation {
        annotation: &'h AnnotationRecordId,
    },
    VersionSetEnumerate {
        limit: usize,
        continuation: Option<&'h VersionSetAnnotationContinuationId>,
    },
    VersionSetInspect {
        annotation: &'h VersionSetAnnotationRecordId,
    },
    VersionSetTargetLocation {
        annotation: &'h VersionSetAnnotationRecordId,
    },
}
#[derive(Clone, Copy)]
pub enum AnnotationArgumentFactsV1<'a> {
    Single(&'a TrustedAnnotationContextFactsV1<'a>),
    VersionSet(&'a TrustedVersionSetContextFactsV1<'a>),
}
#[derive(Clone, Copy)]
pub struct DecodedAnnotationFacadeRequestV1<'a, 'h> {
    pub envelope: &'a AnnotationEnvelopeValueV1<'a, 'h>,
    pub actor: &'a SettingsExecutionContextV1,
    pub scope: &'a SourceReadScopeV1,
    pub limits: CaptureLimitsV1,
    pub facts: AnnotationArgumentFactsV1<'a>,
}
pub struct NormalizedAnnotationArgumentsV1<'h> {
    target: AnnotationArgumentTargetV1<'h>,
    document: &'h DocumentId,
    actor: SettingsExecutionContextV1,
    query: AnnotationArgumentQueryV1<'h>,
    profile: String,
    limits: CaptureLimitsV1,
    usage: [CaptureUsageV1; 3],
    owned_bytes: usize,
}
macro_rules! redact {
    ($($t:ty),+ $(,)?) => { $(impl std::fmt::Debug for $t {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("<native-annotation-arguments:redacted>")
        }
    })+ };
}
redact!(
    AnnotationArgumentTargetV1<'_>,
    AnnotationEnvelopeValueV1<'_, '_>,
    AnnotationArgumentQueryV1<'_>,
    AnnotationArgumentFactsV1<'_>,
    DecodedAnnotationFacadeRequestV1<'_, '_>,
    NormalizedAnnotationArgumentsV1<'_>
);
impl PartialEq for NormalizedAnnotationArgumentsV1<'_> {
    fn eq(&self, b: &Self) -> bool {
        self.target == b.target
            && self.document == b.document
            && self.actor == b.actor
            && self.query == b.query
            && self.profile == b.profile
            && self.limits == b.limits
    }
}
impl Eq for NormalizedAnnotationArgumentsV1<'_> {}
impl<'h> NormalizedAnnotationArgumentsV1<'h> {
    pub fn target(&self) -> AnnotationArgumentTargetV1<'h> {
        self.target
    }
    pub fn document(&self) -> &'h DocumentId {
        self.document
    }
    pub fn actor(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn query(&self) -> AnnotationArgumentQueryV1<'h> {
        self.query
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    /// Logical native units, not wire bytes, physical usage or a read grant.
    pub fn capture_usage(&self) -> [CaptureUsageV1; 3] {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
}
/// Pure bounded native arguments; original opaque roles remain borrowed.
pub fn normalize_annotation_envelope_v1<'a, 'h>(
    envelope: &'a AnnotationEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedAnnotationArgumentsV1<'h>, Error> {
    let out = normalize(envelope, actor, limits, maximum, r);
    observation(
        r,
        Boundary::AnnotationArgumentNormalization,
        if matches!(&out, Err(Error::Faulted)) {
            Disposition::Unknown
        } else if out.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
    );
    out
}
/// Fixed header classification only; the funded decoder remains mandatory.
pub(in crate::coordinator) fn installed_selector(
    envelope: &AnnotationEnvelopeValueV1<'_, '_>,
) -> Result<&'static str, Error> {
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !matches!(
        (route, data(field(header, keys[0])?)?),
        ("typed", InertValueV1::Number(2.0)) | ("dynamic", InertValueV1::Text("read-annotations"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    match text(field(header, "variant")?)? {
        "enumerate" => Ok("enumerate"),
        "inspect" => Ok("inspect"),
        "target-location" => Ok("target-location"),
        _ => Err(Error::InvalidRequest),
    }
}
fn fields<'a, 'h>(
    v: &'a AnnotationEnvelopeValueV1<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, AnnotationEnvelopeValueV1<'a, 'h>)], Error> {
    let AnnotationEnvelopeValueV1::Object(rows) = v else {
        return Err(Error::InvalidRequest);
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(old, _)| old == k))
    {
        return Err(Error::InvalidRequest);
    }
    Ok(rows)
}
fn field<'a, 'h>(
    rows: &'a [(&'a str, AnnotationEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a AnnotationEnvelopeValueV1<'a, 'h>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(Error::InvalidRequest)
}
fn data<'a>(v: &AnnotationEnvelopeValueV1<'a, '_>) -> Result<InertValueV1<'a>, Error> {
    match v {
        AnnotationEnvelopeValueV1::Data(v) => Ok(*v),
        _ => Err(Error::InvalidRequest),
    }
}
fn text<'a>(v: &AnnotationEnvelopeValueV1<'a, '_>) -> Result<&'a str, Error> {
    match data(v)? {
        InertValueV1::Text(s) if !s.is_empty() => Ok(s),
        _ => Err(Error::InvalidRequest),
    }
}
fn normalize<'a, 'h>(
    envelope: &'a AnnotationEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedAnnotationArgumentsV1<'h>, Error> {
    let safe = |n: usize| (n as u128) <= 9_007_199_254_740_991;
    if [input.max_bytes, input.max_nodes, input.max_depth, maximum]
        .iter()
        .any(|n| !safe(*n))
        || input.max_nodes == 0
    {
        return Err(Error::InvalidRequest);
    }
    let limits = CaptureLimitsV1 {
        max_bytes: input.max_bytes.min(maximum),
        ..input
    };
    // Fixed live templates: outer2/header4/args7/actor7/limits3/canonical4/bindings3.
    if 27 * size_of::<(&str, InertValueV1<'_>)>() + size_of::<[InertValueV1<'_>; 3]>()
        > limits.max_bytes
    {
        return Err(Error::Limit);
    }
    let outer = fields(envelope, &["route", "request"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return Err(Error::InvalidRequest),
    };
    let header = fields(field(outer, "request")?, &keys)?;
    let selector = data(field(header, keys[0])?)?;
    if !matches!(
        (route, selector),
        ("typed", InertValueV1::Number(2.0)) | ("dynamic", InertValueV1::Text("read-annotations"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    let variant = text(field(header, "variant")?)?;
    let argument_keys: &[&str] = match variant {
        "enumerate" => &[
            "contract",
            "operation",
            "snapshot",
            "document",
            "profile",
            "limit",
            "continuation",
        ],
        "inspect" | "target-location" => &[
            "contract",
            "operation",
            "snapshot",
            "document",
            "profile",
            "annotation",
        ],
        _ => return Err(Error::InvalidRequest),
    };
    let args = fields(field(header, "arguments")?, argument_keys)?;
    if text(field(args, "contract")?)? != "annotation-query/1"
        || text(field(args, "operation")?)? != variant
    {
        return Err(Error::InvalidRequest);
    }
    let AnnotationEnvelopeValueV1::Snapshot(target) = field(args, "snapshot")? else {
        return Err(Error::InvalidRequest);
    };
    let AnnotationEnvelopeValueV1::Document(document) = field(args, "document")? else {
        return Err(Error::InvalidRequest);
    };
    let role = if variant == "enumerate" {
        "continuation"
    } else {
        "annotation"
    };
    let selection = field(args, role)?;
    let mut arguments = [("", InertValueV1::Null); 7];
    for (i, k) in argument_keys.iter().enumerate() {
        arguments[i] = (
            k,
            if matches!(*k, "snapshot" | "document" | "annotation" | "continuation") {
                InertValueV1::Null
            } else {
                data(field(args, k)?)?
            },
        );
    }
    let argument_object = InertValueV1::Object(&arguments[..argument_keys.len()]);
    let request = [
        (keys[0], selector),
        ("variant", InertValueV1::Text(variant)),
        ("version", InertValueV1::Number(1.0)),
        ("arguments", argument_object),
    ];
    let outer = [
        ("route", InertValueV1::Text(route)),
        ("request", InertValueV1::Object(&request)),
    ];
    let whole = frame(&InertValueV1::Object(&outer), limits, r)?;
    let actor_fields = labels(actor);
    if actor_fields
        .iter()
        .any(|(_, v)| matches!(v,InertValueV1::Text(s) if s.is_empty()))
    {
        return Err(Error::InvalidRequest);
    }
    let bound = frame(&InertValueV1::Object(&actor_fields), limits, r)?;
    let limit_fields = [
        (
            "maxRequestBytes",
            InertValueV1::Number(input.max_bytes as f64),
        ),
        (
            "maxInputNodes",
            InertValueV1::Number(input.max_nodes as f64),
        ),
        (
            "maxInputDepth",
            InertValueV1::Number(input.max_depth as f64),
        ),
    ];
    let bindings = [
        InertValueV1::Text("annotation-snapshot"),
        InertValueV1::Text("source-document"),
        InertValueV1::Text(role),
    ];
    let canonical = [
        ("request", argument_object),
        ("context", InertValueV1::Object(&actor_fields)),
        ("limits", InertValueV1::Object(&limit_fields)),
        ("opaqueBindings", InertValueV1::Array(&bindings)),
    ];
    let canonical = frame(&InertValueV1::Object(&canonical), limits, r)?;
    let request = captured(
        &whole,
        captured(&whole, whole.root(), "request")?,
        "arguments",
    )?;
    let CapturedValueV1::Text(profile) = captured(&whole, request, "profile")? else {
        return Err(Error::InvalidRequest);
    };
    if profile.is_empty() {
        return Err(Error::InvalidRequest);
    }
    let limit = if variant == "enumerate" {
        let n = natural(captured(&whole, request, "limit")?)?;
        if n == 0 {
            return Err(Error::InvalidRequest);
        }
        n
    } else {
        0
    };
    use AnnotationArgumentQueryV1 as Q;
    use AnnotationArgumentTargetV1 as T;
    use AnnotationEnvelopeValueV1 as V;
    let query = match (target, variant, selection) {
        (T::Single(_), "enumerate", V::Data(InertValueV1::Null)) => Q::Enumerate {
            limit,
            continuation: None,
        },
        (T::Single(_), "enumerate", V::Continuation(c)) => Q::Enumerate {
            limit,
            continuation: Some(c),
        },
        (T::Single(_), "inspect", V::Record(a)) => Q::Inspect { annotation: a },
        (T::Single(_), "target-location", V::Record(a)) => Q::TargetLocation { annotation: a },
        (T::VersionSet(_), "enumerate", V::Data(InertValueV1::Null)) => Q::VersionSetEnumerate {
            limit,
            continuation: None,
        },
        (T::VersionSet(_), "enumerate", V::VersionSetContinuation(c)) => Q::VersionSetEnumerate {
            limit,
            continuation: Some(c),
        },
        (T::VersionSet(_), "inspect", V::VersionSetRecord(a)) => {
            Q::VersionSetInspect { annotation: a }
        }
        (T::VersionSet(_), "target-location", V::VersionSetRecord(a)) => {
            Q::VersionSetTargetLocation { annotation: a }
        }
        _ => return Err(Error::InvalidRequest),
    };
    let bytes = actor_fields
        .iter()
        .try_fold(
            size_of::<NormalizedAnnotationArgumentsV1<'_>>(),
            |n, (_, v)| {
                let InertValueV1::Text(s) = v else {
                    return None;
                };
                n.checked_add(s.len())
            },
        )
        .and_then(|n| n.checked_add(profile.len()))
        .ok_or(Error::Limit)?;
    if bytes > limits.max_bytes {
        return Err(Error::Limit);
    }
    Ok(NormalizedAnnotationArgumentsV1 {
        target: *target,
        document,
        actor: settings::basis::copy_context(actor).map_err(|_| Error::Capacity)?,
        query,
        profile: owned(profile)?,
        limits: input,
        usage: [whole.usage(), bound.usage(), canonical.usage()],
        owned_bytes: bytes,
    })
}
impl SourceCoordinator {
    pub fn query_decoded_annotation_facade_v1(
        &mut self,
        request: DecodedAnnotationFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Option<AnnotationFacadeReplyV1> {
        self.try_query_decoded_annotation_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_query_decoded_annotation_facade_v1(
        &mut self,
        request: DecodedAnnotationFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<AnnotationFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Annotation, r, |gateway| {
            gateway.dispatch_decoded_annotation(request)
        })
    }
}
enum Lowered {
    Single(AnnotationQueryV1),
    VersionSet(VersionSetAnnotationQueryV1),
}
impl Gateway<'_> {
    fn dispatch_decoded_annotation(
        &mut self,
        request: DecodedAnnotationFacadeRequestV1<'_, '_>,
    ) -> Result<AnnotationFacadeReplyV1, Error> {
        if let Err(e) = self.admit(request.limits) {
            observation(
                self.recorder,
                Boundary::AnnotationFacadeAdmission,
                if e == Error::Faulted {
                    Disposition::Unknown
                } else {
                    Disposition::Refused
                },
            );
            return Err(e);
        }
        #[cfg(test)]
        if self.owner.source_facade_fault == Some(Fault::Reentry) {
            assert!(
                self.owner
                    .query_decoded_annotation_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        let maximum = request
            .limits
            .max_bytes
            .min(self.owner.settings_job_parent_limit());
        let n = normalize_annotation_envelope_v1(
            request.envelope,
            request.actor,
            request.limits,
            maximum,
            self.recorder,
        )?;
        // Unsupported/foreign targeting must not lend current loss to a valid
        // retained owner. Actor attribution is not the recipient scope/grant.
        if n.document != &request.scope.document || n.profile != request.scope.context.profile {
            return Err(Error::Unavailable);
        }
        if size_of::<Lowered>() > maximum {
            return Err(Error::Limit);
        }
        use AnnotationArgumentQueryV1 as Q;
        let q = match n.query {
            Q::Enumerate {
                limit,
                continuation,
            } => Lowered::Single(AnnotationQueryV1::Enumerate {
                limit,
                continuation: continuation.cloned(),
            }),
            Q::Inspect { annotation } => Lowered::Single(AnnotationQueryV1::Inspect {
                annotation: annotation.clone(),
            }),
            Q::TargetLocation { annotation } => {
                Lowered::Single(AnnotationQueryV1::TargetLocation {
                    annotation: annotation.clone(),
                })
            }
            Q::VersionSetEnumerate {
                limit,
                continuation,
            } => Lowered::VersionSet(VersionSetAnnotationQueryV1::Enumerate {
                limit,
                continuation: continuation.cloned(),
            }),
            Q::VersionSetInspect { annotation } => {
                Lowered::VersionSet(VersionSetAnnotationQueryV1::Inspect {
                    annotation: annotation.clone(),
                })
            }
            Q::VersionSetTargetLocation { annotation } => {
                Lowered::VersionSet(VersionSetAnnotationQueryV1::TargetLocation {
                    annotation: annotation.clone(),
                })
            }
        };
        // Clones above retain only the original opaque native identity, never
        // reconstructing selectors from rows, aliases, strings or slot numbers.
        let port = match (n.target, &q, request.facts) {
            (
                AnnotationArgumentTargetV1::Single(context),
                Lowered::Single(query),
                AnnotationArgumentFactsV1::Single(facts),
            ) => AnnotationFacadePortV1::Single {
                context,
                query,
                facts,
            },
            (
                AnnotationArgumentTargetV1::VersionSet(member),
                Lowered::VersionSet(query),
                AnnotationArgumentFactsV1::VersionSet(facts),
            ) => AnnotationFacadePortV1::VersionSet {
                member,
                query,
                facts,
            },
            _ => return Err(Error::Unavailable),
        };
        self.dispatch_annotation_admitted(AnnotationFacadeRequestV1 {
            port,
            scope: request.scope,
            actor: &n.actor,
            limits: request.limits,
        })
    }
}
