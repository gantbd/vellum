use super::*;
use crate::coordinator::shutdown::inventory::{Builder, PoolSnapshot};
impl SourceCoordinator {
    pub(in crate::coordinator) fn capture_shutdown_settings_reader_pool(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::SettingsReader;
        let p = match &self.settings_reader {
            State::Unconfigured => {
                return self.record_shutdown_pool(b, role, PoolSnapshot::Unconfigured);
            }
            State::Closed { issued } => {
                return self.record_shutdown_pool(
                    b,
                    role,
                    PoolSnapshot::Closed {
                        issued: Some(*issued),
                    },
                );
            }
            State::Active(p) => p,
        };
        let retained = b.slots(&p.rows)?;
        let mut actual = (0usize, 0usize, 0usize);
        for row in &p.rows {
            b.step()?;
            match row {
                Some(Row::Cut(_)) => actual.0 += 1,
                Some(Row::Context(c)) => {
                    actual.1 += 1;
                    actual.2 += usize::from(c.data.is_none());
                }
                None => {}
            }
        }
        if actual != (p.packets, p.contexts, p.ended_contexts) {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_pool(
            b,
            role,
            PoolSnapshot::Active {
                issued: p.issued,
                retained,
                charge: p.charge,
                expected: p
                    .limits
                    .required_pool_bytes()
                    .map_err(|_| CoordinatorError::Faulted)?,
                reservation: &p.reservation,
                stopped: false,
                additional_reserved: p.decode_charge,
            },
        )
    }
}
