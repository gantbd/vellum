//! Adapter-neutral annotation interchange and codec algebra.

use serde::Serialize;
use vellum_engine::coordinator::{AnnotationId, AnnotationSnapshot, DocumentId, DocumentVersion};
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::{
    AnnotationEncoderPort, AnnotationRestoration, AnnotationRestorerPort, CapturedAnnotationRead,
    EncodedAnnotations, ResourceError,
};

#[derive(Clone, Copy, Debug)]
pub struct CodecLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_string_bytes: usize,
    pub max_records: usize,
    pub max_integer_digits: usize,
    pub max_body_bytes: usize,
    pub max_total_body_bytes: usize,
    pub max_associations: usize,
    pub max_source_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationCodecFailure {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationRestorationFailure {
    Codec(AnnotationCodecFailure),
    Engine(ResourceError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationEncodingFailure {
    Codec(AnnotationCodecFailure),
    Engine(ResourceError),
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
    pub fn same_name(&self, other: &Self) -> bool {
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
    pub fn new(
        document: DocumentIdentity,
        source: SourceEvidence,
        records: Vec<StoredRecord>,
    ) -> Self {
        Self {
            format: "vellum.annotations",
            version: "1",
            document,
            source,
            records,
        }
    }
    pub fn needs_historical_evidence(&self) -> bool {
        self.records
            .iter()
            .any(|record| !record.authored_target.source().same_name(&self.source))
    }
}

pub struct DecodeContext {
    pub document: DocumentIdentity,
    pub source: SourceEvidence,
    pub text: String,
}
pub struct DeclaredContext {
    pub document: DocumentIdentity,
    pub source: SourceEvidence,
}
pub struct DecodedAnnotationRead {
    capture: CapturedAnnotationRead,
    snapshot: Snapshot,
}
impl DecodedAnnotationRead {
    pub fn new(capture: CapturedAnnotationRead, snapshot: Snapshot) -> Self {
        Self { capture, snapshot }
    }
    pub fn capture(&self) -> &CapturedAnnotationRead {
        &self.capture
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn into_parts(self) -> (CapturedAnnotationRead, Snapshot) {
        (self.capture, self.snapshot)
    }
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
    pub text: String,
}
pub struct CaptureAssociations {
    pub document: DocumentAssociation,
    pub records: Vec<RecordAssociation>,
    pub sources: Vec<SourceAssociation>,
}
pub struct RestorationSourceAssociation {
    pub witness: AnnotationSnapshot,
    pub evidence: SourceEvidence,
}
pub struct RestorationAssociations {
    pub document: DocumentAssociation,
    pub records: Vec<RecordIdentity>,
    pub sources: Vec<RestorationSourceAssociation>,
}

macro_rules! redacted {
    ($($name:ident),* $(,)?) => {$ (impl std::fmt::Debug for $name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(concat!(stringify!($name), "(<redacted>)"))
        }
    })*};
}
redacted!(
    DocumentIdentity,
    RecordIdentity,
    SourceEvidence,
    Target,
    StoredPlacement,
    StoredRecord,
    Snapshot,
    DecodeContext,
    DeclaredContext,
    DecodedAnnotationRead,
    CaptureAssociations,
    RestorationAssociations
);

pub trait AnnotationCodec {
    fn encode(
        &self,
        snapshot: &AnnotationSnapshot,
        associations: &CaptureAssociations,
        limits: CodecLimits,
        encoder: &AnnotationEncoderPort,
        recorder: &mut Recorder,
    ) -> Result<EncodedAnnotations, AnnotationEncodingFailure>;
    fn inspect(
        &self,
        bytes: &[u8],
        limits: CodecLimits,
    ) -> Result<DeclaredContext, AnnotationCodecFailure>;
    fn decode(
        &self,
        capture: CapturedAnnotationRead,
        context: &DecodeContext,
        limits: CodecLimits,
        recorder: &mut Recorder,
    ) -> Result<DecodedAnnotationRead, AnnotationCodecFailure>;
    fn decoded_document<'a>(&self, decoded: &'a DecodedAnnotationRead) -> &'a DocumentId;
    fn restore(
        &self,
        decoded: DecodedAnnotationRead,
        associations: &RestorationAssociations,
        restorer: &AnnotationRestorerPort,
        limits: CodecLimits,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestoration, AnnotationRestorationFailure>;
}
