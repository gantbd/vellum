//! Bounded source invocation state, owned by the existing local coordinator.
//! No public wire format, recipient-scope authorizer or external-effect ledger.

use super::*;
// Each private pair reuses its native original-ticket/exact-funding validator.
macro_rules! shutdown_capture_reservation {
    () => {
        impl Capture {
            pub(super) fn shutdown_reserved_bytes(
                &self,
                owner: &Arc<()>,
                id: &InvocationId,
                hold: &Option<Hold>,
                ledger: &retention::RetentionState,
            ) -> Result<usize, CoordinatorError> {
                if !self.intact(owner, id, hold, ledger) {
                    return Err(CoordinatorError::Faulted);
                }
                Ok(self.limits.max_capture_bytes)
            }
        }
    };
}
pub(in crate::coordinator) mod compound_results;
mod disposal;
mod shutdown_inventory;
pub use disposal::{
    DisposalDispositionV1, ExplicitDisposalRequestV1, PreparedExplicitDisposalV1,
    TrustedDisposalCoverageV1,
};
mod document_facade;
pub use compound_results::{
    CompoundAnnotationResultEvictionV1, CompoundAnnotationResultGrantTargetV1,
    TrustedCompoundAnnotationResultAdmissionV1, TrustedCompoundAnnotationResultGrantV1,
    TrustedCompoundAnnotationResultReleaseV1,
};
pub use document_facade::{
    NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1, NativeDocumentLifecycleAdmissionV1,
    NativeDocumentLifecycleAdmissionV2, NativeDocumentLifecycleArgumentLimitsV1,
    NativeDocumentLifecycleCommandV1, NativeDocumentLifecycleCommandV2,
    NativeDocumentLifecycleEnvelopeValueV1, NativeDocumentLifecycleRequestV1,
    NativeDocumentLifecycleServiceV1, NativeDocumentLifecycleServiceV2,
    NormalizedDocumentLifecycleArgumentsV1, normalize_native_document_lifecycle_arguments_v1,
};
mod annotation_load;
pub(in crate::coordinator) mod live_relation;
mod retained_release;
mod source_load;
mod source_save_as;
pub use annotation_load::{
    AnnotationLoadAdmissionV1, AnnotationLoadLimitsV1, AnnotationLoadOutcomeV1,
    AnnotationLoadRequestV1, AnnotationLoadResultV1, TrustedAnnotationLoadFactsV1,
    TrustedAnnotationLoadResultV1, native_annotation_load_argument_declarations_v1,
};
pub use source_save_as::{
    SourceSaveAsAdmissionV1, SourceSaveAsLimitsV1, SourceSaveAsRequestV1, SourceSaveAsResultV1,
    TrustedSourceSaveAsInvocationFactsV1, TrustedSourceSaveAsResultV1,
};
mod source_rebind;
pub use source_load::{
    SourceLoadAdmissionV1, SourceLoadLimitsV1, SourceLoadResultV1, TrustedSourceLoadFactsV1,
    TrustedSourceLoadResultV1, native_source_load_argument_declaration_v1,
};
pub use source_rebind::{
    SourceRebindAdmissionV1, SourceRebindLimitsV1, SourceRebindRequestV1, SourceRebindResultV1,
    TrustedSourceRebindInvocationFactsV1, TrustedSourceRebindResultV1,
};
pub(in crate::coordinator) mod scratch;
pub use disposal::{
    RetainedDisposalAdmissionV1, RetainedDisposalLimitsV1, RetainedDisposalResultV1,
    TrustedRetainedDisposalFactsV1, TrustedRetainedDisposalResultV1,
};
pub use live_relation::{
    LiveRelationAdmissionV1, LiveRelationErrorV1, LiveRelationKindV1, LiveRelationLimitsV1,
    LiveRelationOperationV1, LiveRelationRequestV1, LiveRelationResultV1,
    TrustedLiveRelationFactsV1, TrustedLiveRelationResultV1,
};
pub use retained_release::{
    RetainedReleaseAdmissionV1, RetainedReleaseLimitsV1, RetainedReleaseOperationV1,
    RetainedReleaseRequestV1, RetainedReleaseResultV1, TrustedRetainedReleaseFactsV1,
    TrustedRetainedReleaseResultV1,
};
pub use scratch::{
    ScratchInvocationAdmissionV1, ScratchInvocationLimitsV1, ScratchResultV1,
    TrustedScratchFactsV1, TrustedScratchResultV1,
};
pub(in crate::coordinator) mod observation;
pub use observation::InvocationReceiptEnvelopeValueV1;
pub use observation::{
    InvocationControlDispositionV1, InvocationLocalEffectV1, InvocationObservationQueryV1,
    InvocationObservationReferenceV1, InvocationObservationReplyV1, InvocationOutcomeProjectionV1,
    InvocationOutcomeReasonV1, InvocationPublicationV1, InvocationRecoveryV1,
    TrustedInvocationOrdinaryObservationReleaseV1,
};
pub use observation::{
    InvocationObservationEnvelopeValueV1, InvocationObservationReadPortV1,
    InvocationObservationReadReplyV1,
};
pub use observation::{
    InvocationReceiptAdmissionV1, InvocationReceiptCapabilitiesV1, InvocationReceiptKindV1,
    InvocationReceiptRequestV1, InvocationReceiptV1, TrustedInvocationReceiptReleaseV1,
};
pub use observation::{InvocationReceiptRequestV2, InvocationReceiptV2};
pub use observation::{InvocationReceiptRequestV3, InvocationReceiptV3};
pub use observation::{InvocationReceiptRequestV4, InvocationReceiptV4};
pub use observation::{InvocationReceiptRequestV5, InvocationReceiptV5};
pub use observation::{InvocationReceiptRequestV6, InvocationReceiptV6};
pub use observation::{InvocationReceiptRequestV7, InvocationReceiptV7};
pub use observation::{InvocationReceiptRequestV8, InvocationReceiptV8};
pub use observation::{InvocationReceiptRequestV9, InvocationReceiptV9};
pub use observation::{InvocationReceiptRequestV10, InvocationReceiptV10};
pub use observation::{
    InvocationWaitClockV1, InvocationWaitEventV1, InvocationWaitIdV1, InvocationWaitReplyV1,
    InvocationWaitRequestV1, InvocationWaitUsageV1, TrustedInvocationWaitFactsV1,
};
mod observation_policy;
pub use observation::{
    InvocationControlObservationV1, InvocationControlReferenceV1, InvocationObservationContextIdV1,
    InvocationObservationContextV1, InvocationObservationLimitsV1, InvocationObservationPhaseV1,
    TrustedInvocationObservationReleaseV1,
};
pub use observation_policy::{
    InvocationObservationPolicyControlV1, InvocationObservationPolicyLimitsV1,
    InvocationObservationPolicyUsageV1, InvocationObservationPolicyV1,
    TrustedInvocationPolicyFactsV1,
};
mod annotation_arguments;
pub(in crate::coordinator) mod annotation_scoped;
pub use annotation_arguments::{
    AnnotationAuthoringArgumentLimitsV1, AnnotationAuthoringEnvelopeValueV1,
    NormalizedAnnotationAuthoringArgumentsV1, normalize_annotation_authoring_arguments_v1,
};
pub use annotation_scoped::{
    AnnotationInvocationLimitsV1, AnnotationInvocationRetirementV1, AnnotationResultCapabilitiesV1,
    AnnotationResultDecisionV1, AnnotationResultGrantTargetV1, AnnotationResultOperationV1,
    AnnotationResultRecordV1, AnnotationResultV1, TrustedAnnotationInvocationV1,
    TrustedAnnotationResultAdmissionV1, TrustedAnnotationResultGrantV1,
    TrustedAnnotationResultReleaseV1,
};
mod configured_line;
pub use configured_line::{
    CONFIGURED_PREFIX_LINE_PROFILE_V1, ConfiguredLineArgumentLimitsV1,
    ConfiguredLineEnvelopeValueV1, NormalizedConfiguredLineArgumentsV1,
    normalize_configured_line_arguments_v1,
};
pub(in crate::coordinator) mod control;
pub(in crate::coordinator) mod measurements;
pub use control::{
    INVOCATION_CONTROL_PROFILE_V1, InvocationCancellationEnvelopeValueV1,
    InvocationCancellationIngressErrorV1, NormalizedInvocationCancellationV1,
    TrustedInvocationControlV1, normalize_invocation_cancellation_v1,
};
pub use measurements::{
    InvocationMeasurementControlV1, InvocationMeasurementLimitsV1, InvocationMeasurementLimitsV2,
    InvocationMeasurementsV1, InvocationResponsivenessSnapshotV1, InvocationResponsivenessV1,
};
mod created_results;
pub use created_results::{
    CreatedLayerResultStatusV1, CreatedLayerResultV1, CreatedSettingLayerReferenceV1,
    CreatedSettingLayerResultV1, TrustedCreatedLayerResultAdmissionV1,
    TrustedCreatedLayerResultReleaseV1,
};
mod line;
pub use line::{
    EXPLICIT_PREFIX_LINE_EDIT_PROFILE_V1, LineEditArgumentLimitsV1, LineEditEnvelopeValueV1,
    NormalizedLineEditArgumentsV1, normalize_line_edit_arguments_v1,
};
mod atomic_authoring;
mod composition;
mod exact_batch;
pub use atomic_authoring::*;
pub use exact_batch::{
    BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1, ExactBatchEnvelopeValueV1,
    NormalizedExactBatchArgumentsV1,
};
mod literal;
pub use composition::{
    CompositionEnvelopeValueV1, NormalizedCompositionArgumentsV1,
    PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1, normalize_composition_arguments_v1,
    normalize_composition_envelope_v1,
};
pub use literal::{
    EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1, LiteralEditArgumentLimitsV1,
    LiteralEditEnvelopeValueV1, NormalizedLiteralEditArgumentsV1,
    normalize_literal_edit_arguments_v1,
};
mod managed_search;
pub use managed_search::replacement::{
    NormalizedReplacementArgumentsV1, ReplacementEnvelopeValueV1,
    native_replacement_argument_declarations_v1, normalize_replacement_arguments_v1,
    normalize_replacement_envelope_v1,
};
#[cfg(test)]
mod read_tests;
mod registration;
pub use registration::{
    NormalizedResourceRegistrationArgumentsV1, ResourceRegistrationEnvelopeValueV1,
    normalize_resource_registration_envelope_v1,
};
pub use registration::{
    NormalizedScopedRegistrationArgumentsV1, ScopedRegistrationEnvelopeValueV1,
    normalize_scoped_registration_envelope_v1,
};
mod relation;
pub use registration::{
    NATIVE_RESOURCE_REGISTRATION_PROFILE_V1, ResourceRegistrationResultV1,
    RetainedResourceRegistrationOutcomeV1, TrustedResourceRegistrationInvocationV1,
    TrustedResourceRegistrationResultV1,
};
pub use registration::{
    SCOPED_RESOURCE_REGISTRATION_PROFILE_V1, ScopedResourceRegistrationChangeV1,
    ScopedResourceRegistrationResultV1, ScopedResourceRegistrationRowV1,
    TrustedScopedResourceRegistrationInvocationV1, TrustedScopedResourceRegistrationResultV1,
};
pub use relation::{
    NATIVE_RELATION_LIFECYCLE_PROFILE_V1, NormalizedRelationArgumentsV1, RelationArgumentLimitsV1,
    RelationEnvelopeValueV1, native_relation_argument_declarations_v1,
    normalize_relation_arguments_v1,
};
pub use relation::{
    RelationResultV1, TrustedRelationResultAdmissionV1, TrustedRelationResultReleaseV1,
};
pub(in crate::coordinator) mod reversal;
pub use reversal::HistoryReversalTargetV1;
pub use reversal::{
    ReversalDecisionV1, ReversalResultV1, TrustedReversalResultAdmissionV1,
    TrustedReversalResultGrantV1, TrustedReversalResultReleaseV1,
};
mod scoped;
mod scoped_definition;
pub use scoped_definition::{
    NormalizedScopedDefinitionArgumentsV1, ScopedDefinitionEnvelopeValueV1,
    normalize_scoped_definition_envelope_v1,
};
mod scoped_lifecycle;
pub use scoped_lifecycle::{
    NormalizedScopedLifecycleArgumentsV1, ScopedLifecycleEnvelopeValueV1,
    normalize_scoped_lifecycle_envelope_v1,
};
mod native_settings;
pub use native_settings::NativeSettingsArgumentDeclarationV1;
mod selection;
pub use selection::{
    NormalizedSelectionArgumentsV1, SelectionArgumentLimitsV1, SelectionEnvelopeValueV1,
    native_selection_argument_declarations_v1, normalize_selection_arguments_v1,
};
mod serialization;
mod settings;
mod settings_definition;
mod settings_layers;
mod settings_reads;
pub use native_settings::TrustedNativeSettingsInvocationV1;
pub use native_settings::{
    NativeDefinitionEnvelopeValueV1, NormalizedNativeDefinitionArgumentsV1,
    normalize_native_definition_envelope_v1,
};
pub use native_settings::{
    NativeLifecycleEnvelopeValueV1, NormalizedNativeLifecycleArgumentsV1,
    normalize_native_lifecycle_envelope_v1,
};
pub use native_settings::{
    NativeSettingsDefinitionRequestV1, TrustedNativeSettingsDefinitionInvocationV1,
};
pub use native_settings::{
    NativeSettingsEnvelopeValueV1, NormalizedNativeSettingsArgumentsV1,
    normalize_native_settings_envelope_v1,
};
pub use native_settings::{
    NativeSettingsLifecycleRequestV1, TrustedNativeSettingsLifecycleInvocationV1,
};
mod settings_scoped;
pub use settings_scoped::{
    NormalizedScopedSettingsArgumentsV1, ScopedSettingsEnvelopeValueV1,
    normalize_scoped_settings_envelope_v1,
};
mod unicode;
pub use unicode::UnicodeOperationV1;
pub use unicode::{
    BoundaryEditArgumentLimitsV1, BoundaryEditEnvelopeValueV1, NormalizedBoundaryEditArgumentsV1,
    UNICODE_LOGICAL_EDIT_PROFILE_V1, normalize_boundary_edit_arguments_v1,
};

