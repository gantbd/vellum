//! Original catalog dependency for the existing shared writer, not a locator resolver.
use super::*;
mod exports;
#[cfg(test)]
mod tests;
pub struct TrustedCatalogWriterBindingV1 {
    pub profile: ResourceCatalogLimitsV1,
    pub expected_generation: u64,
    pub max_binding_bytes: usize,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub write_allowed: Option<bool>,
    pub provider_available: Option<bool>,
    pub correspondence_current: Option<bool>,
    pub resources_ready: Option<bool>,
}
/// Trusted host-established correspondence, validated and sealed by the engine.
/// Does not rebind a document, authorize an effect or resolve catalog names.
#[derive(Debug)]
pub struct CatalogWriterBindingV1 {
    capture: Arc<WriteCatalogCaptureV1>,
}
pub struct OwnedCatalogSourceSaveRequestV1<'a> {
    pub native: &'a OwnedSourceSaveRequestV1<'a>,
    pub writer: &'a CatalogWriterBindingV1,
}
pub struct OwnedCatalogAnnotationSaveRequestV1<'a> {
    pub native: &'a OwnedAnnotationSaveRequestV1<'a>,
    pub writer: &'a CatalogWriterBindingV1,
}
impl CatalogWriterBindingV1 {
    pub(in crate::coordinator) fn retained_capture(&self) -> Arc<WriteCatalogCaptureV1> {
        self.capture.clone()
    }
}
pub(crate) struct WriteCatalogCaptureV1 {
    owner: Arc<()>,
    registration: Arc<Registration>,
    identity: Identity,
    profile: ResourceCatalogLimitsV1,
    row: ResourceCatalogRowV1,
}
impl std::fmt::Debug for WriteCatalogCaptureV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CatalogWriterBinding(<opaque>)")
    }
}
impl WriteCatalogCaptureV1 {
    pub(in crate::coordinator) fn export_condition_allowed(
        &self,
        absent: bool,
        protection: Protection,
        after: SaveResultVersionProfileV1,
    ) -> Result<()> {
        let ResourceCatalogCapabilitiesV1::Write {
            exclusive_create,
            conditional_write,
            causal_receipts,
            ..
        } = self.row.configuration.capabilities
        else {
            return Err(ResourceError::Faulted);
        };
        if (absent && !exclusive_create)
            || (!absent && protection == Protection::AtomicConditional && !conditional_write)
            || (after == SaveResultVersionProfileV1::EpochToken && !causal_receipts)
        {
            return Err(ResourceError::Unsupported);
        }
        Ok(())
    }
    pub(in crate::coordinator) fn allows_own_chain(&self) -> bool {
        matches!(
            self.row.configuration.capabilities,
            ResourceCatalogCapabilitiesV1::Write {
                logical_continuity: true,
                causal_receipts: true,
                non_reuse_tokens: true,
                ..
            }
        )
    }
    pub(in crate::coordinator) fn same_intent(
        a: Option<&Arc<Self>>,
        b: Option<&Arc<Self>>,
    ) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                Arc::ptr_eq(&a.owner, &b.owner)
                    && Arc::ptr_eq(&a.registration, &b.registration)
                    && a.identity == b.identity
                    && a.profile == b.profile
                    && a.row == b.row
            }
            _ => false,
        }
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
        if !Arc::ptr_eq(owner, &self.owner)
            || !Arc::ptr_eq(owner, &registration.owner)
            || !Arc::ptr_eq(&self.registration, registration)
            || self.identity != *identity
            || identity.domain != registration.domain
            || !self.row.address.valid(self.profile)
            || self.row.address.table != ResourceCatalogTableV1::Write
            || self.row.generation > MAX_STAMP
            || !self
                .row
                .configuration
                .valid(ResourceCatalogTableV1::Write, self.profile)
        {
            return Err(ResourceError::Faulted);
        }
        self.native_bytes()
    }
    pub(in crate::coordinator) fn same(a: Option<&Arc<Self>>, b: Option<&Arc<Self>>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl SourceCoordinator {
    pub fn request_owned_catalog_source_save_v1(
        &mut self,
        request: &OwnedCatalogSourceSaveRequestV1<'_>,
        release: &TrustedSaveOperationReleaseV1<'_>,
        admission: Option<&TrustedOrdinarySourceSaveAdmissionV1<'_>>,
        pair: Option<&OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1> {
        self.request_owned_source_save_catalog(
            (request.native, Some(request.writer)),
            release,
            admission,
            pair,
            r,
        )
    }
    pub fn request_owned_catalog_annotation_save_v1(
        &mut self,
        request: &OwnedCatalogAnnotationSaveRequestV1<'_>,
        release: &TrustedAnnotationSaveOperationReleaseV1<'_>,
        admission: Option<&TrustedOrdinaryAnnotationSaveAdmissionV1>,
        candidate: Option<EncodedAnnotations>,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1> {
        self.request_owned_annotation_save_catalog(
            (request.native, Some(request.writer)),
            release,
            admission,
            candidate,
            pair,
            r,
        )
    }
    pub fn bind_catalog_writer_v1(
        &mut self,
        target: SourceBindingTargetV1,
        address: &ResourceCatalogAddressV1,
        f: &TrustedCatalogWriterBindingV1,
        r: &mut Recorder,
    ) -> Result<CatalogWriterBindingV1> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if [
                f.authorized,
                f.scope,
                f.continuous,
                f.live,
                f.identity_allowed,
                f.write_allowed,
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
            if address.table != ResourceCatalogTableV1::Write {
                return Err(ResourceError::Unsupported);
            }
            let bytes = add(
                add(size_of::<WriteCatalogCaptureV1>(), row.payload()?)?,
                target.identity.key.0.len(),
            )?;
            if bytes > f.max_binding_bytes || bytes > profile.max_plan_bytes {
                return Err(ResourceError::Limit);
            }
            Ok(CatalogWriterBindingV1 {
                capture: Arc::new(WriteCatalogCaptureV1 {
                    owner: self.namespace.clone(),
                    registration: target.registration,
                    identity: target.identity,
                    profile,
                    row: row.copy()?,
                }),
            })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        observe(r, Boundary::CatalogWriterBinding, &result, timer.finish());
        result
    }
    pub(in crate::coordinator) fn writer_catalog_current(
        &self,
        capture: Option<&Arc<WriteCatalogCaptureV1>>,
        registration: &Arc<Registration>,
        identity: &Identity,
        synchronized: bool,
        own_chain: bool,
        r: &mut Recorder,
    ) -> Result<()> {
        let Some(c) = capture else {
            return Ok(());
        };
        let timer = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&c.owner, &self.namespace)
                || !Arc::ptr_eq(&c.registration, registration)
                || c.identity != *identity
            {
                return Err(ResourceError::Unavailable);
            }
            c.intact(&self.namespace, registration, identity)?;
            let (root, profile) = self.resources.catalog.current()?;
            if profile != c.profile || !root.rows.iter().any(|row| row == &c.row) {
                return Err(ResourceError::Stale);
            }
            if !c.row.configuration.identity_known {
                return Err(ResourceError::Unavailable);
            }
            if own_chain && !c.allows_own_chain() {
                return Err(ResourceError::Unsupported);
            }
            if synchronized
                && !matches!(
                    c.row.configuration.capabilities,
                    ResourceCatalogCapabilitiesV1::Write {
                        synchronization_supported: true,
                        ..
                    }
                )
            {
                return Err(ResourceError::Unsupported);
            }
            Ok(())
        })();
        observe(
            r,
            Boundary::CatalogWriterCurrentness,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn queue_catalog_pinned_source_save_v1(
        &mut self,
        expected: &DocumentVersion,
        binding: &CatalogWriterBindingV1,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_pinned_save_checked(
            expected,
            requirements,
            pair.as_ref(),
            false,
            Some(binding.capture.clone()),
            r,
        )
    }
    pub fn queue_catalog_own_chain_source_save_v1(
        &mut self,
        expected: &DocumentVersion,
        binding: &CatalogWriterBindingV1,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_pinned_save_checked(
            expected,
            requirements,
            pair.as_ref(),
            true,
            Some(binding.capture.clone()),
            r,
        )
    }
    pub fn queue_catalog_annotation_save_v1(
        &mut self,
        encoded: EncodedAnnotations,
        destination: ResourceRead,
        binding: &CatalogWriterBindingV1,
        requirements: SaveRequirements,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_annotation_save_catalog(
            encoded,
            destination,
            requirements,
            pair,
            Some(binding.capture.clone()),
            r,
        )
    }
}
