//! Actual native pool state and funding; no body/snapshot acquisition.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_settings_preparations(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        b.step()?;
        let open = Arc::strong_count(&self.settings_plan_life)
            .checked_sub(1)
            .ok_or(CoordinatorError::Faulted)?;
        self.record_shutdown_local_owner(
            b,
            ShutdownOwnerRoleV1::SettingsPreparations,
            crate::coordinator::shutdown::inventory::LocalOwnerSnapshot {
                configured: self.settings_activation.is_some()
                    || self.settings_definition_limits.is_some()
                    || self.settings_layer_limits.is_some()
                    || open != 0,
                issued: None,
                retained: open,
                held: None,
                reserved: None,
                limit: None,
                additional_reserved: None,
            },
        )
    }
    pub(in crate::coordinator) fn capture_shutdown_settings_retention_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::SettingsRetention;
        let p = match &self.settings_retention {
            RetentionPool::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            RetentionPool::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            RetentionPool::Active(p) => p,
        };
        let retained = b.slots(&p.rows)?;
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.rows.len(),
                retained,
                charge: p.charge,
                expected: cost(p.limits).map_err(|_| CoordinatorError::Faulted)?,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: 0,
            },
        )
    }
}
