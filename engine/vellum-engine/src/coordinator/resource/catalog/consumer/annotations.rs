//! Annotation-specific permission facts; shared immutable catalog capture.
use super::*;
#[cfg(test)]
mod tests;
pub struct TrustedCatalogAnnotationReadBindingV1 {
    pub profile: ResourceCatalogLimitsV1,
    pub expected_generation: u64,
    pub max_binding_bytes: usize,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub read_allowed: Option<bool>,
    /// Independent annotation grant; source-read permission cannot substitute.
    pub annotation_readable: Option<bool>,
    pub provider_available: Option<bool>,
    pub correspondence_current: Option<bool>,
    pub resources_ready: Option<bool>,
}
impl SourceCoordinator {
    /// Trusted host correspondence only; no locator resolution or inferred identity.
    pub fn bind_catalog_annotation_read_target_v1(
        &mut self,
        target: AnnotationReadTarget,
        address: &ResourceCatalogAddressV1,
        f: &TrustedCatalogAnnotationReadBindingV1,
        r: &mut Recorder,
    ) -> Result<AnnotationReadTarget> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            if f.annotation_readable != Some(true) {
                return Err(ResourceError::Unavailable);
            }
            let common = TrustedCatalogSourceReadBindingV1 {
                profile: f.profile,
                expected_generation: f.expected_generation,
                max_binding_bytes: f.max_binding_bytes,
                authorized: f.authorized,
                scope: f.scope,
                continuous: f.continuous,
                live: f.live,
                identity_allowed: f.identity_allowed,
                read_allowed: f.read_allowed,
                provider_available: f.provider_available,
                correspondence_current: f.correspondence_current,
                resources_ready: f.resources_ready,
            };
            let target = self.bind_catalog_read_target(
                SourceReadTargetV1 {
                    catalog: target.catalog,
                    registration: target.registration,
                    identity: target.identity,
                },
                address,
                &common,
                ReadKind::Annotations,
            )?;
            Ok(AnnotationReadTarget {
                catalog: target.catalog,
                registration: target.registration,
                identity: target.identity,
            })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        observe(
            r,
            Boundary::CatalogAnnotationReadBinding,
            &result,
            timer.finish(),
        );
        result
    }
    pub(in crate::coordinator) fn annotation_read_catalog_current(
        &self,
        capture: Option<&Arc<ReadCatalogCaptureV1>>,
        registration: &Arc<Registration>,
        identity: &Identity,
        activation: bool,
        r: &mut Recorder,
    ) -> Result<()> {
        if capture.is_none() {
            return Ok(());
        }
        let timer = r.begin_service_timing();
        let result = self.read_catalog_current(
            capture,
            registration,
            identity,
            activation,
            ReadKind::Annotations,
        );
        observe(
            r,
            Boundary::CatalogAnnotationReadCurrentness,
            &result,
            timer.finish(),
        );
        result
    }
}
