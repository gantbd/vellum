//! Single-user host persistence labels, never native identities or authority.
use super::*;
use sha2::{Digest, Sha256};
use vellum_engine::coordinator::AnnotationId;
use vellum_sdk::annotation::{
    DocumentAssociation, DocumentIdentity, RecordAssociation, RecordIdentity, SourceAssociation,
    SourceEvidence,
};

const DOCUMENT_NS: &str = "vellum.document.uuid.v1";
const RECORD_NS: &str = "vellum.note.uuid.v1";
const SOURCE_DOMAIN: &str = "vellum.canonical-utf8.sha256-uuid.v1";

/// One original UI confirmation, not a write grant or a path-derived identity.
#[derive(Clone)]
pub struct CommentFilePlan {
    marker: std::sync::Arc<()>,
    source: DocumentVersion,
    set: u64,
    path: PathBuf,
    load: bool,
}
impl CommentFilePlan {
    pub fn destination(&self) -> &Path {
        &self.path
    }
    pub fn is_load(&self) -> bool {
        self.load
    }
}
impl PartialEq for CommentFilePlan {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for CommentFilePlan {}
impl std::fmt::Debug for CommentFilePlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CommentFilePlan(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommentFileLimits {
    pub max_records: usize,
    pub max_sources: usize,
    pub max_source_bytes: usize,
    pub codec: CodecLimits,
}
impl CommentFileLimits {
    /// Existing reference-app ceilings, not an adopted product capacity or RSS
    /// guarantee. Host witness retention has its OWN explicit byte/count bounds.
    pub fn reference() -> Self {
        Self {
            max_records: 4096,
            max_sources: 128,
            max_source_bytes: 1048576,
            codec: CodecLimits {
                max_input_bytes: 65536,
                max_output_bytes: 65536,
                max_nodes: 8192,
                max_depth: 32,
                max_string_bytes: 65536,
                max_records: 128,
                max_integer_digits: 20,
                max_body_bytes: 4096,
                max_total_body_bytes: 65536,
                max_associations: 4225,
                max_source_bytes: 1048576,
            },
        }
    }
}
pub(in crate::app) struct Names {
    pub(super) document: DocumentIdentity,
    pub(super) records: Vec<RecordAssociation>,
    pub(super) sources: Vec<SourceAssociation>,
}
pub(in crate::app) struct PreparedNames {
    source: Option<SourceAssociation>,
    records: Vec<RecordIdentity>,
}
fn uuid() -> Result<String, EditorError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| EditorError::IdentityUnavailable)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut value = String::new();
    value
        .try_reserve_exact(36)
        .map_err(|_| EditorError::Capacity)?;
    use std::fmt::Write;
    for (i, byte) in bytes.iter().enumerate() {
        if [4, 6, 8, 10].contains(&i) {
            value.push('-');
        }
        write!(&mut value, "{byte:02x}").expect("preallocated UUID string");
    }
    Ok(value)
}
pub(super) fn evidence(text: &str) -> Result<SourceEvidence, EditorError> {
    Ok(SourceEvidence {
        domain: SOURCE_DOMAIN.into(),
        token: format!("{}:{:x}", uuid()?, Sha256::digest(text.as_bytes())),
        byte_length: text.len() as u64,
    })
}
impl Editor {
    pub fn comment_file_needs_load(&self) -> bool {
        self.tabs[self.active].comment_file_requires_load
    }
    pub fn plan_comment_file(
        &mut self,
        path: PathBuf,
        load: bool,
    ) -> Result<CommentFilePlan, EditorError> {
        self.live()?;
        if self.comment_file_limits.is_none()
            || self.comment_file_bound()
            || path.as_os_str().is_empty()
        {
            return Err(EditorError::CommentsUnavailable);
        }
        let source = self.version()?;
        let set = self
            .host
            .annotation_set_version(&self.tabs[self.active].document)?;
        let marker = std::sync::Arc::new(());
        self.comment_file_plan = Some(marker.clone());
        Ok(CommentFilePlan {
            marker,
            source,
            set,
            path,
            load,
        })
    }
    pub fn cancel_comment_file_plan(&mut self) {
        self.comment_file_plan = None;
    }
    pub fn confirm_comment_file(&mut self, plan: &CommentFilePlan) -> Result<(), EditorError> {
        self.live()?;
        let pending = self.comment_file_plan.take().ok_or(EditorError::Position)?;
        if !std::sync::Arc::ptr_eq(&pending, &plan.marker)
            || self.version()? != plan.source
            || self
                .host
                .annotation_set_version(&self.tabs[self.active].document)?
                != plan.set
        {
            return Err(EditorError::Position);
        }
        self.bind_comment_file(&plan.path)?;
        if plan.load {
            self.load_comment_file()
        } else {
            self.save_comment_file()
        }
    }
    /// Explicitly load the selected UUID-profile companion. File labels remain
    /// inert until format, current canonical content, live-label conflicts and
    /// the native restoration owner have all validated their respective facts.
    pub fn load_comment_file(&mut self) -> Result<(), EditorError> {
        use super::read::{Approval, PreparedLoad};
        use vellum_sdk::annotation::{
            DecodeContext, RestorationAssociations, RestorationSourceAssociation,
        };
        let limits = self
            .comment_file_limits
            .ok_or(EditorError::CommentsUnavailable)?;
        self.live()?;
        if self
            .host
            .annotation_document_summary(&self.tabs[self.active].document)?
            .set_version
            != 0
            || self.last_comment_read_status()?.is_some()
        {
            return Err(EditorError::Position);
        }
        if self
            .host
            .annotation_baseline_evidence(&self.tabs[self.active].document)?
            .is_none()
        {
            self.tabs[self.active].comment_file_requires_load = true;
        }
        let mut existing = Vec::new();
        existing
            .try_reserve_exact(self.tabs.len())
            .map_err(|_| EditorError::Capacity)?;
        existing.extend(
            self.tabs
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != self.active)
                .filter_map(|(_, t)| t.comment_names.as_ref().map(|n| n.document.clone())),
        );
        self.load_comments_prepared(
            limits.codec,
            limits.max_source_bytes,
            move |host, codec, attempt, document| {
                let header = host.inspect_annotations(codec, attempt, limits.codec)?;
                if header.document.namespace != DOCUMENT_NS || !is_uuid(&header.document.value) {
                    return Err(EditorError::IdentityUnavailable);
                }
                if existing.contains(&header.document) {
                    return Err(EditorError::IdentityConflict);
                }
                let source = host.read(document)?.version;
                let set = host.annotation_set_version(document)?;
                let witness = host.capture_annotations(&source, set)?;
                let text = witness.source_text();
                let valid_source =
                    header
                        .source
                        .token
                        .split_once(':')
                        .is_some_and(|(id, digest)| {
                            is_uuid(id)
                                && digest == format!("{:x}", Sha256::digest(text.as_bytes()))
                        });
                if header.source.domain != SOURCE_DOMAIN
                    || header.source.byte_length != text.len() as u64
                    || !valid_source
                {
                    return Err(EditorError::Session(SessionError::AnnotationCodec(
                        vellum_sdk::annotation::AnnotationCodecFailure::SourceMismatch,
                    )));
                }
                if text.len() > limits.max_source_bytes || limits.max_sources == 0 {
                    return Err(EditorError::Capacity);
                }
                let context = DecodeContext {
                    document: header.document.clone(),
                    source: header.source.clone(),
                    text: text.into(),
                };
                let decoded = host.decode_annotations(codec, attempt, &context, limits.codec)?;
                if decoded.snapshot().needs_historical_evidence() {
                    return Err(EditorError::Session(SessionError::AnnotationCodec(
                        vellum_sdk::annotation::AnnotationCodecFailure::HistoricalEvidenceMissing,
                    )));
                }
                let mut records = Vec::new();
                let count = decoded.snapshot().records.len();
                if count > limits.max_records {
                    return Err(EditorError::Capacity);
                }
                records
                    .try_reserve_exact(count)
                    .map_err(|_| EditorError::Capacity)?;
                for row in &decoded.snapshot().records {
                    if row.identity.namespace != RECORD_NS
                        || !is_uuid(&row.identity.value)
                        || row.identity.incarnation != 1
                    {
                        return Err(EditorError::IdentityUnavailable);
                    }
                    records.push(row.identity.clone());
                }
                let mut aliases = Vec::new();
                aliases
                    .try_reserve_exact(count)
                    .map_err(|_| EditorError::Capacity)?;
                let names = Names {
                    document: header.document.clone(),
                    records: aliases,
                    sources: vec![SourceAssociation {
                        native: source.clone(),
                        evidence: header.source.clone(),
                        text: context.text,
                    }],
                };
                let approved = RestorationAssociations {
                    document: DocumentAssociation {
                        native: source.document,
                        identity: header.document,
                    },
                    records: records.clone(),
                    sources: vec![RestorationSourceAssociation {
                        witness,
                        evidence: header.source,
                    }],
                };
                Ok(PreparedLoad {
                    decoded,
                    approved: Approval::Owned(approved),
                    names: Some((names, records)),
                })
            },
        )
        .map(|_| ())
    }
    /// Enable the approved local-app naming policy explicitly. External identity
    /// APIs stay available; this does not add a kernel registry or auto sidecar.
    pub fn enable_comment_files(&mut self, limits: CommentFileLimits) -> Result<(), EditorError> {
        self.live()?;
        if !self.comments_enabled() {
            return Err(EditorError::CommentsUnavailable);
        }
        if self.comment_file_limits.is_some() {
            return Err(EditorError::Position);
        }
        for tab in &self.tabs {
            if self
                .host
                .annotation_document_summary(&tab.document)?
                .set_version
                != 0
            {
                return Err(EditorError::Position);
            }
        }
        self.comment_file_limits = Some(limits);
        Ok(())
    }
    pub(in crate::app) fn prepare_comment_names(
        &mut self,
        expected: &DocumentVersion,
        count: usize,
    ) -> Result<Option<PreparedNames>, EditorError> {
        let Some(limits) = self.comment_file_limits else {
            return Ok(None);
        };
        if self.version()? != *expected {
            return Err(EditorError::Position);
        }
        if self.tabs[self.active].comment_names.is_none() {
            let value = uuid()?;
            if self
                .tabs
                .iter()
                .filter_map(|t| t.comment_names.as_ref())
                .any(|n| n.document.value == value)
            {
                return Err(EditorError::IdentityConflict);
            }
            self.tabs[self.active].comment_names = Some(Names {
                document: DocumentIdentity {
                    namespace: DOCUMENT_NS.into(),
                    value,
                },
                records: Vec::new(),
                sources: Vec::new(),
            });
        }
        let names = self.tabs[self.active]
            .comment_names
            .as_ref()
            .expect("host names");
        if names
            .records
            .len()
            .checked_add(count)
            .is_none_or(|n| n > limits.max_records)
        {
            return Err(EditorError::Capacity);
        }
        let source = if names.sources.iter().any(|s| s.native == *expected) {
            None
        } else {
            let snapshot = self.snapshot()?;
            let names = self.tabs[self.active]
                .comment_names
                .as_ref()
                .expect("host names");
            let held = names
                .sources
                .iter()
                .try_fold(0usize, |n, s| n.checked_add(s.text.len()))
                .ok_or(EditorError::Capacity)?;
            if names.sources.len() >= limits.max_sources
                || held
                    .checked_add(snapshot.text.len())
                    .is_none_or(|n| n > limits.max_source_bytes)
            {
                return Err(EditorError::Capacity);
            }
            Some(SourceAssociation {
                native: snapshot.version,
                evidence: evidence(&snapshot.text)?,
                text: snapshot.text,
            })
        };
        let names = self.tabs[self.active]
            .comment_names
            .as_mut()
            .expect("host names");
        names
            .records
            .try_reserve(count)
            .map_err(|_| EditorError::Capacity)?;
        if source.is_some() {
            names
                .sources
                .try_reserve(1)
                .map_err(|_| EditorError::Capacity)?;
        }
        let mut records: Vec<RecordIdentity> = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|_| EditorError::Capacity)?;
        for _ in 0..count {
            let value = uuid()?;
            if names.records.iter().any(|r| r.identity.value == value)
                || records.iter().any(|r| r.value == value)
            {
                return Err(EditorError::IdentityConflict);
            }
            records.push(RecordIdentity {
                namespace: RECORD_NS.into(),
                value,
                incarnation: 1,
            });
        }
        Ok(Some(PreparedNames { source, records }))
    }
    pub(in crate::app) fn commit_comment_names(
        &mut self,
        prepared: Option<PreparedNames>,
        created: &[AnnotationId],
    ) {
        if let Some(prepared) = prepared {
            let names = self.tabs[self.active]
                .comment_names
                .as_mut()
                .expect("prepared names");
            if let Some(source) = prepared.source {
                names.sources.push(source);
            }
            assert_eq!(
                prepared.records.len(),
                created.len(),
                "original native creation identities"
            );
            names.records.extend(
                created
                    .iter()
                    .cloned()
                    .zip(prepared.records)
                    .map(|(native, identity)| RecordAssociation { native, identity }),
            );
        }
    }
    /// Explicit ordinary save to the previously associated resource, using the
    /// host's retained exact witnesses and original native record associations.
    pub fn save_comment_file(&mut self) -> Result<(), EditorError> {
        let limits = self
            .comment_file_limits
            .ok_or(EditorError::CommentsUnavailable)?;
        self.live()?;
        if self.tabs[self.active].comment_file_requires_load {
            return Err(EditorError::CommentsIncomplete);
        }
        if self
            .host
            .annotation_baseline_evidence(&self.tabs[self.active].document)?
            .is_some()
            && !self.comments_dirty()?
        {
            self.status =
                "Comments unchanged — no write; existing Loaded/Written facts remain distinct"
                    .into();
            return Ok(());
        }
        let source = self.version()?;
        let prepared = self.prepare_comment_names(&source, 0)?;
        self.commit_comment_names(prepared, &[]);
        let snapshot = self.capture_comments()?;
        let names = self.tabs[self.active]
            .comment_names
            .as_ref()
            .expect("host names");
        let mut records = Vec::new();
        records
            .try_reserve_exact(snapshot.records().len())
            .map_err(|_| EditorError::Capacity)?;
        for row in snapshot.records() {
            records.push(
                names
                    .records
                    .iter()
                    .find(|r| r.native == row.annotation)
                    .ok_or(EditorError::IdentityUnavailable)?
                    .clone(),
            );
        }
        let mut sources = Vec::new();
        for version in std::iter::once(snapshot.source())
            .chain(snapshot.records().iter().map(|r| &r.authored.source))
        {
            if sources
                .iter()
                .any(|s: &SourceAssociation| s.native == *version)
            {
                continue;
            }
            let original = names
                .sources
                .iter()
                .find(|s| s.native == *version)
                .ok_or(EditorError::IdentityUnavailable)?;
            // Version labels remain distinct even if Undo returns to identical
            // bytes; the digest is content evidence, not the version identity.
            let mut associated = SourceAssociation {
                native: original.native.clone(),
                evidence: original.evidence.clone(),
                text: String::new(),
            };
            associated
                .text
                .try_reserve_exact(original.text.len())
                .map_err(|_| EditorError::Capacity)?;
            associated.text.push_str(&original.text);
            sources.try_reserve(1).map_err(|_| EditorError::Capacity)?;
            sources.push(associated);
        }
        let associations = CaptureAssociations {
            document: DocumentAssociation {
                native: snapshot.source().document.clone(),
                identity: names.document.clone(),
            },
            records,
            sources,
        };
        self.save_comments(&snapshot, &associations, limits.codec)
    }
}
fn is_uuid(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 36
        && b[14] == b'4'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
        && b.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_digit() || matches!(c, b'a'..=b'f')
            }
        })
}
