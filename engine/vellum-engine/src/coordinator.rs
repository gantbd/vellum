//! Source and authored-annotation coordination for one trusted local embedding.
//!
//! The non-cloneable owner is a privileged host API, not the future recipient
//! command facade. Exclusive Rust borrowing serializes final validation and
//! publication; no callback or I/O occurs. Bounded source invocation replay is
//! optional, as are linked source/annotation history and resource profiles.
//! Recipient/workspace authority projections, complete annotation persistence and
//! complete external-effect/lifecycle obligations remain unimplemented.

pub use invocation::{
    AnnotationLoadAdmissionV1, AnnotationLoadLimitsV1, AnnotationLoadOutcomeV1,
    AnnotationLoadRequestV1, AnnotationLoadResultV1, TrustedAnnotationLoadFactsV1,
    TrustedAnnotationLoadResultV1, native_annotation_load_argument_declarations_v1,
};
pub use invocation::{
    DisposalDispositionV1, ExplicitDisposalRequestV1, PreparedExplicitDisposalV1,
    TrustedDisposalCoverageV1,
};
pub use invocation::{InvocationReceiptRequestV2, InvocationReceiptV2};
pub use invocation::{InvocationReceiptRequestV3, InvocationReceiptV3};
pub use invocation::{InvocationReceiptRequestV4, InvocationReceiptV4};
pub use invocation::{InvocationReceiptRequestV5, InvocationReceiptV5};
pub use invocation::{InvocationReceiptRequestV6, InvocationReceiptV6};
pub use invocation::{InvocationReceiptRequestV7, InvocationReceiptV7};
pub use invocation::{InvocationReceiptRequestV8, InvocationReceiptV8};
pub use invocation::{InvocationReceiptRequestV9, InvocationReceiptV9};
mod collection_publication;
mod document_context_pins;
mod document_read_pins;
mod facet_publication;
mod publication_command;
mod shutdown;
pub use shutdown::*;
mod publication_stream;
pub use collection_publication::{
    COLLECTION_PUBLICATION_COMMAND_PROFILE_V1, COLLECTION_PUBLICATION_RECEIPT_PROFILE_V1,
    CollectionAnnotationReferenceV1, CollectionAnnotationRowV1, CollectionPublicationBaselineV1,
    CollectionPublicationCommandFactsV1, CollectionPublicationCommandIdV1,
    CollectionPublicationCommandLimitsV1, CollectionPublicationCommandOperationV1,
    CollectionPublicationCommandPortV1, CollectionPublicationCommandReplyV1,
    CollectionPublicationCommandRequestV1, CollectionPublicationCommandUsageV1,
    CollectionPublicationControlV1, CollectionPublicationDeliveryV1,
    CollectionPublicationEnvelopeValueV1, CollectionPublicationErrorV1,
    CollectionPublicationEventV1, CollectionPublicationGenerationV1,
    CollectionPublicationIngressErrorV1, CollectionPublicationLimitsV1,
    CollectionPublicationProjectionV1, CollectionPublicationReceiptAuthorizationBasisIdV1,
    CollectionPublicationReceiptAuthorizationBasisV1, CollectionPublicationReceiptCeilingV1,
    CollectionPublicationReceiptContextIdV1, CollectionPublicationReceiptLimitsV1,
    CollectionPublicationReceiptPortV1, CollectionPublicationReceiptReplyV1,
    CollectionPublicationReceiptUsageV1, CollectionPublicationSampleV1,
    CollectionPublicationScopeV1, CollectionPublicationStreamIdV1, CollectionPublicationUsageV1,
    NormalizedCollectionPublicationCommandV1, NormalizedCollectionPublicationReceiptV1,
    TrustedCollectionPublicationFactsV1, TrustedCollectionPublicationReceiptFactsV1,
    TrustedCollectionPublicationReceiptHistoricalFactsV1,
    normalize_collection_publication_command_v1, normalize_collection_publication_receipt_v1,
};
pub use facet_publication::{
    FACET_PUBLICATION_COMMAND_PROFILE_V1, FACET_PUBLICATION_RECEIPT_PROFILE_V1,
    FacetPublicationBaselineV1, FacetPublicationCommandFactsV1, FacetPublicationCommandIdV1,
    FacetPublicationCommandLimitsV1, FacetPublicationCommandOperationV1,
    FacetPublicationCommandPortV1, FacetPublicationCommandReplyV1,
    FacetPublicationCommandRequestV1, FacetPublicationCommandUsageV1, FacetPublicationControlV1,
    FacetPublicationDeliveryV1, FacetPublicationEnvelopeValueV1, FacetPublicationErrorV1,
    FacetPublicationEventV1, FacetPublicationGenerationV1, FacetPublicationIngressErrorV1,
    FacetPublicationLimitsV1, FacetPublicationProjectionV1,
    FacetPublicationReceiptAuthorizationBasisIdV1, FacetPublicationReceiptAuthorizationBasisV1,
    FacetPublicationReceiptCeilingV1, FacetPublicationReceiptContextIdV1,
    FacetPublicationReceiptLimitsV1, FacetPublicationReceiptPortV1, FacetPublicationReceiptReplyV1,
    FacetPublicationReceiptUsageV1, FacetPublicationSampleV1, FacetPublicationScopeV1,
    FacetPublicationStreamIdV1, FacetPublicationUsageV1, NormalizedFacetPublicationCommandV1,
    NormalizedFacetPublicationReceiptV1, PublicationAnnotationFacetV1,
    PublicationAnnotationSelectionV1, PublicationSourceFacetV1, PublicationViewFacetV1,
    PublicationViewSelectionV1, TrustedFacetPublicationFactsV1,
    TrustedFacetPublicationReceiptFactsV1, TrustedFacetPublicationReceiptHistoricalFactsV1,
    normalize_facet_publication_command_v1, normalize_facet_publication_receipt_v1,
};
pub use publication_command::PublicationCommandPhaseV1;
pub use publication_stream::commands::*;
pub use publication_stream::{
    SourcePublicationBaselineV1, SourcePublicationDeliveryV1, SourcePublicationErrorV1,
    SourcePublicationEventV1, SourcePublicationGenerationV1, SourcePublicationLimitsV1,
    SourcePublicationProjectionV1, SourcePublicationSampleV1, SourcePublicationStreamIdV1,
    SourcePublicationUsageV1, TrustedSourcePublicationFactsV1,
};
mod observation_control;
pub use document_context_pins::DocumentContextPinsV1;
pub use document_read_pins::DocumentReadPinsV1;
pub use invocation::InvocationReceiptEnvelopeValueV1;
pub use invocation::{
    InvocationObservationEnvelopeValueV1, InvocationObservationReadPortV1,
    InvocationObservationReadReplyV1,
};
pub use invocation::{InvocationReceiptRequestV10, InvocationReceiptV10};
pub use invocation::{
    LiveRelationAdmissionV1, LiveRelationErrorV1, LiveRelationKindV1, LiveRelationLimitsV1,
    LiveRelationOperationV1, LiveRelationRequestV1, LiveRelationResultV1,
    TrustedLiveRelationFactsV1, TrustedLiveRelationResultV1,
};
pub use invocation::{
    NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1, NativeDocumentLifecycleAdmissionV1,
    NativeDocumentLifecycleAdmissionV2, NativeDocumentLifecycleArgumentLimitsV1,
    NativeDocumentLifecycleCommandV1, NativeDocumentLifecycleCommandV2,
    NativeDocumentLifecycleEnvelopeValueV1, NativeDocumentLifecycleRequestV1,
    NativeDocumentLifecycleServiceV1, NativeDocumentLifecycleServiceV2,
    NormalizedDocumentLifecycleArgumentsV1, normalize_native_document_lifecycle_arguments_v1,
};
pub use invocation::{
    RetainedDisposalAdmissionV1, RetainedDisposalLimitsV1, RetainedDisposalResultV1,
    TrustedRetainedDisposalFactsV1, TrustedRetainedDisposalResultV1,
};
pub use invocation::{
    RetainedReleaseAdmissionV1, RetainedReleaseLimitsV1, RetainedReleaseOperationV1,
    RetainedReleaseRequestV1, RetainedReleaseResultV1, TrustedRetainedReleaseFactsV1,
    TrustedRetainedReleaseResultV1,
};
pub use invocation::{
    SourceSaveAsAdmissionV1, SourceSaveAsLimitsV1, SourceSaveAsRequestV1, SourceSaveAsResultV1,
    TrustedSourceSaveAsInvocationFactsV1, TrustedSourceSaveAsResultV1,
};
pub use observation_control::{
    DecodedObservationStopRequestV2, DecodedObservationStopRequestV3,
    DecodedObservationStopRequestV4, NormalizedObservationStopArgumentsV2,
    NormalizedObservationStopArgumentsV3, NormalizedObservationStopArgumentsV4,
    ObservationStopEnvelopeValueV2, ObservationStopEnvelopeValueV3, ObservationStopEnvelopeValueV4,
    ObservationStopErrorV1, ObservationStopErrorV2, ObservationStopErrorV3, ObservationStopErrorV4,
    ObservationStopPortV1, ObservationStopPortV2, ObservationStopPortV3, ObservationStopPortV4,
    ObservationStopTargetV2, ObservationStopTargetV3, ObservationStopTargetV4,
    normalize_observation_stop_envelope_v2, normalize_observation_stop_envelope_v3,
    normalize_observation_stop_envelope_v4,
};
mod observation_events;
pub use observation_events::*;
mod observation_revocation;
pub use observation_revocation::*;
pub use resource::{SourceSaveAsAuthorityFactsV1, SourceSaveAsAuthorityV1};
use std::sync::Arc;

