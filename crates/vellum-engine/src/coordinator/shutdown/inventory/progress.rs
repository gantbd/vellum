//! Bounded as-of native obligations after fencing; no whole-engine drain claim.
use super::*;
#[cfg(test)]
mod tests;

/// Historical metadata only. This report neither retains producer/effect leases
/// nor authorizes retirement. Inspect its explicit coverage: unrepresented owner
/// families remain unknown even when the represented work tables are empty.
pub struct ShutdownProgressV1 {
    fence: ShutdownFenceIdV1,
    snapshot: ShutdownInventoryV1,
}
impl std::fmt::Debug for ShutdownProgressV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownProgressV1(<opaque>)")
    }
}
impl SourceCoordinator {
    /// Capture current native document/writer/reader/binding obligations under
    /// this actual installed fence. This performs no waiting or worker control.
    pub fn capture_shutdown_progress_v1(
        &self,
        fence: &ShutdownFenceIdV1,
        limits: ShutdownInventoryLimitsV1,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownProgressV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational()?;
            self.validate_shutdown_disposition_fence(fence)?;
            let installed = self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?;
            self.shutdown_inventory_roles(&installed.basis, f)?;
            // Builder charges the complete inventory header. Include this
            // report wrapper too; neither header nor rows are hidden/unbounded.
            let extra = size_of::<ShutdownProgressV1>() - size_of::<ShutdownInventoryV1>();
            let limits = ShutdownInventoryLimitsV1 {
                max_capture_bytes: limits
                    .max_capture_bytes
                    .checked_sub(extra)
                    .ok_or(CoordinatorError::Limit)?,
                ..limits
            };
            let cut = self.capture_shutdown_cut(limits)?;
            Ok(ShutdownProgressV1 {
                fence: fence.clone(),
                snapshot: ShutdownInventoryV1 {
                    marker: Arc::new(()),
                    basis: installed.basis.clone(),
                    limits,
                    cut: Arc::new(cut),
                },
            })
        })();
        observe_service(
            r,
            Boundary::ShutdownProgressCapture,
            &result,
            timer.finish(),
        );
        result
    }
    /// Reauthorize release of the original as-of report, never refresh it from
    /// current state or convert it into discharge/physical-effect evidence.
    pub fn read_shutdown_progress_v1<'a>(
        &self,
        progress: &'a ShutdownProgressV1,
        f: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownInventoryReadV1<'a>, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_disposition_fence(&progress.fence)?;
            self.read_shutdown_inventory_v1(&progress.snapshot, f, r)
        })();
        observe_service(
            r,
            Boundary::ShutdownProgressRelease,
            &result,
            timer.finish(),
        );
        result
    }
}
