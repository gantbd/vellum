//! Reference annotation JSON v1. Pure bounded codec, outside the semantic engine.
//! Decoded names/versions are untrusted data, never live engine edit tokens.
//! Native/source aliases are privileged host assertions, not inferred paths,
//! authentication or continuity proof. No native restoration or baseline publication.
mod parser;
mod restoration;
pub use restoration::{
    RestorationAssociations, RestorationBoundaryError, RestorationSourceAssociation,
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::io::Write;
use vellum_engine::attachment::{
    Affinity as NativeAffinity, AttachmentTarget, LocationEvidence, Placement,
    PointProfile as NativeProfile,
};
use vellum_engine::coordinator::{AnnotationId, AnnotationSnapshot, DocumentId, DocumentVersion};
use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};

#[derive(Clone, Copy, Debug)]
pub struct CodecLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_nodes: usize,
    /// This pinned profile accepts 1..=64; the underlying parser's own recursion
    /// guard remains enabled as an independent backstop.
    pub max_depth: usize,
    pub max_string_bytes: usize,
    pub max_records: usize,
    /// Canonical u64 decimal representation, at most 20 digits in this profile.
    pub max_integer_digits: usize,
    pub max_body_bytes: usize,
    pub max_total_body_bytes: usize,
    pub max_associations: usize,
    pub max_source_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecError {
    InvalidJson,
    DuplicateMember,
    Limit,
    Shape,
    Unsupported,
    Association,
    SourceMismatch,
    InvalidTarget,
    AmbiguousIdentity,
    HistoricalEvidenceMissing,
    UnavailablePlacement,
    Capacity,
    OutputLimit,
}
macro_rules! redacted {
    ($($name:ident),* $(,)?) => {$ (impl std::fmt::Debug for $name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(concat!(stringify!($name), "(<redacted>)")) }
    })*};
}
fn decimal<S: serde::Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct DocumentIdentity {
    pub namespace: String,
    pub value: String,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RecordIdentity {
    pub namespace: String,
    pub value: String,
    #[serde(serialize_with = "decimal")]
    pub incarnation: u64,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct SourceEvidence {
    pub domain: String,
    pub token: String,
    #[serde(rename = "byteLength", serialize_with = "decimal")]
    pub byte_length: u64,
}
impl SourceEvidence {
    fn same_name(&self, other: &Self) -> bool {
        self.domain == other.domain && self.token == other.token
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PointAffinity {
    Before,
    After,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PointMeaning {
    Position,
    Content,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Target {
    Document {
        source: SourceEvidence,
    },
    Point {
        source: SourceEvidence,
        #[serde(serialize_with = "decimal")]
        offset: u64,
        affinity: PointAffinity,
        profile: PointMeaning,
    },
    Range {
        source: SourceEvidence,
        #[serde(serialize_with = "decimal")]
        start: u64,
        #[serde(serialize_with = "decimal")]
        end: u64,
    },
}
impl Target {
    pub fn source(&self) -> &SourceEvidence {
        match self {
            Self::Document { source } | Self::Point { source, .. } | Self::Range { source, .. } => {
                source
            }
        }
    }
}
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum StoredPlacement {
    Attached { target: Target },
    Affected { target: Target },
    Detached,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct StoredRecord {
    pub identity: RecordIdentity,
    #[serde(rename = "contentVersion", serialize_with = "decimal")]
    pub content_version: u64,
    #[serde(rename = "targetGeneration", serialize_with = "decimal")]
    pub target_generation: u64,
    pub body: String,
    #[serde(rename = "authoredTarget")]
    pub authored_target: Target,
    pub placement: StoredPlacement,
}
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    format: &'static str,
    version: &'static str,
    pub document: DocumentIdentity,
    pub source: SourceEvidence,
    pub records: Vec<StoredRecord>,
}
impl Snapshot {
    /// Parsing historical selectors establishes syntax/declared bounds only.
    /// They still require trusted historical evidence at native activation.
    pub fn needs_historical_evidence(&self) -> bool {
        self.records
            .iter()
            .any(|r| !r.authored_target.source().same_name(&self.source))
    }
}
pub struct DecodeContext {
    pub document: DocumentIdentity,
    pub source: SourceEvidence,
    pub text: String,
}
/// Bounded, duplicate-checked UNTRUSTED header. Inspection does not validate
/// records, approve identities, authenticate an author or establish source truth.
pub struct DeclaredContext {
    pub document: DocumentIdentity,
    pub source: SourceEvidence,
}
pub fn inspect_context(bytes: &[u8], limits: CodecLimits) -> Result<DeclaredContext, CodecError> {
    profile(limits)?;
    if bytes.len() > limits.max_input_bytes {
        return Err(CodecError::Limit);
    }
    let value = parser::parse(bytes, limits)?;
    let root = object(
        &value,
        &["format", "version", "document", "source", "records"],
    )?;
    if root["format"].as_str() != Some("vellum.annotations")
        || root["version"].as_str() != Some("1")
    {
        return Err(CodecError::Unsupported);
    }
    Ok(DeclaredContext {
        document: document(&root["document"])?,
        source: source(&root["source"], limits)?,
    })
}
/// Exact decoded original owned input, not a native identity/restoration grant.
pub struct DecodedAnnotationRead {
    capture: vellum_engine::resource::CapturedAnnotationRead,
    snapshot: Snapshot,
}
impl DecodedAnnotationRead {
    pub fn capture(&self) -> &vellum_engine::resource::CapturedAnnotationRead {
        &self.capture
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}
pub fn decode_read(
    capture: vellum_engine::resource::CapturedAnnotationRead,
    context: &DecodeContext,
    limits: CodecLimits,
    recorder: &mut Recorder,
) -> Result<DecodedAnnotationRead, CodecError> {
    let snapshot = decode(capture.bytes(), context, limits, recorder)?;
    Ok(DecodedAnnotationRead { capture, snapshot })
}
#[derive(Clone)]
pub struct DocumentAssociation {
    pub native: DocumentId,
    pub identity: DocumentIdentity,
}
#[derive(Clone)]
pub struct RecordAssociation {
    pub native: AnnotationId,
    pub identity: RecordIdentity,
}
pub struct SourceAssociation {
    pub native: DocumentVersion,
    pub evidence: SourceEvidence,
    /// Canonical document scalar text, NOT a file-byte/BOM decoding input.
    pub text: String,
}
pub struct CaptureAssociations {
    pub document: DocumentAssociation,
    pub records: Vec<RecordAssociation>,
    pub sources: Vec<SourceAssociation>,
}
pub struct Artifact {
    bytes: Vec<u8>,
    basis: AnnotationSnapshot,
}
impl Artifact {
    /// Transfers the validated bytes and original native capture to the engine's
    /// configured privileged producer seam; this does not retain/dispatch them.
    pub fn into_encoded(
        self,
        port: &vellum_engine::resource::AnnotationEncoderPort,
    ) -> Result<vellum_engine::resource::EncodedAnnotations, vellum_engine::resource::ResourceError>
    {
        port.seal(self.basis, self.bytes)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn snapshot(&self) -> &AnnotationSnapshot {
        &self.basis
    }
}
redacted!(
    DocumentIdentity,
    RecordIdentity,
    SourceEvidence,
    Target,
    StoredPlacement,
    StoredRecord,
    Snapshot,
    Artifact,
    DecodeContext,
    CaptureAssociations,
    DecodedAnnotationRead
);

fn observe<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, CodecError>) {
    recorder.record(Observation {
        boundary,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: None,
    });
}
fn profile(limits: CodecLimits) -> Result<(), CodecError> {
    if !(1..=64).contains(&limits.max_depth) || !(1..=20).contains(&limits.max_integer_digits) {
        return Err(CodecError::Limit);
    }
    Ok(())
}
fn object<'a>(value: &'a Value, fields: &[&str]) -> Result<&'a Map<String, Value>, CodecError> {
    let value = value.as_object().ok_or(CodecError::Shape)?;
    if value.len() != fields.len() || fields.iter().any(|key| !value.contains_key(*key)) {
        return Err(CodecError::Shape);
    }
    Ok(value)
}
fn text(value: &Value) -> Result<String, CodecError> {
    value.as_str().map(str::to_owned).ok_or(CodecError::Shape)
}
fn nonempty(value: &Value) -> Result<String, CodecError> {
    let text = text(value)?;
    if text.is_empty() {
        Err(CodecError::Shape)
    } else {
        Ok(text)
    }
}
fn number(value: &Value, positive: bool, limits: CodecLimits) -> Result<u64, CodecError> {
    let text = value.as_str().ok_or(CodecError::Shape)?;
    if text.is_empty() || text.len() > limits.max_integer_digits {
        return Err(CodecError::Limit);
    }
    if !text.bytes().all(|b| b.is_ascii_digit()) || (text.len() > 1 && text.starts_with('0')) {
        return Err(CodecError::Shape);
    }
    let value: u64 = text.parse().map_err(|_| CodecError::Limit)?;
    if positive && value == 0 {
        Err(CodecError::Shape)
    } else {
        Ok(value)
    }
}
fn document(value: &Value) -> Result<DocumentIdentity, CodecError> {
    let v = object(value, &["namespace", "value"])?;
    Ok(DocumentIdentity {
        namespace: nonempty(&v["namespace"])?,
        value: nonempty(&v["value"])?,
    })
}
fn source(value: &Value, limits: CodecLimits) -> Result<SourceEvidence, CodecError> {
    let v = object(value, &["domain", "token", "byteLength"])?;
    Ok(SourceEvidence {
        domain: nonempty(&v["domain"])?,
        token: nonempty(&v["token"])?,
        byte_length: number(&v["byteLength"], false, limits)?,
    })
}
fn target(
    value: &Value,
    context: &DecodeContext,
    located: bool,
    limits: CodecLimits,
) -> Result<Target, CodecError> {
    let v = value.as_object().ok_or(CodecError::Shape)?;
    let kind = v
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(CodecError::Shape)?;
    let target = match kind {
        "document" => {
            let v = object(value, &["kind", "source"])?;
            Target::Document {
                source: source(&v["source"], limits)?,
            }
        }
        "point" => {
            let v = object(value, &["kind", "source", "offset", "affinity", "profile"])?;
            let affinity = match v["affinity"].as_str() {
                Some("before") => PointAffinity::Before,
                Some("after") => PointAffinity::After,
                _ => return Err(CodecError::Shape),
            };
            let profile = match v["profile"].as_str() {
                Some("position") => PointMeaning::Position,
                Some("content") => PointMeaning::Content,
                _ => return Err(CodecError::Shape),
            };
            Target::Point {
                source: source(&v["source"], limits)?,
                offset: number(&v["offset"], false, limits)?,
                affinity,
                profile,
            }
        }
        "range" => {
            let v = object(value, &["kind", "source", "start", "end"])?;
            Target::Range {
                source: source(&v["source"], limits)?,
                start: number(&v["start"], false, limits)?,
                end: number(&v["end"], false, limits)?,
            }
        }
        _ => return Err(CodecError::Unsupported),
    };
    validate_target(&target, context, located)?;
    Ok(target)
}
fn validate_target(
    target: &Target,
    context: &DecodeContext,
    located: bool,
) -> Result<(), CodecError> {
    let evidence = target.source();
    let current = evidence.same_name(&context.source);
    if (located && !current) || (current && evidence.byte_length != context.source.byte_length) {
        return Err(CodecError::SourceMismatch);
    }
    let boundaries = match target {
        Target::Document { .. } => return Ok(()),
        Target::Point { offset, .. } => {
            if *offset > evidence.byte_length {
                return Err(CodecError::InvalidTarget);
            }
            (*offset, *offset)
        }
        Target::Range { start, end, .. } => {
            if start >= end || *end > evidence.byte_length {
                return Err(CodecError::InvalidTarget);
            }
            (*start, *end)
        }
    };
    if current {
        for offset in [boundaries.0, boundaries.1] {
            let offset = usize::try_from(offset).map_err(|_| CodecError::InvalidTarget)?;
            if !context.text.is_char_boundary(offset) {
                return Err(CodecError::InvalidTarget);
            }
        }
    }
    Ok(())
}
pub fn decode(
    bytes: &[u8],
    context: &DecodeContext,
    limits: CodecLimits,
    recorder: &mut Recorder,
) -> Result<Snapshot, CodecError> {
    let result = (|| {
        profile(limits)?;
        if bytes.len() > limits.max_input_bytes || context.text.len() > limits.max_source_bytes {
            return Err(CodecError::Limit);
        }
        if context.text.len() as u64 != context.source.byte_length {
            return Err(CodecError::SourceMismatch);
        }
        let value = parser::parse(bytes, limits)?;
        let root = object(
            &value,
            &["format", "version", "document", "source", "records"],
        )?;
        if root["format"].as_str() != Some("vellum.annotations")
            || root["version"].as_str() != Some("1")
        {
            return Err(CodecError::Unsupported);
        }
        let document = document(&root["document"])?;
        let source = source(&root["source"], limits)?;
        if document != context.document || source != context.source {
            return Err(CodecError::SourceMismatch);
        }
        let rows = root["records"].as_array().ok_or(CodecError::Shape)?;
        if rows.len() > limits.max_records {
            return Err(CodecError::Limit);
        }
        let mut records: Vec<StoredRecord> = Vec::new();
        records
            .try_reserve_exact(rows.len())
            .map_err(|_| CodecError::Capacity)?;
        let mut total = 0usize;
        for value in rows {
            let row = object(
                value,
                &[
                    "identity",
                    "contentVersion",
                    "targetGeneration",
                    "body",
                    "authoredTarget",
                    "placement",
                ],
            )?;
            let id = object(&row["identity"], &["namespace", "value", "incarnation"])?;
            let identity = RecordIdentity {
                namespace: nonempty(&id["namespace"])?,
                value: nonempty(&id["value"])?,
                incarnation: number(&id["incarnation"], true, limits)?,
            };
            if records.iter().any(|r| {
                r.identity.namespace == identity.namespace && r.identity.value == identity.value
            }) {
                return Err(CodecError::AmbiguousIdentity);
            }
            let body = text(&row["body"])?;
            total = total.checked_add(body.len()).ok_or(CodecError::Limit)?;
            if body.len() > limits.max_body_bytes || total > limits.max_total_body_bytes {
                return Err(CodecError::Limit);
            }
            let place = row["placement"].as_object().ok_or(CodecError::Shape)?;
            let placement = match place.get("state").and_then(Value::as_str) {
                Some("detached") => {
                    object(&row["placement"], &["state"])?;
                    StoredPlacement::Detached
                }
                Some(state @ ("attached" | "affected")) => {
                    object(&row["placement"], &["state", "target"])?;
                    let target = target(&place["target"], context, true, limits)?;
                    if state == "attached" {
                        StoredPlacement::Attached { target }
                    } else {
                        StoredPlacement::Affected { target }
                    }
                }
                _ => return Err(CodecError::Shape),
            };
            records.push(StoredRecord {
                identity,
                content_version: number(&row["contentVersion"], true, limits)?,
                target_generation: number(&row["targetGeneration"], true, limits)?,
                body,
                authored_target: target(&row["authoredTarget"], context, false, limits)?,
                placement,
            });
        }
        Ok(Snapshot {
            format: "vellum.annotations",
            version: "1",
            document,
            source,
            records,
        })
    })();
    observe(recorder, Boundary::AnnotationDecoding, &result);
    result
}
fn mapped_target(
    target: AttachmentTarget,
    source: &SourceAssociation,
) -> Result<Target, CodecError> {
    let evidence = source.evidence.clone();
    let mapped = match target {
        AttachmentTarget::Document => Target::Document { source: evidence },
        AttachmentTarget::Point {
            offset,
            affinity,
            profile,
        } => Target::Point {
            source: evidence,
            offset: offset as u64,
            affinity: match affinity {
                NativeAffinity::Before => PointAffinity::Before,
                NativeAffinity::After => PointAffinity::After,
            },
            profile: match profile {
                NativeProfile::Position => PointMeaning::Position,
                NativeProfile::Content => PointMeaning::Content,
            },
        },
        AttachmentTarget::ContentRange { start, end } => Target::Range {
            source: evidence,
            start: start as u64,
            end: end as u64,
        },
    };
    let text = source.text.as_str();
    for offset in match &mapped {
        Target::Document { .. } => vec![],
        Target::Point { offset, .. } => vec![*offset],
        Target::Range { start, end, .. } => {
            if start >= end {
                return Err(CodecError::InvalidTarget);
            }
            vec![*start, *end]
        }
    } {
        if !usize::try_from(offset).is_ok_and(|p| text.is_char_boundary(p)) {
            return Err(CodecError::InvalidTarget);
        }
    }
    Ok(mapped)
}
struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
    failure: Option<CodecError>,
}
// Exact decoded-value accounting for the closed v1 producer, before retaining
// each row. Repeated source aliases count at every occurrence; a short alias
// table cannot amplify into an oversized intermediate wire snapshot.
struct WireBudget {
    limits: CodecLimits,
    nodes: usize,
    strings: usize,
}
impl WireBudget {
    fn node(&mut self) -> Result<(), CodecError> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|n| *n <= self.limits.max_nodes)
            .ok_or(CodecError::Limit)?;
        Ok(())
    }
    fn bytes(&mut self, length: usize) -> Result<(), CodecError> {
        self.strings = self
            .strings
            .checked_add(length)
            .filter(|n| *n <= self.limits.max_string_bytes)
            .ok_or(CodecError::Limit)?;
        Ok(())
    }
    fn object(&mut self, keys: &[&str]) -> Result<(), CodecError> {
        self.node()?;
        for key in keys {
            self.bytes(key.len())?;
        }
        Ok(())
    }
    fn text(&mut self, text: &str) -> Result<(), CodecError> {
        self.node()?;
        self.bytes(text.len())
    }
    fn number(&mut self, number: u64, positive: bool) -> Result<(), CodecError> {
        let text = number.to_string();
        if text.len() > self.limits.max_integer_digits || (positive && number == 0) {
            return Err(CodecError::Limit);
        }
        self.text(&text)
    }
    fn source(&mut self, source: &SourceEvidence) -> Result<(), CodecError> {
        self.object(&["domain", "token", "byteLength"])?;
        self.text(&source.domain)?;
        self.text(&source.token)?;
        self.number(source.byte_length, false)
    }
    fn target(&mut self, target: &Target) -> Result<(), CodecError> {
        match target {
            Target::Document { .. } => {
                self.object(&["kind", "source"])?;
                self.text("document")?;
            }
            Target::Point {
                offset,
                affinity,
                profile,
                ..
            } => {
                self.object(&["kind", "source", "offset", "affinity", "profile"])?;
                self.text("point")?;
                self.number(*offset, false)?;
                self.text(match affinity {
                    PointAffinity::Before => "before",
                    PointAffinity::After => "after",
                })?;
                self.text(match profile {
                    PointMeaning::Position => "position",
                    PointMeaning::Content => "content",
                })?;
            }
            Target::Range { start, end, .. } => {
                self.object(&["kind", "source", "start", "end"])?;
                self.text("range")?;
                self.number(*start, false)?;
                self.number(*end, false)?;
            }
        }
        self.source(target.source())
    }
    fn header(
        &mut self,
        document: &DocumentIdentity,
        source: &SourceEvidence,
    ) -> Result<(), CodecError> {
        self.object(&["format", "version", "document", "source", "records"])?;
        self.text("vellum.annotations")?;
        self.text("1")?;
        self.object(&["namespace", "value"])?;
        self.text(&document.namespace)?;
        self.text(&document.value)?;
        self.source(source)?;
        self.node()
    }
    fn row(&mut self, row: &StoredRecord) -> Result<(), CodecError> {
        self.object(&[
            "identity",
            "contentVersion",
            "targetGeneration",
            "body",
            "authoredTarget",
            "placement",
        ])?;
        self.object(&["namespace", "value", "incarnation"])?;
        self.text(&row.identity.namespace)?;
        self.text(&row.identity.value)?;
        self.number(row.identity.incarnation, true)?;
        self.number(row.content_version, true)?;
        self.number(row.target_generation, true)?;
        self.text(&row.body)?;
        self.target(&row.authored_target)?;
        match &row.placement {
            StoredPlacement::Detached => {
                self.object(&["state"])?;
                self.text("detached")
            }
            StoredPlacement::Attached { target } | StoredPlacement::Affected { target } => {
                self.object(&["state", "target"])?;
                self.text(
                    if matches!(row.placement, StoredPlacement::Attached { .. }) {
                        "attached"
                    } else {
                        "affected"
                    },
                )?;
                self.target(target)
            }
        }
    }
}
impl Write for BoundedWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(input.len())
            .is_none_or(|n| n > self.limit)
        {
            self.failure = Some(CodecError::OutputLimit);
            return Err(std::io::Error::other("bounded output"));
        }
        if self.bytes.try_reserve_exact(input.len()).is_err() {
            self.failure = Some(CodecError::Capacity);
            return Err(std::io::Error::other("bounded allocation"));
        }
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn encode_capture(
    snapshot: &AnnotationSnapshot,
    associations: &CaptureAssociations,
    limits: CodecLimits,
    recorder: &mut Recorder,
) -> Result<Artifact, CodecError> {
    let result = (|| {
        profile(limits)?;
        if associations
            .records
            .len()
            .checked_add(associations.sources.len())
            .and_then(|n| n.checked_add(1))
            .is_none_or(|n| n > limits.max_associations)
            || snapshot.records().len() > limits.max_records
        {
            return Err(CodecError::Limit);
        }
        if associations.document.native != snapshot.source().document
            || associations.records.len() != snapshot.records().len()
        {
            return Err(CodecError::Association);
        }
        let mut strings = associations
            .document
            .identity
            .namespace
            .len()
            .checked_add(associations.document.identity.value.len())
            .ok_or(CodecError::Limit)?;
        let mut source_bytes = 0usize;
        for (index, source) in associations.sources.iter().enumerate() {
            if source.native.document != snapshot.source().document
                || source.text.len() as u64 != source.evidence.byte_length
                || source.evidence.domain.is_empty()
                || source.evidence.token.is_empty()
            {
                return Err(CodecError::Association);
            }
            if associations.sources[..index]
                .iter()
                .any(|s| s.native == source.native || s.evidence.same_name(&source.evidence))
            {
                return Err(CodecError::Association);
            }
            source_bytes = source_bytes
                .checked_add(source.text.len())
                .ok_or(CodecError::Limit)?;
            strings = strings
                .checked_add(source.evidence.domain.len())
                .and_then(|n| n.checked_add(source.evidence.token.len()))
                .ok_or(CodecError::Limit)?;
        }
        for (index, record) in associations.records.iter().enumerate() {
            if !snapshot
                .records()
                .iter()
                .any(|r| r.annotation == record.native)
                || record.identity.namespace.is_empty()
                || record.identity.value.is_empty()
                || record.identity.incarnation == 0
            {
                return Err(CodecError::Association);
            }
            if associations.records[..index].iter().any(|r| {
                r.native == record.native
                    || (r.identity.namespace == record.identity.namespace
                        && r.identity.value == record.identity.value)
            }) {
                return Err(CodecError::Association);
            }
            if let Some(restored) = snapshot
                .records()
                .iter()
                .find(|r| r.annotation == record.native)
                .and_then(|r| r.restoration_identity())
            {
                if record
                    .identity
                    .namespace
                    .len()
                    .checked_add(record.identity.value.len())
                    .is_none_or(|n| n > limits.max_string_bytes)
                {
                    return Err(CodecError::Limit);
                }
                let key = serde_json::to_vec(&[&record.identity.namespace, &record.identity.value])
                    .map_err(|_| CodecError::Capacity)?;
                if key.as_slice() != restored.key()
                    || record.identity.incarnation != restored.incarnation()
                {
                    return Err(CodecError::Association);
                }
            }
            strings = strings
                .checked_add(record.identity.namespace.len())
                .and_then(|n| n.checked_add(record.identity.value.len()))
                .ok_or(CodecError::Limit)?;
        }
        if source_bytes > limits.max_source_bytes || strings > limits.max_string_bytes {
            return Err(CodecError::Limit);
        }
        let current = associations
            .sources
            .iter()
            .find(|s| s.native == *snapshot.source())
            .ok_or(CodecError::HistoricalEvidenceMissing)?;
        if current.text != snapshot.source_text() {
            return Err(CodecError::SourceMismatch);
        }
        let mut budget = WireBudget {
            limits,
            nodes: 0,
            strings: 0,
        };
        budget.header(&associations.document.identity, &current.evidence)?;
        let mut rows = Vec::new();
        let mut bodies = 0usize;
        for record in snapshot.records() {
            bodies = bodies
                .checked_add(record.body().len())
                .ok_or(CodecError::Limit)?;
            if bodies > limits.max_total_body_bytes || record.body().len() > limits.max_body_bytes {
                return Err(CodecError::Limit);
            }
            let identity = &associations
                .records
                .iter()
                .find(|r| r.native == record.annotation)
                .ok_or(CodecError::Association)?
                .identity;
            let authored = associations
                .sources
                .iter()
                .find(|s| s.native == record.authored.source)
                .ok_or(CodecError::HistoricalEvidenceMissing)?;
            let placement = match record.placement {
                Placement::Detached => StoredPlacement::Detached,
                Placement::Unavailable => return Err(CodecError::UnavailablePlacement),
                Placement::Located { target, evidence } => {
                    if record.placement_revision != Some(snapshot.source().revision) {
                        return Err(CodecError::SourceMismatch);
                    }
                    let target = mapped_target(target, current)?;
                    match evidence {
                        LocationEvidence::Located => StoredPlacement::Attached { target },
                        LocationEvidence::Affected => StoredPlacement::Affected { target },
                    }
                }
            };
            let row = StoredRecord {
                identity: identity.clone(),
                content_version: record.content_version,
                target_generation: record.target_generation,
                body: record.body().to_owned(),
                authored_target: mapped_target(record.authored.target, authored)?,
                placement,
            };
            budget.row(&row)?;
            rows.try_reserve(1).map_err(|_| CodecError::Capacity)?;
            rows.push(row);
        }
        let value = Snapshot {
            format: "vellum.annotations",
            version: "1",
            document: associations.document.identity.clone(),
            source: current.evidence.clone(),
            records: rows,
        };
        let mut writer = BoundedWriter {
            bytes: Vec::new(),
            limit: limits.max_output_bytes,
            failure: None,
        };
        if serde_json::to_writer(&mut writer, &value).is_err() {
            return Err(writer.failure.unwrap_or(CodecError::InvalidJson));
        }
        let context = DecodeContext {
            document: value.document.clone(),
            source: value.source.clone(),
            text: snapshot.source_text().into(),
        };
        // The exact retained output passes the same strict v1 decoder before
        // delivery. This establishes encoding conformance, not provider truth.
        decode(&writer.bytes, &context, limits, &mut Recorder::disabled())?;
        Ok(Artifact {
            bytes: writer.bytes,
            basis: snapshot.clone(),
        })
    })();
    observe(recorder, Boundary::AnnotationEncoding, &result);
    result
}
