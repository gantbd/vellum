//! Explicit bounded source version/receipt profile; no parsing opaque validators.
use super::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProviderCausalCapabilitiesV1 {
    pub causal_receipts: bool,
    pub non_reuse_tokens: bool,
}
/// Explicit host version assertion. Both nonempty fields are independently
/// bounded by max_validator_bytes at original-issuer factories. An epoch is not
/// derived from an opaque validator or from observing a few different tokens.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ResourceVersionV1<'a> {
    pub epoch: &'a [u8],
    pub token: &'a [u8],
}
impl std::fmt::Debug for ResourceVersionV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceVersionV1(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CausalVersion {
    pub(crate) epoch: Blob,
    pub(crate) token: Blob,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CausalEdge {
    pub(crate) before: CausalVersion,
    pub(crate) after: CausalVersion,
}
#[derive(Debug)]
pub struct SourceContinuityEdgeV1 {
    pub(crate) producer: SaveId,
    pub(crate) serial: u64,
    pub(crate) versions: CausalEdge,
}
impl SourceContinuityEdgeV1 {
    pub fn producer(&self) -> &SaveId {
        &self.producer
    }
    pub fn dispatch_serial(&self) -> u64 {
        self.serial
    }
    pub fn before(&self) -> ResourceVersionV1<'_> {
        view(&self.versions.before)
    }
    pub fn after(&self) -> ResourceVersionV1<'_> {
        view(&self.versions.after)
    }
}
#[derive(Debug)]
pub struct SourceContinuityReceiptV1 {
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) source: crate::coordinator::DocumentVersion,
    pub(crate) serialization: crate::coordinator::SourceSerializationStateV1,
    pub(crate) binding_generation: u64,
    pub(crate) anchor_serial: u64,
    pub(crate) serial: u64,
    pub(crate) original: CausalVersion,
    pub(crate) derived: CausalVersion,
    pub(crate) edges: Vec<SourceContinuityEdgeV1>,
}
impl SourceContinuityReceiptV1 {
    pub fn source(&self) -> &crate::coordinator::DocumentVersion {
        &self.source
    }
    pub fn serialization(&self) -> crate::coordinator::SourceSerializationStateV1 {
        self.serialization
    }
    pub fn binding_generation(&self) -> u64 {
        self.binding_generation
    }
    pub fn anchor_serial(&self) -> u64 {
        self.anchor_serial
    }
    pub fn dispatch_serial(&self) -> u64 {
        self.serial
    }
    pub fn original(&self) -> ResourceVersionV1<'_> {
        view(&self.original)
    }
    pub fn derived(&self) -> ResourceVersionV1<'_> {
        view(&self.derived)
    }
    pub fn edges(&self) -> &[SourceContinuityEdgeV1] {
        &self.edges
    }
}
fn view(v: &CausalVersion) -> ResourceVersionV1<'_> {
    ResourceVersionV1 {
        epoch: &v.epoch.0,
        token: &v.token.0,
    }
}
fn owned(v: ResourceVersionV1<'_>, max: usize) -> Result<CausalVersion, ResourceError> {
    Ok(CausalVersion {
        epoch: blob(v.epoch, max, true)?,
        token: blob(v.token, max, true)?,
    })
}
impl ProviderPort {
    pub fn read_result_causal(
        &self,
        key: &[u8],
        version: ResourceVersionV1<'_>,
        bytes: &[u8],
    ) -> Result<ResourceRead, ResourceError> {
        if !self.registration.causal.causal_receipts {
            return Err(ResourceError::Unsupported);
        }
        let v = owned(version, self.registration.limits.max_validator_bytes)?;
        let mut read = self.read_result(key, &v.token.0, bytes)?;
        read.epoch = Some(v.epoch);
        Ok(read)
    }
    pub fn source_save_as_probe_causal(
        &self,
        key: &[u8],
        version: Option<ResourceVersionV1<'_>>,
    ) -> Result<SourceSaveAsProbeV1, ResourceError> {
        if !self.registration.causal.causal_receipts {
            return Err(ResourceError::Unsupported);
        }
        let version = version
            .map(|v| owned(v, self.registration.limits.max_validator_bytes))
            .transpose()?;
        Ok(SourceSaveAsProbeV1 {
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.registration.domain.clone(),
                key: blob(key, self.registration.limits.max_key_bytes, true)?,
            },
            epoch: version.as_ref().map(|v| v.epoch.clone()),
            version: version.map(|v| v.token),
        })
    }
    fn causal_dispatch(&self, dispatch: &SaveDispatch) -> Result<(), ResourceError> {
        if !Arc::ptr_eq(&self.registration, &dispatch.registration) {
            return Err(ResourceError::Unavailable);
        }
        if !self.registration.causal.causal_receipts
            || dispatch.result_version_profile != SaveResultVersionProfileV1::EpochToken
        {
            return Err(ResourceError::Unsupported);
        }
        Ok(())
    }
    pub fn result_causal(
        &self,
        dispatch: &SaveDispatch,
        application: Application,
        observed: usize,
        synchronization: Synchronization,
        version: Option<ResourceVersionV1<'_>>,
    ) -> Result<SaveEvidence, ResourceError> {
        self.causal_dispatch(dispatch)?;
        if observed > dispatch.payload.0.len()
            || (application == Application::Written && observed != dispatch.payload.0.len())
            || (application == Application::NotWritten && observed != 0)
        {
            return Err(ResourceError::Malformed);
        }
        let version = version
            .map(|v| owned(v, self.registration.limits.max_validator_bytes))
            .transpose()?;
        self.attest(
            dispatch,
            EvidenceKind::Result(ResultData {
                application,
                observed,
                synchronization,
                epoch: version.as_ref().map(|v| v.epoch.clone()),
                version: version.map(|v| v.token),
            }),
        )
    }
    pub fn causal_edge(
        &self,
        dispatch: &SaveDispatch,
        before: ResourceVersionV1<'_>,
        after: ResourceVersionV1<'_>,
    ) -> Result<SaveEvidence, ResourceError> {
        // Attest the actual write edge, including an incompatible before/after
        // fact. Reconciliation belongs to protected engine intake, not omission
        // of inconvenient observations by the issuer factory.
        self.causal_dispatch(dispatch)?;
        let max = self.registration.limits.max_validator_bytes;
        self.attest(
            dispatch,
            EvidenceKind::CausalEdge(CausalEdge {
                before: owned(before, max)?,
                after: owned(after, max)?,
            }),
        )
    }
}
