//! Confirmed destination workflows above the trusted SDK, never engine verbs.
use super::*;
use std::sync::Arc;
use vellum_engine::coordinator::{
    TrustedSourceExportControlV1, TrustedSourceExportFactsV1, TrustedSourceSaveAsFactsV1,
};
use vellum_engine::resource::SourceExportPurposeV1;

/// Immutable confirmation context, not a write grant. Equality means the same
/// issued UI proposal, not equal-looking source/path strings after an ABA change.
#[derive(Clone)]
pub struct DestinationPlan {
    marker: Arc<()>,
    expected: DocumentVersion,
    path: PathBuf,
    save_as: bool,
}
impl PartialEq for DestinationPlan {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for DestinationPlan {}
impl std::fmt::Debug for DestinationPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DestinationPlan(<redacted>)")
    }
}
impl DestinationPlan {
    pub fn destination(&self) -> &Path {
        &self.path
    }
    pub fn is_save_as(&self) -> bool {
        self.save_as
    }
}
// This synchronous reference application is the trusted single-user host, not
// an untrusted plugin endpoint. Explicit confirmation supplies destination intent;
// native owners still enforce actual source, binding, policy and resource guards.
fn facts() -> TrustedSourceExportFactsV1 {
    TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: Some(true),
    }
}
fn control() -> TrustedSourceExportControlV1 {
    TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    }
}
impl Editor {
    pub fn plan_destination(
        &mut self,
        path: PathBuf,
        save_as: bool,
    ) -> Result<DestinationPlan, EditorError> {
        self.live()?;
        if path.as_os_str().is_empty() {
            return Err(EditorError::Empty);
        }
        let expected = self.host.read(&self.tabs[self.active].document)?.version;
        Ok(DestinationPlan {
            marker: Arc::new(()),
            expected,
            path,
            save_as,
        })
    }
    /// Explicit UI/host confirmation of the original source and destination.
    /// No automatic refresh/retry, implicit create or compensating overwrite.
    pub fn confirm_destination(&mut self, plan: &DestinationPlan) -> Result<(), EditorError> {
        let result = (|| {
            self.live()?;
            if self
                .host
                .read(&self.tabs[self.active].document)?
                .version
                .document
                != plan.expected.document
            {
                return Err(SessionError::WrongDocument.into());
            }
            self.clear_export_preparation()?;
            self.exports
                .try_reserve(1)
                .map_err(|_| EditorError::Capacity)?;
            self.export_preparation = Some(self.host.prepare_source_export(&plan.expected)?);
            let queued = self.host.queue_source_export(
                self.export_preparation
                    .as_ref()
                    .expect("retained original preparation"),
                &plan.path,
                if plan.save_as {
                    SourceExportPurposeV1::SaveAs
                } else {
                    SourceExportPurposeV1::CopyExport
                },
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
                &facts(),
            );
            let attempt = match queued {
                Ok(attempt) => {
                    self.export_preparation = None;
                    attempt
                }
                Err(error) => {
                    self.clear_export_preparation()?;
                    return Err(error.into());
                }
            };
            self.exports.push(attempt);
            let index = self.exports.len() - 1;
            let report = self
                .host
                .service_source_export(&self.exports[index], &facts())?;
            self.status = format!(
                "Destination write: {:?}; sync: {:?}; cleanup failed: {}; binding not activated",
                report.outcome.application, report.outcome.synchronization, report.cleanup_failed
            );
            if report.failure.is_some()
                || report.cleanup_failed
                || report.outcome.application != Application::Written
                || report.outcome.synchronization != Synchronization::Succeeded
                || !report.outcome.no_future_effects
                || !report.outcome.quiescent
            {
                return Err(EditorError::SaveIncomplete);
            }
            if plan.save_as {
                let activation = self.host.activate_source_save_as(
                    &self.exports[index],
                    16,
                    &TrustedSourceSaveAsFactsV1 {
                        control: control(),
                        probe_attested: Some(true),
                        resource_continuity_attested: Some(true),
                        create_allowed: None,
                        overwrite_allowed: Some(true),
                    },
                )?;
                self.tabs[self.active].document = activation.document;
                self.status = if self.comments_dirty()? {
                    "Source Saved As; binding activated; session comments remain unsaved"
                } else {
                    "Saved As — Written; synchronization Succeeded; binding activated"
                }
                .into();
            } else {
                self.host
                    .release_source_export(&self.exports[index], &control())?;
                self.host.retire_save(self.exports[index].native_id())?;
                self.exports.remove(index);
                self.status = "Copy exported — Written; synchronization Succeeded; original binding unchanged".into();
            }
            Ok(())
        })();
        self.collect_diagnostics();
        result
    }
    fn clear_export_preparation(&mut self) -> Result<(), EditorError> {
        if let Some(prepared) = &self.export_preparation {
            match self.host.discard_source_export_preparation(prepared) {
                Ok(()) => {}
                // Only our original privately held handle; native admission can
                // already spend it on refusal. Actual occupancy is checked too.
                Err(SessionError::Resource(ResourceError::Unavailable))
                    if self.host.resource_usage().prepared_sources == 0 => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.export_preparation = None;
        Ok(())
    }
    pub(super) fn settle_exports(&mut self) -> Result<(), EditorError> {
        self.clear_export_preparation()?;
        for attempt in &self.exports {
            self.host.stop_save(attempt.native_id())?;
            let outcome = self.host.save_outcome(attempt.native_id())?;
            if !outcome.quiescent || !outcome.no_future_effects {
                return Err(EditorError::SaveIncomplete);
            }
            self.host.release_source_export(attempt, &control())?;
        }
        Ok(())
    }
    pub(super) fn retire_exports(&mut self) -> Result<(), EditorError> {
        while let Some(attempt) = self.exports.last() {
            self.host.retire_save(attempt.native_id())?;
            self.exports.pop();
        }
        Ok(())
    }
}
