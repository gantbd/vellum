//! Owned provider-attributed annotation read values. No parser or native load.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadStrength {
    CapturedRead,
    AtomicResourceSnapshot,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadPhase {
    Ready,
    Submitted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadFailure {
    Unavailable,
    Interrupted,
    Unsupported,
    Limit,
}
#[derive(Clone)]
pub struct AnnotationReadId {
    pub(crate) owner: Arc<()>,
    pub(crate) slot: usize,
}
impl PartialEq for AnnotationReadId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for AnnotationReadId {}
impl std::fmt::Debug for AnnotationReadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationReadId(<opaque>)")
    }
}
#[derive(Debug)]
pub struct AnnotationReadTarget {
    pub(crate) catalog: Option<Arc<crate::coordinator::ReadCatalogCaptureV1>>,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
}
#[derive(Debug)]
pub struct AnnotationReadDispatch {
    pub(crate) id: AnnotationReadId,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) nonce: Arc<()>,
    pub(crate) strength: ReadStrength,
}
impl AnnotationReadDispatch {
    pub fn resource_key(&self) -> &[u8] {
        &self.identity.key.0
    }
    pub fn strength(&self) -> ReadStrength {
        self.strength
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReadAtom {
    Start {
        version: Blob,
    },
    Chunk {
        sequence: usize,
        offset: usize,
        bytes: Blob,
    },
    End {
        version: Blob,
        chunks: usize,
        bytes: usize,
    },
    Failure(ReadFailure),
    Quiescence,
}
#[derive(Clone, Debug)]
pub struct AnnotationReadEvidence {
    pub(crate) id: AnnotationReadId,
    pub(crate) registration: Arc<Registration>,
    pub(crate) nonce: Arc<()>,
    pub(crate) atom: ReadAtom,
}
pub(crate) struct ReadCaptureData {
    pub(crate) id: AnnotationReadId,
    pub(crate) source: DocumentVersion,
    pub(crate) set: u64,
    pub(crate) bytes: Blob,
    pub(crate) version: Blob,
    pub(crate) strength: ReadStrength,
}
/// Already-disclosed immutable original input. A copy is not a native load grant;
/// owning consumers must revalidate the actual live task before activation.
#[derive(Clone)]
pub struct CapturedAnnotationRead {
    pub(crate) data: Arc<ReadCaptureData>,
}
impl std::fmt::Debug for CapturedAnnotationRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CapturedAnnotationRead(<redacted>)")
    }
}
impl CapturedAnnotationRead {
    pub fn bytes(&self) -> &[u8] {
        &self.data.bytes.0
    }
    pub fn source(&self) -> &DocumentVersion {
        &self.data.source
    }
    pub fn set_version(&self) -> u64 {
        self.data.set
    }
    pub fn origin(&self) -> &AnnotationReadId {
        &self.data.id
    }
    pub fn version(&self) -> &[u8] {
        &self.data.version.0
    }
    pub fn strength(&self) -> ReadStrength {
        self.data.strength
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationReadStatus {
    pub restored: bool,
    pub phase: ReadPhase,
    pub stopped: bool,
    pub quiescent: bool,
    pub contested: bool,
    pub refused: bool,
    pub complete: bool,
    pub failure: Option<ReadFailure>,
    pub received_chunks: usize,
    pub received_bytes: usize,
    pub witness_count: usize,
}
impl ProviderPort {
    pub fn annotation_read_target(
        &self,
        key: &[u8],
    ) -> Result<AnnotationReadTarget, ResourceError> {
        Ok(AnnotationReadTarget {
            catalog: None,
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.identity_domain(),
                key: blob(key, self.registration.limits.max_key_bytes, true)?,
            },
        })
    }
    fn read_attest(
        &self,
        dispatch: &AnnotationReadDispatch,
        atom: ReadAtom,
    ) -> Result<AnnotationReadEvidence, ResourceError> {
        if !Arc::ptr_eq(&self.registration, &dispatch.registration) {
            return Err(ResourceError::Unavailable);
        }
        Ok(AnnotationReadEvidence {
            id: dispatch.id.clone(),
            registration: self.registration.clone(),
            nonce: dispatch.nonce.clone(),
            atom,
        })
    }
    pub fn annotation_read_start(
        &self,
        dispatch: &AnnotationReadDispatch,
        version: &[u8],
    ) -> Result<AnnotationReadEvidence, ResourceError> {
        self.read_attest(
            dispatch,
            ReadAtom::Start {
                version: blob(version, self.registration.limits.max_validator_bytes, true)?,
            },
        )
    }
    pub fn annotation_read_chunk(
        &self,
        dispatch: &AnnotationReadDispatch,
        sequence: usize,
        offset: usize,
        bytes: &[u8],
    ) -> Result<AnnotationReadEvidence, ResourceError> {
        if sequence >= self.registration.limits.max_read_chunks
            || offset > self.registration.limits.max_blob_bytes
        {
            return Err(ResourceError::Limit);
        }
        self.read_attest(
            dispatch,
            ReadAtom::Chunk {
                sequence,
                offset,
                bytes: blob(bytes, self.registration.limits.max_blob_bytes, false)?,
            },
        )
    }
    pub fn annotation_read_end(
        &self,
        dispatch: &AnnotationReadDispatch,
        version: &[u8],
        chunks: usize,
        bytes: usize,
    ) -> Result<AnnotationReadEvidence, ResourceError> {
        if chunks > self.registration.limits.max_read_chunks
            || bytes > self.registration.limits.max_blob_bytes
        {
            return Err(ResourceError::Limit);
        }
        self.read_attest(
            dispatch,
            ReadAtom::End {
                version: blob(version, self.registration.limits.max_validator_bytes, true)?,
                chunks,
                bytes,
            },
        )
    }
    pub fn annotation_read_failure(
        &self,
        dispatch: &AnnotationReadDispatch,
        failure: ReadFailure,
    ) -> Result<AnnotationReadEvidence, ResourceError> {
        self.read_attest(dispatch, ReadAtom::Failure(failure))
    }
    pub fn annotation_read_quiescence(
        &self,
        dispatch: &AnnotationReadDispatch,
    ) -> Result<AnnotationReadEvidence, ResourceError> {
        self.read_attest(dispatch, ReadAtom::Quiescence)
    }
}
