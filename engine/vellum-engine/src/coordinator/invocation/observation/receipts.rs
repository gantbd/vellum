//! Admission-bound F06 typed native facets. Native release owns its original
//! grants and provenance; common observation permission never replaces them.
use super::*;
#[cfg(test)]
mod tests;
pub(super) struct NativeReceiptFrame<'a> {
    pub(super) context: &'a SettingsExecutionContextV1,
    pub(super) epoch: u64,
    pub(super) bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationReceiptKindV1 {
    Annotation,
    AtomicAnnotation,
    Reversal,
    Relation,
    Lifecycle,
    HostRegistration,
    ScopedRegistration,
    CreatedSettings,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InvocationReceiptCapabilitiesV1 {
    pub annotation: bool,
    pub atomic_annotation: bool,
    pub reversal: bool,
    pub relation: bool,
    pub lifecycle: bool,
    pub host_registration: bool,
    pub scoped_registration: bool,
    pub created_settings: bool,
}
impl InvocationReceiptCapabilitiesV1 {
    fn permits(self, kind: InvocationReceiptKindV1) -> bool {
        match kind {
            InvocationReceiptKindV1::Annotation => self.annotation,
            InvocationReceiptKindV1::AtomicAnnotation => self.atomic_annotation,
            InvocationReceiptKindV1::Reversal => self.reversal,
            InvocationReceiptKindV1::Relation => self.relation,
            InvocationReceiptKindV1::Lifecycle => self.lifecycle,
            InvocationReceiptKindV1::HostRegistration => self.host_registration,
            InvocationReceiptKindV1::ScopedRegistration => self.scoped_registration,
            InvocationReceiptKindV1::CreatedSettings => self.created_settings,
        }
    }
    /// Conservative complete native wrapper plus the original payload ceiling.
    /// This is not a JSON/wire or physical allocator bound.
    pub fn required_reply_bytes(native_maximum: usize) -> Result<usize, InvocationError> {
        native_maximum
            .checked_add(size_of::<InvocationReceiptV1<'static>>())
            .ok_or(InvocationError::Limit)
    }
}
pub struct InvocationReceiptAdmissionV1<'a> {
    pub observation: &'a TrustedInvocationPolicyFactsV1<'a>,
    pub capabilities: InvocationReceiptCapabilitiesV1,
    pub operation_receipt_admission_allowed: Option<bool>,
}
pub struct TrustedInvocationReceiptReleaseV1<'a> {
    pub observation: &'a TrustedInvocationOrdinaryObservationReleaseV1<'a>,
    pub operation_receipt_allowed: Option<bool>,
}
#[derive(Clone, Copy)]
pub enum InvocationReceiptRequestV1<'a> {
    Annotation(&'a TrustedAnnotationResultReleaseV1<'a>),
    AtomicAnnotation { index: usize },
    Reversal(&'a TrustedReversalResultReleaseV1<'a>),
    Relation(&'a TrustedRelationResultReleaseV1<'a>),
    Lifecycle,
    HostRegistration(&'a TrustedResourceRegistrationResultV1<'a>),
    ScopedRegistration(&'a TrustedScopedResourceRegistrationResultV1<'a>),
    CreatedSettings(&'a TrustedCreatedLayerResultReleaseV1<'a>),
}
impl<'a> InvocationReceiptRequestV1<'a> {
    fn kind(self) -> InvocationReceiptKindV1 {
        match self {
            Self::Annotation(_) => InvocationReceiptKindV1::Annotation,
            Self::AtomicAnnotation { .. } => InvocationReceiptKindV1::AtomicAnnotation,
            Self::Reversal(_) => InvocationReceiptKindV1::Reversal,
            Self::Relation(_) => InvocationReceiptKindV1::Relation,
            Self::Lifecycle => InvocationReceiptKindV1::Lifecycle,
            Self::HostRegistration(_) => InvocationReceiptKindV1::HostRegistration,
            Self::ScopedRegistration(_) => InvocationReceiptKindV1::ScopedRegistration,
            Self::CreatedSettings(_) => InvocationReceiptKindV1::CreatedSettings,
        }
    }
    fn context(self) -> Option<&'a SettingsExecutionContextV1> {
        match self {
            Self::Annotation(f) => Some(f.context),
            Self::Reversal(f) => Some(f.context),
            Self::Relation(f) => Some(f.context),
            Self::HostRegistration(f) => Some(f.context),
            Self::ScopedRegistration(f) => Some(f.context),
            Self::CreatedSettings(f) => Some(f.context),
            Self::AtomicAnnotation { .. } | Self::Lifecycle => None,
        }
    }
}
/// Exhaustive supported native result profile. Unsupported selectors never yield
/// placeholder values. Adding result variants requires an explicit API/version
/// decision; this type is not the complete wire/discovery catalog.
pub enum InvocationReceiptV1<'a> {
    Annotation(AnnotationResultV1),
    Reversal(ReversalResultV1),
    Relation(RelationResultV1),
    HostRegistration(ResourceRegistrationResultV1),
    ScopedRegistration(ScopedResourceRegistrationResultV1),
    CreatedSettings(&'a CreatedLayerResultV1),
}
impl InvocationReceiptV1<'_> {
    pub fn invocation(&self) -> &InvocationId {
        match self {
            Self::Annotation(v) => &v.invocation,
            Self::Reversal(v) => &v.invocation,
            Self::Relation(v) => &v.invocation,
            Self::HostRegistration(v) => &v.invocation,
            Self::ScopedRegistration(v) => &v.invocation,
            Self::CreatedSettings(v) => &v.invocation,
        }
    }
}
impl std::fmt::Debug for InvocationReceiptV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptV1(<redacted>)")
    }
}
impl SourceCoordinator {
    pub(super) fn validate_lifecycle_receipt_release(
        &self,
        observer: &mut InvocationObservationContextV1,
        frame: NativeReceiptFrame<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
    ) -> Result<(), InvocationError> {
        self.validate_native_receipt_release(
            observer,
            InvocationReceiptKindV1::Lifecycle,
            frame,
            current,
            facts,
        )
    }
    pub(super) fn validate_native_receipt_release(
        &self,
        observer: &mut InvocationObservationContextV1,
        kind: InvocationReceiptKindV1,
        frame: NativeReceiptFrame<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
    ) -> Result<(), InvocationError> {
        if observer.receipt_faulted
            || observer.receipt_ceiling.is_none_or(|c| !c.permits(kind))
            || facts.operation_receipt_allowed != Some(true)
        {
            return Err(InvocationError::Unavailable);
        }
        if let Err(e) = self.current_invocation_observation(observer, current, facts.observation) {
            if e == InvocationError::Faulted {
                observer.close();
            }
            return Err(e);
        }
        let original = observer
            .original
            .as_ref()
            .ok_or(InvocationError::Unavailable)?;
        if frame.context != &observer.context
            || frame.epoch != current.epoch
            || !(original.policy.publication && observer.ceiling.publication && current.publication)
            || !matches!(
                self.invocation_slot(&observer.invocation)?.phase,
                Phase::Terminal(Some(_))
            )
        {
            return Err(InvocationError::Unavailable);
        }
        if frame.bytes > original.limits.max_result_bytes {
            return Err(InvocationError::Limit);
        }
        Ok(())
    }
    pub fn open_invocation_observation_with_receipts_v1(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        ceiling: &InvocationObservationPolicyV1,
        limits: InvocationObservationLimitsV1,
        admission: InvocationReceiptAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationObservationContextV1, InvocationError> {
        self.open_invocation_receipts_profile(
            id,
            c,
            ceiling,
            limits,
            admission,
            ReceiptProfile::V1,
            r,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn open_invocation_receipts_profile(
        &mut self,
        id: &InvocationId,
        c: &SettingsExecutionContextV1,
        ceiling: &InvocationObservationPolicyV1,
        limits: InvocationObservationLimitsV1,
        admission: InvocationReceiptAdmissionV1<'_>,
        profile: ReceiptProfile,
        r: &mut Recorder,
    ) -> Result<InvocationObservationContextV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let caps = admission.capabilities;
            if admission.operation_receipt_admission_allowed != Some(true)
                || caps.atomic_annotation
                    && !matches!(
                        profile,
                        ReceiptProfile::V4
                            | ReceiptProfile::V5
                            | ReceiptProfile::V6
                            | ReceiptProfile::V7
                            | ReceiptProfile::V8
                            | ReceiptProfile::V9
                            | ReceiptProfile::V10
                    )
                || caps.lifecycle
                    && !matches!(
                        profile,
                        ReceiptProfile::V2
                            | ReceiptProfile::V3
                            | ReceiptProfile::V4
                            | ReceiptProfile::V5
                            | ReceiptProfile::V6
                            | ReceiptProfile::V7
                            | ReceiptProfile::V8
                            | ReceiptProfile::V9
                            | ReceiptProfile::V10
                    )
                || profile == ReceiptProfile::Unrequested
            {
                return Err(InvocationError::Unavailable);
            }
            let original = self.original_observation_capture(id, c)?;
            let maximum = original.limits.max_result_bytes;
            let slot = self.invocation_slot(id)?;
            let fixed = [
                (caps.annotation, AnnotationResultV1::required_bytes()),
                (caps.atomic_annotation, AnnotationResultV1::required_bytes()),
                (caps.reversal, ReversalResultV1::required_bytes()),
                (caps.relation, RelationResultV1::required_bytes()),
                (caps.lifecycle, size_of::<RetainedDisposalResultV1>()),
                (
                    caps.lifecycle
                        && matches!(
                            profile,
                            ReceiptProfile::V3
                                | ReceiptProfile::V4
                                | ReceiptProfile::V5
                                | ReceiptProfile::V6
                                | ReceiptProfile::V7
                                | ReceiptProfile::V8
                                | ReceiptProfile::V9
                                | ReceiptProfile::V10
                        ),
                    size_of::<RetainedReleaseResultV1>(),
                ),
                (
                    caps.lifecycle
                        && matches!(
                            profile,
                            ReceiptProfile::V5
                                | ReceiptProfile::V6
                                | ReceiptProfile::V7
                                | ReceiptProfile::V8
                                | ReceiptProfile::V9
                                | ReceiptProfile::V10
                        ),
                    size_of::<LiveRelationResultV1>(),
                ),
                (
                    caps.lifecycle
                        && matches!(
                            profile,
                            ReceiptProfile::V6
                                | ReceiptProfile::V7
                                | ReceiptProfile::V8
                                | ReceiptProfile::V9
                                | ReceiptProfile::V10
                        ),
                    size_of::<ScratchResultV1>(),
                ),
                (
                    caps.lifecycle
                        && matches!(
                            profile,
                            ReceiptProfile::V7
                                | ReceiptProfile::V8
                                | ReceiptProfile::V9
                                | ReceiptProfile::V10
                        ),
                    size_of::<SourceRebindResultV1>(),
                ),
                (
                    caps.lifecycle
                        && matches!(
                            profile,
                            ReceiptProfile::V8 | ReceiptProfile::V9 | ReceiptProfile::V10
                        ),
                    size_of::<SourceLoadResultV1>(),
                ),
                (
                    caps.lifecycle && matches!(profile, ReceiptProfile::V9 | ReceiptProfile::V10),
                    size_of::<AnnotationLoadResultV1>(),
                ),
                (
                    caps.lifecycle && profile == ReceiptProfile::V10,
                    size_of::<SourceSaveAsResultV1>(),
                ),
            ];
            if fixed.into_iter().any(|(enabled, n)| enabled && n > maximum)
                || registration::original_receipt_maximum(slot).is_some_and(|(scoped, n)| {
                    (if scoped {
                        caps.scoped_registration
                    } else {
                        caps.host_registration
                    }) && n > maximum
                })
                || (caps.created_settings
                    && created_results::original_reply_maximum(slot).is_some_and(|n| n > maximum))
                || (match profile {
                    ReceiptProfile::V1 => {
                        InvocationReceiptCapabilitiesV1::required_reply_bytes(maximum)?
                    }
                    ReceiptProfile::V2 => InvocationReceiptV2::required_reply_bytes(maximum)?,
                    ReceiptProfile::V3 => InvocationReceiptV3::required_reply_bytes(maximum)?,
                    ReceiptProfile::V4 => InvocationReceiptV4::required_reply_bytes(maximum)?,
                    ReceiptProfile::V5 => InvocationReceiptV5::required_reply_bytes(maximum)?,
                    ReceiptProfile::V6 => InvocationReceiptV6::required_reply_bytes(maximum)?,
                    ReceiptProfile::V7 => InvocationReceiptV7::required_reply_bytes(maximum)?,
                    ReceiptProfile::V8 => InvocationReceiptV8::required_reply_bytes(maximum)?,
                    ReceiptProfile::V9 => InvocationReceiptV9::required_reply_bytes(maximum)?,
                    ReceiptProfile::V10 => InvocationReceiptV10::required_reply_bytes(maximum)?,
                    ReceiptProfile::Unrequested => return Err(InvocationError::Unavailable),
                }) > limits.max_reply_bytes
            {
                return Err(InvocationError::Limit);
            }
            let mut observer = self.open_invocation_observation_v1(
                id,
                c,
                ceiling,
                limits,
                admission.observation,
                r,
            )?;
            // Still local/unexposed: no post-admission setter or retrofit route.
            observer.receipt_ceiling = Some(caps);
            observer.receipt_profile = profile;
            Ok(observer)
        })();
        capture_timed(
            r,
            Boundary::InvocationReceiptAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn query_invocation_receipt_v1<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV1<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV1<'a>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = if observer.receipt_profile != ReceiptProfile::V1 {
            Err(InvocationError::Unavailable)
        } else {
            self.query_invocation_receipt_native(observer, request, current, facts, r)
        };
        capture_timed(
            r,
            Boundary::InvocationReceiptRelease,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn query_invocation_receipt_native<'a>(
        &'a self,
        observer: &mut InvocationObservationContextV1,
        request: InvocationReceiptRequestV1<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV1<'a>, InvocationError> {
        let mut result = (|| {
            let caps = observer
                .receipt_ceiling
                .ok_or(InvocationError::Unavailable)?;
            if observer.receipt_faulted
                || !caps.permits(request.kind())
                || facts.operation_receipt_allowed != Some(true)
            {
                return Err(InvocationError::Unavailable);
            }
            if let Err(e) =
                self.current_invocation_observation(observer, current, facts.observation)
            {
                if e == InvocationError::Faulted {
                    observer.close();
                }
                return Err(e);
            }
            let original = observer
                .original
                .as_ref()
                .ok_or(InvocationError::Unavailable)?;
            if request.context() != Some(&observer.context)
                || !(original.policy.publication
                    && observer.ceiling.publication
                    && current.publication)
                || !matches!(
                    self.invocation_slot(&observer.invocation)?.phase,
                    Phase::Terminal(Some(_))
                )
            {
                return Err(InvocationError::Unavailable);
            }
            let maximum = original.limits.max_result_bytes;
            let id = &observer.invocation;
            let value = match request {
                InvocationReceiptRequestV1::Annotation(f) => {
                    if f.publication_epoch != current.epoch {
                        return Err(InvocationError::Unavailable);
                    }
                    let f = TrustedAnnotationResultReleaseV1 {
                        context: f.context,
                        publication_epoch: f.publication_epoch,
                        publication_allowed: f.publication_allowed,
                        outcome_readable: f.outcome_readable,
                        result_context_current: f.result_context_current,
                        reply_ready: f.reply_ready,
                        resources_ready: f.resources_ready,
                        max_result_bytes: f.max_result_bytes.min(maximum),
                        grant: f.grant,
                    };
                    InvocationReceiptV1::Annotation(self.retained_annotation_result_v1(id, &f, r)?)
                }
                InvocationReceiptRequestV1::Reversal(f) => {
                    if f.publication_epoch != current.epoch {
                        return Err(InvocationError::Unavailable);
                    }
                    let f = TrustedReversalResultReleaseV1 {
                        context: f.context,
                        publication_epoch: f.publication_epoch,
                        publication_allowed: f.publication_allowed,
                        outcome_readable: f.outcome_readable,
                        result_context_current: f.result_context_current,
                        reply_ready: f.reply_ready,
                        resources_ready: f.resources_ready,
                        max_result_bytes: f.max_result_bytes.min(maximum),
                        target_grant: f.target_grant,
                    };
                    InvocationReceiptV1::Reversal(self.retained_reversal_result_v1(id, &f, r)?)
                }
                InvocationReceiptRequestV1::Relation(f) => {
                    let f = TrustedRelationResultReleaseV1 {
                        context: f.context,
                        max_result_bytes: f.max_result_bytes.min(maximum),
                        outcome_readable: f.outcome_readable,
                        relation_result_readable: f.relation_result_readable,
                        relation_result_context_current: f.relation_result_context_current,
                        historical_target_readable: f.historical_target_readable,
                        reply_ready: f.reply_ready,
                        resources_ready: f.resources_ready,
                    };
                    InvocationReceiptV1::Relation(self.retained_relation_result_v1(id, &f, r)?)
                }
                InvocationReceiptRequestV1::HostRegistration(f) => {
                    let f = TrustedResourceRegistrationResultV1 {
                        context: f.context,
                        max_result_bytes: f.max_result_bytes.min(maximum),
                        outcome_readable: f.outcome_readable,
                        registration_result_readable: f.registration_result_readable,
                        registration_result_context_current: f.registration_result_context_current,
                        host_result_allowed: f.host_result_allowed,
                        reply_ready: f.reply_ready,
                        resources_ready: f.resources_ready,
                    };
                    InvocationReceiptV1::HostRegistration(
                        self.retained_resource_registration_result_v1(id, &f, r)?,
                    )
                }
                InvocationReceiptRequestV1::ScopedRegistration(f) => {
                    let f = TrustedScopedResourceRegistrationResultV1 {
                        context: f.context,
                        scope: f.scope,
                        basis: f.basis,
                        max_result_bytes: f.max_result_bytes.min(maximum),
                        interpretation_current: f.interpretation_current,
                        basis_scope_current: f.basis_scope_current,
                        outcome_readable: f.outcome_readable,
                        scoped_result_readable: f.scoped_result_readable,
                        scoped_result_context_current: f.scoped_result_context_current,
                        scoped_result_allowed: f.scoped_result_allowed,
                        reply_ready: f.reply_ready,
                        resources_ready: f.resources_ready,
                    };
                    InvocationReceiptV1::ScopedRegistration(
                        self.retained_scoped_resource_registration_result_v1(id, &f, r)?,
                    )
                }
                InvocationReceiptRequestV1::CreatedSettings(f) => {
                    let value = self.created_layer_results(id, f, r)?;
                    let bytes = value
                        .created
                        .len()
                        .checked_mul(size_of::<CreatedSettingLayerResultV1>())
                        .and_then(|n| n.checked_add(size_of::<CreatedLayerResultV1>()))
                        .ok_or(InvocationError::Limit)?;
                    if bytes > maximum {
                        return Err(InvocationError::Limit);
                    }
                    InvocationReceiptV1::CreatedSettings(value)
                }
                InvocationReceiptRequestV1::AtomicAnnotation { .. }
                | InvocationReceiptRequestV1::Lifecycle => {
                    return Err(InvocationError::Unavailable);
                }
            };
            Ok(value)
        })();
        if result
            .as_ref()
            .is_ok_and(|v| v.invocation() != &observer.invocation)
        {
            result = Err(InvocationError::Faulted);
        }
        if matches!(result, Err(InvocationError::Faulted)) {
            observer.receipt_faulted = true;
        }
        result
    }
}
