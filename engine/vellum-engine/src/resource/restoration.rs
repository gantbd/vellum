//! Privileged typed restoration boundary. JSON names/counters remain data until
//! independently associated by this configured producer and checked natively.
use super::*;
use crate::attachment::Placement;
use crate::coordinator::{AnnotationFact, AnnotationLimits, SourceTarget};
#[derive(Clone)]
pub struct RestorationContext {
    pub(crate) owner: Arc<()>,
    marker: Arc<()>,
}
impl PartialEq for RestorationContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for RestorationContext {}
impl std::fmt::Debug for RestorationContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RestorationContext(<opaque>)")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct RestorationIdentity {
    context: RestorationContext,
    pub(crate) key: Blob,
    incarnation: u64,
}
impl RestorationIdentity {
    pub fn key(&self) -> &[u8] {
        &self.key.0
    }
    pub fn incarnation(&self) -> u64 {
        self.incarnation
    }
}
impl std::fmt::Debug for RestorationIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RestorationIdentity(<opaque>)")
    }
}
pub struct RestoredAnnotation {
    pub identity: RestorationIdentity,
    pub body: String,
    pub content_version: u64,
    pub target_generation: u64,
    pub authored: SourceTarget,
    pub placement: Placement,
}
impl std::fmt::Debug for RestoredAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RestoredAnnotation(<redacted>)")
    }
}
#[derive(Debug)]
pub(crate) struct RestorationRegistration {
    pub(crate) encoding: Option<Arc<EncoderRegistration>>,
    pub(crate) context: RestorationContext,
    pub(crate) limits: ResourceLimits,
    pub(crate) annotations: AnnotationLimits,
}
#[derive(Clone, Debug)]
pub struct AnnotationRestorerPort {
    pub(crate) registration: Arc<RestorationRegistration>,
}
pub(crate) struct SealedRestoredAnnotation {
    pub(crate) identity: RestorationIdentity,
    pub(crate) body: Arc<str>,
    pub(crate) content_version: u64,
    pub(crate) target_generation: u64,
    pub(crate) authored: SourceTarget,
    pub(crate) placement: Placement,
}
pub struct AnnotationRestoration {
    pub(crate) registration: Arc<RestorationRegistration>,
    pub(crate) capture: CapturedAnnotationRead,
    pub(crate) sources: Vec<AnnotationSnapshot>,
    pub(crate) records: Vec<SealedRestoredAnnotation>,
    pub(crate) charge: usize,
}
impl std::fmt::Debug for AnnotationRestoration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationRestoration(<redacted>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationRestorationOutcome {
    pub source: DocumentVersion,
    pub set_version: u64,
    pub changed: bool,
    pub context: RestorationContext,
    pub annotations: Vec<AnnotationFact>,
    pub evidence_contested: bool,
    pub loaded_baseline_adopted: bool,
}
#[derive(Debug)]
pub(crate) struct RestorationProvenance {
    pub(crate) package: AnnotationRestoration,
    pub(crate) outcome: AnnotationRestorationOutcome,
}
impl AnnotationRestoration {
    pub(crate) fn copy_for_invocation(&self) -> Result<Self, ResourceError> {
        if self.logical_size() != Some(self.charge) {
            return Err(ResourceError::Faulted);
        }
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(self.sources.len())
            .map_err(|_| ResourceError::Capacity)?;
        sources.extend(self.sources.iter().cloned());
        let mut records = Vec::new();
        records
            .try_reserve_exact(self.records.len())
            .map_err(|_| ResourceError::Capacity)?;
        records.extend(self.records.iter().map(|r| SealedRestoredAnnotation {
            identity: r.identity.clone(),
            body: r.body.clone(),
            content_version: r.content_version,
            target_generation: r.target_generation,
            authored: r.authored.clone(),
            placement: r.placement,
        }));
        Ok(Self {
            registration: self.registration.clone(),
            capture: self.capture.clone(),
            sources,
            records,
            charge: self.charge,
        })
    }
    pub(crate) fn logical_size(&self) -> Option<usize> {
        let mut n = std::mem::size_of::<RestorationProvenance>().checked_add(
            self.records.len().checked_mul(
                std::mem::size_of::<SealedRestoredAnnotation>()
                    .checked_add(std::mem::size_of::<AnnotationFact>())?,
            )?,
        )?;
        for row in &self.records {
            n = n
                .checked_add(row.body.len())?
                .checked_add(row.identity.key.0.len())?;
        }
        for source in &self.sources {
            n = n
                .checked_add(std::mem::size_of::<AnnotationSnapshot>())?
                .checked_add(source.logical_bytes()?)?;
        }
        Some(n)
    }
}
impl AnnotationRestorerPort {
    pub fn context(&self) -> RestorationContext {
        self.registration.context.clone()
    }
    /// Host-approved persistent association data, not a live annotation handle.
    pub fn identity(
        &self,
        key: &[u8],
        incarnation: u64,
    ) -> Result<RestorationIdentity, ResourceError> {
        if incarnation == 0 {
            return Err(ResourceError::Malformed);
        }
        Ok(RestorationIdentity {
            context: self.context(),
            key: blob(key, self.registration.limits.max_key_bytes, true)?,
            incarnation,
        })
    }
    /// The first boundary profile is reference annotation JSON v1. It attests
    /// strict decoding, complete injective alias
    /// association and historical/source applicability. The engine still verifies
    /// actual read provenance, managed source witnesses and native state itself.
    pub fn seal(
        &self,
        capture: CapturedAnnotationRead,
        sources: Vec<AnnotationSnapshot>,
        records: Vec<RestoredAnnotation>,
    ) -> Result<AnnotationRestoration, ResourceError> {
        if !Arc::ptr_eq(&capture.data.id.owner, &self.registration.context.owner)
            || sources
                .iter()
                .any(|s| !s.belongs_to(&self.registration.context.owner))
        {
            return Err(ResourceError::Unavailable);
        }
        if records.len() > self.registration.annotations.max_records
            || sources.len() > self.registration.annotations.max_live_snapshots
        {
            return Err(ResourceError::Limit);
        }
        let mut sealed = Vec::new();
        sealed
            .try_reserve_exact(records.len())
            .map_err(|_| ResourceError::Capacity)?;
        for row in records {
            if row.identity.context != self.registration.context {
                return Err(ResourceError::Unavailable);
            }
            if row.content_version == 0 || row.target_generation == 0 {
                return Err(ResourceError::Malformed);
            }
            if row.body.len() > self.registration.annotations.max_body_bytes {
                return Err(ResourceError::Limit);
            }
            if sealed
                .iter()
                .any(|r: &SealedRestoredAnnotation| r.identity.key == row.identity.key)
            {
                return Err(ResourceError::Malformed);
            }
            sealed.push(SealedRestoredAnnotation {
                identity: row.identity,
                body: Arc::from(row.body),
                content_version: row.content_version,
                target_generation: row.target_generation,
                authored: row.authored,
                placement: row.placement,
            });
        }
        let mut package = AnnotationRestoration {
            registration: self.registration.clone(),
            capture,
            sources,
            records: sealed,
            charge: 0,
        };
        package.charge = package.logical_size().ok_or(ResourceError::Limit)?;
        if package.charge > self.registration.limits.max_restoration_bytes {
            return Err(ResourceError::Limit);
        }
        Ok(package)
    }
}
impl RestorationRegistration {
    pub(crate) fn new(
        owner: Arc<()>,
        limits: ResourceLimits,
        annotations: AnnotationLimits,
        encoding: Option<Arc<EncoderRegistration>>,
    ) -> Self {
        Self {
            encoding,
            context: RestorationContext {
                owner,
                marker: Arc::new(()),
            },
            limits,
            annotations,
        }
    }
}
