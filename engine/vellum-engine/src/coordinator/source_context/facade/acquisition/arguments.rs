//! Native acquisition bindings, not serialized document/reference codecs.
use super::super::arguments::captured;
use super::*;
use crate::capture::CaptureUsageV1;
use crate::capture::CapturedValueV1;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ReadAcquisitionParticipantV1<'h> {
    pub document: &'h DocumentId,
    pub data: AnnotationProjectionClassesV1,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReadAcquisitionSelectionV1<'h> {
    Current,
    Source(&'h SourceReadContextId),
    Annotation(&'h AnnotationReadContextId),
    VersionSet(&'h VersionSetContextId),
}
#[derive(Clone, Copy)]
pub enum ReadAcquisitionEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ReadAcquisitionEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Document(&'h DocumentId),
    Classes(AnnotationProjectionClassesV1),
    Participants(&'a [ReadAcquisitionParticipantV1<'h>]),
    Reference(ReadAcquisitionSelectionV1<'h>),
}
#[derive(Clone, Copy)]
pub struct DecodedReadAcquisitionFacadeRequestV1<'a, 'h> {
    pub envelope: &'a ReadAcquisitionEnvelopeValueV1<'a, 'h>,
    /// Actual first-party port/preparation and separately attributed host facts.
    /// Normalized arguments must match it before any native loss/consumption.
    pub native: ReadAcquisitionFacadeRequestV1<'a>,
}
pub struct NormalizedReadAcquisitionArgumentsV1<'h> {
    coherent: bool,
    participants: Vec<ReadAcquisitionParticipantV1<'h>>,
    selection: ReadAcquisitionSelectionV1<'h>,
    actor: SettingsExecutionContextV1,
    profile: String,
    limits: CaptureLimitsV1,
    usage: [CaptureUsageV1; 3],
    owned_bytes: usize,
}
macro_rules! redact {
    ($($t:ty),+ $(,)?)=>{$(impl std::fmt::Debug for $t {
        fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.write_str("<native-read-arguments:redacted>")}
    })+};
}
redact!(
    ReadAcquisitionParticipantV1<'_>,
    ReadAcquisitionSelectionV1<'_>,
    ReadAcquisitionEnvelopeValueV1<'_, '_>,
    DecodedReadAcquisitionFacadeRequestV1<'_, '_>,
    NormalizedReadAcquisitionArgumentsV1<'_>
);
impl PartialEq for NormalizedReadAcquisitionArgumentsV1<'_> {
    fn eq(&self, b: &Self) -> bool {
        self.coherent == b.coherent
            && self.participants == b.participants
            && self.selection == b.selection
            && self.actor == b.actor
            && self.profile == b.profile
            && self.limits == b.limits
    }
}
impl Eq for NormalizedReadAcquisitionArgumentsV1<'_> {}
impl<'h> NormalizedReadAcquisitionArgumentsV1<'h> {
    pub fn coherent(&self) -> bool {
        self.coherent
    }
    pub fn participants(&self) -> &[ReadAcquisitionParticipantV1<'h>] {
        &self.participants
    }
    pub fn selection(&self) -> ReadAcquisitionSelectionV1<'h> {
        self.selection
    }
    pub fn actor(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    pub fn capture_usage(&self) -> [CaptureUsageV1; 3] {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
}
/// Closed native classes/participant fields are typed rather than wire arrays.
/// Documents and retained references remain original capabilities. No grant follows.
pub fn normalize_read_acquisition_envelope_v1<'a, 'h>(
    envelope: &'a ReadAcquisitionEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedReadAcquisitionArgumentsV1<'h>, Error> {
    let out = normalize(envelope, actor, limits, maximum, r);
    observation(
        r,
        Boundary::ReadAcquisitionArgumentNormalization,
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
fn fields<'a, 'h>(
    v: &'a ReadAcquisitionEnvelopeValueV1<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, ReadAcquisitionEnvelopeValueV1<'a, 'h>)], Error> {
    let ReadAcquisitionEnvelopeValueV1::Object(rows) = v else {
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
    rows: &'a [(&'a str, ReadAcquisitionEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a ReadAcquisitionEnvelopeValueV1<'a, 'h>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(Error::InvalidRequest)
}
fn data<'a>(v: &ReadAcquisitionEnvelopeValueV1<'a, '_>) -> Result<InertValueV1<'a>, Error> {
    match v {
        ReadAcquisitionEnvelopeValueV1::Data(v) => Ok(*v),
        _ => Err(Error::InvalidRequest),
    }
}
/// Fixed header/contract classification, without scanning participant payloads.
pub(in crate::coordinator) fn installed_selector(
    envelope: &ReadAcquisitionEnvelopeValueV1<'_, '_>,
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
        ("typed", InertValueV1::Number(1.0)) | ("dynamic", InertValueV1::Text("read-document"))
    ) || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
        || text(field(header, "variant")?)? != "snapshot-handoff"
    {
        return Err(Error::InvalidRequest);
    }
    let ReadAcquisitionEnvelopeValueV1::Object(args) = field(header, "arguments")? else {
        return Err(Error::InvalidRequest);
    };
    if args.len() != 6 {
        return Err(Error::InvalidRequest);
    }
    match text(field(args, "contract")?)? {
        "read-context/1" => Ok("read-context/1"),
        "read-version-set/1" => Ok("read-version-set/1"),
        _ => Err(Error::InvalidRequest),
    }
}
fn text<'a>(v: &ReadAcquisitionEnvelopeValueV1<'a, '_>) -> Result<&'a str, Error> {
    match data(v)? {
        InertValueV1::Text(s) if !s.is_empty() => Ok(s),
        _ => Err(Error::InvalidRequest),
    }
}
fn classes(c: AnnotationProjectionClassesV1) -> InertValueV1<'static> {
    InertValueV1::Array(match c {
        AnnotationProjectionClassesV1::Source => &[InertValueV1::Text("source")],
        AnnotationProjectionClassesV1::Annotations => &[InertValueV1::Text("annotations")],
        AnnotationProjectionClassesV1::Combined => &[
            InertValueV1::Text("annotations"),
            InertValueV1::Text("source"),
        ],
    })
}
fn normalize<'a, 'h>(
    envelope: &'a ReadAcquisitionEnvelopeValueV1<'a, 'h>,
    actor: &SettingsExecutionContextV1,
    input: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedReadAcquisitionArgumentsV1<'h>, Error> {
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
    // Fixed outer/header/arguments/selection/actor/limits/canonical/binding templates.
    if 28 * size_of::<(&str, InertValueV1<'_>)>() + size_of::<[InertValueV1<'_>; 2]>()
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
        ("typed", InertValueV1::Number(1.0)) | ("dynamic", InertValueV1::Text("read-document"))
    ) || text(field(header, "variant")?)? != "snapshot-handoff"
        || !matches!(data(field(header, "version")?)?, InertValueV1::Number(1.0))
    {
        return Err(Error::InvalidRequest);
    }
    let raw = field(header, "arguments")?;
    let single_keys = [
        "contract",
        "operation",
        "document",
        "selection",
        "data",
        "profile",
    ];
    let multi_keys = [
        "contract",
        "operation",
        "authority",
        "participants",
        "selection",
        "profile",
    ];
    let ReadAcquisitionEnvelopeValueV1::Object(raw_rows) = raw else {
        return Err(Error::InvalidRequest);
    };
    // Bounded header classification before any caller-sized traversal.
    if raw_rows.len() != 6 {
        return Err(Error::InvalidRequest);
    }
    let contract = text(field(raw_rows, "contract")?)?;
    let coherent = match contract {
        "read-context/1" => false,
        "read-version-set/1" => true,
        _ => return Err(Error::InvalidRequest),
    };
    let args = fields(raw, if coherent { &multi_keys } else { &single_keys })?;
    if text(field(args, "operation")?)? != "snapshot-handoff"
        || text(field(args, "profile")?)?.is_empty()
    {
        return Err(Error::InvalidRequest);
    }
    use ReadAcquisitionEnvelopeValueV1 as V;
    let single;
    let participants = if coherent {
        if text(field(args, "authority")?)? != actor.authority {
            return Err(Error::InvalidRequest);
        }
        let V::Participants(rows) = field(args, "participants")? else {
            return Err(Error::InvalidRequest);
        };
        *rows
    } else {
        let V::Document(document) = field(args, "document")? else {
            return Err(Error::InvalidRequest);
        };
        let V::Classes(data) = field(args, "data")? else {
            return Err(Error::InvalidRequest);
        };
        single = [ReadAcquisitionParticipantV1 {
            document,
            data: *data,
        }];
        &single
    };
    let count = participants.len();
    let row_unit = size_of::<[(&str, InertValueV1<'_>); 2]>() + size_of::<InertValueV1<'_>>();
    if count == 0
        || count > limits.max_nodes
        || count
            .checked_mul(row_unit)
            .and_then(|n| {
                n.checked_add(
                    size_of::<Vec<[(&str, InertValueV1<'_>); 2]>>()
                        + size_of::<Vec<InertValueV1<'_>>>(),
                )
            })
            .is_none_or(|n| n > limits.max_bytes)
    {
        return Err(Error::Limit);
    }
    for (i, p) in participants.iter().enumerate() {
        if participants[..i]
            .iter()
            .any(|old| old.document == p.document)
        {
            return Err(Error::InvalidRequest);
        }
    }
    let select = field(args, "selection")?;
    let V::Object(select_rows) = select else {
        return Err(Error::InvalidRequest);
    };
    if !(1..=2).contains(&select_rows.len()) {
        return Err(Error::InvalidRequest);
    }
    let kind = text(field(select_rows, "kind")?)?;
    let selection = match kind {
        "capture-current" => {
            fields(select, &["kind"])?;
            ReadAcquisitionSelectionV1::Current
        }
        "retained" => {
            let rows = fields(select, &["kind", "reference"])?;
            let V::Reference(reference) = field(rows, "reference")? else {
                return Err(Error::InvalidRequest);
            };
            match (coherent, reference) {
                (true, ReadAcquisitionSelectionV1::VersionSet(_))
                | (false, ReadAcquisitionSelectionV1::Annotation(_)) => *reference,
                (false, ReadAcquisitionSelectionV1::Source(_))
                    if participants[0].data == AnnotationProjectionClassesV1::Source =>
                {
                    *reference
                }
                _ => return Err(Error::InvalidRequest),
            }
        }
        _ => return Err(Error::InvalidRequest),
    };
    let current = selection == ReadAcquisitionSelectionV1::Current;
    let select_shadow = [
        ("kind", InertValueV1::Text(kind)),
        ("reference", InertValueV1::Null),
    ];
    let mut row_templates = Vec::new();
    row_templates
        .try_reserve_exact(count)
        .map_err(|_| Error::Capacity)?;
    for p in participants {
        row_templates.push([("document", InertValueV1::Null), ("data", classes(p.data))]);
    }
    let mut row_values = Vec::new();
    row_values
        .try_reserve_exact(count)
        .map_err(|_| Error::Capacity)?;
    row_values.extend(row_templates.iter().map(|row| InertValueV1::Object(row)));
    let args_shadow = [
        ("contract", InertValueV1::Text(contract)),
        ("operation", InertValueV1::Text("snapshot-handoff")),
        (
            if coherent { "authority" } else { "document" },
            if coherent {
                InertValueV1::Text(&actor.authority)
            } else {
                InertValueV1::Null
            },
        ),
        (
            if coherent { "participants" } else { "data" },
            if coherent {
                InertValueV1::Array(&row_values)
            } else {
                classes(participants[0].data)
            },
        ),
        (
            "selection",
            InertValueV1::Object(&select_shadow[..if current { 1 } else { 2 }]),
        ),
        ("profile", data(field(args, "profile")?)?),
    ];
    let request = [
        (keys[0], selector),
        ("variant", InertValueV1::Text("snapshot-handoff")),
        ("version", InertValueV1::Number(1.0)),
        ("arguments", InertValueV1::Object(&args_shadow)),
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
        InertValueV1::Text("native-documents"),
        InertValueV1::Text("retained-reference"),
    ];
    let canonical = [
        ("request", InertValueV1::Object(&args_shadow)),
        ("context", InertValueV1::Object(&actor_fields)),
        ("limits", InertValueV1::Object(&limit_fields)),
        (
            "opaqueBindings",
            InertValueV1::Array(&bindings[..if current { 1 } else { 2 }]),
        ),
    ];
    let canonical = frame(&InertValueV1::Object(&canonical), limits, r)?;
    let request = captured(
        &whole,
        captured(&whole, whole.root(), "request")?,
        "arguments",
    )?;
    let CapturedValueV1::Text(profile) = captured(&whole, request, "profile")? else {
        return Err(Error::Faulted);
    };
    let bytes = actor_fields
        .iter()
        .try_fold(
            size_of::<NormalizedReadAcquisitionArgumentsV1<'_>>(),
            |n, (_, v)| {
                let InertValueV1::Text(s) = v else {
                    return None;
                };
                n.checked_add(s.len())
            },
        )
        .and_then(|n| {
            n.checked_add(profile.len())?
                .checked_add(count.checked_mul(size_of::<ReadAcquisitionParticipantV1<'_>>())?)
        })
        .ok_or(Error::Limit)?;
    if bytes > limits.max_bytes {
        return Err(Error::Limit);
    }
    let mut owned_participants = Vec::new();
    owned_participants
        .try_reserve_exact(count)
        .map_err(|_| Error::Capacity)?;
    owned_participants.extend_from_slice(participants);
    Ok(NormalizedReadAcquisitionArgumentsV1 {
        coherent,
        participants: owned_participants,
        selection,
        actor: settings::basis::copy_context(actor).map_err(|_| Error::Capacity)?,
        profile: owned(profile)?,
        limits: input,
        usage: [whole.usage(), bound.usage(), canonical.usage()],
        owned_bytes: bytes,
    })
}
impl SourceCoordinator {
    pub fn acquire_decoded_read_facade_v1(
        &mut self,
        request: DecodedReadAcquisitionFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Option<ReadAcquisitionFacadeReplyV1> {
        self.try_acquire_decoded_read_facade_v1(request, r).ok()
    }
    /// Explicit native adapter outcome. Authority refusals remain indistinguishable;
    /// late faults do not imply rollback or discharge of retained obligations.
    pub fn try_acquire_decoded_read_facade_v1(
        &mut self,
        request: DecodedReadAcquisitionFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<ReadAcquisitionFacadeReplyV1, SourceReadContextErrorV1> {
        lifetime::execute(self, Family::Acquisition, r, |gateway| {
            gateway.dispatch_decoded_acquisition(request)
        })
    }
}
impl Gateway<'_> {
    fn dispatch_decoded_acquisition(
        &mut self,
        request: DecodedReadAcquisitionFacadeRequestV1<'_, '_>,
    ) -> Result<ReadAcquisitionFacadeReplyV1, Error> {
        let native = request.native;
        if let Err(e) = self.admit(native.limits) {
            observation(
                self.recorder,
                Boundary::AcquisitionFacadeAdmission,
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
                    .acquire_decoded_read_facade_v1(request, self.recorder)
                    .is_none()
            );
        }
        let n = normalize_read_acquisition_envelope_v1(
            request.envelope,
            native.actor,
            native.limits,
            native
                .limits
                .max_bytes
                .min(self.owner.settings_job_parent_limit()),
            self.recorder,
        )?;
        use ReadAcquisitionFacadePortV1 as P;
        let coherent = matches!(
            native.port,
            P::VersionSetCurrent { .. } | P::VersionSetRetained { .. }
        );
        let reference = match native.port {
            P::SourceRetained { original, .. } => ReadAcquisitionSelectionV1::Source(original),
            P::AnnotationRetained { original, .. } => {
                ReadAcquisitionSelectionV1::Annotation(original)
            }
            P::VersionSetRetained { original, .. } => {
                ReadAcquisitionSelectionV1::VersionSet(original)
            }
            _ => ReadAcquisitionSelectionV1::Current,
        };
        // Argument meaning cannot silently narrow, recapture, redirect or refresh
        // a trusted native port/preparation. Check before observing current loss.
        if coherent != n.coherent
            || reference != n.selection
            || n.participants.len() != native.port.count()
            || n.participants.iter().enumerate().any(|(i, p)| {
                let (scope, classes) = native.port.at(i);
                p.document != &scope.document
                    || p.data != classes
                    || n.profile != scope.context.profile
            })
        {
            return Err(Error::Unavailable);
        }
        self.dispatch_acquisition_admitted(ReadAcquisitionFacadeRequestV1 {
            port: native.port,
            actor: &n.actor,
            limits: native.limits,
        })
    }
}