use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};
use crate::source::{Limits, Replacement, SourceError, Utf8Source};

mod annotation;
pub use annotation::{
    CollectionFacetCeilingV1, CollectionVisibilityCaptureV1, CollectionVisibilityPreparationIdV1,
    TrustedCollectionVisibilityFactsV1,
};
mod birth;
pub use birth::ScratchDocumentRequestV1;
pub use invocation::{
    ScratchInvocationAdmissionV1, ScratchInvocationLimitsV1, ScratchResultV1,
    TrustedScratchFactsV1, TrustedScratchResultV1,
};
mod boundary;
pub use boundary::{
    BoundaryCaptureId, BoundaryError, BoundaryEvidenceId, BoundaryEvidenceRead, BoundaryLimits,
    BoundaryProfileV1, BoundarySourceRead, BoundaryTableV1, BoundaryUsage,
};
mod handler_declaration;
mod history;
pub use annotation::native_annotation_authoring_declaration_v1;
pub use annotation::{
    AnnotationDocumentSummaryV1, AnnotationError, AnnotationFact, AnnotationId, AnnotationLimits,
    AnnotationMutation, AnnotationRead, AnnotationSnapshot, AnnotationSnapshotRecord,
    AnnotationUsage, SourceTarget, TargetBasis, TrustedAnnotationRetirementV1,
};
pub use handler_declaration::{
    NativeArgumentDeclarationV1, NativeHandlerDeclarationV1, NativeHandlerVariantDeclarationV1,
    capture_native_context_declaration_v1,
};
pub use history::native_history_reversal_declaration_v1;
mod invocation;
pub use invocation::HistoryReversalTargetV1;
pub use invocation::NativeSettingsArgumentDeclarationV1;
pub use invocation::TrustedNativeSettingsInvocationV1;
pub use invocation::{
    AnnotationAuthoringArgumentLimitsV1, AnnotationAuthoringEnvelopeValueV1,
    InvocationObservationPolicyControlV1, InvocationObservationPolicyLimitsV1,
    InvocationObservationPolicyUsageV1, InvocationObservationPolicyV1,
    NormalizedAnnotationAuthoringArgumentsV1, TrustedInvocationPolicyFactsV1,
    normalize_annotation_authoring_arguments_v1,
};
pub use invocation::{
    AnnotationInvocationLimitsV1, AnnotationInvocationRetirementV1, AnnotationResultCapabilitiesV1,
    AnnotationResultDecisionV1, AnnotationResultGrantTargetV1, AnnotationResultOperationV1,
    AnnotationResultRecordV1, AnnotationResultV1, TrustedAnnotationInvocationV1,
    TrustedAnnotationResultAdmissionV1, TrustedAnnotationResultGrantV1,
    TrustedAnnotationResultReleaseV1,
};
pub use invocation::{
    AtomicAnnotationComponentV1, AtomicAuthoringEnvelopeValueV1,
    BOUNDED_ATOMIC_AUTHORING_PROFILE_V1, NormalizedAtomicAuthoringArgumentsV1,
    TrustedAtomicAnnotationBindingV1, TrustedAtomicAuthoringV1,
};
pub use invocation::{
    BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1, ExactBatchEnvelopeValueV1,
    NormalizedExactBatchArgumentsV1,
};
pub use invocation::{
    BoundaryEditArgumentLimitsV1, BoundaryEditEnvelopeValueV1, NormalizedBoundaryEditArgumentsV1,
    UNICODE_LOGICAL_EDIT_PROFILE_V1, normalize_boundary_edit_arguments_v1,
};
pub use invocation::{
    CONFIGURED_PREFIX_LINE_PROFILE_V1, ConfiguredLineArgumentLimitsV1,
    ConfiguredLineEnvelopeValueV1, NormalizedConfiguredLineArgumentsV1,
    normalize_configured_line_arguments_v1,
};
pub use invocation::{
    CompositionEnvelopeValueV1, NormalizedCompositionArgumentsV1,
    PRIMARY_COMPOSITION_REPLACEMENT_PROFILE_V1, normalize_composition_arguments_v1,
    normalize_composition_envelope_v1,
};
pub use invocation::{
    CompoundAnnotationResultEvictionV1, CompoundAnnotationResultGrantTargetV1,
    TrustedCompoundAnnotationResultAdmissionV1, TrustedCompoundAnnotationResultGrantV1,
    TrustedCompoundAnnotationResultReleaseV1,
};
pub use invocation::{
    EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1, LiteralEditArgumentLimitsV1,
    LiteralEditEnvelopeValueV1, NormalizedLiteralEditArgumentsV1,
    normalize_literal_edit_arguments_v1,
};
pub use invocation::{
    EXPLICIT_PREFIX_LINE_EDIT_PROFILE_V1, LineEditArgumentLimitsV1, LineEditEnvelopeValueV1,
    NormalizedLineEditArgumentsV1, normalize_line_edit_arguments_v1,
};
pub use invocation::{
    InvocationControlDispositionV1, InvocationLocalEffectV1, InvocationObservationQueryV1,
    InvocationObservationReferenceV1, InvocationObservationReplyV1, InvocationOutcomeProjectionV1,
    InvocationOutcomeReasonV1, InvocationPublicationV1, InvocationRecoveryV1,
    TrustedInvocationOrdinaryObservationReleaseV1,
};
pub use invocation::{
    InvocationControlObservationV1, InvocationControlReferenceV1, InvocationObservationContextIdV1,
    InvocationObservationContextV1, InvocationObservationLimitsV1, InvocationObservationPhaseV1,
    TrustedInvocationObservationReleaseV1,
};
pub use invocation::{
    InvocationMeasurementControlV1, InvocationMeasurementLimitsV1, InvocationMeasurementLimitsV2,
    InvocationMeasurementsV1, InvocationResponsivenessSnapshotV1, InvocationResponsivenessV1,
};
pub use invocation::{
    InvocationReceiptAdmissionV1, InvocationReceiptCapabilitiesV1, InvocationReceiptKindV1,
    InvocationReceiptRequestV1, InvocationReceiptV1, TrustedInvocationReceiptReleaseV1,
};
pub use invocation::{
    InvocationWaitClockV1, InvocationWaitEventV1, InvocationWaitIdV1, InvocationWaitReplyV1,
    InvocationWaitRequestV1, InvocationWaitUsageV1, TrustedInvocationWaitFactsV1,
};
pub use invocation::{
    NATIVE_RELATION_LIFECYCLE_PROFILE_V1, NormalizedRelationArgumentsV1, RelationArgumentLimitsV1,
    RelationEnvelopeValueV1, native_relation_argument_declarations_v1,
    normalize_relation_arguments_v1,
};
pub use invocation::{
    NATIVE_RESOURCE_REGISTRATION_PROFILE_V1, ResourceRegistrationResultV1,
    RetainedResourceRegistrationOutcomeV1, TrustedResourceRegistrationInvocationV1,
    TrustedResourceRegistrationResultV1,
};
pub use invocation::{
    NativeDefinitionEnvelopeValueV1, NormalizedNativeDefinitionArgumentsV1,
    normalize_native_definition_envelope_v1,
};
pub use invocation::{
    NativeLifecycleEnvelopeValueV1, NormalizedNativeLifecycleArgumentsV1,
    normalize_native_lifecycle_envelope_v1,
};
pub use invocation::{
    NativeSettingsDefinitionRequestV1, TrustedNativeSettingsDefinitionInvocationV1,
};
pub use invocation::{
    NativeSettingsEnvelopeValueV1, NormalizedNativeSettingsArgumentsV1,
    normalize_native_settings_envelope_v1,
};
pub use invocation::{
    NativeSettingsLifecycleRequestV1, TrustedNativeSettingsLifecycleInvocationV1,
};
pub use invocation::{
    NormalizedReplacementArgumentsV1, ReplacementEnvelopeValueV1,
    native_replacement_argument_declarations_v1, normalize_replacement_arguments_v1,
    normalize_replacement_envelope_v1,
};
pub use invocation::{
    NormalizedResourceRegistrationArgumentsV1, ResourceRegistrationEnvelopeValueV1,
    normalize_resource_registration_envelope_v1,
};
pub use invocation::{
    NormalizedScopedDefinitionArgumentsV1, ScopedDefinitionEnvelopeValueV1,
    normalize_scoped_definition_envelope_v1,
};
pub use invocation::{
    NormalizedScopedLifecycleArgumentsV1, ScopedLifecycleEnvelopeValueV1,
    normalize_scoped_lifecycle_envelope_v1,
};
pub use invocation::{
    NormalizedScopedRegistrationArgumentsV1, ScopedRegistrationEnvelopeValueV1,
    normalize_scoped_registration_envelope_v1,
};
pub use invocation::{
    NormalizedScopedSettingsArgumentsV1, ScopedSettingsEnvelopeValueV1,
    normalize_scoped_settings_envelope_v1,
};
pub use invocation::{
    NormalizedSelectionArgumentsV1, SelectionArgumentLimitsV1, SelectionEnvelopeValueV1,
    native_selection_argument_declarations_v1, normalize_selection_arguments_v1,
};
pub use invocation::{
    RelationResultV1, TrustedRelationResultAdmissionV1, TrustedRelationResultReleaseV1,
};
pub use invocation::{
    ReversalDecisionV1, ReversalResultV1, TrustedReversalResultAdmissionV1,
    TrustedReversalResultGrantV1, TrustedReversalResultReleaseV1,
};
pub use invocation::{
    SCOPED_RESOURCE_REGISTRATION_PROFILE_V1, ScopedResourceRegistrationChangeV1,
    ScopedResourceRegistrationResultV1, ScopedResourceRegistrationRowV1,
    TrustedScopedResourceRegistrationInvocationV1, TrustedScopedResourceRegistrationResultV1,
};
pub use invocation::{
    SourceLoadAdmissionV1, SourceLoadLimitsV1, SourceLoadResultV1, TrustedSourceLoadFactsV1,
    TrustedSourceLoadResultV1, native_source_load_argument_declaration_v1,
};
pub use invocation::{
    SourceRebindAdmissionV1, SourceRebindLimitsV1, SourceRebindRequestV1, SourceRebindResultV1,
    TrustedSourceRebindInvocationFactsV1, TrustedSourceRebindResultV1,
};
pub use source_context::normalize_literal_search_envelope_v1;
pub use source_context::{
    BoundLiteralSearchErrorV1, BoundLiteralSearchProductV1, BoundLiteralSearchReplyV1,
    BoundLiteralSearchResultV1, LITERAL_EXACT_MATCHING_PROFILE_V1, LogicalSearchBindingV1,
    TrustedLogicalSearchBindingV1,
};
pub use source_context::{
    LiteralSearchArgumentLimitsV1, LiteralSearchEnvelopeValueV1,
    LiteralSearchNavigationArgumentsV1, LiteralSearchOperationV1, LiteralSearchReferenceV1,
    NormalizedLiteralSearchArgumentsV1, normalize_literal_search_arguments_v1,
};
pub use source_context::{LiteralSearchValueFieldsV1, literal_search_value_fields_v1};
mod discovery;
mod lifecycle;
mod resource;
pub use discovery::*;
pub use lifecycle::{
    LifecyclePhase, LifecycleState, LifecycleUsage, PreparedDisposal, TrustedDocumentReclamationV1,
};
pub use resource::DocumentResourcePinsV1;
pub use resource::TrustedSourceSaveAsFactsV1;
pub use resource::catalog::basis::*;
pub(crate) use resource::catalog::consumer::ReadCatalogCaptureV1;
pub use resource::catalog::consumer::TrustedCatalogAnnotationReadBindingV1;
pub use resource::catalog::consumer::TrustedCatalogSourceReadBindingV1;
pub(crate) use resource::catalog::consumer::WriteCatalogCaptureV1;
pub use resource::catalog::consumer::{CatalogWriterBindingV1, TrustedCatalogWriterBindingV1};
pub use resource::catalog::consumer::{
    OwnedCatalogAnnotationSaveRequestV1, OwnedCatalogSourceSaveRequestV1,
};
pub use resource::catalog::observation::{
    NormalizedResourceRegistryCommandArgumentsV1, RESOURCE_REGISTRY_COMMAND_PROFILE_V1,
    ResourceRegistryCommandEnvelopeValueV1, ResourceRegistryCommandFactsV1,
    ResourceRegistryCommandHistoricalSlotV1, ResourceRegistryCommandIdV1,
    ResourceRegistryCommandOperationV1, ResourceRegistryCommandPhaseV1,
    ResourceRegistryCommandPortV1, ResourceRegistryCommandProtocolFaultV1,
    ResourceRegistryCommandReplyV1, ResourceRegistryCommandRequestV1,
    TrustedResourceRegistryCommandAdmissionV1, normalize_resource_registry_command_envelope_v1,
};
pub use resource::catalog::observation::{
    NormalizedResourceRegistryReceiptArgumentsV1, RESOURCE_REGISTRY_RECEIPT_PROFILE_V1,
    ResourceRegistryReceiptAuthorizationBasisIdV1, ResourceRegistryReceiptAuthorizationBasisV1,
    ResourceRegistryReceiptCeilingV1, ResourceRegistryReceiptContextIdV1,
    ResourceRegistryReceiptEnvelopeValueV1, ResourceRegistryReceiptPortV1,
    ResourceRegistryReceiptReplyV1, ResourceRegistryReceiptUsageV1,
    TrustedResourceRegistryReceiptFactsV1, TrustedResourceRegistryReceiptHistoricalFactsV1,
    normalize_resource_registry_receipt_envelope_v1,
};
pub use resource::catalog::observation::{
    RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1, ResourceRegistryBaselineV1,
    ResourceRegistryDeliveryV1, ResourceRegistryEventV1, ResourceRegistryGenerationV1,
    ResourceRegistryHeadIdV1, ResourceRegistryHistoricalHeadV1,
    ResourceRegistryHistoricalRowFactsV1, ResourceRegistryObservationControlV1,
    ResourceRegistryObservationLimitsV1, ResourceRegistryObservationRowV1,
    ResourceRegistryObservationScopeV1, ResourceRegistryObservationUsageV1,
    ResourceRegistryPreparationIdV1, ResourceRegistryProjectionV1, ResourceRegistrySampleV1,
    ResourceRegistrySlotReferenceV1, ResourceRegistrySnapshotRowFactsV1,
    ResourceRegistryStreamIdV1, TrustedResourceRegistryHistoricalFactsV1,
    TrustedResourceRegistryObservationFactsV1, TrustedResourceRegistrySnapshotFactsV1,
};
pub use resource::catalog::read::{
    NormalizedCatalogReadArgumentsV1, RESOURCE_CATALOG_READ_PROFILE_V1,
    ResourceCatalogReadContextIdV1, ResourceCatalogReadContinuationIdV1,
    ResourceCatalogReadDecoderUsageV1, ResourceCatalogReadDescriptorV1,
    ResourceCatalogReadEntryIdV1, ResourceCatalogReadEnvelopeValueV1, ResourceCatalogReadLimitsV1,
    ResourceCatalogReadPageEntryV1, ResourceCatalogReadQueryV1, ResourceCatalogReadReplyV1,
    ResourceCatalogReadScopeV1, ResourceCatalogReadUsageV1, ResourceCatalogVisibilityIdV1,
    ResourceCatalogVisibilityV1, TrustedResourceCatalogReadFactsV1,
    normalize_resource_catalog_read_envelope_v1, resource_catalog_read_classification_bytes_v1,
};
pub use resource::{
    AnnotationLoadAuthorityFactsV1, AnnotationLoadAuthorityV1,
    AnnotationReadOperationAdmissionStatusV1, AnnotationReadOperationAdmissionV1,
    AnnotationReadOperationLimitsV1, AnnotationReadOperationOriginV1,
    AnnotationReadOperationUsageV1, ExpectedAnnotationReadBindingV1, NativeAnnotationReadReceiptV1,
    NativeReadTerminalWitnessV1, ObservedReadVersionV1, OwnedAnnotationReadRequestV1,
    TrustedAnnotationReadOperationFactsV1,
};
pub use resource::{
    CatalogExpectationV1, CatalogInsertionLineBreakV1, PreparedResourceCatalogV1,
    ResourceCatalogActionV1, ResourceCatalogAddressV1, ResourceCatalogCapabilitiesV1,
    ResourceCatalogChangeV1, ResourceCatalogConfigurationV1, ResourceCatalogIdentityV1,
    ResourceCatalogLimitsV1, ResourceCatalogOutcomeV1, ResourceCatalogResultV1,
    ResourceCatalogRowV1, ResourceCatalogStatusV1, ResourceCatalogTableV1, ResourceCatalogUsageV1,
    TrustedResourceCatalogFactsV1,
};
pub use resource::{
    ExpectedSaveBindingV1, NativeOperationObservationPoolUsageV1, OwnedAnnotationSaveRequestV1,
    OwnedSourceSaveRequestV1, SaveOperationAdmissionStatusV1, SaveOperationAdmissionV1,
    SaveOperationConditionV1, SaveOperationLimitsV1, SaveOperationOriginV1, SaveOperationUsageV1,
    TrustedAnnotationSaveOperationReleaseV1, TrustedOrdinaryAnnotationSaveAdmissionV1,
    TrustedOrdinarySourceSaveAdmissionV1, TrustedSaveOperationReleaseV1,
};
pub use resource::{
    NativeOperationDecodedReplyV1, NativeOperationDecoderLimitsV1, NativeOperationDecoderUsageV1,
};
pub use resource::{
    NativeOperationEnvelopeValueV1, NativeOperationKindV1, NativeOperationTaskV1,
    NormalizedNativeOperationArgumentsV1, NormalizedNativeOperationQueryV1,
    native_operation_argument_declarations_v1, normalize_native_operation_envelope_v1,
    recognize_native_operation_envelope_v1,
};
pub use resource::{
    NativeOperationObservationControlV1, NativeOperationObservationErrorV1,
    NativeOperationObservationFactsV1, NativeOperationObservationIdV1,
    NativeOperationObservationLimitsV1, NativeOperationObservationPhaseV1,
    NativeOperationObservationPolicyV1, NativeOperationObservationPoolLimitsV1,
    NativeOperationObservationQueryV1, NativeOperationObservationReplyV1,
    NativeOperationObservationUsageV1, NativeOperationProgressIdV1, NativeOperationReceiptV1,
    NativeOperationWaitClockV1, NativeOperationWaitEventV1, NativeOperationWaitIdV1,
    NativeOperationWaitReplyV1, NativeOperationWaitRequestV1, NativeSaveKindV1,
    NativeSaveReceiptV1, NativeSaveRequiredAcknowledgmentV1, NativeSaveWritePhaseV1,
    ObservedSaveVersionV1,
};
pub use resource::{
    NativeSourceReadReceiptV1, NativeSourceReadTerminalWitnessV1, ObservedSourceReadVersionV1,
    OwnedSourceReadRequestV1, SourceLoadAuthorityFactsV1, SourceLoadAuthorityV1,
    SourceReadActivationDispositionV1, SourceReadActivationV1,
    SourceReadOperationAdmissionStatusV1, SourceReadOperationAdmissionV1,
    SourceReadOperationLimitsV1, SourceReadOperationOriginV1, SourceReadOperationUsageV1,
    SourceReadStatusV1, SourceReaderLimitsV1, SourceReaderUsageV1,
    TrustedSourceReadOperationFactsV1,
};
pub use resource::{
    ResourceQueueAttemptTimingV1, ResourceQueueAttemptV1, ResourceQueuePhaseV1,
    ResourceResultAttemptTimingV1,
};
pub use resource::{
    ResourceQueueKindV1, ResourceQueuePopulationV1, ResourceQueueResponsivenessV1,
    ResourceQueueSnapshotV1,
};
pub use resource::{SourceContinuityLimitsV1, TrustedSourceContinuityLaunchV1};
pub use resource::{TrustedSourceBindingHistoryReleaseV1, TrustedSourceRebindFactsV1};
pub use resource::{TrustedSourceExportControlV1, TrustedSourceExportFactsV1};
mod publication_measurements;
mod release_measurements;
pub use release_measurements::{ReleaseMeasurementControlV1, ReleaseMeasurementLimitsV1};
mod read_dependency;
pub use publication_measurements::{
    PublicationMeasurementControlV1, PublicationMeasurementLimitsV1,
};
mod scope;
pub use read_dependency::{
    ReadDependencyBudget, ReadDependencyError, ReadDependencyRequest, ReadDependencyV1,
};
pub use read_dependency::{ReadDependencyRequestV2, ReadDependencyV2};
pub use read_dependency::{ReadDependencyRequestV3, ReadDependencyV3};
mod search;
mod source_context;
pub use annotation::projection::version_set::{
    PreparedVersionSetId, TrustedPreparedVersionSetFactsV1, TrustedVersionSetContextFactsV1,
    TrustedVersionSetMemberFactsV1, VersionSetAcquisitionV1, VersionSetAnnotationContinuationId,
    VersionSetAnnotationQueryV1, VersionSetAnnotationRecordId, VersionSetAnnotationRecordV1,
    VersionSetAnnotationReplyV1, VersionSetContextId, VersionSetContextLimitsV1,
    VersionSetContextUsageV1, VersionSetControlTargetV1, VersionSetMemberAcquisitionV1,
    VersionSetMemberId, VersionSetParticipantV1, VersionSetPreparationLimitsV1,
    VersionSetPreparationUsageV1, VersionSetSourceReplyV1,
};
pub use annotation::projection::{
    AnnotationContentV1, AnnotationContextAcquisitionV1, AnnotationContextLimitsV1,
    AnnotationContextUsageV1, AnnotationContinuationId, AnnotationFacetGrantV1,
    AnnotationLocationV1, AnnotationPreparationLimitsV1, AnnotationPreparationUsageV1,
    AnnotationProjectionClassesV1, AnnotationQueryReplyV1, AnnotationQueryV1,
    AnnotationReadContextId, AnnotationRecordId, AnnotationRecordV1, AnnotationSourceReplyV1,
    PreparedAnnotationProjectionId, SourceProjectionLimitsV1, TrustedAnnotationCaptureFactsV1,
    TrustedAnnotationContextFactsV1, TrustedPreparedAnnotationFactsV1,
};
pub use source_context::ReadObservationStopPortV1;
pub use source_context::{
    AnnotationArgumentFactsV1, AnnotationArgumentQueryV1, AnnotationArgumentTargetV1,
    AnnotationEnvelopeValueV1, DecodedAnnotationFacadeRequestV1, NormalizedAnnotationArgumentsV1,
    normalize_annotation_envelope_v1,
};
pub use source_context::{
    AnnotationFacadePortV1, AnnotationFacadeReplyV1, AnnotationFacadeRequestV1, ReadFacadeUsageV1,
};
pub use source_context::{
    DecodedHistoryFacadeRequestV1, HistoryEnvelopeValueV1, HistoryFacadeRequestV1,
    NormalizedHistoryArgumentsV1, normalize_history_envelope_v1,
};
pub use source_context::{
    DecodedReadAcquisitionFacadeRequestV1, NormalizedReadAcquisitionArgumentsV1,
    ReadAcquisitionEnvelopeValueV1, ReadAcquisitionParticipantV1, ReadAcquisitionSelectionV1,
    normalize_read_acquisition_envelope_v1,
};
pub use source_context::{
    DecodedReadObservationStopRequestV1, NormalizedReadObservationStopArgumentsV1,
    ReadObservationStopEnvelopeValueV1, ReadObservationStopTargetV1,
    normalize_read_observation_stop_envelope_v1,
};
pub use source_context::{
    DecodedSourceFacadeRequestV1, NormalizedSourceArgumentsV1, SourceEnvelopeValueV1,
    normalize_source_envelope_v1,
};
pub use source_context::{SearchFacadePortV1, SearchFacadeReplyV1, SearchFacadeRequestV1};
pub use source_context::{SourceFacadeReplyV1, SourceFacadeRequestV1, SourceFacadeUsageV1};
pub use source_context::{SourceSearchConsumerV1, TrustedSourceSearchConsumerFactsV1};
mod source_query;
pub use history::inspection::*;
pub use history::{HistoryBasisLimitsV1, HistoryBasisScopeV1, HistoryBasisUsageV1, HistoryBasisV1};
pub use scope::{
    InsertionSiteRange, SOURCE_SCOPE_PROFILE_V1, SourceOperationScope, SourceScopeBudget,
    SourceScopeError,
};
pub use source_context::{
    ReadAcquisitionFacadePortV1, ReadAcquisitionFacadeReplyV1, ReadAcquisitionFacadeRequestV1,
};
pub use source_context::{
    SearchReadContextV1, SearchReadFactsV1, SearchResultControlV1, SourceContextContentV1,
    SourceContextQueryV1, SourceContextReplyV1, SourceReadContextControlV1,
    SourceReadContextErrorV1, SourceReadContextId, SourceReadContextLimitsV1,
    SourceReadContextUsageV1, SourceReadHoldFactsV1, SourceReadHoldId, SourceReadPoolControlV1,
    SourceReadScopeV1, SourceSearchFoundV1, SourceSearchLimitsV1, SourceSearchNavigationReplyV1,
    SourceSearchNavigationV1, SourceSearchOccurrenceId, SourceSearchReplyV1, SourceSearchResultId,
    SourceSearchUsageV1, TrustedSourceReadFactsV1, TrustedSourceSearchFactsV1,
    TrustedVersionSetSearchFactsV1,
};
pub use source_query::result_contract::ReadResultInvariantV1;
mod configured_line;
mod diagnostic_capture;
mod proposal_advice;
mod relation;
mod retention;
pub use diagnostic_capture::*;
mod root_accounting;
mod root_identity;
mod transaction_candidate;
pub use proposal_advice::{
    CompoundAdviceBasisV1, CompoundAdviceEnvelopeValueV1, CompoundAdviceLimitsV1,
    CompoundAdviceReplyV1, CompoundAdviceRequestV1, NATIVE_COMPOUND_ADVICE_PROFILE_V1,
    TrustedCompoundAdviceFactsV1,
};
pub use root_accounting::{
    NATIVE_SEMANTIC_ROOT_PROFILE_V1, RootAccountingErrorV1, RootCapacityUsageV1,
    SemanticRootUsageV1,
};
mod scoped_definition;
mod scoped_lifecycle;
pub use scoped_lifecycle::{
    BasisSettingLayerReferenceV1, BasisSettingTargetV1, BasisSettingsLifecycleRequestV1,
    PreparedScopedSettingsLifecycleV1, ScopedSettingLayerCreateV1, ScopedSettingLayerReferenceV1,
    ScopedSettingTargetV1, ScopedSettingsLifecycleRequestV1,
    TrustedSettingsLifecycleMutationFactsV1,
};
mod settings;
pub use settings::reader::{
    NormalizedSettingsReadArgumentsV1, SETTINGS_READ_PROFILE_V1, SettingsReadAcquisitionV1,
    SettingsReadContextIdV1, SettingsReadControlV1, SettingsReadCutIdV1,
    SettingsReadDecoderUsageV1, SettingsReadEnvelopeValueV1, SettingsReadOperationV1,
    SettingsReadOriginRefV1, SettingsReadOriginsV1, SettingsReadReplyV1, SettingsReadResultV1,
    SettingsReadScopeV1, SettingsReadTargetRefV1, SettingsReaderLimitsV1, SettingsReaderUsageV1,
    TrustedSettingsReadFactsV1, TrustedSettingsReadPreparationV1,
    normalize_settings_read_envelope_v1, settings_read_decode_classification_bytes_v1,
};
mod settings_definition;
pub use scoped_definition::{
    PreparedScopedSettingsDefinitionV1, ScopedSettingsDefinitionRequestV1,
    TrustedSettingsDefinitionMutationFactsV1,
};
mod settings_layers;
pub use settings_layers::PreparedSettingsLayersV1;
mod serialization;
mod settings_reads;
mod settings_scoped;
pub use configured_line::{ConfiguredLineEditV1, ConfiguredLineErrorV1, ConfiguredLineLimitsV1};
pub use retention::{RetentionErrorV1, RetentionUsageV1};
pub use serialization::{
    NormalizedScopedSerializationArgumentsV1, NormalizedSourceSerializationArgumentsV1,
    PreparedSourceSerializationV1, ScopedSourceSerializationRequestV1,
    SerializationBasisAvailabilityV1, SerializationBasisId, SerializationBasisLimitsV1,
    SerializationBasisUsageV1, SerializationEnvelopeValueV1, SerializationPoolControlV1,
    SourceSerializationErrorV1, SourceSerializationLimitsV1, SourceSerializationProfileV1,
    SourceSerializationRequestV1, SourceSerializationStateV1, SourceSerializationUpdateV1,
    SourceSerializationV1, TrustedSerializationBasisFactsV1, TrustedSerializationFactsV1,
    normalize_scoped_serialization_envelope_v1, normalize_source_serialization_envelope_v1,
};
pub use settings::basis::{SettingsDefinitionBasisId, TrustedSettingsDefinitionBasisFactsV1};
pub use settings::basis::{SettingsLifecycleBasisId, TrustedSettingsLifecycleBasisFactsV1};
pub use settings::{PreparedSettingsChange, SettingsBasisV1, SettingsRegistryId, SettingsTargetId};
pub use settings::{
    SettingsBasisAvailabilityV1, SettingsBasisControlV1, SettingsBasisLimitsV1,
    SettingsExecutionContextV1, SettingsWriteBasisId, TrustedSettingsBasisFactsV1,
};
pub use settings::{SettingsInterpretationId, SettingsRetentionLimitsV1, SettingsRetentionUsageV1};
pub use settings_definition::PreparedSettingsDefinitionV1;
pub use settings_reads::{PreparedSettingsChangeV2, SettingsChangeErrorV2};
pub use settings_scoped::{
    PreparedScopedSettingsV1, ScopedSettingChangeV1, ScopedSettingsRequestV1,
    ScopedSettingsUpdateV1, TrustedSettingsMutationFactsV1,
};
mod views;
pub use relation::{RelationCommandV1, RelationErrorV1, RelationReceiptV1};
mod workspace;
pub use history::{
    HistoryAvailability, HistoryDirection, HistoryError, HistoryGroupId, HistoryLimits, HistoryPin,
    HistoryUsage,
};
pub use invocation::{
    CompoundIntent, CreatedLayerResultStatusV1, CreatedLayerResultV1,
    CreatedSettingLayerReferenceV1, CreatedSettingLayerResultV1, INVOCATION_CONTROL_PROFILE_V1,
    InvocationCancellationEnvelopeValueV1, InvocationCancellationIngressErrorV1, InvocationError,
    InvocationId, InvocationLimits, InvocationOutcome, InvocationState, InvocationUsage,
    NormalizedInvocationCancellationV1, SourceIntent, TrustedCreatedLayerResultAdmissionV1,
    TrustedCreatedLayerResultReleaseV1, TrustedInvocationControlV1, UnicodeOperationV1,
    normalize_invocation_cancellation_v1,
};
pub use search::{
    ByteRange, SearchBudget, SearchDirection, SearchError, SearchFound, SearchLimits, SearchMatch,
    SearchNavigation, SearchRequest, SearchResult, SearchScope, SearchSelection, SearchSession,
    SearchStatus, SearchUsage,
};
pub use source_query::{
    CoordinateReplyV1, CoordinateRequestV1, CoordinateResultV1, SourceLineReply, SourceQueryBudget,
    SourceQueryError, SourceRangeReply,
};
pub use views::SelectionAfterV1;
pub use views::SelectionCommandV1;
pub use views::{GeneratedNewlineV1, LineEditBudget, LineEditError, LineEditV1, NewlineIndentV1};
pub use views::{
    LogicalEndpointV1, LogicalViewContext, LogicalViewId, LogicalViewLimits, LogicalViewSnapshot,
    LogicalViewStamp, LogicalViewUpdate, LogicalViewUsage,
};
pub use views::{
    PreparedUnicodeMovement, UnicodeCommandBudget, UnicodeCommandError, UnicodeCommandV1,
    UnicodeDirectionV1, UnicodeUnitV1,
};
pub use views::{SearchSelectionModeV1, SelectionDirectionV1};
pub use workspace::{
    WorkspaceContext, WorkspaceId, WorkspaceLimits, WorkspaceSnapshot, WorkspaceUpdate,
    WorkspaceUsage,
};