#[derive(Clone, Copy, Debug)]
pub struct InvocationLimits {
    pub max_lifetime_issued: usize,
    /// Includes issued, pending, terminal and forgotten slots until retirement.
    pub max_retained: usize,
    /// Logical descriptor storage: Rust value headers plus string payload bytes.
    /// This is not measured allocator overhead or physical peak memory.
    pub max_intent_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_inventory_search_result_remains_owned_after_session_close_but_not_after_actual_drop()
     {
        use crate::coordinator::shutdown::tests::inventory_owner;
        let (mut o, _, session, result, mut r) = occurrence_owner(8);
        let document = result.version().document.clone();
        let row = inventory_owner(
            &o,
            &mut r,
            ShutdownOwnerRoleV1::SearchSessions,
            &[&document],
        );
        assert_eq!(row.retained_records, Some(2));
        o.close_search(&session, &mut r).unwrap();
        let row = inventory_owner(
            &o,
            &mut r,
            ShutdownOwnerRoleV1::SearchSessions,
            &[&document],
        );
        assert_eq!(row.retained_records, Some(1));
        drop(result);
        let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::SearchSessions, &[]);
        assert_eq!(row.state, ShutdownOwnerStateV1::Active);
        assert_eq!(row.retained_records, Some(0));
    }
    use crate::attachment::Affinity;
    use crate::coordinates::BytePosition;
    use crate::selection::{Selection, SelectionBudget, SelectionError};

    fn occurrence_owner(
        max_matches: usize,
    ) -> (
        SourceCoordinator,
        LogicalViewContext,
        SearchSession,
        SearchResult,
        Recorder,
    ) {
        let mut owner = SourceCoordinator::configured(CoordinatorConfig {
            limits: CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 8192,
                source: Limits {
                    max_source_bytes: 4096,
                    max_edits: 128,
                    max_edit_payload_bytes: 8192,
                },
            },
            invocation: Some(InvocationLimits {
                max_lifetime_issued: 16,
                max_retained: 16,
                max_intent_bytes: 16384,
            }),
            history: Some(HistoryLimits {
                max_retained_groups: 16,
                max_journal_bytes: 65536,
                max_pins: 8,
            }),
            resources: None,
            annotations: None,
            search: Some(SearchLimits {
                max_sessions_issued: 8,
                max_live_sessions: 4,
                max_live_results: 8,
                max_root_bytes: 4096,
                max_scope_ranges: 16,
                max_query_bytes: 1024,
                max_comparisons: 65536,
                max_matches,
                max_match_bytes: 4096,
                max_replacement_bytes: 8192,
            }),
        });
        let mut r = Recorder::disabled();
        owner
            .configure_logical_views(
                LogicalViewLimits {
                    max_live_views: 8,
                    max_issued_views: 8,
                    max_total_selections: 2,
                    normalization: SelectionBudget {
                        max_input_selections: 16,
                        max_output_selections: 16,
                        max_work: 65536,
                    },
                    max_mapping_steps: 65536,
                },
                &mut r,
            )
            .unwrap();
        let source = owner.create(b"aa aa", &mut r).unwrap();
        let view = owner.create_logical_view(&mut r).unwrap();
        let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
        owner
            .attach_logical_view(&view, stamp, &source, &[point(4)], 0, &mut r)
            .unwrap();
        let context = owner
            .logical_view(&view, &mut r)
            .unwrap()
            .context()
            .unwrap();
        let session = owner.open_search(&source.document, &mut r).unwrap();
        let result = owner
            .search(
                &session,
                &source,
                &SearchRequest {
                    query: "aa".into(),
                    scope: SearchScope::Document,
                },
                &mut r,
            )
            .unwrap();
        (owner, context, session, result, r)
    }
    fn occurrence_command(
        occurrence: SearchMatch,
        mode: SearchSelectionModeV1,
    ) -> SelectionCommandV1 {
        SelectionCommandV1::SearchMatch {
            occurrence,
            direction: SelectionDirectionV1::Forward,
            affinity: Affinity::After,
            mode,
        }
    }
    #[test]
    fn occurrence_descriptor_charges_the_owned_result_not_only_the_visible_proxy() {
        let (mut owner, context, _, result, mut r) = occurrence_owner(128);
        let command = occurrence_command(
            result.match_id(0).unwrap(),
            SearchSelectionModeV1::ReplaceAll,
        );
        let bytes = command.retained_bytes().unwrap() + std::mem::size_of::<LogicalViewContext>();
        assert!(
            bytes
                > std::mem::size_of::<SelectionCommandV1>()
                    + std::mem::size_of::<LogicalViewContext>()
        );
        let id = owner.issue_invocation(&mut r).unwrap();
        owner.invocations.limits.max_intent_bytes = bytes - 1;
        assert_eq!(
            owner.submit_selection_command(&id, &context, &command, &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(
            owner.invocation_state(&id, &mut r),
            Ok(InvocationState::Issued)
        );
        owner.invocations.limits.max_intent_bytes = bytes;
        owner
            .submit_selection_command(&id, &context, &command, &mut r)
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[Selection {
                anchor: BytePosition(0),
                head: BytePosition(2),
                affinity: Affinity::After
            }]
        );
    }
    #[test]
    fn occurrence_add_capacity_refusal_has_no_selection_prefix_or_history() {
        let (mut owner, context, _, result, mut r) = occurrence_owner(128);
        // An unrelated peer can consume the shared capacity after preparation.
        let id = owner.issue_invocation(&mut r).unwrap();
        let command = occurrence_command(
            result.match_id(0).unwrap(),
            SearchSelectionModeV1::AddPrimary,
        );
        owner
            .submit_selection_command(&id, &context, &command, &mut r)
            .unwrap();
        let peer = owner.create_logical_view(&mut r).unwrap();
        let stamp = owner.logical_view(&peer, &mut r).unwrap().stamp;
        owner
            .attach_logical_view(&peer, stamp, &context.source, &[point(0)], 0, &mut r)
            .unwrap();
        let generation = owner.generation();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::SelectionRejected(SelectionError::Limit))
        );
        assert_eq!(owner.generation(), generation);
        assert_eq!(owner.history_usage().retained_groups, 0);
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[point(4)]
        );
    }
    #[test]
    fn partial_current_replacement_still_revalidates_original_query_at_publication() {
        let (mut owner, context, session, result, mut r) = occurrence_owner(1);
        let plan = owner
            .prepare_search_replace(
                &SearchSelection::Current(result.match_id(0).unwrap()),
                "X",
                &mut r,
            )
            .unwrap();
        owner
            .search(
                &session,
                &context.source,
                &SearchRequest {
                    query: "aa".into(),
                    scope: SearchScope::Document,
                },
                &mut r,
            )
            .unwrap();
        let generation = owner.generation();
        assert_eq!(
            owner.commit(plan, &mut r),
            Err(CoordinatorError::Search(SearchError::Superseded))
        );
        assert_eq!(owner.generation(), generation);
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            "aa aa"
        );
    }

    fn unicode_owner(
        activate: bool,
    ) -> (
        SourceCoordinator,
        LogicalViewContext,
        BoundaryEvidenceId,
        Recorder,
    ) {
        let (mut owner, context, mut r) = selection_owner();
        owner
            .configure_boundaries(
                BoundaryLimits {
                    max_issued: 8,
                    max_retained: 4,
                    max_source_bytes: 1024,
                    max_boundaries: 64,
                    max_words: 8,
                    max_evidence_bytes: 4096,
                    max_retained_bytes: 8192,
                },
                &mut r,
            )
            .unwrap();
        if activate {
            owner
                .configure_unicode_commands(unicode_budget(), &mut r)
                .unwrap();
        }
        let capture = owner
            .begin_boundary_capture(&context.source, &mut r)
            .unwrap();
        let table = BoundaryTableV1 {
            profile: BoundaryProfileV1::Unicode18Uax29Revision49,
            graphemes: vec![0, 1, 2, 3].into_iter().map(BytePosition).collect(),
            words: vec![BytePosition(0), BytePosition(3)],
            significant_words: vec![ByteRange { start: 0, end: 3 }],
        };
        let evidence = owner
            .accept_boundary_evidence(&capture, &table, &mut r)
            .unwrap();
        (owner, context, evidence, r)
    }
    fn unicode_budget() -> UnicodeCommandBudget {
        UnicodeCommandBudget {
            max_steps: 64,
            max_derived_bytes: 4096,
        }
    }
    fn unicode_request() -> UnicodeCommandV1 {
        UnicodeCommandV1 {
            unit: UnicodeUnitV1::Grapheme,
            direction: UnicodeDirectionV1::Forward,
            count: 1,
            extend: false,
        }
    }
    #[test]
    fn complete_unicode_descriptor_is_bounded_before_binding_at_the_exact_native_header() {
        let (mut owner, context, evidence, mut r) = unicode_owner(true);
        let id = owner.issue_invocation(&mut r).unwrap();
        let size = std::mem::size_of::<unicode::UnicodeIntent>();
        owner.invocations.limits.max_intent_bytes = size - 1;
        assert_eq!(
            owner.submit_unicode_command(
                &id,
                &context,
                &evidence,
                UnicodeOperationV1::Movement,
                unicode_request(),
                &mut r
            ),
            Err(InvocationError::Limit)
        );
        assert_eq!(
            owner.invocation_state(&id, &mut r),
            Ok(InvocationState::Issued)
        );
        owner.invocations.limits.max_intent_bytes = size;
        owner
            .submit_unicode_command(
                &id,
                &context,
                &evidence,
                UnicodeOperationV1::Movement,
                unicode_request(),
                &mut r,
            )
            .unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[point(2)]
        );
    }
    #[test]
    fn absent_unicode_work_profile_is_a_retained_refusal_not_a_future_preparation() {
        for operation in [UnicodeOperationV1::Movement, UnicodeOperationV1::Deletion] {
            let (mut owner, context, evidence, mut r) = unicode_owner(false);
            let id = owner.issue_invocation(&mut r).unwrap();
            owner
                .submit_unicode_command(
                    &id,
                    &context,
                    &evidence,
                    operation,
                    unicode_request(),
                    &mut r,
                )
                .unwrap();
            owner
                .configure_unicode_commands(unicode_budget(), &mut r)
                .unwrap();
            owner.run(&id, &mut r).unwrap();
            assert_eq!(
                owner.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::UnicodeRejected(
                    UnicodeCommandError::Selection(SelectionError::Unavailable)
                ))
            );
            assert_eq!(
                owner.submit_unicode_command(
                    &id,
                    &context,
                    &evidence,
                    operation,
                    unicode_request(),
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(
                owner
                    .logical_view(&context.view, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[point(1)]
            );
        }
    }
    #[test]
    fn unicode_original_capture_error_kind_operation_and_work_profile_drift_are_unknown() {
        for mode in 0..11 {
            let (mut owner, context, evidence, mut r) = unicode_owner(true);
            let id = owner.issue_invocation(&mut r).unwrap();
            let operation = if matches!(mode, 3..=6 | 8..=10) {
                UnicodeOperationV1::Deletion
            } else {
                UnicodeOperationV1::Movement
            };
            owner
                .submit_unicode_command(
                    &id,
                    &context,
                    &evidence,
                    operation,
                    unicode_request(),
                    &mut r,
                )
                .unwrap();
            let slot = owner.invocations.slots[id.slot].as_mut().unwrap();
            match mode {
                0 | 2 | 6 | 7 => {
                    let Some(BoundIntent::Unicode(intent)) = &mut slot.intent else {
                        panic!("not unicode")
                    };
                    match mode {
                        0 => intent.request.count += 1,
                        7 => intent.budget.as_mut().unwrap().max_steps += 1,
                        _ => {
                            intent.operation = if operation == UnicodeOperationV1::Movement {
                                UnicodeOperationV1::Deletion
                            } else {
                                UnicodeOperationV1::Movement
                            }
                        }
                    }
                }
                1 => {
                    let Phase::UnicodeMovementPending(prepared) = &mut slot.phase else {
                        panic!("not motion")
                    };
                    *prepared = Err(UnicodeCommandError::Faulted);
                }
                9 => {
                    slot.intent = Some(BoundIntent::Selection {
                        context: context.clone(),
                        command: SelectionCommandV1::Set {
                            selections: vec![point(1)],
                            primary: 0,
                        },
                    })
                }
                10 => {
                    slot.intent = Some(BoundIntent::Source(SourceIntent {
                        writes: vec![],
                        reads: vec![],
                    }))
                }
                _ => {
                    let Phase::Pending { prepared, .. } = &mut slot.phase else {
                        panic!("not deletion")
                    };
                    match mode {
                        3 => prepared.as_mut().unwrap().derived_edit.clear(),
                        4 => {
                            *prepared = Err(NativePreparationError::Unicode(
                                UnicodeCommandError::Boundary(BoundaryError::Faulted),
                            ))
                        }
                        8 => prepared.as_mut().unwrap().derived_kind = None,
                        _ => {
                            *prepared = Err(NativePreparationError::Coordinator(
                                CoordinatorError::Unavailable,
                            ))
                        }
                    }
                }
            }
            let generation = owner.generation();
            assert_eq!(owner.run(&id, &mut r), Err(InvocationError::Faulted));
            assert!(owner.is_faulted());
            assert_eq!(owner.generation(), generation);
            assert_eq!(owner.history_usage().retained_groups, 0);
            assert_eq!(owner.invocation_usage().faulted_executions, 1);
            assert_eq!(
                owner.invocation_outcome(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(
                owner.documents[context.source.document.slot]
                    .as_ref()
                    .unwrap()
                    .source
                    .text(),
                "abc"
            );
        }
    }

    fn line_owner() -> (SourceCoordinator, LogicalViewContext, Recorder) {
        let (mut owner, context, mut r) = selection_owner();
        owner
            .configure_line_edits(
                LineEditBudget {
                    max_lines: 8,
                    max_prefix_bytes: 1024,
                    max_indent_columns: 128,
                    max_unit_bytes: 8,
                    max_derived_bytes: 4096,
                    max_source_scan_bytes: 1024,
                },
                &mut r,
            )
            .unwrap();
        (owner, context, r)
    }

    #[test]
    fn line_intent_bounds_include_context_request_profile_and_unit_before_binding() {
        let (mut owner, context, mut r) = line_owner();
        let request = LineEditV1::Indent {
            unit: "  ".into(),
            tab_width: 4,
        };
        let bytes = std::mem::size_of::<line::LineIntent>() + 2;
        owner.invocations.limits.max_intent_bytes = bytes - 1;
        let id = owner.issue_invocation(&mut r).unwrap();
        assert_eq!(
            owner.submit_line_edit(&id, &context, &request, &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(
            owner.invocation_state(&id, &mut r),
            Ok(InvocationState::Issued)
        );
        owner.invocations.limits.max_intent_bytes = bytes;
        assert_eq!(
            owner.submit_line_edit(&id, &context, &request, &mut r),
            Ok(InvocationState::Pending)
        );
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner
                .read(&context.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            "  abc"
        );
        let budget = owner.line_edit_profile().unwrap();
        let (mut owner, context, mut r) = selection_owner();
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_line_edit(&id, &context, &request, &mut r)
            .unwrap();
        owner.configure_line_edits(budget, &mut r).unwrap();
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::LineRejected(
                LineEditError::Coordinator(CoordinatorError::Unavailable)
            ))
        );
        assert_eq!(
            owner.submit_line_edit(&id, &context, &request, &mut r),
            Ok(InvocationState::Finished)
        );
    }

    #[test]
    fn missing_line_capture_changed_profile_and_faulted_or_wrong_kind_preparation_remain_unknown() {
        for mode in 0..4 {
            let (mut owner, context, mut r) = line_owner();
            let request = LineEditV1::Newline {
                delimiter: GeneratedNewlineV1::Lf,
                indent: NewlineIndentV1::None,
            };
            let id = owner.issue_invocation(&mut r).unwrap();
            owner
                .submit_line_edit(&id, &context, &request, &mut r)
                .unwrap();
            let slot = owner.invocations.slots[id.slot].as_mut().unwrap();
            let Phase::Pending { prepared, .. } = &mut slot.phase else {
                panic!("not pending")
            };
            match mode {
                0 => prepared.as_mut().unwrap().derived_edit.clear(),
                1 => {
                    let Some(BoundIntent::LineEdit(intent)) = &mut slot.intent else {
                        panic!("not line")
                    };
                    intent.budget.as_mut().unwrap().max_lines += 1;
                }
                2 => {
                    *prepared = Err(NativePreparationError::Line(LineEditError::Selection(
                        SelectionError::Faulted,
                    )))
                }
                _ => {
                    *prepared = Err(NativePreparationError::Coordinator(
                        CoordinatorError::Unavailable,
                    ))
                }
            }
            let generation = owner.generation();
            assert_eq!(owner.run(&id, &mut r), Err(InvocationError::Faulted));
            assert!(owner.is_faulted());
            assert_eq!(owner.generation(), generation);
            assert_eq!(owner.invocation_usage().faulted_executions, 1);
            assert_eq!(
                owner.invocation_outcome(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(
                owner.documents[context.source.document.slot]
                    .as_ref()
                    .unwrap()
                    .source
                    .text(),
                "abc"
            );
            assert_eq!(owner.history_usage().retained_groups, 0);
        }
    }

    fn selection_owner() -> (SourceCoordinator, LogicalViewContext, Recorder) {
        let mut owner = owner();
        owner.history = history::HistoryState::new(HistoryLimits {
            max_retained_groups: 8,
            max_journal_bytes: 65536,
            max_pins: 8,
        });
        let mut r = Recorder::disabled();
        owner
            .configure_logical_views(
                LogicalViewLimits {
                    max_live_views: 4,
                    max_issued_views: 4,
                    max_total_selections: 8,
                    normalization: SelectionBudget {
                        max_input_selections: 8,
                        max_output_selections: 8,
                        max_work: 4096,
                    },
                    max_mapping_steps: 4096,
                },
                &mut r,
            )
            .unwrap();
        let source = owner.create(b"abc", &mut r).unwrap();
        let view = owner.create_logical_view(&mut r).unwrap();
        let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
        owner
            .attach_logical_view(&view, stamp, &source, &[point(1)], 0, &mut r)
            .unwrap();
        let context = owner
            .logical_view(&view, &mut r)
            .unwrap()
            .context()
            .unwrap();
        (owner, context, r)
    }
    fn point(at: usize) -> Selection {
        Selection {
            anchor: BytePosition(at),
            head: BytePosition(at),
            affinity: Affinity::After,
        }
    }
    fn set_point(at: usize) -> SelectionCommandV1 {
        SelectionCommandV1::Set {
            selections: vec![point(at)],
            primary: 0,
        }
    }

    #[test]
    fn selection_descriptor_accounts_headers_and_complete_raw_list_before_binding() {
        let (mut owner, context, mut r) = selection_owner();
        let command = set_point(2);
        let bytes = command.retained_bytes().unwrap() + std::mem::size_of::<LogicalViewContext>();
        owner.invocations.limits.max_intent_bytes = bytes - 1;
        let id = owner.issue_invocation(&mut r).unwrap();
        assert_eq!(
            owner.submit_selection_command(&id, &context, &command, &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(
            owner.invocation_state(&id, &mut r),
            Ok(InvocationState::Issued)
        );
        owner.invocations.limits.max_intent_bytes = bytes;
        assert_eq!(
            owner.submit_selection_command(&id, &context, &command, &mut r),
            Ok(InvocationState::Pending)
        );
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[point(2)]
        );
    }

    #[test]
    fn altered_selection_intent_or_derived_result_is_unknown_and_cannot_publish() {
        for change_request in [false, true] {
            let (mut owner, context, mut r) = selection_owner();
            let id = owner.issue_invocation(&mut r).unwrap();
            owner
                .submit_selection_command(&id, &context, &set_point(2), &mut r)
                .unwrap();
            let slot = owner.invocations.slots[id.slot].as_mut().unwrap();
            if change_request {
                let Some(BoundIntent::Selection { command, .. }) = &mut slot.intent else {
                    panic!("not selection")
                };
                *command = set_point(3);
            } else {
                let Phase::SelectionPending(Ok(prepared)) = &mut slot.phase else {
                    panic!("not selection")
                };
                prepared.corrupt_result_for_test();
            }
            let generation = owner.generation();
            assert_eq!(owner.run(&id, &mut r), Err(InvocationError::Faulted));
            assert!(owner.is_faulted());
            assert_eq!(owner.generation(), generation);
            assert_eq!(
                owner.invocation_outcome(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(owner.invocation_usage().faulted_executions, 1);
        }
    }

    #[test]
    fn selection_publication_exhaustion_refuses_change_but_retains_a_complete_noop() {
        for changed in [false, true] {
            let (mut owner, context, mut r) = selection_owner();
            owner.generation = u64::MAX;
            let id = owner.issue_invocation(&mut r).unwrap();
            owner
                .submit_selection_command(
                    &id,
                    &context,
                    &set_point(if changed { 2 } else { 1 }),
                    &mut r,
                )
                .unwrap();
            owner.run(&id, &mut r).unwrap();
            match owner.invocation_outcome(&id, &mut r).unwrap() {
                InvocationOutcome::SelectionRejected(SelectionError::VersionExhausted)
                    if changed => {}
                InvocationOutcome::SelectionCommitted(update) if !changed => {
                    assert!(!update.changed)
                }
                other => panic!("unexpected {other:?}"),
            }
            assert!(!owner.is_faulted());
            assert_eq!(owner.generation(), u64::MAX);
            assert_eq!(
                owner
                    .logical_view(&context.view, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[point(1)]
            );
        }
    }

    #[test]
    fn selection_execution_preserves_a_compatible_later_authored_comment_and_its_history() {
        let (mut owner, context, mut r) = selection_owner();
        owner.annotations.limits = Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 4096,
            max_live_snapshots: 4,
            max_snapshot_bytes: 4096,
        });
        let id = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_selection_command(&id, &context, &set_point(2), &mut r)
            .unwrap();
        let plan = owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: context.source.clone(),
                        basis: TargetBasis::Before,
                        target: crate::attachment::AttachmentTarget::Point {
                            offset: 1,
                            affinity: Affinity::After,
                            profile: crate::attachment::PointProfile::Position,
                        },
                    },
                    body: "later comment".into(),
                }],
                &mut r,
            )
            .unwrap();
        let record = owner.commit(plan, &mut r).unwrap().annotation_changes[0]
            .annotation
            .clone();
        let history = owner.history_usage();
        assert_eq!(history.retained_groups, 1);
        owner.run(&id, &mut r).unwrap();
        assert_eq!(
            owner.annotation(&record, &mut r).unwrap().body,
            "later comment"
        );
        assert_eq!(owner.history_usage(), history);
        assert_eq!(
            owner
                .logical_view(&context.view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[point(2)]
        );
    }

    #[test]
    fn owned_compound_annotation_identity_drift_is_unknown_before_any_source_installation() {
        let mut owner = owner();
        owner.limits.max_staged_bytes = 4096;
        owner.annotations.limits = Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 4096,
            max_live_snapshots: 4,
            max_snapshot_bytes: 4096,
        });
        let mut r = timed_service_recorder();
        let source = owner.create(b"A", &mut r).unwrap();
        let ticket = owner.issue_invocation(&mut r).unwrap();
        owner
            .submit_compound(
                &ticket,
                &CompoundIntent {
                    source: SourceIntent {
                        writes: vec![write(source.clone(), "A", "B")],
                        reads: vec![],
                    },
                    annotations: vec![AnnotationMutation::Create {
                        body: "PRIVATE_BODY".into(),
                        target: SourceTarget {
                            source: source.clone(),
                            target: crate::attachment::AttachmentTarget::ContentRange {
                                start: 0,
                                end: 1,
                            },
                            basis: TargetBasis::Before,
                        },
                    }],
                },
                &mut r,
            )
            .unwrap();
        let Phase::Pending {
            prepared: Ok(p), ..
        } = &mut owner.invocations.slots[ticket.slot].as_mut().unwrap().phase
        else {
            panic!("native prepared compound required")
        };
        p.annotations[0].id.namespace = Arc::new(());
        while r.pop_front().is_some() {}
        let generation = owner.generation();
        assert_eq!(owner.run(&ticket, &mut r), Err(InvocationError::Faulted));
        assert!(owner.is_faulted());
        assert_eq!(owner.generation(), generation);
        assert_eq!(
            owner.documents[source.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "A"
        );
        assert_eq!(owner.annotation_usage().retained_records, 0);
        assert_eq!(
            owner.invocation_outcome(&ticket, &mut r),
            Err(InvocationError::Faulted)
        );
        let mut seen = false;
        while let Some(event) = r.pop_front() {
            if event.boundary == Boundary::AnnotationPlanValidation {
                seen = true;
                assert_eq!(event.disposition, Disposition::Unknown);
                assert!(event.service_time.is_some());
            }
            assert_ne!(event.boundary, Boundary::Publication);
            assert_ne!(event.boundary, Boundary::AnnotationPublication);
        }
        assert!(seen);
    }

    #[test]
    fn owned_candidate_drift_keeps_invocation_unknown_instead_of_retaining_rejected_facts() {
        let mut owner = owner();
        let mut recorder = Recorder::disabled();
        let version = owner.create(b"A", &mut recorder).unwrap();
        let id = owner.issue_invocation(&mut recorder).unwrap();
        let request = SourceIntent {
            writes: vec![write(version, "A", "B")],
            reads: vec![],
        };
        owner.submit(&id, &request, &mut recorder).unwrap();
        let slot = owner.invocations.slots[id.slot].as_mut().unwrap();
        let Phase::Pending {
            prepared: Ok(prepared),
            ..
        } = &mut slot.phase
        else {
            panic!("not prepared")
        };
        prepared.candidates[0].edits[0].inserted = "different intent".into();
        assert_eq!(owner.run(&id, &mut recorder), Err(InvocationError::Faulted));
        assert!(owner.is_faulted());
        assert_eq!(
            owner.invocation_outcome(&id, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(owner.invocation_usage().faulted_executions, 1);
    }

    fn owner() -> SourceCoordinator {
        SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 128,
                source: Limits {
                    max_source_bytes: 128,
                    max_edits: 8,
                    max_edit_payload_bytes: 128,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 4,
                max_retained: 4,
                max_intent_bytes: 4096,
            },
        )
    }

    fn write(expected: DocumentVersion, from: &str, to: &str) -> DocumentEdits {
        DocumentEdits {
            expected,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: from.into(),
                inserted: to.into(),
            }],
        }
    }

    #[test]
    fn unissued_in_namespace_identity_is_not_a_fresh_submission() {
        let mut owner = owner();
        let mut recorder = Recorder::disabled();
        let version = owner.create(b"A", &mut recorder).unwrap();
        let unissued = InvocationId {
            namespace: owner.namespace.clone(),
            slot: usize::MAX,
        };
        let intent = SourceIntent {
            writes: vec![write(version, "A", "B")],
            reads: vec![],
        };
        assert_eq!(
            owner.submit(&unissued, &intent, &mut recorder),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            owner.run(&unissued, &mut recorder),
            Err(InvocationError::Unavailable)
        );
        assert!(owner.invocations.slots.is_empty());
    }

    #[test]
    fn failed_result_reservation_consumes_neither_identity_nor_retention_slot() {
        let mut owner = owner();
        let mut recorder = Recorder::disabled();
        owner.limits.max_participants = usize::MAX;
        assert_eq!(
            owner.issue_invocation(&mut recorder),
            Err(InvocationError::Capacity)
        );
        assert!(owner.invocations.slots.is_empty());
        assert_eq!(owner.invocations.retained, 0);
        owner.limits.max_participants = 2;
        let issued = owner.issue_invocation(&mut recorder).unwrap();
        assert_eq!(issued.slot, 0);
        let Phase::Issued(reserved) = &owner.invocation_slot(&issued).unwrap().phase else {
            panic!("not issued")
        };
        assert!(reserved.capacity() >= 2);
    }

    #[test]
    fn interrupted_invocation_retains_unknown_and_fences_the_owner() {
        let mut owner = owner();
        let mut recorder = Recorder::try_with_capacity(32).unwrap();
        let a = owner.create(b"A", &mut recorder).unwrap();
        let b = owner.create(b"B", &mut recorder).unwrap();
        let id = owner.issue_invocation(&mut recorder).unwrap();
        let request = SourceIntent {
            writes: vec![write(a.clone(), "A", "a"), write(b.clone(), "B", "b")],
            reads: vec![],
        };
        owner.submit(&id, &request, &mut recorder).unwrap();
        owner.publication_fault_after = Some(1);
        let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.run(&id, &mut recorder)
        }));
        assert!(interrupted.is_err());
        assert!(owner.is_faulted());
        // The injected integrity failure deliberately crosses the normal all-or-
        // none premises; partial internal state must not be reported as rejected.
        assert_eq!(
            owner.documents[a.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "a"
        );
        assert_eq!(
            owner.documents[b.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "B"
        );
        assert_eq!(
            owner.invocation_outcome(&id, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(owner.run(&id, &mut recorder), Err(InvocationError::Faulted));
        assert_eq!(
            owner.submit(&id, &request, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            owner.stop_invocation(&id, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            owner.retire_invocation(&id, &mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            owner.issue_invocation(&mut recorder),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            owner.create(b"new", &mut recorder),
            Err(CoordinatorError::Faulted)
        );
        assert_eq!(
            owner.read(&a.document, &mut recorder).err(),
            Some(CoordinatorError::Faulted)
        );
        assert_eq!(
            owner.dispose(&a.document, &mut recorder),
            Err(CoordinatorError::Faulted)
        );
        let records: Vec<_> = std::iter::from_fn(|| recorder.pop_front()).collect();
        assert!(
            records
                .iter()
                .any(|r| r.boundary == Boundary::Outcome && r.disposition == Disposition::Unknown)
        );
        assert!(!records.iter().any(|r| r.boundary == Boundary::Publication));
    }

    #[test]
    fn interrupted_direct_source_commit_also_fences_the_same_owner() {
        let mut owner = owner();
        let mut recorder = Recorder::disabled();
        let a = owner.create(b"A", &mut recorder).unwrap();
        let prepared = owner
            .prepare(&[write(a.clone(), "A", "a")], &[], &mut recorder)
            .unwrap();
        owner.publication_fault_after = Some(1);
        let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            owner.commit(prepared, &mut recorder)
        }));
        assert!(interrupted.is_err());
        assert!(owner.is_faulted());
        assert_eq!(
            owner.create(b"new", &mut recorder),
            Err(CoordinatorError::Faulted)
        );
    }
}

impl InvocationLimits {
    pub fn disabled() -> Self {
        Self {
            max_lifetime_issued: 0,
            max_retained: 0,
            max_intent_bytes: 0,
        }
    }
}

/// Identity only, never a recipient grant. No slot or namespace is exported.
#[derive(Clone)]
pub struct InvocationId {
    namespace: Arc<()>,
    slot: usize,
}

impl PartialEq for InvocationId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.namespace, &other.namespace) && self.slot == other.slot
    }
}
impl Eq for InvocationId {}
impl std::fmt::Debug for InvocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationId(<opaque>)")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceIntent {
    pub writes: Vec<DocumentEdits>,
    pub reads: Vec<DocumentVersion>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompoundIntent {
    pub source: SourceIntent,
    pub annotations: Vec<AnnotationMutation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationState {
    Issued,
    Pending,
    Finished,
}

/// Trusted-owner diagnostic gauges, not quiescence/discharge or export authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvocationUsage {
    pub issued_lifetime: usize,
    pub retained: usize,
    pub issued_unsubmitted: usize,
    pub pending: usize,
    pub finished: usize,
    pub outcomes_available: usize,
    pub faulted_executions: usize,
    pub max_lifetime_issued: usize,
    pub max_retained: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvocationOutcome {
    ScratchCommitted(DocumentVersion),
    ScratchRejected(CoordinatorError),
    LiveRelationCommitted(bool),
    LiveRelationRejected(LiveRelationErrorV1),
    RetainedReleaseCommitted(bool),
    SourceRebindCommitted(bool),
    SourceRebindRejected(crate::resource::ResourceError),
    SourceLoadCommitted(SourceReadActivationV1),
    SourceLoadRejected(crate::resource::ResourceError),
    SourceSaveAsCommitted(DocumentId, bool),
    SourceSaveAsRejected(crate::resource::ResourceError),
    AnnotationLoadCommitted(AnnotationLoadOutcomeV1),
    AnnotationLoadRejected(crate::resource::ResourceError),
    RetainedReleaseRejected(crate::resource::ResourceError),
    RetainedDisposalCommitted(LifecycleState),
    RetainedDisposalRejected(CoordinatorError),
    ResourceCatalogCommitted(RetainedResourceRegistrationOutcomeV1),
    ResourceCatalogRejected(crate::resource::ResourceError),
    ScopedLifecycleCommitted(ScopedSettingsUpdateV1),
    ScopedLifecycleRejected(SettingsChangeErrorV2),
    ScopedDefinitionCommitted(ScopedSettingsUpdateV1),
    ScopedDefinitionRejected(SettingsChangeErrorV2),
    SerializationCommitted(SourceSerializationUpdateV1),
    SerializationRejected(SourceSerializationErrorV1),
    ScopedSettingsCommitted(ScopedSettingsUpdateV1),
    ScopedSettingsRejected(SettingsChangeErrorV2),
    SettingsLayersCommitted(crate::settings::SettingsLayerOutcomeV1),
    SettingsLayersRejected(SettingsChangeErrorV2),
    SettingsDefinitionCommitted(crate::settings::SettingsDefinitionOutcomeV1),
    SettingsDefinitionRejected(SettingsChangeErrorV2),
    SettingsRejectedV2(SettingsChangeErrorV2),
    ConfiguredLineRejected(ConfiguredLineErrorV1),
    SettingsCommitted(crate::settings::SettingsUpdateV1),
    SettingsRejected(crate::settings::SettingsError),
    RelationCommitted(RelationReceiptV1),
    RelationRejected(RelationErrorV1),
    Committed(CommitOutcome),
    Rejected(CoordinatorError),
    SelectionCommitted(LogicalViewUpdate),
    SelectionRejected(crate::selection::SelectionError),
    LineRejected(LineEditError),
    UnicodeMovementCommitted(LogicalViewUpdate),
    UnicodeRejected(UnicodeCommandError),
    StoppedBeforePublication,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationError {
    Unavailable,
    OutcomeUnavailable,
    NotSubmitted,
    Pending,
    IntentMismatch,
    ConflictingParticipant,
    MalformedIntent,
    Limit,
    Capacity,
    /// Execution/publication may be uncertain. Never a known rejected outcome.
    Faulted,
}

#[derive(Debug)]
enum Phase {
    ScratchPending(Arc<scratch::Capture>),
    LiveRelationPending(Arc<live_relation::Capture>),
    RetainedReleasePending(Arc<retained_release::Capture>),
    SourceRebindPending(Arc<source_rebind::Capture>),
    SourceLoadPending(Arc<source_load::Capture>),
    SourceSaveAsPending(Arc<source_save_as::Capture>),
    AnnotationLoadPending(Arc<annotation_load::Capture>),
    RetainedDisposalPending(Arc<disposal::Capture>),
    AnnotationPending(annotation_scoped::Prepared),
    /// Capsule discharged after a pre-service owner fault, without asserting
    /// execution, nonpublication or a terminal outcome.
    AnnotationFaulted,
    NativeSettingsPending(native_settings::Prepared),
    ResourceCatalogPending(Arc<registration::Capture>),
    ManagedSearchPending(managed_search::Pending),
    ScopedLifecyclePending(PreparedScopedSettingsLifecycleV1),
    ScopedDefinitionPending(PreparedScopedSettingsDefinitionV1),
    SerializationPending(PreparedSourceSerializationV1),
    ScopedSettingsPending(PreparedScopedSettingsV1),
    SettingsLayersPending(Result<PreparedSettingsLayersV1, SettingsChangeErrorV2>),
    SettingsDefinitionPending(Result<PreparedSettingsDefinitionV1, SettingsChangeErrorV2>),
    DependentSettingsPending(Result<PreparedSettingsChangeV2, SettingsChangeErrorV2>),
    ConfiguredLinePending {
        prepared: Result<PreparedTransaction, ConfiguredLineErrorV1>,
        reserved: Vec<SourceChange>,
    },
    SettingsPending(Result<PreparedSettingsChange, crate::settings::SettingsError>),
    RelationPending(Result<super::relation::PreparedRelation, RelationErrorV1>),
    Issued(Vec<SourceChange>),
    Pending {
        prepared: Result<PreparedTransaction, NativePreparationError>,
        reserved: Vec<SourceChange>,
    },
    SelectionPending(Result<views::PreparedSelectionCommand, crate::selection::SelectionError>),
    UnicodeMovementPending(Result<PreparedUnicodeMovement, UnicodeCommandError>),
    /// Remains on unwinding; never interpreted as safe nonpublication.
    Running,
    Terminal(Option<InvocationOutcome>),
}

impl Phase {
    fn state(&self) -> Result<InvocationState, InvocationError> {
        match self {
            Self::Issued(_) => Ok(InvocationState::Issued),
            Self::ScratchPending(_)
            | Self::LiveRelationPending(_)
            | Self::RetainedReleasePending(_)
            | Self::SourceRebindPending(_)
            | Self::SourceLoadPending(_)
            | Self::SourceSaveAsPending(_)
            | Self::AnnotationLoadPending(_)
            | Self::RetainedDisposalPending(_)
            | Self::ManagedSearchPending(_)
            | Self::AnnotationPending(_)
            | Self::ResourceCatalogPending(_)
            | Self::Pending { .. }
            | Self::SelectionPending(_)
            | Self::UnicodeMovementPending(_)
            | Self::RelationPending(_)
            | Self::SettingsPending(_)
            | Self::SettingsDefinitionPending(_)
            | Self::SettingsLayersPending(_)
            | Self::ScopedSettingsPending(_)
            | Self::ScopedDefinitionPending(_)
            | Self::ScopedLifecyclePending(_)
            | Self::SerializationPending(_)
            | Self::DependentSettingsPending(_)
            | Self::NativeSettingsPending(_) => Ok(InvocationState::Pending),
            Self::ConfiguredLinePending { .. } => Ok(InvocationState::Pending),
            Self::Terminal(_) => Ok(InvocationState::Finished),
            Self::Running | Self::AnnotationFaulted => Err(InvocationError::Faulted),
        }
    }
}

#[derive(Debug)]
struct Slot {
    observation_policy: observation_policy::State,
    annotation_hold: Option<annotation_scoped::Hold>,
    compound_results: Option<compound_results::Hold>,
    disposal_hold: Option<disposal::Hold>,
    release_hold: Option<retained_release::Hold>,
    rebind_hold: Option<source_rebind::Hold>,
    load_hold: Option<source_load::Hold>,
    saveas_hold: Option<source_save_as::Hold>,
    annotation_load_hold: Option<annotation_load::Hold>,
    live_relation_hold: Option<live_relation::Hold>,
    scratch_hold: Option<scratch::Hold>,
    intent: Option<BoundIntent>,
    phase: Phase,
    created_results: created_results::State,
    stop_requested: bool,
    measurement_cohort: bool,
}

#[derive(Debug)]
enum NativePreparationError {
    Coordinator(CoordinatorError),
    Line(LineEditError),
    Unicode(UnicodeCommandError),
}
impl NativePreparationError {
    fn faulted(&self) -> bool {
        match self {
            Self::Coordinator(error) => matches!(
                error,
                CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
            ),
            Self::Line(error) => matches!(
                error,
                LineEditError::Coordinator(
                    CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted)
                ) | LineEditError::Selection(crate::selection::SelectionError::Faulted)
            ),
            Self::Unicode(error) => error.is_faulted(),
        }
    }
}

#[derive(Debug)]
enum BoundIntent {
    Scratch(Arc<scratch::Capture>),
    RetiredScratch,
    LiveRelation(Arc<live_relation::Capture>),
    RetiredLiveRelation,
    RetainedRelease(Arc<retained_release::Capture>),
    RetiredRelease,
    SourceRebind(Arc<source_rebind::Capture>),
    RetiredRebind,
    SourceLoad(Arc<source_load::Capture>),
    RetiredLoad,
    SourceSaveAs(Arc<source_save_as::Capture>),
    RetiredSaveAs,
    AnnotationLoad(Arc<annotation_load::Capture>),
    RetiredAnnotationLoad,
    RetainedDisposal(Arc<disposal::Capture>),
    RetiredDisposal,
    Annotation(Arc<annotation_scoped::Capture>),
    RetiredAnnotation,
    NativeSettings(Arc<native_settings::Capture>),
    ResourceCatalog(Arc<registration::Capture>),
    ManagedSearch(Arc<source_context::ConsumerCapture>),
    ReturnedSearchReplacement(Box<managed_search::replacement::Capture>),
    RetiredSearchReplacement,
    ScopedLifecycle(Arc<super::scoped_lifecycle::ScopedLifecycleCapture>),
    ScopedDefinition(Arc<super::scoped_definition::ScopedDefinitionCapture>),
    Serialization(Arc<super::serialization::SerializationCapture>),
    ScopedSettings(Arc<super::settings_scoped::ScopedCapture>),
    SettingsLayers(settings_layers::LayerIntent),
    SettingsDefinition(settings_definition::DefinitionIntent),
    DependentSettings(settings_reads::SettingsReadIntent),
    ConfiguredLine(configured_line::ConfiguredLineIntent),
    Settings(settings::SettingsIntent),
    Relation {
        command: RelationCommandV1,
        profile: Option<super::relation::RelationProfile>,
        result: Option<Box<relation::results::Capture>>,
    },
    Source(SourceIntent),
    ScopedSource {
        source: SourceIntent,
        scope: scope::ScopeCapture,
    },
    ScopedCompound {
        intent: CompoundIntent,
        scope: scope::ScopeCapture,
    },
    DependentCompound {
        intent: CompoundIntent,
        reads: read_dependency::ReadCapture,
        scope: Option<scope::ScopeCapture>,
    },
    Compound(CompoundIntent),
    LiteralEdit(literal::LiteralIntent),
    Composition(composition::CompositionIntent),
    ExactBatch(exact_batch::ExactBatchIntent),
    AtomicAuthoring(Arc<atomic_authoring::Capture>),
    RetiredAtomicAuthoring,
    LineEdit(line::LineIntent),
    Unicode(unicode::UnicodeIntent),
    Selection {
        context: LogicalViewContext,
        command: SelectionCommandV1,
    },
    SearchReplacement {
        selected: SearchSelection,
        replacement: String,
    },
    Reversal {
        group: HistoryGroupId,
        direction: HistoryDirection,
        result: Option<Box<reversal::results::State>>,
    },
    DependentReversal(reversal::ReversalReadIntent),
}
mod document_pins;

#[derive(Debug)]
pub(super) struct InvocationTable {
    observers: observation::registry::Registry,
    slots: Vec<Option<Slot>>,
    retained: usize,
    limits: InvocationLimits,
}

impl InvocationTable {
    pub(super) fn shutdown_execution_busy(&self) -> bool {
        self.slots
            .iter()
            .flatten()
            .any(|s| matches!(s.phase, Phase::Running | Phase::AnnotationFaulted))
    }
    pub(super) fn new(limits: InvocationLimits) -> Self {
        Self {
            observers: Default::default(),
            slots: Vec::new(),
            retained: 0,
            limits,
        }
    }
}

fn capture<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, InvocationError>) {
    capture_timed(recorder, boundary, result, None);
}

