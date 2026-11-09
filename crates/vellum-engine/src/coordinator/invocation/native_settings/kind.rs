//! Closed native participants. No caller-supplied implementation or fallback arm.
use super::*;
use crate::settings::{
    DefinitionCapture, SettingsDeclarationV1, SettingsDefinitionLimitsV1,
    SettingsInterpretationEpochsV1,
};
use crate::settings::{
    LayerCapture, SettingsLayerBasisV1, SettingsLayerChangeV1, SettingsLayerLimitsV1,
};
/// Explicit allocator/generation conditions, never a lifecycle basis capability.
pub struct NativeSettingsLifecycleRequestV1<'a> {
    pub expected: SettingsLayerBasisV1,
    pub changes: &'a SettingsLayerChangeV1,
    pub dependencies: &'a ReadDependencyRequestV3,
}
/// Fresh host premises; affected full identities use common.writable_layers.
pub struct TrustedNativeSettingsLifecycleInvocationV1<'a> {
    pub common: &'a TrustedSettingsBasisFactsV1<'a>,
    pub profile: SettingsLayerLimitsV1,
    pub layer_registry_writable: Option<bool>,
    pub associations_writable: Option<bool>,
    pub required_reads: &'a [ReadDependencyV3],
}
/// Explicit native epochs and proposal data, not a conditional basis capability.
pub struct NativeSettingsDefinitionRequestV1<'a> {
    pub expected: SettingsInterpretationEpochsV1,
    pub declaration: &'a SettingsDeclarationV1,
    pub dependencies: &'a ReadDependencyRequestV3,
}
/// Fresh host premises. Each changed facet needs its own exact-true grant;
/// native conditions and basis-observation permission do not grant those writes.
pub struct TrustedNativeSettingsDefinitionInvocationV1<'a> {
    pub common: &'a TrustedSettingsBasisFactsV1<'a>,
    pub profile: SettingsDefinitionLimitsV1,
    pub schema_writable: Option<bool>,
    pub restrictions_writable: Option<bool>,
    pub associations_writable: Option<bool>,
    pub required_reads: &'a [ReadDependencyV3],
}
#[derive(Clone, Copy)]
pub(super) enum Input<'a> {
    Lifecycle {
        expected: SettingsLayerBasisV1,
        changes: &'a SettingsLayerChangeV1,
    },
    Values(&'a [SettingChangeV1]),
    Definition {
        expected: SettingsInterpretationEpochsV1,
        declaration: &'a SettingsDeclarationV1,
    },
}
#[derive(Clone, Copy)]
pub(super) enum Facts<'a, 'h> {
    Lifecycle(&'a TrustedNativeSettingsLifecycleInvocationV1<'h>),
    Values(&'a TrustedNativeSettingsInvocationV1<'h>),
    Definition(&'a TrustedNativeSettingsDefinitionInvocationV1<'h>),
}
pub(super) enum Native {
    Lifecycle {
        capture: Arc<LayerCapture>,
        expected: SettingsLayerBasisV1,
    },
    Values(Box<SettingsRequestCapture>),
    Definition {
        capture: Arc<DefinitionCapture>,
        expected: SettingsInterpretationEpochsV1,
    },
}
pub(super) enum Preparation {
    Lifecycle(Box<PreparedSettingsLayersV1>),
    Values(Box<PreparedSettingsChangeV2>),
    Definition(Box<PreparedSettingsDefinitionV1>),
}
pub(super) enum Stage {
    Lifecycle(crate::coordinator::settings_layers::LayerStage),
    Values(crate::coordinator::settings::SettingsStage),
    Definition(crate::coordinator::settings_definition::DefinitionStage),
}
impl Input<'_> {
    pub(super) fn matches(self, f: Facts<'_, '_>) -> bool {
        matches!(
            (self, f),
            (Self::Values(_), Facts::Values(_))
                | (Self::Definition { .. }, Facts::Definition(_))
                | (Self::Lifecycle { .. }, Facts::Lifecycle(_))
        )
    }
}
impl<'a, 'h> Facts<'a, 'h> {
    pub(super) fn common(self) -> &'h TrustedSettingsBasisFactsV1<'h> {
        match self {
            Self::Lifecycle(f) => f.common,
            Self::Values(f) => f.common,
            Self::Definition(f) => f.common,
        }
    }
    pub(super) fn required(self) -> &'h [ReadDependencyV3] {
        match self {
            Self::Lifecycle(f) => f.required_reads,
            Self::Values(f) => f.required_reads,
            Self::Definition(f) => f.required_reads,
        }
    }
    pub(super) fn matches(self, n: &Native) -> bool {
        matches!(
            (self, n),
            (Self::Values(_), Native::Values(_))
                | (Self::Definition(_), Native::Definition { .. })
                | (Self::Lifecycle(_), Native::Lifecycle { .. })
        )
    }
    pub(super) fn profile_matches(self, n: &Native) -> bool {
        match (self, n) {
            (Self::Lifecycle(f), Native::Lifecycle { capture, .. }) => f.profile == capture.limits,
            (Self::Values(f), Native::Values(q)) => q.profile == f.common.activation,
            (Self::Definition(f), Native::Definition { capture, .. }) => {
                f.profile == capture.limits
            }
            _ => false,
        }
    }
    pub(super) fn bound(self, o: &SourceCoordinator, c: &SettingsExecutionContextV1) -> bool {
        ordinary(self.common())
            && match self {
                Self::Lifecycle(f) => {
                    o.basis_context_current(c, f.common)
                        && o.settings_layer_limits == Some(f.profile)
                }
                Self::Values(f) => o.basis_context_current(c, f.common),
                Self::Definition(f) => {
                    o.basis_binding_current(c, f.common)
                        && o.settings_definition_limits == Some(f.profile)
                }
            }
    }
    pub(super) fn gate_preparation(
        self,
        o: &SourceCoordinator,
        c: &SettingsExecutionContextV1,
        n: &Native,
        p: &Preparation,
    ) -> Result<(), Error> {
        if !self.bound(o, c) || !self.profile_matches(n) {
            return Err(settings(SettingsError::Unavailable));
        }
        match (self, n, p) {
            (Self::Lifecycle(f), Native::Lifecycle { capture, .. }, Preparation::Lifecycle(p)) => {
                let root = o
                    .settings
                    .as_ref()
                    .ok_or(settings(SettingsError::Unavailable))?;
                let (_, outcome) = p.plan.decide(root).map_err(settings)?;
                if outcome.associations_changed && f.associations_writable != Some(true) {
                    return Err(settings(SettingsError::Unavailable));
                }
                if outcome.created_layers == 0 && outcome.removed_layers == 0 {
                    return Ok(());
                }
                if f.layer_registry_writable != Some(true) {
                    return Err(settings(SettingsError::Unavailable));
                }
                o.native_identity_grants(
                    f.common,
                    capture
                        .created_identities()
                        .chain(capture.removals().iter().map(|r| &r.identity)),
                )
            }
            (Self::Values(f), Native::Values(q), Preparation::Values(_)) => {
                o.native_layer_grants(f.common, q)
            }
            (Self::Definition(f), Native::Definition { .. }, Preparation::Definition(p)) => {
                let root = o
                    .settings
                    .as_ref()
                    .ok_or(settings(SettingsError::Unavailable))?;
                let (_, outcome) = p.plan.decide(root).map_err(settings)?;
                if (!outcome.schema_changed || f.schema_writable == Some(true))
                    && (!outcome.restrictions_changed || f.restrictions_writable == Some(true))
                    && (!outcome.associations_changed || f.associations_writable == Some(true))
                {
                    Ok(())
                } else {
                    Err(settings(SettingsError::Unavailable))
                }
            }
            _ => Err(settings(SettingsError::Faulted)),
        }
    }
    pub(super) fn rejected(self, e: Error) -> InvocationOutcome {
        match self {
            Self::Lifecycle(_) => InvocationOutcome::SettingsLayersRejected(e),
            Self::Values(_) => InvocationOutcome::SettingsRejectedV2(e),
            Self::Definition(_) => InvocationOutcome::SettingsDefinitionRejected(e),
        }
    }
}
impl Native {
    pub(super) fn capture(
        o: &SourceCoordinator,
        input: Input<'_>,
        f: Facts<'_, '_>,
        ceiling: usize,
    ) -> Result<Self, SettingsError> {
        let ceiling = ceiling
            .checked_sub(std::mem::size_of::<Self>())
            .ok_or(SettingsError::Limit)?;
        match (input, f) {
            (Input::Lifecycle { expected, changes }, Facts::Lifecycle(f)) => {
                expected.validate()?;
                let root = o.settings.as_ref().ok_or(SettingsError::Unavailable)?;
                Ok(Self::Lifecycle {
                    capture: Arc::new(LayerCapture::copy(changes, root, f.profile, ceiling)?),
                    expected,
                })
            }
            (Input::Values(q), Facts::Values(f)) => Ok(Self::Values(Box::new(
                SettingsRequestCapture::copy(q, f.common.activation, ceiling)?,
            ))),
            (
                Input::Definition {
                    expected,
                    declaration,
                },
                Facts::Definition(f),
            ) => {
                DefinitionCapture::validate_expected(expected)?;
                let root = o.settings.as_ref().ok_or(SettingsError::Unavailable)?;
                Ok(Self::Definition {
                    capture: Arc::new(DefinitionCapture::from_state(
                        declaration,
                        root,
                        f.profile,
                        ceiling,
                    )?),
                    expected,
                })
            }
            _ => Err(SettingsError::Faulted),
        }
    }
    pub(super) fn bytes(&self) -> Result<usize, SettingsError> {
        let sum = match self {
            Self::Lifecycle { capture, .. } => {
                std::mem::size_of::<Self>().checked_add(capture.bytes())
            }
            Self::Values(q) => std::mem::size_of::<Self>().checked_add(q.retained_bytes()),
            Self::Definition { capture, .. } => {
                std::mem::size_of::<Self>().checked_add(capture.bytes())
            }
        };
        sum.ok_or(SettingsError::Limit)
    }
    pub(super) fn max_plan_bytes(&self) -> usize {
        match self {
            Self::Lifecycle { capture, .. } => capture.limits.max_plan_bytes,
            Self::Values(q) => q.profile.max_plan_bytes,
            Self::Definition { capture, .. } => capture.limits.max_plan_bytes,
        }
    }
    pub(super) fn same_input(
        &self,
        input: Input<'_>,
        ceiling: usize,
    ) -> Result<bool, SettingsError> {
        match (self, input) {
            (
                Self::Lifecycle { capture, expected },
                Input::Lifecycle {
                    expected: current,
                    changes,
                },
            ) => {
                if *expected != current {
                    return Ok(false);
                }
                capture.same(&capture.recopy(changes, ceiling)?)
            }
            (Self::Values(q), Input::Values(input)) => {
                q.same(&SettingsRequestCapture::copy(input, q.profile, ceiling)?)
            }
            (
                Self::Definition { capture, expected },
                Input::Definition {
                    expected: current,
                    declaration,
                },
            ) => {
                if *expected != current {
                    return Ok(false);
                }
                capture.same(&capture.recopy(declaration, ceiling)?)
            }
            _ => Ok(false),
        }
    }
}
impl Preparation {
    pub(super) fn prepare(
        o: &SourceCoordinator,
        n: &Native,
        input: Input<'_>,
        reads: read_dependency::ReadCapture,
        r: &mut Recorder,
    ) -> Result<Self, Error> {
        match (n, input) {
            (Native::Lifecycle { capture, expected }, Input::Lifecycle { .. }) => o
                .prepare_layer_capture(*expected, capture.clone(), reads, r)
                .map(|p| Self::Lifecycle(Box::new(p))),
            (Native::Values(_), Input::Values(changes)) => o
                .prepare_settings_read_capture(changes, reads, r)
                .map(|p| Self::Values(Box::new(p))),
            (Native::Definition { capture, expected }, Input::Definition { .. }) => o
                .prepare_definition_capture(*expected, capture.clone(), reads, r)
                .map(|p| Self::Definition(Box::new(p))),
            _ => Err(settings(SettingsError::Faulted)),
        }
    }
    pub(super) fn bytes(&self) -> Result<usize, SettingsError> {
        let (embedded, bytes) = match self {
            Self::Lifecycle(p) => (
                std::mem::size_of::<Box<PreparedSettingsLayersV1>>(),
                p.retained_bytes()?,
            ),
            Self::Values(p) => (
                std::mem::size_of::<Box<PreparedSettingsChangeV2>>(),
                p.retained_bytes()?,
            ),
            Self::Definition(p) => (
                std::mem::size_of::<Box<PreparedSettingsDefinitionV1>>(),
                p.retained_bytes()?,
            ),
        };
        (std::mem::size_of::<Self>() - embedded)
            .checked_add(bytes)
            .ok_or(SettingsError::Limit)
    }
    pub(super) fn intact(&self, n: &Native, reads: &read_dependency::ReadCapture) -> bool {
        match (self, n) {
            (Self::Lifecycle(p), Native::Lifecycle { capture, expected }) => {
                p.read_count == reads.predicates.len()
                    && p.reads == *reads
                    && p.plan.matches(capture, *expected)
            }
            (Self::Values(p), Native::Values(q)) => {
                p.reads_required
                    && p.read_count == reads.predicates.len()
                    && p.reads == *reads
                    && p.settings.plan.matches(q).unwrap_or(false)
            }
            (Self::Definition(p), Native::Definition { capture, expected }) => {
                p.read_count == reads.predicates.len()
                    && p.reads == *reads
                    && p.plan.matches(capture, *expected)
            }
            _ => false,
        }
    }
    pub(super) fn stage(self, o: &SourceCoordinator, r: &mut Recorder) -> Result<Stage, Error> {
        match self {
            Self::Lifecycle(p) => o.stage_settings_layers(*p, r).map(Stage::Lifecycle),
            Self::Values(p) => o.stage_settings_with_reads(*p, r).map(Stage::Values),
            Self::Definition(p) => o.stage_settings_definition(*p, r).map(Stage::Definition),
        }
    }
}
impl Stage {
    pub(super) fn install(self, o: &mut SourceCoordinator) -> InvocationOutcome {
        match self {
            Self::Lifecycle(stage) => {
                let outcome = InvocationOutcome::SettingsLayersCommitted(stage.outcome);
                stage.install(o);
                outcome
            }
            Self::Values(stage) => {
                let outcome = InvocationOutcome::SettingsCommitted(stage.outcome);
                stage.install(o);
                outcome
            }
            Self::Definition(stage) => {
                let outcome = InvocationOutcome::SettingsDefinitionCommitted(stage.outcome);
                stage.install(o);
                outcome
            }
        }
    }
}