/// Opaque, process-local identity. Its slot is never reused by its owner.
#[derive(Clone)]
pub struct DocumentId {
    namespace: Arc<()>,
    slot: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner() -> SourceCoordinator {
        SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 128,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 8,
                max_edit_payload_bytes: 128,
            },
        })
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
    fn exhausted_last_participant_cannot_publish_earlier_candidates() {
        let mut owner = owner();
        let mut telemetry = Recorder::disabled();
        let a = owner.create(b"A", &mut telemetry).unwrap();
        let mut b = owner.create(b"B", &mut telemetry).unwrap();
        owner.documents[b.document.slot].as_mut().unwrap().revision = u64::MAX;
        b.revision = u64::MAX;
        let before = owner.generation();
        let prepared = owner
            .prepare(
                &[write(a.clone(), "A", "a"), write(b.clone(), "B", "b")],
                &[],
                &mut telemetry,
            )
            .unwrap();
        assert_eq!(
            owner.commit(prepared, &mut telemetry),
            Err(CoordinatorError::VersionExhausted)
        );
        assert_eq!(owner.generation(), before);
        assert_eq!(
            owner
                .read(&a.document, &mut telemetry)
                .unwrap()
                .source
                .text(),
            "A"
        );
        assert_eq!(
            owner
                .read(&b.document, &mut telemetry)
                .unwrap()
                .source
                .text(),
            "B"
        );
    }

    #[test]
    fn exhausted_counters_allow_validated_unchanged_but_no_real_transition() {
        let mut owner = owner();
        let mut telemetry = Recorder::disabled();
        let mut a = owner.create(b"A", &mut telemetry).unwrap();
        owner.generation = u64::MAX;
        owner.documents[a.document.slot].as_mut().unwrap().revision = u64::MAX;
        a.revision = u64::MAX;
        let noop = owner
            .prepare(&[write(a.clone(), "A", "A")], &[], &mut telemetry)
            .unwrap();
        assert!(
            owner
                .commit(noop, &mut telemetry)
                .unwrap()
                .changes
                .is_empty()
        );
        let change = owner
            .prepare(&[write(a.clone(), "A", "a")], &[], &mut telemetry)
            .unwrap();
        assert_eq!(
            owner.commit(change, &mut telemetry),
            Err(CoordinatorError::VersionExhausted)
        );
        assert_eq!(
            owner.create(b"new", &mut telemetry),
            Err(CoordinatorError::VersionExhausted)
        );
        assert_eq!(
            owner.dispose(&a.document, &mut telemetry),
            Err(CoordinatorError::VersionExhausted)
        );
        assert_eq!(
            owner.set_read_only(&a.document, true, &mut telemetry),
            Err(CoordinatorError::VersionExhausted)
        );
        assert_eq!(owner.read(&a.document, &mut telemetry).unwrap().version, a);
    }

    #[test]
    fn exhausted_policy_counter_refuses_without_changing_current_policy() {
        let mut owner = owner();
        let mut telemetry = Recorder::disabled();
        let a = owner.create(b"A", &mut telemetry).unwrap();
        owner.documents[a.document.slot]
            .as_mut()
            .unwrap()
            .policy_generation = u64::MAX;
        let before = owner.generation();
        assert_eq!(
            owner.set_read_only(&a.document, true, &mut telemetry),
            Err(CoordinatorError::VersionExhausted)
        );
        assert_eq!(owner.generation(), before);
        let prepared = owner
            .prepare(&[write(a, "A", "a")], &[], &mut telemetry)
            .unwrap();
        assert!(owner.commit(prepared, &mut telemetry).is_ok());
    }
}

