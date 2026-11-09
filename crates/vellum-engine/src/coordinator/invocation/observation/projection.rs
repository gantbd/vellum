//! Coarse authoritative local-command facts. No instrumentation state, raw
//! errors, participant identities, content, correlation or recovery authority.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationPublicationV1 {
    Applied,
    Unchanged,
    NotApplied,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationControlDispositionV1 {
    Done,
    Rejected,
    Stopped,
    Faulted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationLocalEffectV1 {
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationOutcomeReasonV1 {
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationRecoveryV1 {
    Inspect,
    Reconcile,
}
/// Fixed-size native metadata-only projection, not the public wire product.
/// Details remain withheld: there are no participant, count or correlation values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationOutcomeProjectionV1 {
    pub publication: Option<InvocationPublicationV1>,
    pub control: Option<InvocationControlDispositionV1>,
    pub effect: Option<InvocationLocalEffectV1>,
    pub reason: InvocationOutcomeReasonV1,
    pub recovery: Option<InvocationRecoveryV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::coordinator::invocation) enum TerminalKind {
    Applied,
    Unchanged,
    Rejected,
    Stopped,
    Unknown,
}
pub(in crate::coordinator::invocation) fn classify(out: &InvocationOutcome) -> TerminalKind {
    let changed = match out {
        InvocationOutcome::ScratchCommitted(_) => true,
        InvocationOutcome::ScratchRejected(e) => {
            return if matches!(
                e,
                CoordinatorError::Faulted
                    | CoordinatorError::History(HistoryError::Faulted)
                    | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
            ) {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::LiveRelationCommitted(changed) => *changed,
        InvocationOutcome::LiveRelationRejected(e) => {
            return if e.faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::RetainedReleaseCommitted(changed)
        | InvocationOutcome::SourceRebindCommitted(changed) => *changed,
        InvocationOutcome::SourceLoadCommitted(v) => {
            v.disposition == SourceReadActivationDispositionV1::Created
        }
        InvocationOutcome::AnnotationLoadCommitted(v) => v.changed(),
        InvocationOutcome::SourceSaveAsCommitted(_, changed) => *changed,
        InvocationOutcome::RetainedReleaseRejected(e)
        | InvocationOutcome::SourceRebindRejected(e)
        | InvocationOutcome::SourceLoadRejected(e)
        | InvocationOutcome::AnnotationLoadRejected(e)
        | InvocationOutcome::SourceSaveAsRejected(e) => {
            return if *e == crate::resource::ResourceError::Faulted {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::RetainedDisposalCommitted(_) => true,
        InvocationOutcome::RetainedDisposalRejected(e) => {
            return if matches!(
                e,
                CoordinatorError::Faulted
                    | CoordinatorError::Resource(crate::resource::ResourceError::Faulted)
            ) {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::ResourceCatalogCommitted(v) => {
            v.result().status == ResourceCatalogStatusV1::Applied
        }
        InvocationOutcome::ResourceCatalogRejected(e) => {
            return if *e == crate::resource::ResourceError::Faulted {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::ScopedLifecycleCommitted(v)
        | InvocationOutcome::ScopedDefinitionCommitted(v)
        | InvocationOutcome::ScopedSettingsCommitted(v) => v.changed,
        InvocationOutcome::SerializationCommitted(v) => v.changed,
        InvocationOutcome::SettingsLayersCommitted(v) => v.changed,
        InvocationOutcome::SettingsDefinitionCommitted(v) => v.changed,
        InvocationOutcome::SettingsCommitted(v) => v.changed,
        InvocationOutcome::RelationCommitted(_) => true,
        InvocationOutcome::Committed(v) => v.operation.is_some(),
        InvocationOutcome::SelectionCommitted(v)
        | InvocationOutcome::UnicodeMovementCommitted(v) => v.changed,
        InvocationOutcome::StoppedBeforePublication => return TerminalKind::Stopped,
        InvocationOutcome::ScopedLifecycleRejected(e)
        | InvocationOutcome::ScopedDefinitionRejected(e)
        | InvocationOutcome::ScopedSettingsRejected(e)
        | InvocationOutcome::SettingsLayersRejected(e)
        | InvocationOutcome::SettingsDefinitionRejected(e)
        | InvocationOutcome::SettingsRejectedV2(e) => {
            return if e.faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::SerializationRejected(e) => {
            return if e.faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::ConfiguredLineRejected(e) => {
            return if e.faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::SettingsRejected(e) => {
            return if *e == crate::settings::SettingsError::Faulted {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::RelationRejected(e) => {
            return if e.faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::Rejected(e) => {
            return if NativePreparationError::Coordinator(*e).faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::SelectionRejected(e) => {
            return if *e == crate::selection::SelectionError::Faulted {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::LineRejected(e) => {
            return if NativePreparationError::Line(*e).faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
        InvocationOutcome::UnicodeRejected(e) => {
            return if NativePreparationError::Unicode(*e).faulted() {
                TerminalKind::Unknown
            } else {
                TerminalKind::Rejected
            };
        }
    };
    if changed {
        TerminalKind::Applied
    } else {
        TerminalKind::Unchanged
    }
}
pub(super) fn project(
    out: &InvocationOutcome,
    original: &InvocationObservationPolicyV1,
    ceiling: &InvocationObservationPolicyV1,
    current: &InvocationObservationPolicyV1,
) -> InvocationOutcomeProjectionV1 {
    use InvocationControlDispositionV1 as C;
    use InvocationPublicationV1 as P;
    let (publication, control, recovery) = match classify(out) {
        TerminalKind::Applied => (P::Applied, C::Done, InvocationRecoveryV1::Inspect),
        TerminalKind::Unchanged => (P::Unchanged, C::Done, InvocationRecoveryV1::Inspect),
        TerminalKind::Rejected => (P::NotApplied, C::Rejected, InvocationRecoveryV1::Inspect),
        TerminalKind::Stopped => (P::NotApplied, C::Stopped, InvocationRecoveryV1::Inspect),
        TerminalKind::Unknown => (P::Unknown, C::Faulted, InvocationRecoveryV1::Reconcile),
    };
    InvocationOutcomeProjectionV1 {
        publication: (original.publication && ceiling.publication && current.publication)
            .then_some(publication),
        control: (original.control && ceiling.control && current.control).then_some(control),
        effect: (original.effect && ceiling.effect && current.effect)
            .then_some(InvocationLocalEffectV1::None),
        // No supplied public causes exist in the native local-command profile.
        reason: InvocationOutcomeReasonV1::Unavailable,
        recovery: (original.recovery && ceiling.recovery && current.recovery).then_some(recovery),
    }
}
