//! Context-bearing native settings over one sealed ledger/publisher carrier.
//! Closed participant branches are not extension callbacks or new command families.
use super::super::settings::basis::definition::ordinary;
use super::super::settings::basis::{copy_context, labels};
use super::settings_scoped::error;
use super::*;
mod arguments;
pub use arguments::*;
mod declaration;
pub use declaration::NativeSettingsArgumentDeclarationV1;
mod definition_arguments;
pub use definition_arguments::*;
mod lifecycle_arguments;
pub use lifecycle_arguments::*;
mod kind;
use kind::{Facts, Input, Native, Preparation};
pub use kind::{NativeSettingsDefinitionRequestV1, TrustedNativeSettingsDefinitionInvocationV1};
pub use kind::{NativeSettingsLifecycleRequestV1, TrustedNativeSettingsLifecycleInvocationV1};
#[cfg(test)]
mod tests;
use crate::settings::{
    SettingChangeV1, SettingsActivationLimitsV1, SettingsError, SettingsRequestCapture,
};
type Error = SettingsChangeErrorV2;
/// Current host premises; native identities do not authorize writes.
pub struct TrustedNativeSettingsInvocationV1<'a> {
    pub common: &'a TrustedSettingsBasisFactsV1<'a>,
    pub required_reads: &'a [ReadDependencyV3],
}
pub(super) struct Capture {
    pub(super) owner: Arc<()>,
    pub(super) context: SettingsExecutionContextV1,
    activation: SettingsActivationLimitsV1,
    native: Native,
    caller: read_dependency::ReadCapture,
    required: read_dependency::ReadCapture,
    combined: read_dependency::ReadCapture,
    preparation: std::sync::Weak<Preparation>,
    bytes: usize,
}
pub(super) struct Prepared {
    capture: Arc<Capture>,
    seal: Arc<Capture>,
    native: Arc<Preparation>,
}
impl Prepared {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.capture.refers_to_document(document)
    }
}
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeSettingsCapture(<redacted>)")
    }
}
impl Capture {
    pub(super) fn refers_to_document(&self, document: &DocumentId) -> bool {
        self.caller.refers_to_document(document)
            || self.required.refers_to_document(document)
            || self.combined.refers_to_document(document)
    }
}
impl std::fmt::Debug for Prepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedNativeSettings(<redacted>)")
    }
}
fn settings(e: SettingsError) -> Error {
    Error::Settings(e)
}
fn reads(e: ReadDependencyError) -> Error {
    Error::Reads(e.into())
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(settings(SettingsError::Limit))
}
impl Prepared {
    pub(super) fn intact(&self, c: &Arc<Capture>, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(c, &self.capture)
            && Arc::ptr_eq(c, &self.seal)
            && Arc::ptr_eq(&c.owner, owner)
            && c.preparation.ptr_eq(&Arc::downgrade(&self.native))
            && self.native.intact(&c.native, &c.combined)
    }
    fn bytes(&self) -> Result<usize, Error> {
        add(
            add(self.capture.bytes, self.native.bytes().map_err(settings)?)?,
            std::mem::size_of::<Self>(),
        )
    }
}
impl SourceCoordinator {
    fn native_layer_grants(
        &self,
        f: &TrustedSettingsBasisFactsV1<'_>,
        request: &SettingsRequestCapture,
    ) -> Result<(), Error> {
        self.native_identity_grants(f, request.layers())
    }
    fn native_identity_grants<'a>(
        &self,
        f: &TrustedSettingsBasisFactsV1<'_>,
        layers: impl Iterator<Item = &'a crate::settings::SettingIdentityV1>,
    ) -> Result<(), Error> {
        let maximum = self
            .settings_basis_profile
            .as_ref()
            .ok_or(settings(SettingsError::Unavailable))?
            .grant_work_limit();
        let grants = f.writable_layers.unwrap_or(&[]);
        let mut work = add(7, grants.len())?;
        for layer in layers {
            let mut allowed = false;
            for grant in grants {
                work = add(work, 1)?;
                if work > maximum {
                    return Err(settings(SettingsError::Limit));
                }
                allowed |= grant == layer;
            }
            if !allowed {
                return Err(settings(SettingsError::Unavailable));
            }
        }
        Ok(())
    }
    fn capture_native_settings(
        &self,
        input: Input<'_>,
        request: &ReadDependencyRequestV3,
        c: &SettingsExecutionContextV1,
        f: Facts<'_, '_>,
    ) -> Result<Capture, Error> {
        self.ensure_operational().map_err(Error::Reads)?;
        if !f.bound(self, c) || !input.matches(f) {
            return Err(settings(SettingsError::Unavailable));
        }
        let parent = self.settings_job_parent_limit();
        let caller_input = read_dependency::ReadInput::V3(request);
        let caller_bytes = caller_input
            .retained_bytes(&[])
            .ok_or(settings(SettingsError::Limit))?;
        let required_bytes = read_dependency::ReadCapture::size_for_predicates(f.required().len())
            .ok_or(settings(SettingsError::Limit))?;
        if add(caller_bytes, required_bytes)? > parent {
            return Err(settings(SettingsError::Limit));
        }
        let caller = caller_input.copy(&[]).map_err(reads)?;
        let required =
            read_dependency::ReadCapture::configuration_slice(f.required(), request.budget)
                .map_err(reads)?;
        let combined = caller.conjoin(&required).map_err(reads)?;
        let mut floor = std::mem::size_of::<Capture>()
            - std::mem::size_of::<Native>()
            - 3 * std::mem::size_of::<read_dependency::ReadCapture>();
        for capture in [&caller, &required, &combined] {
            floor = add(
                floor,
                capture
                    .logical_bytes()
                    .ok_or(settings(SettingsError::Limit))?,
            )?;
        }
        for label in labels(c) {
            floor = add(floor, label.len())?;
        }
        let ceiling = parent
            .checked_sub(floor)
            .ok_or(settings(SettingsError::Limit))?;
        let native = Native::capture(self, input, f, ceiling).map_err(settings)?;
        if let Native::Values(q) = &native {
            self.native_layer_grants(f.common(), q)?;
        }
        let bytes = add(floor, native.bytes().map_err(settings)?)?;
        Ok(Capture {
            owner: self.namespace.clone(),
            context: copy_context(c).map_err(settings)?,
            activation: f.common().activation,
            native,
            caller,
            required,
            combined,
            preparation: std::sync::Weak::new(),
            bytes,
        })
    }
    fn prepare_native_settings(
        &self,
        mut capture: Capture,
        input: Input<'_>,
        r: &mut Recorder,
    ) -> Result<Prepared, Error> {
        let native = Arc::new(Preparation::prepare(
            self,
            &capture.native,
            input,
            capture.combined.copy_owned().map_err(reads)?,
            r,
        )?);
        capture.preparation = Arc::downgrade(&native);
        let capture = Arc::new(capture);
        let p = Prepared {
            seal: capture.clone(),
            capture,
            native,
        };
        if p.bytes()? > self.settings_job_parent_limit()
            || p.bytes()? > p.capture.native.max_plan_bytes()
        {
            return Err(settings(SettingsError::Limit));
        }
        Ok(p)
    }
    pub fn submit_native_settings_change_v1(
        &mut self,
        id: &InvocationId,
        changes: &[SettingChangeV1],
        request: &ReadDependencyRequestV3,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_native_settings_input(
            id,
            Input::Values(changes),
            request,
            c,
            (Facts::Values(f), None),
            r,
        )
    }
    pub fn submit_native_settings_definition_v1(
        &mut self,
        id: &InvocationId,
        q: &NativeSettingsDefinitionRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsDefinitionInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_native_settings_input(
            id,
            Input::Definition {
                expected: q.expected,
                declaration: q.declaration,
            },
            q.dependencies,
            c,
            (Facts::Definition(f), None),
            r,
        )
    }
    pub fn submit_native_settings_lifecycle_v1(
        &mut self,
        id: &InvocationId,
        q: &NativeSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_native_settings_input(
            id,
            Input::Lifecycle {
                expected: q.expected,
                changes: q.changes,
            },
            q.dependencies,
            c,
            (Facts::Lifecycle(f), None),
            r,
        )
    }
    /// Optional, issued-only result admission; ordinary replay uses the same native request.
    pub fn submit_native_lifecycle_with_created_results_v1(
        &mut self,
        id: &InvocationId,
        q: &NativeSettingsLifecycleRequestV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        result: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_native_settings_input(
            id,
            Input::Lifecycle {
                expected: q.expected,
                changes: q.changes,
            },
            q.dependencies,
            c,
            (Facts::Lifecycle(f), Some(result)),
            r,
        )
    }
    fn submit_native_settings_input(
        &mut self,
        id: &InvocationId,
        input: Input<'_>,
        request: &ReadDependencyRequestV3,
        c: &SettingsExecutionContextV1,
        admission: (
            Facts<'_, '_>,
            Option<&TrustedCreatedLayerResultAdmissionV1<'_>>,
        ),
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let (f, result_facts) = admission;
        let timer = r.begin_service_timing();
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(result) = result_facts
                && (result.context != c || slot.intent.is_some())
            {
                return Err(InvocationError::Unavailable);
            }
            if let Some(original) = &slot.intent {
                let BoundIntent::NativeSettings(original) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if !input.matches(f) || !f.matches(&original.native) {
                    return Err(InvocationError::IntentMismatch);
                }
                if original.context != *c
                    || f.common().context != c
                    || original.activation != f.common().activation
                    || !f.profile_matches(&original.native)
                {
                    return Err(InvocationError::Unavailable);
                }
                let caller = read_dependency::ReadInput::V3(request)
                    .copy(&[])
                    .map_err(|e| error(reads(e)))?;
                if original.caller != caller
                    || !original
                        .native
                        .same_input(input, self.settings_job_parent_limit())
                        .map_err(|e| error(settings(e)))?
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let p = self
                .capture_native_settings(input, request, c, f)
                .and_then(|capture| self.prepare_native_settings(capture, input, r))
                .map_err(error)?;
            f.gate_preparation(self, c, &p.capture.native, &p.native)
                .map_err(error)?;
            if let Some(result) = result_facts {
                let Native::Lifecycle { capture, .. } = &p.capture.native else {
                    return Err(InvocationError::Unavailable);
                };
                self.attach_native_created_hold(id, capture, result, r)?;
            }
            let capture = p.capture.clone();
            self.install_invocation_admission(
                id,
                BoundIntent::NativeSettings(capture),
                Phase::NativeSettingsPending(p),
            );
            Ok(InvocationState::Pending)
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        let elapsed = timer.finish();
        capture_timed(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
            elapsed,
        );
        capture_timed(r, Boundary::NativeSettingsAdmission, &result, elapsed);
        result
    }
    pub fn run_native_settings_change_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_native_settings_service(id, c, Facts::Values(f), r)
    }
    pub fn run_native_settings_definition_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsDefinitionInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_native_settings_service(id, c, Facts::Definition(f), r)
    }
    pub fn run_native_settings_lifecycle_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_native_settings_service(id, c, Facts::Lifecycle(f), r)
    }
    fn run_native_settings_service(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: Facts<'_, '_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timer = r.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|s| matches!(s.phase, Phase::Terminal(_)));
        let result = self.run_native_settings_inner(id, c, f, r);
        capture_timed(
            r,
            if terminal {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationExecution
            },
            &result,
            timer.finish(),
        );
        result
    }
    fn run_native_settings_inner(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        f: Facts<'_, '_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let Some(BoundIntent::NativeSettings(original)) = &slot.intent else {
            return Err(InvocationError::IntentMismatch);
        };
        if !f.matches(&original.native) {
            return Err(InvocationError::IntentMismatch);
        }
        if original.context != *c
            || f.common().context != c
            || original.activation != f.common().activation
            || !f.profile_matches(&original.native)
        {
            return Err(InvocationError::Unavailable);
        }
        match slot.phase {
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running => return Err(InvocationError::Faulted),
            Phase::NativeSettingsPending(_) => {}
            _ => return Err(InvocationError::IntentMismatch),
        }
        if self.service_requested_stop(id, r)? {
            return Ok(InvocationState::Finished);
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated native settings slot");
        let Phase::NativeSettingsPending(p) = std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("native settings phase")
        };
        self.faulted = true;
        let intact = matches!(&slot.intent,Some(BoundIntent::NativeSettings(capture)) if p.intact(capture,&self.namespace));
        if !intact {
            super::super::settings_reads::observe::<()>(
                r,
                Boundary::NativeSettingsPublication,
                &Err(settings(SettingsError::Faulted)),
            );
            return Err(InvocationError::Faulted);
        }
        let decision = (|| {
            f.gate_preparation(self, c, &p.capture.native, &p.native)?;
            let current_required = read_dependency::ReadCapture::configuration_slice(
                f.required(),
                p.capture.caller.budget,
            )
            .map_err(reads)?;
            if current_required != p.capture.required {
                return Err(settings(SettingsError::Stale));
            }
            if p.bytes()? > self.settings_job_parent_limit() {
                return Err(settings(SettingsError::Limit));
            }
            let native = Arc::try_unwrap(p.native).map_err(|_| settings(SettingsError::Faulted))?;
            let stage = native.stage(self, r)?;
            let created = match (&p.capture.native, &stage) {
                (Native::Lifecycle { capture, .. }, kind::Stage::Lifecycle(stage)) => {
                    self.stage_created_layer_results(id, capture, stage, r)?
                }
                _ => None,
            };
            Ok((stage, created))
        })();
        super::super::settings_reads::observe(r, Boundary::NativeSettingsValidation, &decision);
        if decision.as_ref().err().is_some_and(|e| e.faulted()) {
            return Err(InvocationError::Faulted);
        }
        let observation = decision.as_ref().map(|_| ()).map_err(|e| *e);
        let mut created = None;
        let outcome = match decision {
            Ok((stage, result)) => {
                created = result;
                let outcome = stage.install(self);
                #[cfg(test)]
                if self.settings_publication_fault {
                    panic!("injected native settings publication interruption");
                }
                outcome
            }
            Err(e) => f.rejected(e),
        };
        self.install_invocation_terminal(id, outcome);
        self.publish_created_layer_results(id, created, r);
        self.faulted = false;
        super::super::settings_reads::observe(r, Boundary::NativeSettingsPublication, &observation);
        Ok(InvocationState::Finished)
    }
}
