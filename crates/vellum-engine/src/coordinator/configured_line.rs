//! Actual settings consumption, lowered through the existing explicit line editor.
use super::*;
use crate::settings::{CompiledSettingsV1, SettingDomainV1, SettingsError};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfiguredLineLimitsV1 {
    pub max_identifier_bytes: usize,
    pub max_request_bytes: usize,
    pub max_inner_intent_bytes: usize,
}
#[derive(PartialEq, Eq)]
pub enum ConfiguredLineEditV1 {
    Indent {
        target: SettingsTargetId,
        unit_key: String,
        tab_width_key: String,
    },
    Outdent {
        target: SettingsTargetId,
        unit_key: String,
        tab_width_key: String,
    },
    Newline {
        target: SettingsTargetId,
        delimiter_key: String,
        auto_indent_key: String,
    },
}
impl std::fmt::Debug for ConfiguredLineEditV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConfiguredLineEditV1(<redacted>)")
    }
}
impl ConfiguredLineEditV1 {
    pub(super) fn parts(&self) -> (&SettingsTargetId, &str, &str) {
        match self {
            Self::Indent {
                target,
                unit_key,
                tab_width_key,
            }
            | Self::Outdent {
                target,
                unit_key,
                tab_width_key,
            } => (target, unit_key, tab_width_key),
            Self::Newline {
                target,
                delimiter_key,
                auto_indent_key,
            } => (target, delimiter_key, auto_indent_key),
        }
    }
    pub(super) fn payload_bytes(&self) -> Option<usize> {
        let (_, a, b) = self.parts();
        a.len().checked_add(b.len())
    }
    pub(super) fn copy(&self, p: ConfiguredLineLimitsV1) -> Result<Self, InvocationError> {
        let (target, a, b) = self.parts();
        if a.is_empty() || b.is_empty() {
            return Err(InvocationError::MalformedIntent);
        }
        if a.len() > p.max_identifier_bytes
            || b.len() > p.max_identifier_bytes
            || self
                .payload_bytes()
                .and_then(|n| n.checked_add(std::mem::size_of::<Self>()))
                .is_none_or(|n| n > p.max_request_bytes)
        {
            return Err(InvocationError::Limit);
        }
        fn text(s: &str) -> Result<String, InvocationError> {
            let mut v = String::new();
            v.try_reserve_exact(s.len())
                .map_err(|_| InvocationError::Capacity)?;
            v.push_str(s);
            Ok(v)
        }
        let a = text(a)?;
        let b = text(b)?;
        let target = target.clone();
        Ok(match self {
            Self::Indent { .. } => Self::Indent {
                target,
                unit_key: a,
                tab_width_key: b,
            },
            Self::Outdent { .. } => Self::Outdent {
                target,
                unit_key: a,
                tab_width_key: b,
            },
            Self::Newline { .. } => Self::Newline {
                target,
                delimiter_key: a,
                auto_indent_key: b,
            },
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfiguredLineErrorV1 {
    InvalidConfiguration,
    Limit,
    Capacity,
    Settings(SettingsError),
    Line(LineEditError),
    Coordinator(CoordinatorError),
}
impl ConfiguredLineErrorV1 {
    pub(super) fn faulted(self) -> bool {
        matches!(
            self,
            Self::Settings(SettingsError::Faulted)
                | Self::Coordinator(
                    CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
                )
                | Self::Line(
                    LineEditError::Coordinator(
                        CoordinatorError::Faulted
                            | CoordinatorError::History(HistoryError::Faulted)
                    ) | LineEditError::Selection(crate::selection::SelectionError::Faulted)
                )
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Witness {
    pub(super) lease: SettingsInterpretationId,
    basis: SettingsBasisV1,
    source: u64,
}
#[derive(Debug)]
pub(super) struct ConfiguredLineCapture {
    pub(super) line_budget: Option<LineEditBudget>,
    pub(super) request: ConfiguredLineEditV1,
    pub(super) witness: Witness,
    pub(super) limits: ConfiguredLineLimitsV1,
}
pub(super) fn observe<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, ConfiguredLineErrorV1>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(error) if error.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: None,
    });
}
fn validation_error(error: ConfiguredLineErrorV1) -> CoordinatorError {
    if error == ConfiguredLineErrorV1::Capacity {
        CoordinatorError::Capacity
    } else {
        CoordinatorError::Faulted
    }
}
impl SourceCoordinator {
    pub fn configure_configured_line_edits(
        &mut self,
        p: ConfiguredLineLimitsV1,
        r: &mut Recorder,
    ) -> Result<bool, ConfiguredLineErrorV1> {
        let result = (|| {
            self.ensure_operational()
                .map_err(ConfiguredLineErrorV1::Coordinator)?;
            match self.configured_line_limits {
                Some(old) if old == p => Ok(false),
                Some(_) => Err(ConfiguredLineErrorV1::Coordinator(
                    CoordinatorError::Unavailable,
                )),
                None => {
                    self.configured_line_limits = Some(p);
                    Ok(true)
                }
            }
        })();
        observe(r, Boundary::ConfiguredLineConfiguration, &result);
        result
    }
    pub(super) fn resolve_configured_line(
        &self,
        state: &CompiledSettingsV1,
        request: &ConfiguredLineEditV1,
    ) -> Result<LineEditV1, ConfiguredLineErrorV1> {
        let (target, a, b) = request.parts();
        let target = self
            .owned_settings_target(state, target)
            .map_err(ConfiguredLineErrorV1::Settings)?;
        let a = state
            .read_index(target, a)
            .map_err(ConfiguredLineErrorV1::Settings)?;
        let b = state
            .read_index(target, b)
            .map_err(ConfiguredLineErrorV1::Settings)?;
        if a.domain != SettingDomainV1::Source || b.domain != SettingDomainV1::Source {
            return Err(ConfiguredLineErrorV1::InvalidConfiguration);
        }
        match request {
            ConfiguredLineEditV1::Indent { .. } | ConfiguredLineEditV1::Outdent { .. } => {
                let unit = a
                    .value
                    .text()
                    .ok_or(ConfiguredLineErrorV1::InvalidConfiguration)?;
                let width = b
                    .value
                    .number()
                    .filter(|n| {
                        n.is_finite()
                            && *n >= 1.0
                            && n.fract() == 0.0
                            && *n <= 9_007_199_254_740_991.0
                    })
                    .ok_or(ConfiguredLineErrorV1::InvalidConfiguration)?;
                let width = usize::try_from(width as u64)
                    .map_err(|_| ConfiguredLineErrorV1::InvalidConfiguration)?;
                let budget = self.line_edit_profile().ok_or(ConfiguredLineErrorV1::Line(
                    LineEditError::Coordinator(CoordinatorError::Unavailable),
                ))?;
                if unit.len() > budget.max_unit_bytes || width > budget.max_indent_columns {
                    return Err(ConfiguredLineErrorV1::Limit);
                }
                let mut copy = String::new();
                copy.try_reserve_exact(unit.len())
                    .map_err(|_| ConfiguredLineErrorV1::Capacity)?;
                copy.push_str(unit);
                Ok(if matches!(request, ConfiguredLineEditV1::Indent { .. }) {
                    LineEditV1::Indent {
                        unit: copy,
                        tab_width: width,
                    }
                } else {
                    LineEditV1::Outdent {
                        unit: copy,
                        tab_width: width,
                    }
                })
            }
            ConfiguredLineEditV1::Newline { .. } => {
                let delimiter = match a.value.text() {
                    Some("LF") => GeneratedNewlineV1::Lf,
                    Some("CRLF") => GeneratedNewlineV1::CrLf,
                    Some("CR") => GeneratedNewlineV1::Cr,
                    _ => return Err(ConfiguredLineErrorV1::InvalidConfiguration),
                };
                let indent = match b.value.text() {
                    Some("none") => NewlineIndentV1::None,
                    Some("copy-prefix") => NewlineIndentV1::CopyPrefix,
                    _ => return Err(ConfiguredLineErrorV1::InvalidConfiguration),
                };
                Ok(LineEditV1::Newline { delimiter, indent })
            }
        }
    }
    pub(super) fn prepare_configured_line(
        &mut self,
        context: &LogicalViewContext,
        request: ConfiguredLineEditV1,
        p: ConfiguredLineLimitsV1,
        remaining: usize,
        r: &mut Recorder,
    ) -> Result<(PreparedTransaction, Witness, LineEditV1), ConfiguredLineErrorV1> {
        let result = (|| {
            let state = self
                .settings
                .as_ref()
                .ok_or(ConfiguredLineErrorV1::Settings(SettingsError::Unavailable))?;
            let explicit = self.resolve_configured_line(state, &request)?;
            let inner = std::mem::size_of::<LineEditV1>()
                .checked_add(std::mem::size_of::<LogicalViewContext>())
                .and_then(|n| n.checked_add(std::mem::size_of::<LineEditBudget>()))
                .and_then(|n| n.checked_add(explicit.unit_bytes()))
                .ok_or(ConfiguredLineErrorV1::Limit)?;
            if inner > p.max_inner_intent_bytes || inner > remaining {
                return Err(ConfiguredLineErrorV1::Limit);
            }
            let mut prepared = self
                .prepare_line_edit(context, &explicit, r)
                .map_err(ConfiguredLineErrorV1::Line)?;
            let target = self
                .owned_settings_target(state, request.parts().0)
                .map_err(ConfiguredLineErrorV1::Settings)?;
            let source = state
                .generations(target)
                .map_err(ConfiguredLineErrorV1::Settings)?
                .source;
            let basis = state.basis();
            let lease = self
                .acquire_settings_interpretation(r)
                .map_err(ConfiguredLineErrorV1::Settings)?;
            let witness = Witness {
                lease,
                basis,
                source,
            };
            prepared.configured_required = true;
            prepared.configured_line = Some(ConfiguredLineCapture {
                line_budget: self.line_edit_profile(),
                request,
                witness: witness.clone(),
                limits: p,
            });
            Ok((prepared, witness, explicit))
        })();
        observe(r, Boundary::ConfiguredLinePreparation, &result);
        result
    }
    pub(super) fn validate_configured_line(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        if !prepared.configured_required && prepared.configured_line.is_none() {
            return Ok(());
        }
        let result = (|| {
            let captured = prepared
                .configured_line
                .as_ref()
                .filter(|_| prepared.configured_required)
                .ok_or(CoordinatorError::Faulted)?;
            if self.configured_line_limits != Some(captured.limits) {
                return Err(CoordinatorError::Stale);
            }
            if self.line_edit_profile() != captured.line_budget {
                return Err(CoordinatorError::Stale);
            }
            let held = self
                .retained_settings_interpretation(&captured.witness.lease)
                .map_err(|_| CoordinatorError::Faulted)?;
            if held.basis() != captured.witness.basis {
                return Err(CoordinatorError::Faulted);
            }
            let current = self.settings.as_ref().ok_or(CoordinatorError::Faulted)?;
            let a = current.basis();
            let b = captured.witness.basis;
            if (
                a.schema_generation,
                a.restriction_generation,
                a.association_generation,
            ) != (
                b.schema_generation,
                b.restriction_generation,
                b.association_generation,
            ) {
                return Err(CoordinatorError::Stale);
            }
            let target = self
                .owned_settings_target(current, captured.request.parts().0)
                .map_err(|_| CoordinatorError::Faulted)?;
            let held_target = self
                .owned_settings_target(held, captured.request.parts().0)
                .map_err(|_| CoordinatorError::Faulted)?;
            if held
                .generations(held_target)
                .map_err(|_| CoordinatorError::Faulted)?
                .source
                != captured.witness.source
            {
                return Err(CoordinatorError::Faulted);
            }
            if current
                .generations(target)
                .map_err(|_| CoordinatorError::Faulted)?
                .source
                != captured.witness.source
            {
                return Err(CoordinatorError::Stale);
            }
            let explicit = self
                .resolve_configured_line(current, &captured.request)
                .map_err(validation_error)?;
            let original = self
                .resolve_configured_line(held, &captured.request)
                .map_err(validation_error)?;
            if original != explicit {
                return Err(CoordinatorError::Faulted);
            }
            // The native line verifier independently checks derived edits and current profile.
            if !Self::line_intent_matches(
                prepared,
                self.configured_line_context(prepared)?,
                &explicit,
                self.line_edit_profile(),
            ) {
                return Err(CoordinatorError::Faulted);
            }
            Ok(())
        })();
        let observed = result.map_err(ConfiguredLineErrorV1::Coordinator);
        observe(r, Boundary::ConfiguredLineValidation, &observed);
        result
    }
}
