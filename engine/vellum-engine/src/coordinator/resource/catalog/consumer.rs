//! Host establishes native correspondence. Engine issues immutable captures and
//! enforces currentness; names/locators cannot manufacture native capabilities.
use super::*;
mod annotations;
mod writer;
pub use annotations::TrustedCatalogAnnotationReadBindingV1;
pub(crate) use writer::WriteCatalogCaptureV1;
pub use writer::{
    CatalogWriterBindingV1, OwnedCatalogAnnotationSaveRequestV1, OwnedCatalogSourceSaveRequestV1,
    TrustedCatalogWriterBindingV1,
};
#[cfg(test)]
mod tests;
pub struct TrustedCatalogSourceReadBindingV1 {
    pub profile: ResourceCatalogLimitsV1,
    /// Original expected current row. Zero is a valid present generation.
    pub expected_generation: u64,
    /// Per-call native logical capture, not global caller-held sponsorship.
    pub max_binding_bytes: usize,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub read_allowed: Option<bool>,
    pub provider_available: Option<bool>,
    /// Trusted host assertion for this exact row/port/opaque identity pairing.
    /// The engine does not derive it from a path, key, label or capability bit.
    pub correspondence_current: Option<bool>,
    pub resources_ready: Option<bool>,
}
pub(crate) struct ReadCatalogCaptureV1 {
    kind: ReadKind,
    owner: Arc<()>,
    registration: Arc<Registration>,
    identity: Identity,
    profile: ResourceCatalogLimitsV1,
    row: ResourceCatalogRowV1,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadKind {
    Source,
    Annotations,
}
impl ReadKind {
    fn accepts(self, c: &ResourceCatalogCapabilitiesV1) -> bool {
        matches!(
            (self, c),
            (
                Self::Source,
                ResourceCatalogCapabilitiesV1::StrictUtf8OneBom { .. }
            ) | (
                Self::Annotations,
                ResourceCatalogCapabilitiesV1::AnnotationsV1 { .. }
            )
        )
    }
}
impl std::fmt::Debug for ReadCatalogCaptureV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadCatalogBinding(<opaque>)")
    }
}
impl ReadCatalogCaptureV1 {
    pub(in crate::coordinator) fn source_insertion_policy(&self) -> Result<GeneratedNewlineV1> {
        if self.kind != ReadKind::Source {
            return Err(ResourceError::Faulted);
        }
        match self.row.configuration.capabilities {
            ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
                insertion_line_break,
                ..
            } => Ok(match insertion_line_break {
                CatalogInsertionLineBreakV1::Lf => GeneratedNewlineV1::Lf,
                CatalogInsertionLineBreakV1::CrLf => GeneratedNewlineV1::CrLf,
                CatalogInsertionLineBreakV1::Cr => GeneratedNewlineV1::Cr,
            }),
            _ => Err(ResourceError::Faulted),
        }
    }
    pub(in crate::coordinator) fn source_intact(
        &self,
        owner: &Arc<()>,
        registration: &Arc<Registration>,
        identity: &Identity,
    ) -> Result<usize> {
        if self.kind != ReadKind::Source {
            return Err(ResourceError::Faulted);
        }
        self.intact(owner, registration, identity)
    }
    pub(in crate::coordinator) fn annotation_intact(
        &self,
        owner: &Arc<()>,
        registration: &Arc<Registration>,
        identity: &Identity,
    ) -> Result<usize> {
        if self.kind != ReadKind::Annotations {
            return Err(ResourceError::Faulted);
        }
        self.intact(owner, registration, identity)
    }
    pub(in crate::coordinator) fn native_bytes(&self) -> Result<usize> {
        add(
            add(size_of::<Self>(), self.row.payload()?)?,
            self.identity.key.0.len(),
        )
    }
    pub(in crate::coordinator) fn intact(
        &self,
        owner: &Arc<()>,
        registration: &Arc<Registration>,
        identity: &Identity,
    ) -> Result<usize> {
        if !Arc::ptr_eq(&self.owner, owner)
            || !Arc::ptr_eq(&self.registration, registration)
            || !Arc::ptr_eq(&self.registration.owner, owner)
            || self.identity != *identity
            || self.identity.domain != self.registration.domain
            || !self.row.address.valid(self.profile)
            || self.row.address.table != ResourceCatalogTableV1::Read
            || self.row.generation > MAX_STAMP
            || !self
                .row
                .configuration
                .valid(ResourceCatalogTableV1::Read, self.profile)
            || !self.kind.accepts(&self.row.configuration.capabilities)
        {
            return Err(ResourceError::Faulted);
        }
        self.native_bytes()
    }
    pub(in crate::coordinator) fn same(a: Option<&Arc<Self>>, b: Option<&Arc<Self>>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                Arc::ptr_eq(&a.owner, &b.owner)
                    && Arc::ptr_eq(&a.registration, &b.registration)
                    && a.identity == b.identity
                    && a.profile == b.profile
                    && a.row == b.row
                    && a.kind == b.kind
            }
            _ => false,
        }
    }
}
impl SourceCoordinator {
    /// Trusted host bridge only. This adds a dependency to an already-issued
    /// native target; it resolves no locator and grants no stronger read ability.
    /// The complete captured row is immutable and is never rebound in place.
    pub fn bind_catalog_source_read_target_v1(
        &mut self,
        target: SourceReadTargetV1,
        address: &ResourceCatalogAddressV1,
        f: &TrustedCatalogSourceReadBindingV1,
        r: &mut Recorder,
    ) -> Result<SourceReadTargetV1> {
        let timer = r.begin_service_timing();
        let result = self.bind_catalog_read_target(target, address, f, ReadKind::Source);
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        observe(
            r,
            Boundary::CatalogSourceReadBinding,
            &result,
            timer.finish(),
        );
        result
    }
    fn bind_catalog_read_target(
        &mut self,
        mut target: SourceReadTargetV1,
        address: &ResourceCatalogAddressV1,
        f: &TrustedCatalogSourceReadBindingV1,
        kind: ReadKind,
    ) -> Result<SourceReadTargetV1> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if [
                f.authorized,
                f.scope,
                f.continuous,
                f.live,
                f.identity_allowed,
                f.read_allowed,
                f.provider_available,
                f.correspondence_current,
                f.resources_ready,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            self.provider_current(&target.registration)?;
            if target.catalog.is_some() {
                return Err(ResourceError::Unavailable);
            }
            if target.identity.domain != target.registration.domain {
                return Err(ResourceError::Faulted);
            }
            let (root, profile) = self.resources.catalog.current()?;
            if profile != f.profile {
                return Err(ResourceError::Unavailable);
            }
            if !address.valid(profile) || f.expected_generation > MAX_STAMP {
                return Err(ResourceError::Malformed);
            }
            let row = root
                .rows
                .iter()
                .find(|row| row.address == *address)
                .ok_or(ResourceError::Stale)?;
            if row.generation != f.expected_generation {
                return Err(ResourceError::Stale);
            }
            if address.table != ResourceCatalogTableV1::Read
                || !kind.accepts(&row.configuration.capabilities)
            {
                return Err(ResourceError::Unsupported);
            }
            let bytes = add(
                add(size_of::<ReadCatalogCaptureV1>(), row.payload()?)?,
                target.identity.key.0.len(),
            )?;
            if bytes > f.max_binding_bytes || bytes > profile.max_plan_bytes {
                return Err(ResourceError::Limit);
            }
            target.catalog = Some(Arc::new(ReadCatalogCaptureV1 {
                kind,
                owner: self.namespace.clone(),
                registration: target.registration.clone(),
                identity: target.identity.clone(),
                profile,
                row: row.copy()?,
            }));
            Ok(target)
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        result
    }
    pub(in crate::coordinator) fn source_read_catalog_current(
        &self,
        capture: Option<&Arc<ReadCatalogCaptureV1>>,
        registration: &Arc<Registration>,
        identity: &Identity,
        activation: bool,
        r: &mut Recorder,
    ) -> Result<()> {
        if capture.is_none() {
            return Ok(());
        } // Existing privileged native-only profile.
        let timer = r.begin_service_timing();
        let result = self.read_catalog_current(
            capture,
            registration,
            identity,
            activation,
            ReadKind::Source,
        );
        observe(
            r,
            Boundary::CatalogSourceReadCurrentness,
            &result,
            timer.finish(),
        );
        result
    }
    fn read_catalog_current(
        &self,
        capture: Option<&Arc<ReadCatalogCaptureV1>>,
        registration: &Arc<Registration>,
        identity: &Identity,
        activation: bool,
        kind: ReadKind,
    ) -> Result<()> {
        let Some(capture) = capture else {
            return Ok(());
        };
        if capture.kind != kind {
            return Err(ResourceError::Faulted);
        }
        capture.intact(&self.namespace, registration, identity)?;
        let (root, profile) = self.resources.catalog.current()?;
        if profile != capture.profile || !root.rows.iter().any(|row| row == &capture.row) {
            return Err(ResourceError::Stale);
        }
        if activation && !capture.row.configuration.identity_known {
            return Err(ResourceError::Unavailable);
        }
        Ok(())
    }
}
fn observe<T>(
    r: &mut Recorder,
    boundary: Boundary,
    out: &Result<T>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match out {
            Ok(_) => Disposition::Completed,
            Err(ResourceError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
