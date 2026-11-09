//! Source-specific provider capabilities. No paths, I/O or implicit publication.
use super::*;
#[derive(Clone)]
pub struct SourceReadIdV1 {
    pub(crate) owner: Arc<()>,
    pub(crate) slot: usize,
}
impl PartialEq for SourceReadIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SourceReadIdV1 {}
impl std::fmt::Debug for SourceReadIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceReadIdV1(<opaque>)")
    }
}
#[derive(Debug)]
pub struct SourceReadTargetV1 {
    pub(crate) catalog: Option<Arc<crate::coordinator::ReadCatalogCaptureV1>>,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
}
#[derive(Debug)]
pub struct SourceReadDispatchV1 {
    pub(crate) id: SourceReadIdV1,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) nonce: Arc<()>,
    pub(crate) max_input_bytes: usize,
    pub(crate) max_chunks: usize,
}
impl SourceReadDispatchV1 {
    pub fn resource_key(&self) -> &[u8] {
        &self.identity.key.0
    }
    pub fn strength(&self) -> ReadStrength {
        ReadStrength::CapturedRead
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SourceReadVersionV1<'a> {
    Opaque(&'a [u8]),
    EpochToken(ResourceVersionV1<'a>),
}
impl std::fmt::Debug for SourceReadVersionV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceReadVersionV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReadFailureV1 {
    NotFound,
    Denied,
    Unsupported,
    IoFailure,
    Cancelled,
    Exhausted,
    InvalidSource,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceReadVersion {
    pub(crate) epoch: Option<Blob>,
    pub(crate) token: Blob,
}
impl SourceReadVersion {
    fn copy(value: SourceReadVersionV1<'_>, limit: usize) -> Result<Self, ResourceError> {
        let (epoch, token) = match value {
            SourceReadVersionV1::Opaque(v) => (None, v),
            SourceReadVersionV1::EpochToken(v) => (Some(blob(v.epoch, limit, true)?), v.token),
        };
        Ok(Self {
            epoch,
            token: blob(token, limit, true)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceReadAtom {
    Start(SourceReadVersion),
    Chunk {
        sequence: usize,
        offset: usize,
        bytes: Blob,
    },
    End {
        version: SourceReadVersion,
        chunks: usize,
        bytes: usize,
    },
    Failure(SourceReadFailureV1),
    Quiescence,
}
#[derive(Debug)]
pub struct SourceReadEvidenceV1 {
    pub(crate) id: SourceReadIdV1,
    pub(crate) registration: Arc<Registration>,
    pub(crate) nonce: Arc<()>,
    pub(crate) atom: SourceReadAtom,
}
impl ProviderPort {
    pub fn source_read_target_v1(&self, key: &[u8]) -> Result<SourceReadTargetV1, ResourceError> {
        Ok(SourceReadTargetV1 {
            catalog: None,
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.identity_domain(),
                key: blob(key, self.registration.limits.max_key_bytes, true)?,
            },
        })
    }
    fn source_read_attest(
        &self,
        dispatch: &SourceReadDispatchV1,
        atom: SourceReadAtom,
    ) -> Result<SourceReadEvidenceV1, ResourceError> {
        if !Arc::ptr_eq(&self.registration, &dispatch.registration) {
            return Err(ResourceError::Unavailable);
        }
        Ok(SourceReadEvidenceV1 {
            id: dispatch.id.clone(),
            registration: self.registration.clone(),
            nonce: dispatch.nonce.clone(),
            atom,
        })
    }
    pub fn source_read_start_v1(
        &self,
        dispatch: &SourceReadDispatchV1,
        version: SourceReadVersionV1<'_>,
    ) -> Result<SourceReadEvidenceV1, ResourceError> {
        self.source_read_attest(
            dispatch,
            SourceReadAtom::Start(SourceReadVersion::copy(
                version,
                self.registration.limits.max_validator_bytes,
            )?),
        )
    }
    pub fn source_read_chunk_v1(
        &self,
        dispatch: &SourceReadDispatchV1,
        sequence: usize,
        offset: usize,
        bytes: &[u8],
    ) -> Result<SourceReadEvidenceV1, ResourceError> {
        if sequence >= dispatch.max_chunks || offset > dispatch.max_input_bytes {
            return Err(ResourceError::Limit);
        }
        self.source_read_attest(
            dispatch,
            SourceReadAtom::Chunk {
                sequence,
                offset,
                bytes: blob(bytes, self.registration.limits.max_blob_bytes, false)?,
            },
        )
    }
    pub fn source_read_end_v1(
        &self,
        dispatch: &SourceReadDispatchV1,
        version: SourceReadVersionV1<'_>,
        chunks: usize,
        bytes: usize,
    ) -> Result<SourceReadEvidenceV1, ResourceError> {
        if chunks > dispatch.max_chunks || bytes > dispatch.max_input_bytes {
            return Err(ResourceError::Limit);
        }
        self.source_read_attest(
            dispatch,
            SourceReadAtom::End {
                version: SourceReadVersion::copy(
                    version,
                    self.registration.limits.max_validator_bytes,
                )?,
                chunks,
                bytes,
            },
        )
    }
    pub fn source_read_failure_v1(
        &self,
        dispatch: &SourceReadDispatchV1,
        reason: SourceReadFailureV1,
    ) -> Result<SourceReadEvidenceV1, ResourceError> {
        self.source_read_attest(dispatch, SourceReadAtom::Failure(reason))
    }
    pub fn source_read_quiescence_v1(
        &self,
        dispatch: &SourceReadDispatchV1,
    ) -> Result<SourceReadEvidenceV1, ResourceError> {
        self.source_read_attest(dispatch, SourceReadAtom::Quiescence)
    }
}