impl PartialEq for DocumentId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.namespace, &other.namespace) && self.slot == other.slot
    }
}
impl Eq for DocumentId {}
impl std::fmt::Debug for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DocumentId(<opaque>)")
    }
}

/// Owner-local logical operation identity, not a resource or invocation grant.
#[derive(Clone)]
pub struct OperationId {
    namespace: Arc<()>,
    generation: u64,
}
impl PartialEq for OperationId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.namespace, &other.namespace) && self.generation == other.generation
    }
}
impl Eq for OperationId {}
impl std::fmt::Debug for OperationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OperationId(<opaque>)")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentVersion {
    pub document: DocumentId,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoordinatorLimits {
    /// Lifetime admission bound: disposed slots still count, preventing ID reuse.
    pub max_document_incarnations: usize,
    /// Combined write participants and declared read dependencies per preparation.
    pub max_participants: usize,
    /// Sum of candidate text bytes in one prepared transaction (BOM excluded).
    pub max_staged_bytes: usize,
    pub source: Limits,
}

/// Explicit construction-time profiles; omitted modules are not silently enabled.
#[derive(Clone, Copy, Debug)]
pub struct CoordinatorConfig {
    pub limits: CoordinatorLimits,
    pub invocation: Option<InvocationLimits>,
    pub history: Option<HistoryLimits>,
    pub resources: Option<crate::resource::ResourceLimits>,
    pub search: Option<SearchLimits>,
    pub annotations: Option<AnnotationLimits>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentEdits {
    pub expected: DocumentVersion,
    pub edits: Vec<Replacement>,
}

/// Trusted-owner refusals; not a recipient-safe public command result profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinatorError {
    /// Owner integrity is uncertain; not evidence of original nonpublication.
    Faulted,
    Unavailable,
    Stale,
    DuplicateParticipant,
    Limit,
    VersionExhausted,
    Capacity,
    Source(SourceError),
    History(HistoryError),
    Resource(crate::resource::ResourceError),
    Search(SearchError),
    Annotation(AnnotationError),
    Scope(SourceScopeError),
    ReadDependency(ReadDependencyError),
}