fn capture_timed<T>(
    recorder: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, InvocationError>,
    service_time: Option<std::time::Duration>,
) {
    // This classifies the returned service attempt, not the retained job outcome.
    // In particular, Finished can contain a rejected or stopped job. Producers
    // intentionally do not install an unwind-time endpoint or repeat execution.
    recorder.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(InvocationError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}

#[cfg(test)]
fn timed_service_recorder() -> Recorder {
    Recorder::try_with_capacity(256)
        .unwrap()
        .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1)
}

#[cfg(test)]
fn assert_service_markers(r: &mut Recorder, expected: &[(Boundary, Disposition)]) {
    let mut measured = Vec::new();
    while let Some(v) = r.pop_front() {
        // This helper asserts invocation service episodes, not every timed
        // boundary in fixture setup (source queries are independently timed).
        if v.service_time.is_some()
            && matches!(
                v.boundary,
                Boundary::InvocationExecution | Boundary::InvocationReplay
            )
        {
            measured.push((v.boundary, v.disposition));
        }
    }
    assert_eq!(measured, expected);
}

impl SourceCoordinator {
    fn guard_shutdown_invocation_admission(
        &self,
        id: &InvocationId,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        if self.shutdown_fence_marker().is_none() {
            return Ok(());
        }
        let result = self.invocation_slot(id).and_then(|s| {
            if s.intent.is_some() {
                return Ok(());
            }
            self.ensure_admission_unfenced().map_err(|e| match e {
                CoordinatorError::Faulted => InvocationError::Faulted,
                _ => InvocationError::Unavailable,
            })
        });
        capture(r, Boundary::ShutdownOrdinaryAdmission, &result);
        result
    }
    pub(super) fn settings_job_parent_limit(&self) -> usize {
        self.invocations.limits.max_intent_bytes
    }
    pub(super) fn catalog_read_parent_intent_limit(&self) -> usize {
        self.invocations.limits.max_intent_bytes
    }
    #[cfg(test)]
    pub(super) fn set_settings_parent_limit_for_test(&mut self, bytes: usize) {
        self.invocations.limits.max_intent_bytes = bytes;
    }
    pub fn submit_compound_with_reads_v3(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV3,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_compound_with_scope(
            id,
            intent,
            None,
            Some(read_dependency::ReadInput::V3(request)),
            r,
        )
    }
    pub fn submit_scoped_compound_with_reads_v3(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV3,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_compound_with_scope(
            id,
            intent,
            Some((scopes, budget)),
            Some(read_dependency::ReadInput::V3(request)),
            r,
        )
    }
    pub fn submit_compound(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, recorder)?;
        self.submit_compound_with_scope(id, intent, None, None, recorder)
    }

    pub fn submit_scoped_compound(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, recorder)?;
        self.submit_compound_with_scope(id, intent, Some((scopes, budget)), None, recorder)
    }

    pub fn submit_compound_with_reads(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        request: &ReadDependencyRequest,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_compound_with_scope(
            id,
            intent,
            None,
            Some(read_dependency::ReadInput::V1(request)),
            r,
        )
    }

    pub fn submit_scoped_compound_with_reads(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        request: &ReadDependencyRequest,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_compound_with_scope(
            id,
            intent,
            Some((scopes, budget)),
            Some(read_dependency::ReadInput::V1(request)),
            r,
        )
    }

    pub fn submit_compound_with_reads_v2(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV2,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_compound_with_scope(
            id,
            intent,
            None,
            Some(read_dependency::ReadInput::V2(request)),
            r,
        )
    }
    pub fn submit_scoped_compound_with_reads_v2(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        request: &ReadDependencyRequestV2,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        self.submit_compound_with_scope(
            id,
            intent,
            Some((scopes, budget)),
            Some(read_dependency::ReadInput::V2(request)),
            r,
        )
    }

    fn submit_compound_with_scope(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        scope_input: Option<(&[SourceOperationScope], SourceScopeBudget)>,
        read_input: Option<read_dependency::ReadInput<'_>>,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.submit_compound_owned(id, intent, scope_input, read_input, &mut None, recorder)
    }

    fn submit_compound_owned(
        &mut self,
        id: &InvocationId,
        intent: &CompoundIntent,
        scope_input: Option<(&[SourceOperationScope], SourceScopeBudget)>,
        read_input: Option<read_dependency::ReadInput<'_>>,
        receipt_hold: &mut Option<compound_results::Hold>,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            self.invocation_slot(id)?;
            let mut bytes = std::mem::size_of::<CompoundIntent>();
            let mut charge = |n: usize| {
                bytes = bytes
                    .checked_add(n)
                    .filter(|n| *n <= self.invocations.limits.max_intent_bytes)
                    .ok_or(InvocationError::Limit)?;
                Ok::<(), InvocationError>(())
            };
            charge(0)?;
            for write in &intent.source.writes {
                charge(std::mem::size_of::<DocumentEdits>())?;
                for edit in &write.edits {
                    charge(std::mem::size_of::<Replacement>())?;
                    charge(edit.expected.len())?;
                    charge(edit.inserted.len())?;
                }
            }
            for _ in &intent.source.reads {
                charge(std::mem::size_of::<DocumentVersion>())?;
            }
            for mutation in &intent.annotations {
                charge(std::mem::size_of::<AnnotationMutation>())?;
                if let AnnotationMutation::Create { body, .. }
                | AnnotationMutation::UpdateBody { body, .. } = mutation
                {
                    charge(body.len())?;
                }
            }
            let scope = if let Some((scopes, budget)) = scope_input {
                charge(scope::ScopeCapture::retained_bytes(scopes).ok_or(InvocationError::Limit)?)?;
                Some(
                    scope::ScopeCapture::copy(scopes, budget).map_err(|e| match e {
                        SourceScopeError::Capacity => InvocationError::Capacity,
                        _ => InvocationError::Limit,
                    })?,
                )
            } else {
                None
            };
            let reads = if let Some(request) = read_input {
                charge(
                    request
                        .retained_bytes(&intent.source.reads)
                        .ok_or(InvocationError::Limit)?,
                )?;
                Some(request.copy(&intent.source.reads).map_err(|e| match e {
                    ReadDependencyError::Capacity => InvocationError::Capacity,
                    ReadDependencyError::Limit => InvocationError::Limit,
                    ReadDependencyError::DuplicateSubject | ReadDependencyError::InvalidVersion => {
                        InvocationError::MalformedIntent
                    }
                })?)
            } else {
                None
            };
            // The new profile owns one whole canonical read conjunction. Do not
            // retain a second source-read vector or make its input location intent.
            let source = self.normalize_intent_read_profile(&intent.source, reads.is_none())?;
            let mut annotations = Vec::new();
            annotations
                .try_reserve_exact(intent.annotations.len())
                .map_err(|_| InvocationError::Capacity)?;
            let copy = |text: &str| {
                let mut owned = String::new();
                owned
                    .try_reserve_exact(text.len())
                    .map_err(|_| InvocationError::Capacity)?;
                owned.push_str(text);
                Ok::<_, InvocationError>(owned)
            };
            for mutation in &intent.annotations {
                annotations.push(match mutation {
                    AnnotationMutation::Create { target, body } => AnnotationMutation::Create {
                        target: target.clone(),
                        body: copy(body)?,
                    },
                    AnnotationMutation::UpdateBody {
                        annotation,
                        expected_content,
                        body,
                    } => AnnotationMutation::UpdateBody {
                        annotation: annotation.clone(),
                        expected_content: *expected_content,
                        body: copy(body)?,
                    },
                    AnnotationMutation::Reattach {
                        annotation,
                        expected_target,
                        target,
                    } => AnnotationMutation::Reattach {
                        annotation: annotation.clone(),
                        expected_target: *expected_target,
                        target: target.clone(),
                    },
                    AnnotationMutation::Delete {
                        annotation,
                        expected_content,
                        expected_target,
                    } => AnnotationMutation::Delete {
                        annotation: annotation.clone(),
                        expected_content: *expected_content,
                        expected_target: *expected_target,
                    },
                });
            }
            let owned = CompoundIntent {
                source,
                annotations,
            };
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                let same = match (original, &scope, &reads) {
                    (BoundIntent::Compound(captured), None, None) => captured == &owned,
                    (BoundIntent::ScopedCompound { intent, scope: old }, Some(scope), None) => {
                        intent == &owned && old == scope
                    }
                    (
                        BoundIntent::DependentCompound {
                            intent,
                            reads: old_reads,
                            scope: old_scope,
                        },
                        scope,
                        Some(reads),
                    ) => intent == &owned && old_reads == reads && old_scope == scope,
                    _ => false,
                };
                if !same {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let checked = if let Some(reads) = &reads {
                self.check_owned_read_capture(reads, recorder)
            } else {
                Ok(())
            };
            let prepared = checked
                .and_then(|()| {
                    if let Some(scope) = &scope {
                        self.prepare_scoped_compound(&owned, &scope.scopes, scope.budget, recorder)
                    } else {
                        self.prepare_compound(
                            &owned.source.writes,
                            &owned.source.reads,
                            &owned.annotations,
                            recorder,
                        )
                    }
                })
                .and_then(|mut plan| {
                    if let Some(reads) = &reads {
                        plan.reads_required = true;
                        plan.declared_reads = Some(reads.copy_owned()?);
                    }
                    Ok(plan)
                });
            if let Some(hold) = receipt_hold {
                hold.bind_plan(prepared.as_ref().ok())?;
            }
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            slot.compound_results = receipt_hold.take();
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slot is issued")
            };
            self.install_invocation_admission(
                id,
                if let Some(reads) = reads {
                    BoundIntent::DependentCompound {
                        intent: owned,
                        reads,
                        scope,
                    }
                } else if let Some(scope) = scope {
                    BoundIntent::ScopedCompound {
                        intent: owned,
                        scope,
                    }
                } else {
                    BoundIntent::Compound(owned)
                },
                Phase::Pending {
                    prepared: prepared.map_err(NativePreparationError::Coordinator),
                    reserved,
                },
            );
            Ok(InvocationState::Pending)
        })();
        capture(
            recorder,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }
    /// Retains the actual search-result/occurrence identity and literal text,
    /// including unused text on zero matches. Replay never requeries the source.
    pub fn submit_search_replace(
        &mut self,
        id: &InvocationId,
        selected: &SearchSelection,
        replacement: &str,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, recorder)?;
        let repeated = self
            .invocation_slot(id)
            .is_ok_and(|slot| slot.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            let bytes = selected
                .retained_bytes()
                .and_then(|n| n.checked_add(std::mem::size_of::<SearchSelection>()))
                .and_then(|n| n.checked_add(std::mem::size_of::<String>()))
                .and_then(|n| n.checked_add(replacement.len()))
                .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            if let Some(original) = &slot.intent {
                if !matches!(original, BoundIntent::SearchReplacement { selected: original, replacement: text }
                    if original == selected && text == replacement)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return slot.phase.state();
            }
            let mut text = String::new();
            text.try_reserve_exact(replacement.len())
                .map_err(|_| InvocationError::Capacity)?;
            text.push_str(replacement);
            let owned = BoundIntent::SearchReplacement {
                selected: selected.clone(),
                replacement: text,
            };
            let prepared = self.prepare_search_replace(selected, replacement, recorder);
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slots are issued")
            };
            self.install_invocation_admission(
                id,
                owned,
                Phase::Pending {
                    prepared: prepared.map_err(NativePreparationError::Coordinator),
                    reserved,
                },
            );
            Ok(InvocationState::Pending)
        })();
        capture(
            recorder,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }
    /// Bounded scan of local state; does not recursively emit telemetry.
    pub fn invocation_usage(&self) -> InvocationUsage {
        let mut usage = InvocationUsage {
            issued_lifetime: self.invocations.slots.len(),
            retained: self.invocations.retained,
            issued_unsubmitted: 0,
            pending: 0,
            finished: 0,
            outcomes_available: 0,
            faulted_executions: 0,
            max_lifetime_issued: self.invocations.limits.max_lifetime_issued,
            max_retained: self.invocations.limits.max_retained,
        };
        for slot in self.invocations.slots.iter().flatten() {
            match &slot.phase {
                Phase::Issued(_) => usage.issued_unsubmitted += 1,
                Phase::ScratchPending(_)
                | Phase::LiveRelationPending(_)
                | Phase::RetainedReleasePending(_)
                | Phase::SourceRebindPending(_)
                | Phase::SourceLoadPending(_)
                | Phase::SourceSaveAsPending(_)
                | Phase::AnnotationLoadPending(_)
                | Phase::RetainedDisposalPending(_)
                | Phase::ManagedSearchPending(_)
                | Phase::AnnotationPending(_)
                | Phase::Pending { .. }
                | Phase::SelectionPending(_)
                | Phase::UnicodeMovementPending(_)
                | Phase::RelationPending(_)
                | Phase::SettingsPending(_)
                | Phase::NativeSettingsPending(_)
                | Phase::SettingsDefinitionPending(_)
                | Phase::SettingsLayersPending(_)
                | Phase::ScopedSettingsPending(_)
                | Phase::SerializationPending(_)
                | Phase::DependentSettingsPending(_)
                | Phase::ConfiguredLinePending { .. } => usage.pending += 1,
                Phase::ScopedDefinitionPending(_) => usage.pending += 1,
                Phase::ScopedLifecyclePending(_) | Phase::ResourceCatalogPending(_) => {
                    usage.pending += 1
                }
                Phase::Running | Phase::AnnotationFaulted => usage.faulted_executions += 1,
                Phase::Terminal(outcome) => {
                    usage.finished += 1;
                    usage.outcomes_available += usize::from(outcome.is_some());
                }
            }
        }
        usage
    }

    /// Opt-in explicit capacities; `new()` leaves invocation issuance disabled.
    pub fn with_invocation_limits(
        limits: CoordinatorLimits,
        invocation_limits: InvocationLimits,
    ) -> Self {
        let mut owner = Self::new(limits);
        owner.invocations = InvocationTable::new(invocation_limits);
        owner
    }

    fn invocation_slot(&self, id: &InvocationId) -> Result<&Slot, InvocationError> {
        if !Arc::ptr_eq(&self.namespace, &id.namespace) {
            return Err(InvocationError::Unavailable);
        }
        self.invocations
            .slots
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(InvocationError::Unavailable)
    }

    pub fn issue_invocation(
        &mut self,
        recorder: &mut Recorder,
    ) -> Result<InvocationId, InvocationError> {
        let result = (|| {
            self.ensure_ordinary_admission().map_err(|e| match e {
                CoordinatorError::Faulted => InvocationError::Faulted,
                _ => InvocationError::Unavailable,
            })?;
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let table = &mut self.invocations;
            if table.slots.len() >= table.limits.max_lifetime_issued
                || table.retained >= table.limits.max_retained
            {
                return Err(InvocationError::Limit);
            }
            let mut reserved = Vec::new();
            reserved
                .try_reserve_exact(self.limits.max_participants)
                .map_err(|_| InvocationError::Capacity)?;
            table
                .slots
                .try_reserve(1)
                .map_err(|_| InvocationError::Capacity)?;
            let id = InvocationId {
                namespace: self.namespace.clone(),
                slot: table.slots.len(),
            };
            table.slots.push(Some(Slot {
                observation_policy: observation_policy::State::Unrequested,
                annotation_hold: None,
                compound_results: None,
                disposal_hold: None,
                release_hold: None,
                rebind_hold: None,
                load_hold: None,
                saveas_hold: None,
                annotation_load_hold: None,
                live_relation_hold: None,
                scratch_hold: None,
                intent: None,
                phase: Phase::Issued(reserved),
                created_results: created_results::State::Unrequested,
                measurement_cohort: false,
                stop_requested: false,
            }));
            table.retained += 1;
            Ok(id)
        })();
        capture(recorder, Boundary::InvocationIssuance, &result);
        result
    }

    pub fn submit(
        &mut self,
        id: &InvocationId,
        intent: &SourceIntent,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, recorder)?;
        let repeated = self
            .invocation_slot(id)
            .is_ok_and(|slot| slot.intent.is_some());
        let result = self.submit_inner(id, intent);
        capture(
            recorder,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }

    fn submit_inner(
        &mut self,
        id: &InvocationId,
        intent: &SourceIntent,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        self.invocation_slot(id)?;
        let owned = self.normalize_intent(intent)?;
        let slot = self.invocation_slot(id)?;
        if let Some(original) = &slot.intent {
            if !matches!(original, BoundIntent::Source(captured) if captured == &owned) {
                return Err(InvocationError::IntentMismatch);
            }
            return slot.phase.state();
        }
        self.ensure_admission_unfenced().map_err(|e| match e {
            CoordinatorError::Faulted => InvocationError::Faulted,
            _ => InvocationError::Unavailable,
        })?;
        // Preparation failure is a retained known refusal of this original intent,
        // not permission to reacquire a new current source on a matching retry.
        let prepared = self.prepare_inner(&owned.writes, &owned.reads);
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated issued slot");
        let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
            unreachable!("unbound slots are issued")
        };
        self.install_invocation_admission(
            id,
            BoundIntent::Source(owned),
            Phase::Pending {
                prepared: prepared.map_err(NativePreparationError::Coordinator),
                reserved,
            },
        );
        Ok(InvocationState::Pending)
    }

    /// Captures one actual history preparation. Matching retries only compare
    /// the original closed intent; they never reacquire a current frontier.
    pub fn submit_reversal(
        &mut self,
        id: &InvocationId,
        group: &HistoryGroupId,
        direction: HistoryDirection,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, recorder)?;
        let repeated = self
            .invocation_slot(id)
            .is_ok_and(|slot| slot.intent.is_some());
        let result = self.submit_reversal_inner(id, group, direction, recorder);
        capture(
            recorder,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }

    fn submit_reversal_inner(
        &mut self,
        id: &InvocationId,
        group: &HistoryGroupId,
        direction: HistoryDirection,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        let slot = self.invocation_slot(id)?;
        let bytes = reversal::results::direct_bytes();
        if bytes > self.invocations.limits.max_intent_bytes {
            return Err(InvocationError::Limit);
        }
        let owned = BoundIntent::Reversal {
            group: group.clone(),
            direction,
            result: None,
        };
        if let Some(original) = &slot.intent {
            if !matches!(original, BoundIntent::Reversal {group:old,direction:old_direction,..} if old==group && *old_direction==direction)
            {
                return Err(InvocationError::IntentMismatch);
            }
            return slot.phase.state();
        }
        let prepared = self
            .prepare_reversal(group, direction, recorder)
            .map_err(CoordinatorError::History);
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated issued slot");
        let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
            unreachable!("unbound slots are issued")
        };
        self.install_invocation_admission(
            id,
            owned,
            Phase::Pending {
                prepared: prepared.map_err(NativePreparationError::Coordinator),
                reserved,
            },
        );
        Ok(InvocationState::Pending)
    }

    /// Runs one closed synchronous native job; terminal calls only observe.
    /// Opt-in native timing covers this service attempt, not the job's queue
    /// lifetime or an external effect. A finished attempt need not have committed.
    pub fn run(
        &mut self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = recorder.begin_service_timing();
        let terminal = self
            .invocation_slot(id)
            .is_ok_and(|slot| matches!(slot.phase, Phase::Terminal(_)));
        let result = self.run_inner(id, recorder);
        capture_timed(
            recorder,
            if terminal {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationExecution
            },
            &result,
            timing.finish(),
        );
        result
    }

    fn run_inner(
        &mut self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.run_with_atomic_validation(id, None, recorder)
    }
    fn run_with_atomic_validation(
        &mut self,
        id: &InvocationId,
        atomic_validation: Option<Result<(), CoordinatorError>>,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if self.faulted {
            return Err(InvocationError::Faulted);
        }
        if self.service_requested_stop(id, recorder)? {
            return Ok(InvocationState::Finished);
        }
        if matches!(
            self.invocation_slot(id)?.intent,
            Some(BoundIntent::AtomicAuthoring(_))
        ) && matches!(self.invocation_slot(id)?.phase, Phase::Pending { .. })
            && atomic_validation.is_none()
        {
            return Err(InvocationError::Unavailable);
        }
        match &self.invocation_slot(id)?.phase {
            Phase::Issued(_) => return Err(InvocationError::NotSubmitted),
            Phase::Terminal(_) => return Ok(InvocationState::Finished),
            Phase::Running | Phase::AnnotationFaulted => return Err(InvocationError::Faulted),
            Phase::Pending { .. } => {}
            Phase::ScratchPending(_)
            | Phase::LiveRelationPending(_)
            | Phase::RetainedReleasePending(_)
            | Phase::SourceRebindPending(_)
            | Phase::SourceLoadPending(_)
            | Phase::SourceSaveAsPending(_)
            | Phase::AnnotationLoadPending(_)
            | Phase::RetainedDisposalPending(_)
            | Phase::AnnotationPending(_) => {
                return Err(InvocationError::Unavailable);
            }
            Phase::SelectionPending(_) => return self.run_selection_command(id, recorder),
            Phase::UnicodeMovementPending(_) => return self.run_unicode_movement(id, recorder),
            Phase::RelationPending(_) => return self.run_relation(id, recorder),
            Phase::SettingsPending(_) => return self.run_settings(id, recorder),
            Phase::SettingsDefinitionPending(_) => {
                return self.run_settings_definition(id, recorder);
            }
            Phase::SettingsLayersPending(_) => return self.run_settings_layers(id, recorder),
            // This family requires fresh host facts; never reuse admission facts.
            Phase::ScopedSettingsPending(_) => return Err(InvocationError::Unavailable),
            Phase::NativeSettingsPending(_) => return Err(InvocationError::Unavailable),
            Phase::ScopedDefinitionPending(_) => return Err(InvocationError::Unavailable),
            Phase::ScopedLifecyclePending(_) => return Err(InvocationError::Unavailable),
            Phase::SerializationPending(_) => return Err(InvocationError::Unavailable),
            Phase::ManagedSearchPending(_) => return Err(InvocationError::Unavailable),
            Phase::ResourceCatalogPending(_) => return Err(InvocationError::Unavailable),
            Phase::DependentSettingsPending(_) => {
                return self.run_settings_with_reads(id, recorder);
            }
            Phase::ConfiguredLinePending { .. } => return self.run_configured_line(id, recorder),
        }
        let slot = self.invocations.slots[id.slot]
            .as_mut()
            .expect("validated pending slot");
        let Phase::Pending { prepared, reserved } =
            std::mem::replace(&mut slot.phase, Phase::Running)
        else {
            unreachable!("validated pending slot")
        };
        let is_line = matches!(slot.intent, Some(BoundIntent::LineEdit(_)));
        let reversal_capture = reversal::results::capture_for_run(id, &slot.intent);
        let is_unicode = matches!(
            slot.intent,
            Some(BoundIntent::Unicode(unicode::UnicodeIntent {
                operation: UnicodeOperationV1::Deletion,
                ..
            }))
        );
        let scope_family_intact = match (&slot.intent, &prepared) {
            (
                Some(
                    BoundIntent::ScopedSource { .. }
                    | BoundIntent::ScopedCompound { .. }
                    | BoundIntent::DependentCompound { .. },
                ),
                _,
            ) => true,
            (Some(BoundIntent::ExactBatch(_)), _) => true,
            (Some(BoundIntent::AtomicAuthoring(_)), _) => true,
            (_, Ok(plan)) => !plan.scoped && plan.scopes.is_none(),
            _ => true,
        };
        let read_family_intact = match (&slot.intent, &prepared) {
            (
                Some(BoundIntent::DependentCompound { .. } | BoundIntent::DependentReversal(_)),
                _,
            ) => true,
            (Some(BoundIntent::ExactBatch(_)), _) => true,
            (Some(BoundIntent::AtomicAuthoring(_)), _) => true,
            (_, Ok(plan)) => !plan.reads_required && plan.declared_reads.is_none(),
            _ => true,
        };
        let configured_family_intact = match &prepared {
            Ok(plan) => !plan.configured_required && plan.configured_line.is_none(),
            Err(_) => true,
        };
        let intent_intact = configured_family_intact
            && scope_family_intact
            && read_family_intact
            && match (&slot.intent, &prepared) {
                (
                    Some(BoundIntent::DependentCompound {
                        intent,
                        reads,
                        scope,
                    }),
                    Ok(plan),
                ) => {
                    plan.reads_required
                        && plan.declared_reads.as_ref() == Some(reads)
                        && plan.scoped == scope.is_some()
                        && plan.scopes == *scope
                        && plan.derived_kind.is_none()
                        && plan.derived_edit.is_empty()
                        && plan.read_dependencies == intent.source.reads
                        && plan.candidates.len() == intent.source.writes.len()
                        && plan
                            .candidates
                            .iter()
                            .zip(&intent.source.writes)
                            .all(|(c, w)| c.expected == w.expected && c.edits == w.edits)
                }
                (
                    Some(BoundIntent::DependentCompound { .. }),
                    Err(NativePreparationError::Coordinator(_)),
                ) => true,
                (
                    Some(
                        BoundIntent::ScopedSource { source, scope }
                        | BoundIntent::ScopedCompound {
                            intent: CompoundIntent { source, .. },
                            scope,
                        },
                    ),
                    Ok(plan),
                ) => {
                    plan.scoped
                        && plan.scopes.as_ref() == Some(scope)
                        && plan.derived_kind.is_none()
                        && plan.derived_edit.is_empty()
                        && plan.read_dependencies == source.reads
                        && plan.candidates.len() == source.writes.len()
                        && plan
                            .candidates
                            .iter()
                            .zip(&source.writes)
                            .all(|(candidate, write)| {
                                candidate.expected == write.expected
                                    && candidate.edits == write.edits
                            })
                }
                (
                    Some(BoundIntent::ScopedSource { .. } | BoundIntent::ScopedCompound { .. }),
                    Err(NativePreparationError::Coordinator(_)),
                ) => true,
                (Some(BoundIntent::LiteralEdit(intent)), Ok(prepared)) => {
                    Self::literal_intent_matches(
                        prepared,
                        &intent.context,
                        &intent.text,
                        intent.after,
                    )
                }
                (Some(BoundIntent::Composition(intent)), Ok(prepared)) => {
                    Self::composition_intent_matches(prepared, &intent.view, &intent.replacement)
                }
                (Some(BoundIntent::Composition(_)), Err(_)) => false,
                (Some(BoundIntent::ExactBatch(intent)), Ok(plan)) => intent.matches_plan(plan),
                (Some(BoundIntent::ExactBatch(_)), Err(_)) => false,
                (Some(BoundIntent::AtomicAuthoring(c)), Ok(p)) => c.matches_plan(
                    &self.namespace,
                    id,
                    p,
                    slot.compound_results.as_ref(),
                    &self.retention,
                ),
                (Some(BoundIntent::AtomicAuthoring(_)), Err(_)) => false,
                (Some(BoundIntent::LineEdit(intent)), Ok(prepared)) => Self::line_intent_matches(
                    prepared,
                    &intent.context,
                    &intent.request,
                    intent.budget,
                ),
                (Some(BoundIntent::Unicode(intent)), Ok(prepared))
                    if intent.operation == UnicodeOperationV1::Deletion =>
                {
                    Self::unicode_deletion_intent_matches(
                        prepared,
                        &intent.context,
                        &intent.evidence,
                        intent.request,
                        intent.budget,
                    )
                }
                (Some(BoundIntent::Unicode(intent)), Err(NativePreparationError::Unicode(_)))
                    if intent.operation == UnicodeOperationV1::Deletion =>
                {
                    true
                }
                (Some(BoundIntent::Unicode(_)), _)
                | (_, Err(NativePreparationError::Unicode(_))) => false,
                (Some(BoundIntent::LineEdit(_)), Err(NativePreparationError::Line(_))) => true,
                (Some(BoundIntent::LineEdit(_)), Err(_))
                | (_, Err(NativePreparationError::Line(_))) => false,
                (
                    Some(BoundIntent::LiteralEdit(_)),
                    Err(NativePreparationError::Coordinator(_)),
                ) => true,
                (
                    Some(BoundIntent::Reversal {
                        group, direction, ..
                    }),
                    Ok(plan),
                ) => Self::reversal_intent_matches(plan, group, *direction),
                (Some(BoundIntent::DependentReversal(intent)), Ok(plan)) => {
                    plan.reads_required
                        && plan.declared_reads.as_ref() == Some(&intent.reads)
                        && Self::reversal_intent_matches(
                            plan,
                            intent.target.group(),
                            intent.direction,
                        )
                }
                (
                    Some(BoundIntent::Reversal { .. } | BoundIntent::DependentReversal(_)),
                    Err(NativePreparationError::Coordinator(_)),
                ) => true,
                (
                    Some(
                        BoundIntent::Source(_)
                        | BoundIntent::Compound(_)
                        | BoundIntent::SearchReplacement { .. },
                    ),
                    Ok(_) | Err(NativePreparationError::Coordinator(_)),
                ) => match &prepared {
                    Ok(plan) => {
                        !plan.scoped
                            && plan.scopes.is_none()
                            && plan.derived_kind.is_none()
                            && plan.derived_edit.is_empty()
                    }
                    Err(_) => true,
                },
                _ => false,
            };
        let is_exact_batch = matches!(slot.intent, Some(BoundIntent::ExactBatch(_)));
        let is_atomic_authoring = matches!(slot.intent, Some(BoundIntent::AtomicAuthoring(_)));
        self.faulted = true;
        let prepared = if is_atomic_authoring && intent_intact {
            atomic_validation
                .expect("fresh atomic service validation")
                .map_err(NativePreparationError::Coordinator)
                .and(prepared)
        } else {
            prepared
        };
        let prepared = if (is_exact_batch || is_atomic_authoring) && intent_intact {
            prepared.and_then(|mut plan| {
                // No admission-time selection predicate for explicit batches.
                // Source/private annotation/read/scope captures stay unchanged.
                plan.view_dependencies = self
                    .capture_view_dependencies(&plan.candidates)
                    .map_err(NativePreparationError::Coordinator)?;
                if is_exact_batch {
                    exact_batch::check_service_envelope(self, id, &plan)
                        .map_err(NativePreparationError::Coordinator)?;
                } else {
                    atomic_authoring::check_service_envelope(self, id, &plan)
                        .map_err(NativePreparationError::Coordinator)?;
                }
                Ok(plan)
            })
        } else {
            prepared
        };
        let mut reversal_receipt = None;
        let atomic = self.invocation_slot(id)?.compound_results.is_some();
        let mut atomic_receipts = None;
        let decision = if intent_intact && reversal_capture.is_ok() {
            prepared.and_then(|prepared| {
                if atomic {
                    self.commit_with_compound_annotation_storage(
                        prepared,
                        reserved,
                        id,
                        &mut atomic_receipts,
                        recorder,
                    )
                } else {
                    self.commit_with_reversal_storage(
                        prepared,
                        reserved,
                        reversal_capture.as_ref().ok().and_then(Option::as_ref),
                        &mut reversal_receipt,
                        recorder,
                    )
                }
                .map_err(NativePreparationError::Coordinator)
            })
        } else {
            Err(NativePreparationError::Coordinator(
                CoordinatorError::Faulted,
            ))
        };
        if decision
            .as_ref()
            .err()
            .is_some_and(NativePreparationError::faulted)
        {
            recorder.record(Observation {
                boundary: Boundary::Admission,
                disposition: Disposition::Unknown,
                service_time: None,
            });
            return Err(InvocationError::Faulted);
        }
        if let Ok(Some(original)) = &reversal_capture {
            self.finish_reversal_result(id, original, &decision, reversal_receipt)?;
        }
        if atomic && decision.is_ok() {
            let Some(receipts) = atomic_receipts else {
                return Err(InvocationError::Faulted);
            };
            self.invocations.slots[id.slot]
                .as_mut()
                .expect("running compound")
                .compound_results
                .as_mut()
                .expect("funded compound")
                .receipts = Some(receipts);
        }
        let disposition = if decision.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        };
        let published = decision
            .as_ref()
            .is_ok_and(|outcome| outcome.operation.is_some());
        let annotations_published = decision
            .as_ref()
            .is_ok_and(|o| !o.annotation_changes.is_empty());
        let outcome = match decision {
            Ok(outcome) => InvocationOutcome::Committed(outcome),
            Err(NativePreparationError::Line(error)) => InvocationOutcome::LineRejected(error),
            Err(NativePreparationError::Unicode(error)) => {
                InvocationOutcome::UnicodeRejected(error)
            }
            Err(NativePreparationError::Coordinator(error)) if is_unicode => {
                InvocationOutcome::UnicodeRejected(UnicodeCommandError::Coordinator(error))
            }
            Err(NativePreparationError::Coordinator(error)) if is_line => {
                InvocationOutcome::LineRejected(LineEditError::Coordinator(error))
            }
            Err(NativePreparationError::Coordinator(error)) => InvocationOutcome::Rejected(error),
        };
        self.install_invocation_terminal(id, outcome);
        self.faulted = false;
        recorder.record(Observation {
            boundary: Boundary::Admission,
            disposition,
            service_time: None,
        });
        if published {
            recorder.record(Observation {
                boundary: Boundary::Publication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        if annotations_published {
            recorder.record(Observation {
                boundary: Boundary::AnnotationPublication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        Ok(InvocationState::Finished)
    }

    /// Trusted-owner retained-outcome facet. The borrow is an immediate release;
    /// this is not the future scoped recipient/policy projection API.
    pub fn invocation_outcome(
        &self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<&InvocationOutcome, InvocationError> {
        let timing = recorder.begin_service_timing();
        let result = self.invocation_slot(id).and_then(|slot| match &slot.phase {
            Phase::Terminal(Some(outcome)) => Ok(outcome),
            Phase::Running | Phase::AnnotationFaulted => Err(InvocationError::Faulted),
            _ => Err(InvocationError::OutcomeUnavailable),
        });
        capture_timed(recorder, Boundary::Outcome, &result, timing.finish());
        result
    }

    /// Pure trusted-owner status query; never submits or starts a source job.
    pub fn invocation_state(
        &self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timing = recorder.begin_service_timing();
        let result = self.invocation_slot(id).and_then(|slot| slot.phase.state());
        capture_timed(recorder, Boundary::Outcome, &result, timing.finish());
        result
    }

    pub fn forget_invocation_outcome(
        &mut self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            match &self.invocation_slot(id)?.phase {
                Phase::Terminal(_) => {}
                Phase::Running | Phase::AnnotationFaulted => return Err(InvocationError::Faulted),
                _ => return Err(InvocationError::Pending),
            }
            if self.invocation_slot(id)?.created_results.held() {
                return Err(InvocationError::Unavailable);
            }
            if self.invocation_slot(id)?.compound_results.is_some() {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated terminal slot");
            if let Some(BoundIntent::Relation {
                result: Some(capture),
                ..
            }) = &mut slot.intent
            {
                capture.receipt = None;
            }
            if let Some(state) = reversal::results::state_mut(&mut slot.intent) {
                state.receipt = None;
            }
            if let Some(hold) = &mut slot.annotation_hold {
                hold.receipt = None;
            }
            if let Some(hold) = &mut slot.disposal_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.rebind_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.load_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.saveas_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.annotation_load_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.release_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.live_relation_hold {
                hold.candidate = None;
            }
            if let Some(hold) = &mut slot.scratch_hold {
                hold.candidate = None;
            }
            slot.phase = Phase::Terminal(None);
            Ok(())
        })();
        capture_timed(recorder, Boundary::Control, &result, timing.finish());
        result
    }

    pub fn stop_invocation(
        &mut self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            match &self.invocation_slot(id)?.phase {
                Phase::Terminal(_) => return Ok(()),
                Phase::Running | Phase::AnnotationFaulted => return Err(InvocationError::Faulted),
                Phase::Issued(_) => return Err(InvocationError::NotSubmitted),
                Phase::ScratchPending(_)
                | Phase::LiveRelationPending(_)
                | Phase::RetainedReleasePending(_)
                | Phase::SourceRebindPending(_)
                | Phase::SourceLoadPending(_)
                | Phase::SourceSaveAsPending(_)
                | Phase::AnnotationLoadPending(_)
                | Phase::RetainedDisposalPending(_)
                | Phase::Pending { .. }
                | Phase::AnnotationPending(_)
                | Phase::SelectionPending(_)
                | Phase::UnicodeMovementPending(_)
                | Phase::RelationPending(_)
                | Phase::SettingsPending(_)
                | Phase::SettingsDefinitionPending(_)
                | Phase::SettingsLayersPending(_)
                | Phase::ScopedSettingsPending(_)
                | Phase::NativeSettingsPending(_)
                | Phase::SerializationPending(_)
                | Phase::DependentSettingsPending(_)
                | Phase::ConfiguredLinePending { .. }
                | Phase::ScopedDefinitionPending(_)
                | Phase::ScopedLifecyclePending(_)
                | Phase::ManagedSearchPending(_) => {}
                Phase::ResourceCatalogPending(_) => {}
            }
            self.install_invocation_terminal(id, InvocationOutcome::StoppedBeforePublication);
            Ok(())
        })();
        capture_timed(recorder, Boundary::Control, &result, timing.finish());
        result
    }

    pub fn retire_invocation(
        &mut self,
        id: &InvocationId,
        recorder: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            match &self.invocation_slot(id)?.phase {
                Phase::ScratchPending(_)
                | Phase::LiveRelationPending(_)
                | Phase::RetainedReleasePending(_)
                | Phase::SourceRebindPending(_)
                | Phase::SourceLoadPending(_)
                | Phase::SourceSaveAsPending(_)
                | Phase::AnnotationLoadPending(_)
                | Phase::RetainedDisposalPending(_)
                | Phase::Pending { .. }
                | Phase::AnnotationPending(_)
                | Phase::SelectionPending(_)
                | Phase::UnicodeMovementPending(_)
                | Phase::RelationPending(_)
                | Phase::SettingsPending(_)
                | Phase::SettingsDefinitionPending(_)
                | Phase::SettingsLayersPending(_)
                | Phase::ScopedSettingsPending(_)
                | Phase::NativeSettingsPending(_)
                | Phase::SerializationPending(_)
                | Phase::DependentSettingsPending(_)
                | Phase::ConfiguredLinePending { .. }
                | Phase::ScopedDefinitionPending(_)
                | Phase::ScopedLifecyclePending(_)
                | Phase::ManagedSearchPending(_) => {
                    return Err(InvocationError::Pending);
                }
                Phase::ResourceCatalogPending(_) => return Err(InvocationError::Pending),
                Phase::Running | Phase::AnnotationFaulted => return Err(InvocationError::Faulted),
                _ => {}
            }
            if self.invocation_slot(id)?.created_results.held()
                || self.invocation_slot(id)?.annotation_hold.is_some()
                || self.invocation_slot(id)?.compound_results.is_some()
                || self.invocation_slot(id)?.disposal_hold.is_some()
                || self.invocation_slot(id)?.release_hold.is_some()
                || self.invocation_slot(id)?.rebind_hold.is_some()
                || self.invocation_slot(id)?.load_hold.is_some()
                || self.invocation_slot(id)?.saveas_hold.is_some()
                || self.invocation_slot(id)?.annotation_load_hold.is_some()
                || self.invocation_slot(id)?.live_relation_hold.is_some()
                || self.invocation_slot(id)?.scratch_hold.is_some()
                || self.invocation_slot(id)?.observation_policy.held()
                || matches!(
                    self.invocation_slot(id)?.intent,
                    Some(BoundIntent::ReturnedSearchReplacement(_))
                )
            {
                return Err(InvocationError::Unavailable);
            }
            self.release_configured_line_lease(id, recorder)?;
            self.invocations.slots[id.slot] = None;
            self.invocations.retained -= 1;
            Ok(())
        })();
        capture_timed(recorder, Boundary::Control, &result, timing.finish());
        result
    }

    fn source_intent_bytes(&self, intent: &SourceIntent) -> Result<usize, InvocationError> {
        self.source_intent_bytes_read_profile(intent, true)
    }

    fn source_intent_bytes_read_profile(
        &self,
        intent: &SourceIntent,
        legacy_reads: bool,
    ) -> Result<usize, InvocationError> {
        self.source_parts_bytes(&intent.writes, &intent.reads, legacy_reads)
    }
    fn source_parts_bytes(
        &self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        legacy_reads: bool,
    ) -> Result<usize, InvocationError> {
        if writes
            .len()
            .checked_add(if legacy_reads { reads.len() } else { 0 })
            .is_none_or(|n| n > self.limits.max_participants)
        {
            return Err(InvocationError::Limit);
        }
        let mut bytes = std::mem::size_of::<SourceIntent>();
        let mut charge = |n: usize| -> Result<(), InvocationError> {
            bytes = bytes
                .checked_add(n)
                .filter(|n| *n <= self.invocations.limits.max_intent_bytes)
                .ok_or(InvocationError::Limit)?;
            Ok(())
        };
        charge(0)?;
        for write in writes {
            charge(std::mem::size_of::<DocumentEdits>())?;
            if write.edits.len() > self.limits.source.max_edits {
                return Err(InvocationError::Limit);
            }
            for edit in &write.edits {
                charge(std::mem::size_of::<Replacement>())?;
                charge(edit.expected.len())?;
                charge(edit.inserted.len())?;
            }
        }
        for _ in reads {
            charge(std::mem::size_of::<DocumentVersion>())?;
        }
        Ok(bytes)
    }

    fn normalize_intent(&self, intent: &SourceIntent) -> Result<SourceIntent, InvocationError> {
        self.normalize_intent_read_profile(intent, true)
    }

    pub(super) fn compound_advice_request_ceiling(&self) -> usize {
        self.invocations.limits.max_intent_bytes
    }
    pub(super) fn normalize_intent_read_profile(
        &self,
        intent: &SourceIntent,
        legacy_reads: bool,
    ) -> Result<SourceIntent, InvocationError> {
        self.normalize_source_parts(&intent.writes, &intent.reads, legacy_reads)
    }
    fn normalize_source_parts(
        &self,
        input_writes: &[DocumentEdits],
        input_reads: &[DocumentVersion],
        legacy_reads: bool,
    ) -> Result<SourceIntent, InvocationError> {
        self.source_parts_bytes(input_writes, input_reads, legacy_reads)?;
        let mut writes: Vec<DocumentEdits> = Vec::new();
        writes
            .try_reserve_exact(input_writes.len())
            .map_err(|_| InvocationError::Capacity)?;
        for write in input_writes {
            let index = if let Some(index) = writes
                .iter()
                .position(|other| other.expected.document == write.expected.document)
            {
                if writes[index].expected != write.expected {
                    return Err(InvocationError::ConflictingParticipant);
                }
                index
            } else {
                writes.push(DocumentEdits {
                    expected: write.expected.clone(),
                    edits: Vec::new(),
                });
                writes.len() - 1
            };
            let target = &mut writes[index].edits;
            if target
                .len()
                .checked_add(write.edits.len())
                .is_none_or(|n| n > self.limits.source.max_edits)
            {
                return Err(InvocationError::Limit);
            }
            target
                .try_reserve(write.edits.len())
                .map_err(|_| InvocationError::Capacity)?;
            for edit in &write.edits {
                if edit.start > edit.end {
                    return Err(InvocationError::MalformedIntent);
                }
                let copy = |text: &str| -> Result<String, InvocationError> {
                    let mut owned = String::new();
                    owned
                        .try_reserve_exact(text.len())
                        .map_err(|_| InvocationError::Capacity)?;
                    owned.push_str(text);
                    Ok(owned)
                };
                target.push(Replacement {
                    start: edit.start,
                    end: edit.end,
                    expected: copy(&edit.expected)?,
                    inserted: copy(&edit.inserted)?,
                });
            }
        }
        for write in &mut writes {
            write
                .edits
                .sort_unstable_by_key(|edit| (edit.start, edit.end));
            if write
                .edits
                .windows(2)
                .any(|pair| crate::source::edits_ambiguous(&pair[0], &pair[1]))
            {
                return Err(InvocationError::MalformedIntent);
            }
        }
        let mut reads = Vec::new();
        if legacy_reads {
            reads
                .try_reserve_exact(input_reads.len())
                .map_err(|_| InvocationError::Capacity)?;
            reads.extend(input_reads.iter().cloned());
        }
        Ok(SourceIntent { writes, reads })
    }
}