#[derive(Debug)]
struct Document {
    insertion_policy: GeneratedNewlineV1,
    serialization: SourceSerializationStateV1,
    lifecycle: LifecycleState,
    retained_bindings: resource::RetainedBindings,
    source: Utf8Source,
    revision: u64,
    read_only: bool,
    policy_generation: u64,
    history: history::Frontiers,
    annotation_generation: u64,
    annotation_policy: u64,
    annotation_read_only: bool,
}

#[derive(Debug)]
struct Candidate {
    expected: DocumentVersion,
    policy_generation: u64,
    source: Utf8Source,
    edits: Vec<Replacement>,
    annotation_generation: u64,
}

/// Owned, non-cloneable prepared state. Consuming it does not supply invocation
/// deduplication. Retaining arbitrarily many preparations is the embedding's
/// responsibility until the admission/retention subsystem is implemented.
#[derive(Debug)]
pub struct PreparedTransaction {
    configured_required: bool,
    configured_line: Option<configured_line::ConfiguredLineCapture>,
    reads_required: bool,
    declared_reads: Option<read_dependency::ReadCapture>,
    scoped: bool,
    scopes: Option<scope::ScopeCapture>,
    /// Fallibly allocated private 0/1 capture slot, not a batch of edit modes.
    derived_edit: Vec<views::DerivedEdit>,
    derived_kind: Option<views::DerivedKind>,
    view_dependencies: Vec<views::ViewDependency>,
    namespace: Arc<()>,
    candidates: Vec<Candidate>,
    read_dependencies: Vec<DocumentVersion>,
    history: history::PreparedHistory,
    search_dependency: Option<SearchSelection>,
    annotations: Vec<annotation::AnnotationPlan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceChange {
    pub before: DocumentVersion,
    pub after: DocumentVersion,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommitOutcome {
    pub generation: u64,
    /// Only text-changing documents; emptiness does not classify companion effects.
    pub changes: Vec<SourceChange>,
    pub operation: Option<OperationId>,
    pub history_group: Option<HistoryGroupId>,
    pub annotation_changes: Vec<AnnotationFact>,
}

/// Borrowed release to the trusted embedding, already a disclosure of its source.
#[derive(Debug)]
pub struct SourceRead<'a> {
    pub generation: u64,
    pub version: DocumentVersion,
    pub source: &'a Utf8Source,
}

/// Non-cloneable authoritative source owner. No public mutable source references.
/// Move between threads is allowed; shared mutation requires host serialization.
#[derive(Debug)]
pub struct SourceCoordinator {
    diagnostic_capture: diagnostic_capture::State,
    shutdown: shutdown::State,
    registration_bases: resource::catalog::basis::State,
    registry_observations: resource::catalog::observation::State,
    discovery: discovery::State,
    root_byte_ceiling: Option<usize>,
    configured_line_limits: Option<ConfiguredLineLimitsV1>,
    retention: retention::RetentionState,
    publication_measurements: publication_measurements::State,
    release_measurements: release_measurements::State,
    invocation_measurements: invocation::measurements::State,
    source_contexts: source_context::State,
    publication_streams: publication_stream::State,
    facet_publication_streams: facet_publication::State,
    collection_publication_streams: collection_publication::State,
    history_inspection: history::inspection::State,
    managed_search: source_context::ManagedSearchState,
    annotation_preparations: annotation::projection::State,
    annotation_contexts: annotation::projection::InstalledState,
    version_set_preparations: annotation::projection::version_set::VersionSetPreparationState,
    version_set_contexts: annotation::projection::version_set::VersionSetContextState,
    settings_retention: settings::RetentionPool,
    settings_reader: settings::reader::State,
    catalog_reads: resource::catalog::read::State,
    settings_plan_life: Arc<()>,
    settings: Option<Arc<crate::settings::CompiledSettingsV1>>,
    settings_activation: Option<crate::settings::SettingsActivationLimitsV1>,
    settings_definition_limits: Option<crate::settings::SettingsDefinitionLimitsV1>,
    settings_layer_limits: Option<crate::settings::SettingsLayerLimitsV1>,
    settings_basis_profile: Option<settings::BasisProfile>,
    serialization_profile: Option<serialization::SerializationRegistration>,
    serialization_bases: serialization::BasisPool,
    workspaces: workspace::WorkspaceState,
    boundaries: boundary::BoundaryState,
    views: views::ViewState,
    namespace: Arc<()>,
    documents: Vec<Option<Document>>,
    generation: u64,
    limits: CoordinatorLimits,
    invocations: invocation::InvocationTable,
    history: history::HistoryState,
    resources: resource::ResourceState,
    searches: search::SearchState,
    annotations: annotation::AnnotationState,
    faulted: bool,
    read_result_invariant: Option<ReadResultInvariantV1>,
    source_facade_busy: bool,
    source_facade_workspace: usize,
    #[cfg(test)]
    source_facade_fault: Option<source_context::facade::Fault>,
    #[cfg(test)]
    search_result_fault: Option<source_context::search::result_contract::Fault>,
    #[cfg(test)]
    source_read_result_fault: Option<source_query::result_contract::SourceResultFault>,
    #[cfg(test)]
    annotation_read_result_fault: Option<source_query::result_contract::AnnotationResultFault>,
    #[cfg(test)]
    acquisition_result_fault: Option<source_query::result_contract::AcquisitionResultFault>,
    #[cfg(test)]
    publication_fault_after: Option<usize>,
    #[cfg(test)]
    reversal_stage_fault: Option<history::reversal_validation::Fault>,
    #[cfg(test)]
    annotation_context_install_fault: bool,
    #[cfg(test)]
    version_set_preparation_fault: bool,
    #[cfg(test)]
    version_set_install_fault: bool,
    #[cfg(test)]
    version_set_selector_fault: bool,
    #[cfg(test)]
    annotation_selector_registration_fault: bool,
    #[cfg(test)]
    relation_publication_fault: bool,
    #[cfg(test)]
    settings_publication_fault: bool,
    #[cfg(test)]
    created_result_fault_after: Option<usize>,
}

fn observe<T>(recorder: &mut Recorder, boundary: Boundary, result: &Result<T, CoordinatorError>) {
    observe_service(recorder, boundary, result, None);
}

fn observe_service<T>(
    recorder: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, CoordinatorError>,
    service_time: Option<std::time::Duration>,
) {
    recorder.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(
                CoordinatorError::Faulted
                | CoordinatorError::History(HistoryError::Faulted)
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted),
            ) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}

impl SourceCoordinator {
    pub fn new(limits: CoordinatorLimits) -> Self {
        Self {
            diagnostic_capture: Default::default(),
            shutdown: shutdown::State::default(),
            registration_bases: Default::default(),
            registry_observations: Default::default(),
            root_byte_ceiling: None,
            discovery: Default::default(),
            configured_line_limits: None,
            retention: retention::RetentionState::default(),
            publication_measurements: publication_measurements::State::default(),
            release_measurements: release_measurements::State::default(),
            invocation_measurements: invocation::measurements::State::default(),
            source_contexts: source_context::State::default(),
            publication_streams: publication_stream::State::default(),
            facet_publication_streams: facet_publication::State::default(),
            collection_publication_streams: collection_publication::State::default(),
            history_inspection: history::inspection::State::default(),
            managed_search: source_context::ManagedSearchState::default(),
            annotation_preparations: annotation::projection::State::default(),
            annotation_contexts: annotation::projection::InstalledState::default(),
            version_set_preparations:
                annotation::projection::version_set::VersionSetPreparationState::default(),
            version_set_contexts:
                annotation::projection::version_set::VersionSetContextState::default(),
            settings_retention: settings::RetentionPool::default(),
            settings_reader: settings::reader::State::default(),
            catalog_reads: resource::catalog::read::State::default(),
            settings_plan_life: Arc::new(()),
            settings: None,
            settings_activation: None,
            settings_definition_limits: None,
            settings_layer_limits: None,
            settings_basis_profile: None,
            serialization_profile: None,
            serialization_bases: serialization::BasisPool::default(),
            workspaces: workspace::WorkspaceState::default(),
            namespace: Arc::new(()),
            boundaries: boundary::BoundaryState::default(),
            views: views::ViewState::default(),
            documents: Vec::new(),
            generation: 0,
            limits,
            invocations: invocation::InvocationTable::new(InvocationLimits::disabled()),
            history: history::HistoryState::disabled(),
            resources: resource::ResourceState::disabled(),
            searches: search::SearchState::default(),
            annotations: annotation::AnnotationState::default(),
            faulted: false,
            read_result_invariant: None,
            source_facade_busy: false,
            source_facade_workspace: 0,
            #[cfg(test)]
            source_facade_fault: None,
            #[cfg(test)]
            search_result_fault: None,
            #[cfg(test)]
            source_read_result_fault: None,
            #[cfg(test)]
            annotation_read_result_fault: None,
            #[cfg(test)]
            acquisition_result_fault: None,
            #[cfg(test)]
            publication_fault_after: None,
            #[cfg(test)]
            reversal_stage_fault: None,
            #[cfg(test)]
            annotation_context_install_fault: false,
            #[cfg(test)]
            version_set_preparation_fault: false,
            #[cfg(test)]
            version_set_install_fault: false,
            #[cfg(test)]
            version_set_selector_fault: false,
            #[cfg(test)]
            annotation_selector_registration_fault: false,
            #[cfg(test)]
            relation_publication_fault: false,
            #[cfg(test)]
            settings_publication_fault: false,
            #[cfg(test)]
            created_result_fault_after: None,
        }
    }

    /// Last complete generation, not proof that owner integrity is healthy.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn is_faulted(&self) -> bool {
        self.faulted
    }

    pub fn create(
        &mut self,
        bytes: &[u8],
        recorder: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        self.create_with_insertion_policy_v1(bytes, GeneratedNewlineV1::Lf, recorder)
    }
    /// Birth-time default only. Existing text is never normalized; later explicit
    /// delimiter requests and literal insertions keep their own contracts.
    pub fn create_with_insertion_policy_v1(
        &mut self,
        bytes: &[u8],
        policy: GeneratedNewlineV1,
        recorder: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.create_inner(bytes, 0, policy, recorder)
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                observe_service(
                    recorder,
                    Boundary::Lifecycle,
                    &Err::<DocumentVersion, _>(CoordinatorError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if matches!(result, Err(CoordinatorError::Faulted)) {
            self.faulted = true;
        }
        observe_service(recorder, Boundary::Lifecycle, &result, timing.finish());
        result
    }

    fn create_inner(
        &mut self,
        bytes: &[u8],
        additional_root_bytes: usize,
        insertion_policy: GeneratedNewlineV1,
        recorder: &mut Recorder,
    ) -> Result<DocumentVersion, CoordinatorError> {
        let document = self.decode_imported_document(bytes, insertion_policy, recorder)?;
        self.install_document_birth(document, additional_root_bytes, recorder)
    }

    fn decode_imported_document(
        &self,
        bytes: &[u8],
        insertion_policy: GeneratedNewlineV1,
        recorder: &mut Recorder,
    ) -> Result<Document, CoordinatorError> {
        self.ensure_ordinary_admission()?;
        if self.documents.len() >= self.limits.max_document_incarnations {
            return Err(CoordinatorError::Limit);
        }
        let source =
            Utf8Source::decode_observed(bytes, self.limits.source.max_source_bytes, recorder)
                .map_err(CoordinatorError::Source)?;
        let document = Document {
            insertion_policy,
            serialization: SourceSerializationStateV1::initial(source.initial_bom()),
            lifecycle: LifecycleState::default(),
            retained_bindings: resource::RetainedBindings::default(),
            source,
            revision: 0,
            read_only: false,
            policy_generation: 0,
            history: history::Frontiers::default(),
            annotation_generation: 0,
            annotation_policy: 0,
            annotation_read_only: false,
        };
        Ok(document)
    }

    /// Exact current document/default query. No newline heuristic, settings write,
    /// encoding operation, publication or inferred presentation preference.
    pub fn source_insertion_policy_v1(
        &self,
        expected: &DocumentVersion,
        r: &mut Recorder,
    ) -> Result<GeneratedNewlineV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = self
            .ensure_operational()
            .and_then(|()| self.validate_version(expected))
            .map(|d| d.insertion_policy);
        r.record(crate::instrumentation::Observation {
            boundary: Boundary::DocumentInsertionPolicyRead,
            disposition: match &result {
                Ok(_) => crate::instrumentation::Disposition::Completed,
                Err(CoordinatorError::Faulted) => crate::instrumentation::Disposition::Unknown,
                Err(_) => crate::instrumentation::Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }

    fn document(&self, id: &DocumentId) -> Result<&Document, CoordinatorError> {
        self.retained_document(id).and_then(|document| {
            if document.lifecycle.phase == LifecyclePhase::Live {
                Ok(document)
            } else {
                Err(CoordinatorError::Unavailable)
            }
        })
    }

    fn retained_document(&self, id: &DocumentId) -> Result<&Document, CoordinatorError> {
        if !Arc::ptr_eq(&self.namespace, &id.namespace) {
            return Err(CoordinatorError::Unavailable);
        }
        self.documents
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(CoordinatorError::Unavailable)
    }

    fn validate_version(&self, expected: &DocumentVersion) -> Result<&Document, CoordinatorError> {
        let document = self.document(&expected.document)?;
        if document.revision != expected.revision {
            return Err(CoordinatorError::Stale);
        }
        Ok(document)
    }

    pub fn read(
        &self,
        id: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<SourceRead<'_>, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = self
            .ensure_operational()
            .and_then(|()| self.document(id))
            .map(|document| SourceRead {
                generation: self.generation,
                version: DocumentVersion {
                    document: id.clone(),
                    revision: document.revision,
                },
                source: &document.source,
            });
        observe_service(recorder, Boundary::Query, &result, timing.finish());
        result
    }

    /// Privileged local host policy update, not a principal assertion from content.
    pub fn set_read_only(
        &mut self,
        id: &DocumentId,
        read_only: bool,
        recorder: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let result = self.set_read_only_inner(id, read_only);
        observe(recorder, Boundary::Lifecycle, &result);
        result
    }

    fn set_read_only_inner(
        &mut self,
        id: &DocumentId,
        read_only: bool,
    ) -> Result<(), CoordinatorError> {
        self.ensure_operational()?;
        let document = self.document(id)?;
        if document.read_only == read_only {
            return Ok(());
        }
        let policy = document
            .policy_generation
            .checked_add(1)
            .ok_or(CoordinatorError::VersionExhausted)?;
        let generation = self.next_publication_generation(None)?;
        let document = self.documents[id.slot]
            .as_mut()
            .expect("validated live document");
        document.read_only = read_only;
        document.policy_generation = policy;
        self.generation = generation;
        Ok(())
    }

    /// Explicit privileged discard for currently unbound source-only documents.
    /// Future history/effect ownership must add its discharge checks here.
    pub fn dispose(
        &mut self,
        id: &DocumentId,
        recorder: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = (|| {
            self.ensure_operational()?;
            if self
                .annotations
                .records
                .iter()
                .flatten()
                .any(|r| r.value.id.document == *id)
            {
                return Err(CoordinatorError::Annotation(AnnotationError::Retained));
            }
            self.document(id)?;
            if self.history_retains_document(id) {
                return Err(CoordinatorError::History(HistoryError::Retained));
            }
            self.can_dispose_resource(id)
                .map_err(CoordinatorError::Resource)?;
            let generation = self.next_publication_generation(None)?;
            let views = self.stage_view_disposal(id)?;
            let workspaces = self.stage_workspace_disposal(id, recorder)?;
            self.documents[id.slot] = None;
            views.install(self);
            workspaces.install(self);
            self.remove_resource_binding(id);
            self.generation = generation;
            Ok(())
        })();
        if matches!(result, Err(CoordinatorError::Faulted)) {
            self.faulted = true;
        }
        observe_service(recorder, Boundary::Lifecycle, &result, timing.finish());
        result
    }

    pub fn prepare(
        &self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
        recorder: &mut Recorder,
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = self.prepare_inner(writes, reads);
        observe_service(recorder, Boundary::Preparation, &result, timing.finish());
        result
    }

    fn prepare_inner(
        &self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
    ) -> Result<PreparedTransaction, CoordinatorError> {
        let mut prepared = self.prepare_source_candidates(writes, reads)?;
        prepared.view_dependencies = self.capture_view_dependencies(&prepared.candidates)?;
        Ok(prepared)
    }
    /// Source-only candidate preparation for explicit batches. Such proposals
    /// have no selection predicate; their views are derived at actual service.
    /// Ordinary view-bound commands still use prepare_inner above.
    fn prepare_source_candidates(
        &self,
        writes: &[DocumentEdits],
        reads: &[DocumentVersion],
    ) -> Result<PreparedTransaction, CoordinatorError> {
        self.ensure_ordinary_admission()?;
        if writes
            .len()
            .checked_add(reads.len())
            .is_none_or(|n| n > self.limits.max_participants)
        {
            return Err(CoordinatorError::Limit);
        }
        for (index, write) in writes.iter().enumerate() {
            if writes[..index]
                .iter()
                .any(|other| other.expected.document == write.expected.document)
            {
                return Err(CoordinatorError::DuplicateParticipant);
            }
            let document = self.validate_version(&write.expected)?;
            if document.read_only {
                return Err(CoordinatorError::Unavailable);
            }
        }
        for read in reads {
            self.validate_version(read)?;
        }
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(writes.len())
            .map_err(|_| CoordinatorError::Capacity)?;
        let mut staged = 0usize;
        for write in writes {
            let document = self.validate_version(&write.expected)?;
            let source = document
                .source
                .prepare_with_staging_limit(
                    &write.edits,
                    self.limits.source,
                    self.limits.max_staged_bytes - staged,
                )
                .map_err(|error| match error {
                    SourceError::StagingLimit => CoordinatorError::Limit,
                    other => CoordinatorError::Source(other),
                })?;
            staged = staged
                .checked_add(source.text().len())
                .filter(|n| *n <= self.limits.max_staged_bytes)
                .ok_or(CoordinatorError::Limit)?;
            candidates.push(Candidate {
                annotation_generation: document.annotation_generation,
                expected: write.expected.clone(),
                policy_generation: document.policy_generation,
                source,
                edits: {
                    let mut edits = crate::source::copy_replacements(&write.edits)
                        .map_err(CoordinatorError::Source)?;
                    edits.sort_unstable_by_key(|edit| (edit.start, edit.end));
                    edits
                },
            });
        }
        let mut read_dependencies = Vec::new();
        read_dependencies
            .try_reserve_exact(reads.len())
            .map_err(|_| CoordinatorError::Capacity)?;
        read_dependencies.extend(reads.iter().cloned());
        Ok(PreparedTransaction {
            configured_required: false,
            configured_line: None,
            reads_required: false,
            declared_reads: None,
            scoped: false,
            scopes: None,
            derived_edit: Vec::new(),
            derived_kind: None,
            view_dependencies: Vec::new(),
            namespace: self.namespace.clone(),
            candidates,
            read_dependencies,
            history: history::PreparedHistory::Ordinary,
            search_dependency: None,
            annotations: Vec::new(),
        })
    }

    pub fn commit(
        &mut self,
        prepared: PreparedTransaction,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        let timing = recorder.begin_service_timing();
        let result = self.commit_inner(prepared, recorder);
        // This existing marker covers synchronous commit validation/publication,
        // not time spent waiting for a host to call commit or a queue to drain.
        observe_service(recorder, Boundary::Admission, &result, timing.finish());
        if let Ok(outcome) = &result
            && outcome.operation.is_some()
        {
            observe(recorder, Boundary::Publication, &result);
            if !outcome.annotation_changes.is_empty() {
                observe(recorder, Boundary::AnnotationPublication, &result);
            }
        }
        result
    }

    fn commit_inner(
        &mut self,
        prepared: PreparedTransaction,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        self.ensure_operational()?;
        self.faulted = true;
        let decision = self.commit_with_storage(prepared, Vec::new(), recorder);
        if !matches!(
            &decision,
            Err(CoordinatorError::Faulted | CoordinatorError::History(HistoryError::Faulted))
        ) {
            self.faulted = false;
        }
        decision
    }

    fn ensure_operational(&self) -> Result<(), CoordinatorError> {
        if self.faulted {
            Err(CoordinatorError::Faulted)
        } else if self.shutdown.retired {
            Err(CoordinatorError::Unavailable)
        } else {
            Ok(())
        }
    }

    /// Source invocation execution supplies pre-reserved result storage here.
    /// Its own latched execution fence deliberately remains set during this call.
    fn commit_with_storage(
        &mut self,
        prepared: PreparedTransaction,
        changes: Vec<SourceChange>,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        self.commit_with_reversal_storage(prepared, changes, None, &mut None, recorder)
    }

    fn commit_with_reversal_storage(
        &mut self,
        prepared: PreparedTransaction,
        changes: Vec<SourceChange>,
        reversal: Option<&Arc<invocation::reversal::results::Capture>>,
        receipt: &mut Option<invocation::reversal::results::Receipt>,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        self.commit_with_decision_storage(
            prepared, changes, reversal, receipt, None, None, recorder,
        )
    }

    fn commit_with_annotation_storage(
        &mut self,
        prepared: PreparedTransaction,
        id: &InvocationId,
        receipt: &mut Option<invocation::annotation_scoped::receipt::Receipt>,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        self.commit_with_decision_storage(
            prepared,
            Vec::new(),
            None,
            &mut None,
            Some(invocation::annotation_scoped::receipt::Storage {
                invocation: id,
                receipt,
            }),
            None,
            recorder,
        )
    }

    fn commit_with_compound_annotation_storage(
        &mut self,
        prepared: PreparedTransaction,
        changes: Vec<SourceChange>,
        id: &InvocationId,
        receipts: &mut Option<Vec<invocation::annotation_scoped::receipt::Receipt>>,
        r: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        self.commit_with_decision_storage(
            prepared,
            changes,
            None,
            &mut None,
            None,
            Some(invocation::compound_results::Storage {
                invocation: id,
                receipts,
            }),
            r,
        )
    }

    #[allow(clippy::too_many_arguments)] // One private publisher; mutually exclusive receipt owners.
    fn commit_with_decision_storage(
        &mut self,
        prepared: PreparedTransaction,
        changes: Vec<SourceChange>,
        reversal: Option<&Arc<invocation::reversal::results::Capture>>,
        receipt: &mut Option<invocation::reversal::results::Receipt>,
        annotation: Option<invocation::annotation_scoped::receipt::Storage<'_>>,
        atomic: Option<invocation::compound_results::Storage<'_>>,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        let probe = self.publication_measurements.probe();
        let result = self.commit_transaction_candidate(
            prepared, changes, reversal, receipt, annotation, atomic, recorder,
        );
        probe.finish(&result);
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_transaction_candidate(
        &mut self,
        prepared: PreparedTransaction,
        mut changes: Vec<SourceChange>,
        reversal: Option<&Arc<invocation::reversal::results::Capture>>,
        receipt: &mut Option<invocation::reversal::results::Receipt>,
        mut annotation: Option<invocation::annotation_scoped::receipt::Storage<'_>>,
        mut atomic: Option<invocation::compound_results::Storage<'_>>,
        recorder: &mut Recorder,
    ) -> Result<CommitOutcome, CoordinatorError> {
        self.ensure_admission_unfenced()?;
        if [reversal.is_some(), annotation.is_some(), atomic.is_some()]
            .into_iter()
            .filter(|v| *v)
            .count()
            > 1
        {
            return Err(CoordinatorError::Faulted);
        }
        let transaction_candidate::TransactionDecision {
            changed,
            mut annotations,
            views,
        } = self.decide_transaction_candidate(&prepared, false, recorder)?;
        if changed == 0 && !annotations.changed() && !views.changed() {
            if !matches!(prepared.history, history::PreparedHistory::Ordinary) {
                return Err(CoordinatorError::History(HistoryError::Unavailable));
            }
            self.check_root_growth(0, recorder)?;
            if let Some(storage) = &mut atomic {
                *storage.receipts = Some(self.stage_compound_annotation_receipts(
                    storage.invocation,
                    &prepared,
                    &annotations,
                    false,
                    recorder,
                )?);
            }
            if let Some(storage) = &mut annotation {
                *storage.receipt = Some(self.stage_annotation_invocation_receipt(
                    storage.invocation,
                    &prepared,
                    &annotations,
                    recorder,
                )?);
            }
            return Ok(CommitOutcome {
                generation: self.generation,
                changes: Vec::new(),
                operation: None,
                history_group: None,
                annotation_changes: Vec::new(),
            });
        }
        let generation = self.next_publication_generation(None)?;
        #[allow(unused_mut)]
        let mut history = self
            .stage_history(&prepared, generation, &annotations.journals)
            .map_err(CoordinatorError::History)?;
        #[cfg(test)]
        if matches!(prepared.history, history::PreparedHistory::Reversal { .. }) {
            history::reversal_validation::corrupt(&mut history, self.reversal_stage_fault);
        }
        self.validate_reversal_decision(&prepared, &history, recorder)
            .map_err(CoordinatorError::History)?;
        self.check_compound_root(&prepared, &annotations, &views, &history, recorder)?;
        let history_group = history.group_id();
        changes
            .try_reserve_exact(changed)
            .map_err(|_| CoordinatorError::Capacity)?;
        for candidate in &prepared.candidates {
            let document = self.document(&candidate.expected.document)?;
            if document.source != candidate.source {
                changes.push(SourceChange {
                    before: candidate.expected.clone(),
                    after: DocumentVersion {
                        document: candidate.expected.document.clone(),
                        revision: document.revision + 1,
                    },
                });
            }
        }
        if let Some(capture) = reversal {
            *receipt = Some(
                self.stage_reversal_result_for_commit(capture, &prepared, &history, recorder)?,
            );
        }
        if let Some(storage) = &mut annotation {
            *storage.receipt = Some(self.stage_annotation_invocation_receipt(
                storage.invocation,
                &prepared,
                &annotations,
                recorder,
            )?);
        }
        if let Some(storage) = &mut atomic {
            *storage.receipts = Some(self.stage_compound_annotation_receipts(
                storage.invocation,
                &prepared,
                &annotations,
                true,
                recorder,
            )?);
        }
        // All fallible checks and allocations are finished. Exclusive &mut self
        // prevents an intervening observer, mutation or callback at publication.
        #[cfg(test)]
        let mut installed = 0;
        let annotation_changes = std::mem::take(&mut annotations.facts);
        for candidate in prepared.candidates {
            let document = self.documents[candidate.expected.document.slot]
                .as_mut()
                .expect("validated live participant");
            if document.source != candidate.source {
                document.source = candidate.source;
                document.revision += 1;
                #[cfg(test)]
                {
                    installed += 1;
                    assert_ne!(
                        self.publication_fault_after,
                        Some(installed),
                        "injected internal publication fault"
                    );
                }
            }
        }
        annotations.install(self);
        let view_publication = views.changed();
        views.install(self);
        history.install(self);
        self.generation = generation;
        if view_publication {
            observe(
                recorder,
                Boundary::LogicalSelectionPublication,
                &Ok::<(), CoordinatorError>(()),
            );
        }
        Ok(CommitOutcome {
            generation,
            changes,
            operation: Some(OperationId {
                namespace: self.namespace.clone(),
                generation,
            }),
            history_group,
            annotation_changes,
        })
    }
}
