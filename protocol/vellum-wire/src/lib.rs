//! Canonical bounded wire encoding for the Vellum native protocol.
#![forbid(unsafe_code)]

use keystone_codec::{DecodeError, DecodeLimits, Decoder, encode_bytes};
use std::{fmt, sync::Arc};
use vellum_protocol::{
    ClientId, CommandV1, DirectoryAdmissionReceiptV1, DocumentRemovalReceiptV1,
    DocumentVersionObservationV1, DurableEffectIntentV1, DurableEffectReceiptV1,
    DurableExecuteOutcomeV1, DurableExecuteRequestV1, DurableExportCopyFailureV1,
    DurableExportCopyPersistenceV1, DurableExportCopyReceiptV1, DurableOperationRefV1,
    DurableReceiptLookupOutcomeV1, DurableReceiptLookupV1, DurableReceiptRetirementOutcomeV1,
    DurableReceiptRetirementV1, DurableReceiptStatusRequestV1, DurableReceiptStatusV1,
    DurableReplaceAndSaveReceiptV1, DurableSaveAsBindingUnknownReasonV1, DurableSaveAsBindingV1,
    DurableSaveAsDestinationEffectV1, DurableSaveAsFailureV1, DurableSaveAsFenceSetV1,
    DurableSaveAsIntentV1, DurableSaveAsLifecycleV1, DurableSaveAsLookupOutcomeV1,
    DurableSaveAsLookupV1, DurableSaveAsNotActivatedReasonV1, DurableSaveAsNotAttemptedReasonV1,
    DurableSaveAsNotDurableReasonV1, DurableSaveAsPersistenceV1,
    DurableSaveAsPhysicalUnknownReasonV1, DurableSaveAsReceiptV1, DurableSaveAsRetirementV1,
    DurableSaveAsStateV1, DurableSaveAsSubmitOutcomeV1, DurableSaveAsSubmitRequestV1,
    DurableSaveReceiptV1, EditPersistenceV1, FileAdmissionReceiptV1, FileAdmissionStatusV1,
    MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1, MAX_WORKSPACE_EDIT_DOCUMENTS_V1,
    MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1, MAX_WORKSPACE_EDIT_REPLACEMENTS_V1,
    MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1, MembershipRemovalStatusV1, OutcomeV1, RefusalV1,
    ReplaceAndSaveReceiptV1, ReplyV1, RequestId, RequestV1, RootRemovalReceiptV1,
    SaveApplicationV1, SaveReceiptV1, SaveSynchronizationV1, SnapshotV1, VersionWitness,
    ViewCloseStatusV1, ViewOpenReceiptV1, ViewOpenStatusV1, WorkspaceChangeCursorV1,
    WorkspaceChangeGapV1, WorkspaceChangeKindV1, WorkspaceChangePageV1, WorkspaceChangeV1,
    WorkspaceChangesV1, WorkspaceDocumentEditV1, WorkspaceDocumentIdV1, WorkspaceEditReceiptV1,
    WorkspaceEditedDocumentV1, WorkspaceIdV1, WorkspaceInventoryCursorV1, WorkspaceInventoryPageV1,
    WorkspaceInventoryV1, WorkspaceLocatorResolutionV1, WorkspaceReplacementV1, WorkspaceRootIdV1,
    WorkspaceRootKindV1, WorkspaceRootV1,
};

/// Canonical restart-stable durable intent encoding. This profile is separate
/// from `WireRequestV1`: it contains no transport client/request correlation
/// and no session witness.
pub fn encode_durable_intent_v1(
    intent: &DurableEffectIntentV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    let length = durable_intent_length(intent)?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    encode_durable_intent_fields(&mut encoder, intent)?;
    encoder.finish(length)
}

fn encode_durable_intent_fields(
    encoder: &mut Encoder,
    intent: &DurableEffectIntentV1,
) -> Result<(), WireEncodeError> {
    match intent {
        DurableEffectIntentV1::AddFile { locator } => {
            encoder.u8(1)?;
            encoder.text(locator)?;
        }
        DurableEffectIntentV1::AddDirectory { locator } => {
            encoder.u8(2)?;
            encoder.text(locator)?;
        }
        DurableEffectIntentV1::RemoveDocument { document } => {
            encoder.u8(3)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())?;
        }
        DurableEffectIntentV1::RemoveRoot { root } => {
            encoder.u8(4)?;
            encoder.u64(root.workspace().get())?;
            encoder.u64(root.get())?;
        }
        DurableEffectIntentV1::SaveDocument {
            document,
            expected_revision,
        } => {
            encoder.u8(5)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())?;
            encoder.u64(*expected_revision)?;
        }
        DurableEffectIntentV1::ReplaceDocumentAndSave {
            document,
            expected_revision,
            start,
            end,
            inserted,
        } => {
            encoder.u8(6)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())?;
            encoder.u64(*expected_revision)?;
            encoder.u64(*start)?;
            encoder.u64(*end)?;
            encoder.text(inserted)?;
        }
        DurableEffectIntentV1::ExportDocumentCopyExclusive {
            document,
            expected_revision,
            destination,
        } => {
            encoder.u8(7)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())?;
            encoder.u64(*expected_revision)?;
            encoder.text(destination)?;
        }
    }
    Ok(())
}

pub fn decode_durable_intent_v1(
    input: &[u8],
    limits: WireLimits,
) -> Result<DurableEffectIntentV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let intent = decode_durable_intent_fields(&mut decoder)?;
    finish_decode(&decoder)?;
    Ok(intent)
}

fn decode_durable_intent_fields(
    decoder: &mut Decoder<'_>,
) -> Result<DurableEffectIntentV1, WireDecodeError> {
    Ok(match read_u8(decoder)? {
        1 => DurableEffectIntentV1::AddFile {
            locator: read_text(decoder)?,
        },
        2 => DurableEffectIntentV1::AddDirectory {
            locator: read_text(decoder)?,
        },
        3 => DurableEffectIntentV1::RemoveDocument {
            document: workspace_document_id(workspace_id(read_u64(decoder)?)?, read_u64(decoder)?)?,
        },
        4 => DurableEffectIntentV1::RemoveRoot {
            root: workspace_root_id(workspace_id(read_u64(decoder)?)?, read_u64(decoder)?)?,
        },
        5 => DurableEffectIntentV1::SaveDocument {
            document: workspace_document_id(workspace_id(read_u64(decoder)?)?, read_u64(decoder)?)?,
            expected_revision: read_u64(decoder)?,
        },
        6 => DurableEffectIntentV1::ReplaceDocumentAndSave {
            document: workspace_document_id(workspace_id(read_u64(decoder)?)?, read_u64(decoder)?)?,
            expected_revision: read_u64(decoder)?,
            start: read_u64(decoder)?,
            end: read_u64(decoder)?,
            inserted: read_text(decoder)?,
        },
        7 => DurableEffectIntentV1::ExportDocumentCopyExclusive {
            document: workspace_document_id(workspace_id(read_u64(decoder)?)?, read_u64(decoder)?)?,
            expected_revision: read_u64(decoder)?,
            destination: read_text(decoder)?,
        },
        _ => return Err(WireDecodeError::UnknownTag),
    })
}

fn durable_intent_length(intent: &DurableEffectIntentV1) -> Result<usize, WireEncodeError> {
    match intent {
        DurableEffectIntentV1::AddFile { locator }
        | DurableEffectIntentV1::AddDirectory { locator } => {
            check_wire_text_length(locator.len())?;
            checked_add(6, locator.len())
        }
        DurableEffectIntentV1::RemoveDocument { .. } | DurableEffectIntentV1::RemoveRoot { .. } => {
            Ok(18)
        }
        DurableEffectIntentV1::SaveDocument { .. } => Ok(26),
        DurableEffectIntentV1::ReplaceDocumentAndSave { inserted, .. } => {
            check_wire_text_length(inserted.len())?;
            checked_add(46, inserted.len())
        }
        DurableEffectIntentV1::ExportDocumentCopyExclusive { destination, .. } => {
            check_wire_text_length(destination.len())?;
            checked_add(30, destination.len())
        }
    }
}

/// Canonical restart-stable durable outcome encoding used by the operation
/// journal. Unlike an ordinary reply it contains no live session witness.
pub fn encode_durable_outcome_v1(
    outcome: &DurableExecuteOutcomeV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    let fields = validate_durable_outcome(outcome, limits)?;
    check_field_limit(
        fields
            .checked_add(1)
            .ok_or(WireEncodeError::LengthOverflow)?,
        limits,
    )?;
    let length = durable_outcome_length(outcome)?
        .checked_add(1)
        .ok_or(WireEncodeError::LengthOverflow)?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    encode_durable_execute_outcome_fields(&mut encoder, outcome)?;
    encoder.finish(length)
}

pub fn decode_durable_outcome_v1(
    input: &[u8],
    limits: WireLimits,
) -> Result<DurableExecuteOutcomeV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let outcome = decode_durable_execute_outcome_fields(&mut decoder)?;
    finish_decode(&decoder)?;
    Ok(outcome)
}

/// Canonical restart-stable Save As arguments. This value is suitable for the
/// private durable-operation envelope but carries no authority or destination
/// object and therefore cannot execute an effect by itself.
pub fn encode_durable_save_as_intent_v1(
    intent: &DurableSaveAsIntentV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    check_field_limit(6, limits)?;
    check_text_limit(intent.destination().len(), limits)?;
    check_wire_text_length(intent.destination().len())?;
    let length = checked_add(30, intent.destination().len())?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    encoder.u64(intent.document().workspace().get())?;
    encoder.u64(intent.document().get())?;
    encoder.u64(intent.expected_revision())?;
    encoder.u8(match intent.effect() {
        DurableSaveAsDestinationEffectV1::OverwriteExisting => 1,
        DurableSaveAsDestinationEffectV1::CreateExclusive => 2,
    })?;
    encoder.text(intent.destination())?;
    encoder.finish(length)
}

pub fn decode_durable_save_as_intent_v1(
    input: &[u8],
    limits: WireLimits,
) -> Result<DurableSaveAsIntentV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let workspace = workspace_id(read_u64(&mut decoder)?)?;
    let document = workspace_document_id(workspace, read_u64(&mut decoder)?)?;
    let expected_revision = read_u64(&mut decoder)?;
    let effect = match read_u8(&mut decoder)? {
        1 => DurableSaveAsDestinationEffectV1::OverwriteExisting,
        2 => DurableSaveAsDestinationEffectV1::CreateExclusive,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let destination = read_text(&mut decoder)?;
    finish_decode(&decoder)?;
    DurableSaveAsIntentV1::try_new(document, expected_revision, destination, effect)
        .ok_or(WireDecodeError::Malformed)
}

/// Canonical locator-free encoding for one durable Save As receipt. This is a
/// journal/projection value, not an execution request and carries no transport
/// correlation or destination locator.
pub fn encode_durable_save_as_receipt_v1(
    receipt: DurableSaveAsReceiptV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    const LENGTH: usize = 31;
    check_field_limit(10, limits)?;
    let mut encoder = Encoder::new(LENGTH, limits)?;
    encoder.u8(VERSION_V1)?;
    encoder.u64(receipt.document().workspace().get())?;
    encoder.u64(receipt.document().get())?;
    encoder.u64(receipt.source_revision())?;
    encoder.u8(match receipt.persistence() {
        DurableSaveAsPersistenceV1::Durable => 1,
        DurableSaveAsPersistenceV1::NotDurable => 2,
        DurableSaveAsPersistenceV1::Unknown => 3,
        DurableSaveAsPersistenceV1::NotAttempted => 4,
    })?;
    encoder.u8(match receipt.binding() {
        DurableSaveAsBindingV1::Activated => 1,
        DurableSaveAsBindingV1::NotActivated => 2,
        DurableSaveAsBindingV1::Unknown => 3,
    })?;
    let (failure, reason) = encode_durable_save_as_failure(receipt.failure());
    encoder.u8(failure)?;
    encoder.u8(reason)?;
    match receipt.lifecycle() {
        DurableSaveAsLifecycleV1::TerminalReplayable => {
            encoder.u8(1)?;
            encoder.u8(0)?;
        }
        DurableSaveAsLifecycleV1::Fenced(fence) => {
            encoder.u8(2)?;
            encoder.u8(fence.bits())?;
        }
    }
    encoder.finish(LENGTH)
}

pub fn decode_durable_save_as_receipt_v1(
    input: &[u8],
    limits: WireLimits,
) -> Result<DurableSaveAsReceiptV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let workspace = workspace_id(read_u64(&mut decoder)?)?;
    let document = workspace_document_id(workspace, read_u64(&mut decoder)?)?;
    let source_revision = read_u64(&mut decoder)?;
    let persistence = match read_u8(&mut decoder)? {
        1 => DurableSaveAsPersistenceV1::Durable,
        2 => DurableSaveAsPersistenceV1::NotDurable,
        3 => DurableSaveAsPersistenceV1::Unknown,
        4 => DurableSaveAsPersistenceV1::NotAttempted,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let binding = match read_u8(&mut decoder)? {
        1 => DurableSaveAsBindingV1::Activated,
        2 => DurableSaveAsBindingV1::NotActivated,
        3 => DurableSaveAsBindingV1::Unknown,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let failure = decode_durable_save_as_failure(read_u8(&mut decoder)?, read_u8(&mut decoder)?)?;
    let lifecycle_tag = read_u8(&mut decoder)?;
    let fence_bits = read_u8(&mut decoder)?;
    let lifecycle = match (lifecycle_tag, fence_bits) {
        (1, 0) => DurableSaveAsLifecycleV1::TerminalReplayable,
        (2, bits) => DurableSaveAsLifecycleV1::Fenced(
            DurableSaveAsFenceSetV1::try_from_bits(bits).ok_or(WireDecodeError::Malformed)?,
        ),
        _ => return Err(WireDecodeError::Malformed),
    };
    finish_decode(&decoder)?;
    DurableSaveAsReceiptV1::try_new(
        document,
        source_revision,
        persistence,
        binding,
        failure,
        lifecycle,
    )
    .ok_or(WireDecodeError::Malformed)
}

fn encode_durable_save_as_intent_payload(
    encoder: &mut Encoder,
    intent: &DurableSaveAsIntentV1,
    limits: WireLimits,
) -> Result<(), WireEncodeError> {
    encoder.bytes(&encode_durable_save_as_intent_v1(intent, limits)?)
}

fn decode_durable_save_as_intent_payload(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<DurableSaveAsIntentV1, WireDecodeError> {
    decode_durable_save_as_intent_v1(decoder.read_bytes().map_err(map_decode)?, limits)
}

fn encode_durable_save_as_state_fields(
    encoder: &mut Encoder,
    state: &DurableSaveAsStateV1,
    limits: WireLimits,
) -> Result<(), WireEncodeError> {
    if !state.is_congruent() {
        return Err(WireEncodeError::IdentityMismatch);
    }
    match state {
        DurableSaveAsStateV1::Pending => encoder.u8(1),
        DurableSaveAsStateV1::Fenced(receipt) => {
            encoder.u8(2)?;
            encoder.bytes(&encode_durable_save_as_receipt_v1(*receipt, limits)?)
        }
        DurableSaveAsStateV1::Complete(receipt) => {
            encoder.u8(3)?;
            encoder.bytes(&encode_durable_save_as_receipt_v1(*receipt, limits)?)
        }
        DurableSaveAsStateV1::Retired => encoder.u8(4),
    }
}

fn decode_durable_save_as_state_fields(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<DurableSaveAsStateV1, WireDecodeError> {
    let state = match read_u8(decoder)? {
        1 => DurableSaveAsStateV1::Pending,
        2 => DurableSaveAsStateV1::Fenced(decode_durable_save_as_receipt_v1(
            decoder.read_bytes().map_err(map_decode)?,
            limits,
        )?),
        3 => DurableSaveAsStateV1::Complete(decode_durable_save_as_receipt_v1(
            decoder.read_bytes().map_err(map_decode)?,
            limits,
        )?),
        4 => DurableSaveAsStateV1::Retired,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    state
        .is_congruent()
        .then_some(state)
        .ok_or(WireDecodeError::Malformed)
}

fn encode_durable_save_as_submit_outcome_fields(
    encoder: &mut Encoder,
    outcome: &DurableSaveAsSubmitOutcomeV1,
    limits: WireLimits,
) -> Result<(), WireEncodeError> {
    match outcome {
        DurableSaveAsSubmitOutcomeV1::Observed(state) => {
            encoder.u8(1)?;
            encode_durable_save_as_state_fields(encoder, state, limits)
        }
        DurableSaveAsSubmitOutcomeV1::Refused(refusal) => {
            encoder.u8(2)?;
            encoder.u8(refusal_tag(*refusal))
        }
    }
}

fn decode_durable_save_as_submit_outcome_fields(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<DurableSaveAsSubmitOutcomeV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(DurableSaveAsSubmitOutcomeV1::Observed(
            decode_durable_save_as_state_fields(decoder, limits)?,
        )),
        2 => Ok(DurableSaveAsSubmitOutcomeV1::Refused(decode_refusal(
            read_u8(decoder)?,
        )?)),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_durable_save_as_lookup_outcome_fields(
    encoder: &mut Encoder,
    outcome: &DurableSaveAsLookupOutcomeV1,
    limits: WireLimits,
) -> Result<(), WireEncodeError> {
    match outcome {
        DurableSaveAsLookupOutcomeV1::Observed(state) => {
            encoder.u8(1)?;
            encode_durable_save_as_state_fields(encoder, state, limits)
        }
        DurableSaveAsLookupOutcomeV1::RequestConflict => encoder.u8(2),
        DurableSaveAsLookupOutcomeV1::OutcomeUnknown => encoder.u8(3),
        DurableSaveAsLookupOutcomeV1::AbsentOrForeign => encoder.u8(4),
        DurableSaveAsLookupOutcomeV1::Unavailable => encoder.u8(5),
    }
}

fn decode_durable_save_as_lookup_outcome_fields(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<DurableSaveAsLookupOutcomeV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(DurableSaveAsLookupOutcomeV1::Observed(
            decode_durable_save_as_state_fields(decoder, limits)?,
        )),
        2 => Ok(DurableSaveAsLookupOutcomeV1::RequestConflict),
        3 => Ok(DurableSaveAsLookupOutcomeV1::OutcomeUnknown),
        4 => Ok(DurableSaveAsLookupOutcomeV1::AbsentOrForeign),
        5 => Ok(DurableSaveAsLookupOutcomeV1::Unavailable),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_durable_save_as_failure(failure: Option<DurableSaveAsFailureV1>) -> (u8, u8) {
    match failure {
        None => (0, 0),
        Some(DurableSaveAsFailureV1::NotAttempted(reason)) => (
            1,
            match reason {
                DurableSaveAsNotAttemptedReasonV1::AuthorityDenied => 1,
                DurableSaveAsNotAttemptedReasonV1::AuthorityUnavailable => 2,
                DurableSaveAsNotAttemptedReasonV1::StaleSource => 3,
                DurableSaveAsNotAttemptedReasonV1::DestinationRefused => 4,
                DurableSaveAsNotAttemptedReasonV1::Capacity => 5,
                DurableSaveAsNotAttemptedReasonV1::Unsupported => 6,
            },
        ),
        Some(DurableSaveAsFailureV1::NotDurable(reason)) => (
            2,
            match reason {
                DurableSaveAsNotDurableReasonV1::Conflict => 1,
                DurableSaveAsNotDurableReasonV1::Unsupported => 2,
                DurableSaveAsNotDurableReasonV1::Limit => 3,
                DurableSaveAsNotDurableReasonV1::Capacity => 4,
                DurableSaveAsNotDurableReasonV1::Io => 5,
            },
        ),
        Some(DurableSaveAsFailureV1::PhysicalUnknown(reason)) => (
            3,
            match reason {
                DurableSaveAsPhysicalUnknownReasonV1::Io => 1,
                DurableSaveAsPhysicalUnknownReasonV1::Cleanup => 2,
                DurableSaveAsPhysicalUnknownReasonV1::EvidenceIncomplete => 3,
                DurableSaveAsPhysicalUnknownReasonV1::TaskStateUnavailable => 4,
            },
        ),
        Some(DurableSaveAsFailureV1::NotActivated(reason)) => (
            4,
            match reason {
                DurableSaveAsNotActivatedReasonV1::NotAttempted => 1,
                DurableSaveAsNotActivatedReasonV1::ObservedNotApplied => 2,
                DurableSaveAsNotActivatedReasonV1::Refused => 3,
                DurableSaveAsNotActivatedReasonV1::Unavailable => 4,
                DurableSaveAsNotActivatedReasonV1::DestinationChanged => 5,
            },
        ),
        Some(DurableSaveAsFailureV1::BindingUnknown(reason)) => (
            5,
            match reason {
                DurableSaveAsBindingUnknownReasonV1::Faulted => 1,
                DurableSaveAsBindingUnknownReasonV1::TaskStateUnavailable => 2,
            },
        ),
    }
}

fn decode_durable_save_as_failure(
    failure: u8,
    reason: u8,
) -> Result<Option<DurableSaveAsFailureV1>, WireDecodeError> {
    Ok(match (failure, reason) {
        (0, 0) => None,
        (1, 1) => Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::AuthorityDenied,
        )),
        (1, 2) => Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::AuthorityUnavailable,
        )),
        (1, 3) => Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::StaleSource,
        )),
        (1, 4) => Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::DestinationRefused,
        )),
        (1, 5) => Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::Capacity,
        )),
        (1, 6) => Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::Unsupported,
        )),
        (2, 1) => Some(DurableSaveAsFailureV1::NotDurable(
            DurableSaveAsNotDurableReasonV1::Conflict,
        )),
        (2, 2) => Some(DurableSaveAsFailureV1::NotDurable(
            DurableSaveAsNotDurableReasonV1::Unsupported,
        )),
        (2, 3) => Some(DurableSaveAsFailureV1::NotDurable(
            DurableSaveAsNotDurableReasonV1::Limit,
        )),
        (2, 4) => Some(DurableSaveAsFailureV1::NotDurable(
            DurableSaveAsNotDurableReasonV1::Capacity,
        )),
        (2, 5) => Some(DurableSaveAsFailureV1::NotDurable(
            DurableSaveAsNotDurableReasonV1::Io,
        )),
        (3, 1) => Some(DurableSaveAsFailureV1::PhysicalUnknown(
            DurableSaveAsPhysicalUnknownReasonV1::Io,
        )),
        (3, 2) => Some(DurableSaveAsFailureV1::PhysicalUnknown(
            DurableSaveAsPhysicalUnknownReasonV1::Cleanup,
        )),
        (3, 3) => Some(DurableSaveAsFailureV1::PhysicalUnknown(
            DurableSaveAsPhysicalUnknownReasonV1::EvidenceIncomplete,
        )),
        (3, 4) => Some(DurableSaveAsFailureV1::PhysicalUnknown(
            DurableSaveAsPhysicalUnknownReasonV1::TaskStateUnavailable,
        )),
        (4, 1) => Some(DurableSaveAsFailureV1::NotActivated(
            DurableSaveAsNotActivatedReasonV1::NotAttempted,
        )),
        (4, 2) => Some(DurableSaveAsFailureV1::NotActivated(
            DurableSaveAsNotActivatedReasonV1::ObservedNotApplied,
        )),
        (4, 3) => Some(DurableSaveAsFailureV1::NotActivated(
            DurableSaveAsNotActivatedReasonV1::Refused,
        )),
        (4, 4) => Some(DurableSaveAsFailureV1::NotActivated(
            DurableSaveAsNotActivatedReasonV1::Unavailable,
        )),
        (4, 5) => Some(DurableSaveAsFailureV1::NotActivated(
            DurableSaveAsNotActivatedReasonV1::DestinationChanged,
        )),
        (5, 1) => Some(DurableSaveAsFailureV1::BindingUnknown(
            DurableSaveAsBindingUnknownReasonV1::Faulted,
        )),
        (5, 2) => Some(DurableSaveAsFailureV1::BindingUnknown(
            DurableSaveAsBindingUnknownReasonV1::TaskStateUnavailable,
        )),
        _ => return Err(WireDecodeError::UnknownTag),
    })
}

fn encode_operation_ref(
    encoder: &mut Encoder,
    operation: DurableOperationRefV1,
) -> Result<(), WireEncodeError> {
    let bytes = operation.to_bytes();
    encoder.u64(u64::from_be_bytes(
        bytes[..8].try_into().expect("fixed width"),
    ))?;
    encoder.u64(u64::from_be_bytes(
        bytes[8..].try_into().expect("fixed width"),
    ))
}

fn decode_operation_ref(
    decoder: &mut Decoder<'_>,
) -> Result<DurableOperationRefV1, WireDecodeError> {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&read_u64(decoder)?.to_be_bytes());
    bytes[8..].copy_from_slice(&read_u64(decoder)?.to_be_bytes());
    DurableOperationRefV1::new(bytes).ok_or(WireDecodeError::InvalidIdentity)
}

fn encode_durable_execute_outcome_fields(
    encoder: &mut Encoder,
    outcome: &DurableExecuteOutcomeV1,
) -> Result<(), WireEncodeError> {
    match outcome {
        DurableExecuteOutcomeV1::Completed(receipt) => {
            encoder.u8(1)?;
            encode_durable_receipt(encoder, receipt)
        }
        DurableExecuteOutcomeV1::Refused(refusal) => {
            encoder.u8(2)?;
            encoder.u8(refusal_tag(*refusal))
        }
        DurableExecuteOutcomeV1::Retired => encoder.u8(3),
    }
}

fn decode_durable_execute_outcome_fields(
    decoder: &mut Decoder<'_>,
) -> Result<DurableExecuteOutcomeV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(DurableExecuteOutcomeV1::Completed(decode_durable_receipt(
            decoder,
        )?)),
        2 => Ok(DurableExecuteOutcomeV1::Refused(decode_refusal(read_u8(
            decoder,
        )?)?)),
        3 => Ok(DurableExecuteOutcomeV1::Retired),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_durable_lookup_outcome_fields(
    encoder: &mut Encoder,
    outcome: &DurableReceiptLookupOutcomeV1,
) -> Result<(), WireEncodeError> {
    match outcome {
        DurableReceiptLookupOutcomeV1::Complete(outcome) => {
            encoder.u8(1)?;
            encode_durable_execute_outcome_fields(encoder, outcome)
        }
        DurableReceiptLookupOutcomeV1::RequestConflict => encoder.u8(2),
        DurableReceiptLookupOutcomeV1::OutcomeUnknown => encoder.u8(3),
        DurableReceiptLookupOutcomeV1::AbsentOrForeign => encoder.u8(4),
        DurableReceiptLookupOutcomeV1::Unavailable => encoder.u8(5),
    }
}

fn decode_durable_lookup_outcome_fields(
    decoder: &mut Decoder<'_>,
) -> Result<DurableReceiptLookupOutcomeV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(DurableReceiptLookupOutcomeV1::Complete(
            decode_durable_execute_outcome_fields(decoder)?,
        )),
        2 => Ok(DurableReceiptLookupOutcomeV1::RequestConflict),
        3 => Ok(DurableReceiptLookupOutcomeV1::OutcomeUnknown),
        4 => Ok(DurableReceiptLookupOutcomeV1::AbsentOrForeign),
        5 => Ok(DurableReceiptLookupOutcomeV1::Unavailable),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_durable_receipt(
    encoder: &mut Encoder,
    receipt: &DurableEffectReceiptV1,
) -> Result<(), WireEncodeError> {
    match receipt {
        DurableEffectReceiptV1::FileAdmitted(receipt) => {
            encoder.u8(1)?;
            encode_file_admission(encoder, receipt)
        }
        DurableEffectReceiptV1::DirectoryAdmitted(receipt) => {
            encoder.u8(2)?;
            encode_directory_admission(encoder, receipt)
        }
        DurableEffectReceiptV1::DocumentRemoved(receipt) => {
            encoder.u8(3)?;
            encoder.u64(receipt.document.workspace().get())?;
            encoder.u64(receipt.document.get())?;
            encode_membership_removal_status(encoder, receipt.status)
        }
        DurableEffectReceiptV1::RootRemoved(receipt) => {
            encoder.u8(4)?;
            encoder.u64(receipt.root.workspace().get())?;
            encoder.u64(receipt.root.get())?;
            encode_membership_removal_status(encoder, receipt.status)
        }
        DurableEffectReceiptV1::DocumentSaved(receipt) => {
            encoder.u8(5)?;
            encode_durable_save(encoder, receipt)
        }
        DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt) => {
            encoder.u8(6)?;
            encode_durable_replace_and_save(encoder, receipt)
        }
        DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt) => {
            encoder.u8(7)?;
            encode_durable_export_copy(encoder, receipt)
        }
    }
}

fn decode_durable_receipt(
    decoder: &mut Decoder<'_>,
) -> Result<DurableEffectReceiptV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(DurableEffectReceiptV1::FileAdmitted(decode_file_admission(
            decoder,
        )?)),
        2 => Ok(DurableEffectReceiptV1::DirectoryAdmitted(
            decode_directory_admission(decoder)?,
        )),
        3 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Ok(DurableEffectReceiptV1::DocumentRemoved(
                DocumentRemovalReceiptV1 {
                    document: workspace_document_id(workspace, read_u64(decoder)?)?,
                    status: decode_membership_removal_status(decoder)?,
                },
            ))
        }
        4 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Ok(DurableEffectReceiptV1::RootRemoved(RootRemovalReceiptV1 {
                root: workspace_root_id(workspace, read_u64(decoder)?)?,
                status: decode_membership_removal_status(decoder)?,
            }))
        }
        5 => Ok(DurableEffectReceiptV1::DocumentSaved(decode_durable_save(
            decoder,
        )?)),
        6 => Ok(DurableEffectReceiptV1::DocumentReplacedAndSaved(
            decode_durable_replace_and_save(decoder)?,
        )),
        7 => Ok(DurableEffectReceiptV1::DocumentCopyExportedExclusive(
            decode_durable_export_copy(decoder)?,
        )),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_durable_export_copy(
    encoder: &mut Encoder,
    receipt: &DurableExportCopyReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u64(receipt.document().workspace().get())?;
    encoder.u64(receipt.document().get())?;
    encoder.u64(receipt.source_revision())?;
    encoder.u8(match receipt.persistence() {
        DurableExportCopyPersistenceV1::Durable => 1,
        DurableExportCopyPersistenceV1::NotDurable => 2,
        DurableExportCopyPersistenceV1::Unknown => 3,
        DurableExportCopyPersistenceV1::NotAttempted => 4,
    })?;
    encoder.u8(match receipt.failure() {
        None => 0,
        Some(DurableExportCopyFailureV1::Conflict) => 1,
        Some(DurableExportCopyFailureV1::Unsupported) => 2,
        Some(DurableExportCopyFailureV1::Limit) => 3,
        Some(DurableExportCopyFailureV1::Capacity) => 4,
        Some(DurableExportCopyFailureV1::Io) => 5,
    })?;
    encoder.u8(u8::from(receipt.cleanup_failed()))
}

fn decode_durable_export_copy(
    decoder: &mut Decoder<'_>,
) -> Result<DurableExportCopyReceiptV1, WireDecodeError> {
    let workspace = workspace_id(read_u64(decoder)?)?;
    let document = workspace_document_id(workspace, read_u64(decoder)?)?;
    let source_revision = read_u64(decoder)?;
    let persistence = match read_u8(decoder)? {
        1 => DurableExportCopyPersistenceV1::Durable,
        2 => DurableExportCopyPersistenceV1::NotDurable,
        3 => DurableExportCopyPersistenceV1::Unknown,
        4 => DurableExportCopyPersistenceV1::NotAttempted,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let failure = match read_u8(decoder)? {
        0 => None,
        1 => Some(DurableExportCopyFailureV1::Conflict),
        2 => Some(DurableExportCopyFailureV1::Unsupported),
        3 => Some(DurableExportCopyFailureV1::Limit),
        4 => Some(DurableExportCopyFailureV1::Capacity),
        5 => Some(DurableExportCopyFailureV1::Io),
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let cleanup_failed = match read_u8(decoder)? {
        0 => false,
        1 => true,
        _ => return Err(WireDecodeError::Malformed),
    };
    DurableExportCopyReceiptV1::try_new(
        document,
        source_revision,
        persistence,
        failure,
        cleanup_failed,
    )
    .ok_or(WireDecodeError::Malformed)
}

fn encode_durable_save(
    encoder: &mut Encoder,
    receipt: &DurableSaveReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u64(receipt.document.workspace().get())?;
    encoder.u64(receipt.document.get())?;
    encoder.u64(receipt.source_revision)?;
    encode_compact_save_receipt(encoder, receipt.persistence)
}

fn decode_durable_save(decoder: &mut Decoder<'_>) -> Result<DurableSaveReceiptV1, WireDecodeError> {
    let workspace = workspace_id(read_u64(decoder)?)?;
    Ok(DurableSaveReceiptV1 {
        document: workspace_document_id(workspace, read_u64(decoder)?)?,
        source_revision: read_u64(decoder)?,
        persistence: decode_compact_save_receipt(decoder)?,
    })
}

fn encode_durable_replace_and_save(
    encoder: &mut Encoder,
    receipt: &DurableReplaceAndSaveReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u64(receipt.document.workspace().get())?;
    encoder.u64(receipt.document.get())?;
    encoder.u64(receipt.source_revision)?;
    encoder.text(&receipt.text)?;
    encode_compact_edit_persistence(encoder, receipt.persistence)
}

fn decode_durable_replace_and_save(
    decoder: &mut Decoder<'_>,
) -> Result<DurableReplaceAndSaveReceiptV1, WireDecodeError> {
    let workspace = workspace_id(read_u64(decoder)?)?;
    Ok(DurableReplaceAndSaveReceiptV1 {
        document: workspace_document_id(workspace, read_u64(decoder)?)?,
        source_revision: read_u64(decoder)?,
        text: Arc::from(read_text(decoder)?),
        persistence: decode_compact_edit_persistence(decoder)?,
    })
}

fn encode_compact_save_receipt(
    encoder: &mut Encoder,
    receipt: SaveReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u8(match receipt.application {
        SaveApplicationV1::Unknown => 1,
        SaveApplicationV1::NotWritten => 2,
        SaveApplicationV1::Written => 3,
    })?;
    encoder.u8(match receipt.synchronization {
        SaveSynchronizationV1::Unknown => 1,
        SaveSynchronizationV1::NotRequested => 2,
        SaveSynchronizationV1::Succeeded => 3,
        SaveSynchronizationV1::Failed => 4,
    })?;
    let flags = u8::from(receipt.no_future_effects)
        | (u8::from(receipt.quiescent) << 1)
        | (u8::from(receipt.adapter_failure) << 2)
        | (u8::from(receipt.cleanup_failed) << 3)
        | (u8::from(receipt.baseline_published) << 4)
        | (u8::from(receipt.outcome_unknown) << 5);
    encoder.u8(flags)
}

fn decode_compact_save_receipt(
    decoder: &mut Decoder<'_>,
) -> Result<SaveReceiptV1, WireDecodeError> {
    let application = match read_u8(decoder)? {
        1 => SaveApplicationV1::Unknown,
        2 => SaveApplicationV1::NotWritten,
        3 => SaveApplicationV1::Written,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let synchronization = match read_u8(decoder)? {
        1 => SaveSynchronizationV1::Unknown,
        2 => SaveSynchronizationV1::NotRequested,
        3 => SaveSynchronizationV1::Succeeded,
        4 => SaveSynchronizationV1::Failed,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let flags = read_u8(decoder)?;
    if flags & !0x3f != 0 {
        return Err(WireDecodeError::Malformed);
    }
    Ok(SaveReceiptV1 {
        application,
        synchronization,
        no_future_effects: flags & 1 != 0,
        quiescent: flags & 2 != 0,
        adapter_failure: flags & 4 != 0,
        cleanup_failed: flags & 8 != 0,
        baseline_published: flags & 16 != 0,
        outcome_unknown: flags & 32 != 0,
    })
}

fn encode_compact_edit_persistence(
    encoder: &mut Encoder,
    persistence: EditPersistenceV1,
) -> Result<(), WireEncodeError> {
    match persistence {
        EditPersistenceV1::Durable(receipt) => {
            encoder.u8(1)?;
            encode_compact_save_receipt(encoder, receipt)
        }
        EditPersistenceV1::NotDurable(receipt) => {
            encoder.u8(2)?;
            encode_compact_save_receipt(encoder, receipt)
        }
        EditPersistenceV1::Unknown(receipt) => {
            encoder.u8(3)?;
            encode_compact_save_receipt(encoder, receipt)
        }
        EditPersistenceV1::NotAttempted(refusal) => {
            encoder.u8(4)?;
            encoder.u8(refusal_tag(refusal))
        }
    }
}

fn decode_compact_edit_persistence(
    decoder: &mut Decoder<'_>,
) -> Result<EditPersistenceV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(EditPersistenceV1::Durable(decode_compact_save_receipt(
            decoder,
        )?)),
        2 => Ok(EditPersistenceV1::NotDurable(decode_compact_save_receipt(
            decoder,
        )?)),
        3 => Ok(EditPersistenceV1::Unknown(decode_compact_save_receipt(
            decoder,
        )?)),
        4 => Ok(EditPersistenceV1::NotAttempted(decode_refusal(read_u8(
            decoder,
        )?)?)),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

const VERSION_V1: u8 = 1;
const CONNECT_REQUEST: u8 = 0x01;
const EXECUTE_REQUEST: u8 = 0x02;
const DISCONNECT_REQUEST: u8 = 0x03;
const DURABLE_EXECUTE_REQUEST: u8 = 0x04;
const DURABLE_RECEIPT_LOOKUP_REQUEST: u8 = 0x05;
const DURABLE_RECEIPT_RETIRE_REQUEST: u8 = 0x06;
const DURABLE_RECEIPT_STATUS_REQUEST: u8 = 0x07;
const DURABLE_SAVE_AS_SUBMIT_REQUEST: u8 = 0x08;
const DURABLE_SAVE_AS_LOOKUP_REQUEST: u8 = 0x09;
const DURABLE_SAVE_AS_RETIRE_REQUEST: u8 = 0x0a;
const CONNECTED_REPLY: u8 = 0x81;
const CONNECT_REFUSED_REPLY: u8 = 0x82;
const EXECUTED_REPLY: u8 = 0x83;
const DISCONNECTED_REPLY: u8 = 0x84;
const DISCONNECT_REFUSED_REPLY: u8 = 0x85;
const DURABLE_EXECUTED_REPLY: u8 = 0x86;
const DURABLE_RECEIPT_LOOKED_UP_REPLY: u8 = 0x87;
const DURABLE_RECEIPT_RETIRED_REPLY: u8 = 0x88;
const DURABLE_RECEIPT_STATUS_REPLY: u8 = 0x89;
const DURABLE_SAVE_AS_SUBMITTED_REPLY: u8 = 0x8a;
const DURABLE_SAVE_AS_LOOKED_UP_REPLY: u8 = 0x8b;
const DURABLE_SAVE_AS_RETIRED_REPLY: u8 = 0x8c;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireLimits {
    pub max_frame_bytes: usize,
    pub max_text_bytes: usize,
    pub max_fields: usize,
}

impl WireLimits {
    pub const fn reference() -> Self {
        Self {
            max_frame_bytes: 65_600,
            max_text_bytes: 65_536,
            max_fields: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireEncodeError {
    Limit,
    Capacity,
    LengthOverflow,
    IdentityMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireDecodeError {
    Limit,
    Malformed,
    UnsupportedVersion,
    UnknownTag,
    InvalidIdentity,
    InvalidUtf8,
    TrailingData,
    Capacity,
}

#[derive(Clone, PartialEq, Eq)]
pub enum WireRequestV1 {
    Connect,
    Execute(RequestV1),
    Disconnect(ClientId),
    DurableExecute(DurableExecuteRequestV1),
    DurableReceiptLookup(DurableReceiptLookupV1),
    DurableReceiptRetirement(DurableReceiptRetirementV1),
    DurableReceiptStatus(DurableReceiptStatusRequestV1),
    DurableSaveAsSubmit(DurableSaveAsSubmitRequestV1),
    DurableSaveAsLookup(DurableSaveAsLookupV1),
    DurableSaveAsRetirement(DurableSaveAsRetirementV1),
}

impl fmt::Debug for WireRequestV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("Connect"),
            Self::Execute(request) => f.debug_tuple("Execute").field(request).finish(),
            Self::Disconnect(client) => f.debug_tuple("Disconnect").field(client).finish(),
            Self::DurableExecute(request) => {
                f.debug_tuple("DurableExecute").field(request).finish()
            }
            Self::DurableReceiptLookup(lookup) => {
                f.debug_tuple("DurableReceiptLookup").field(lookup).finish()
            }
            Self::DurableReceiptRetirement(retirement) => f
                .debug_tuple("DurableReceiptRetirement")
                .field(retirement)
                .finish(),
            Self::DurableReceiptStatus(request) => f
                .debug_tuple("DurableReceiptStatus")
                .field(request)
                .finish(),
            Self::DurableSaveAsSubmit(request) => {
                f.debug_tuple("DurableSaveAsSubmit").field(request).finish()
            }
            Self::DurableSaveAsLookup(lookup) => {
                f.debug_tuple("DurableSaveAsLookup").field(lookup).finish()
            }
            Self::DurableSaveAsRetirement(retirement) => f
                .debug_tuple("DurableSaveAsRetirement")
                .field(retirement)
                .finish(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum WireReplyV1 {
    Connected(ClientId),
    ConnectRefused(RefusalV1),
    Executed(ReplyV1),
    Disconnected,
    DisconnectRefused(RefusalV1),
    DurableExecuted(DurableExecuteOutcomeV1),
    DurableReceiptLookedUp(DurableReceiptLookupOutcomeV1),
    DurableReceiptRetired(DurableReceiptRetirementOutcomeV1),
    DurableReceiptStatus(DurableReceiptStatusV1),
    DurableSaveAsSubmitted(DurableSaveAsSubmitOutcomeV1),
    DurableSaveAsLookedUp(DurableSaveAsLookupOutcomeV1),
    DurableSaveAsRetired(DurableReceiptRetirementOutcomeV1),
}

impl fmt::Debug for WireReplyV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connected(client) => f.debug_tuple("Connected").field(client).finish(),
            Self::ConnectRefused(refusal) => {
                f.debug_tuple("ConnectRefused").field(refusal).finish()
            }
            Self::Executed(reply) => f.debug_tuple("Executed").field(reply).finish(),
            Self::Disconnected => f.write_str("Disconnected"),
            Self::DisconnectRefused(refusal) => {
                f.debug_tuple("DisconnectRefused").field(refusal).finish()
            }
            Self::DurableExecuted(outcome) => {
                f.debug_tuple("DurableExecuted").field(outcome).finish()
            }
            Self::DurableReceiptLookedUp(outcome) => f
                .debug_tuple("DurableReceiptLookedUp")
                .field(outcome)
                .finish(),
            Self::DurableReceiptRetired(outcome) => f
                .debug_tuple("DurableReceiptRetired")
                .field(outcome)
                .finish(),
            Self::DurableReceiptStatus(status) => {
                f.debug_tuple("DurableReceiptStatus").field(status).finish()
            }
            Self::DurableSaveAsSubmitted(outcome) => f
                .debug_tuple("DurableSaveAsSubmitted")
                .field(outcome)
                .finish(),
            Self::DurableSaveAsLookedUp(outcome) => f
                .debug_tuple("DurableSaveAsLookedUp")
                .field(outcome)
                .finish(),
            Self::DurableSaveAsRetired(outcome) => f
                .debug_tuple("DurableSaveAsRetired")
                .field(outcome)
                .finish(),
        }
    }
}

pub fn encode_request_v1(
    request: &WireRequestV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    validate_request(request, limits)?;
    let length = request_length(request)?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    match request {
        WireRequestV1::Connect => encoder.u8(CONNECT_REQUEST)?,
        WireRequestV1::Execute(request) => {
            encoder.u8(EXECUTE_REQUEST)?;
            encoder.u64(request.client.get())?;
            encoder.u64(request.request.get())?;
            encode_command(&mut encoder, &request.command)?;
        }
        WireRequestV1::Disconnect(client) => {
            encoder.u8(DISCONNECT_REQUEST)?;
            encoder.u64(client.get())?;
        }
        WireRequestV1::DurableExecute(request) => {
            encoder.u8(DURABLE_EXECUTE_REQUEST)?;
            encode_operation_ref(&mut encoder, request.operation)?;
            encoder.u64(request.client.get())?;
            encoder.u64(request.request.get())?;
            encode_durable_intent_fields(&mut encoder, &request.intent)?;
        }
        WireRequestV1::DurableReceiptLookup(lookup) => {
            encoder.u8(DURABLE_RECEIPT_LOOKUP_REQUEST)?;
            encode_operation_ref(&mut encoder, lookup.operation)?;
            encoder.u64(lookup.client.get())?;
            encoder.u64(lookup.request.get())?;
            encode_durable_intent_fields(&mut encoder, &lookup.intent)?;
        }
        WireRequestV1::DurableReceiptRetirement(retirement) => {
            encoder.u8(DURABLE_RECEIPT_RETIRE_REQUEST)?;
            encode_operation_ref(&mut encoder, retirement.operation)?;
            encoder.u64(retirement.client.get())?;
            encoder.u64(retirement.request.get())?;
            encode_durable_intent_fields(&mut encoder, &retirement.intent)?;
        }
        WireRequestV1::DurableReceiptStatus(request) => {
            encoder.u8(DURABLE_RECEIPT_STATUS_REQUEST)?;
            encode_operation_ref(&mut encoder, request.operation)?;
            encoder.u64(request.client.get())?;
            encoder.u64(request.request.get())?;
        }
        WireRequestV1::DurableSaveAsSubmit(request) => {
            encoder.u8(DURABLE_SAVE_AS_SUBMIT_REQUEST)?;
            encode_operation_ref(&mut encoder, request.operation)?;
            encoder.u64(request.client.get())?;
            encoder.u64(request.request.get())?;
            encode_durable_save_as_intent_payload(&mut encoder, &request.intent, limits)?;
        }
        WireRequestV1::DurableSaveAsLookup(lookup) => {
            encoder.u8(DURABLE_SAVE_AS_LOOKUP_REQUEST)?;
            encode_operation_ref(&mut encoder, lookup.operation)?;
            encoder.u64(lookup.client.get())?;
            encoder.u64(lookup.request.get())?;
            encode_durable_save_as_intent_payload(&mut encoder, &lookup.intent, limits)?;
        }
        WireRequestV1::DurableSaveAsRetirement(retirement) => {
            encoder.u8(DURABLE_SAVE_AS_RETIRE_REQUEST)?;
            encode_operation_ref(&mut encoder, retirement.operation)?;
            encoder.u64(retirement.client.get())?;
            encoder.u64(retirement.request.get())?;
            encode_durable_save_as_intent_payload(&mut encoder, &retirement.intent, limits)?;
        }
    }
    encoder.finish(length)
}

pub fn decode_request_v1(
    input: &[u8],
    limits: WireLimits,
) -> Result<WireRequestV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let request = match read_u8(&mut decoder)? {
        CONNECT_REQUEST => WireRequestV1::Connect,
        EXECUTE_REQUEST => {
            let client = client_id(read_u64(&mut decoder)?)?;
            let request = request_id(read_u64(&mut decoder)?)?;
            let command = decode_command(&mut decoder)?;
            WireRequestV1::Execute(RequestV1 {
                client,
                request,
                command,
            })
        }
        DISCONNECT_REQUEST => WireRequestV1::Disconnect(client_id(read_u64(&mut decoder)?)?),
        DURABLE_EXECUTE_REQUEST => WireRequestV1::DurableExecute(DurableExecuteRequestV1 {
            operation: decode_operation_ref(&mut decoder)?,
            client: client_id(read_u64(&mut decoder)?)?,
            request: request_id(read_u64(&mut decoder)?)?,
            intent: decode_durable_intent_fields(&mut decoder)?,
        }),
        DURABLE_RECEIPT_LOOKUP_REQUEST => {
            WireRequestV1::DurableReceiptLookup(DurableReceiptLookupV1 {
                operation: decode_operation_ref(&mut decoder)?,
                client: client_id(read_u64(&mut decoder)?)?,
                request: request_id(read_u64(&mut decoder)?)?,
                intent: decode_durable_intent_fields(&mut decoder)?,
            })
        }
        DURABLE_RECEIPT_RETIRE_REQUEST => {
            WireRequestV1::DurableReceiptRetirement(DurableReceiptRetirementV1 {
                operation: decode_operation_ref(&mut decoder)?,
                client: client_id(read_u64(&mut decoder)?)?,
                request: request_id(read_u64(&mut decoder)?)?,
                intent: decode_durable_intent_fields(&mut decoder)?,
            })
        }
        DURABLE_RECEIPT_STATUS_REQUEST => {
            WireRequestV1::DurableReceiptStatus(DurableReceiptStatusRequestV1 {
                operation: decode_operation_ref(&mut decoder)?,
                client: client_id(read_u64(&mut decoder)?)?,
                request: request_id(read_u64(&mut decoder)?)?,
            })
        }
        DURABLE_SAVE_AS_SUBMIT_REQUEST => {
            WireRequestV1::DurableSaveAsSubmit(DurableSaveAsSubmitRequestV1 {
                operation: decode_operation_ref(&mut decoder)?,
                client: client_id(read_u64(&mut decoder)?)?,
                request: request_id(read_u64(&mut decoder)?)?,
                intent: decode_durable_save_as_intent_payload(&mut decoder, limits)?,
            })
        }
        DURABLE_SAVE_AS_LOOKUP_REQUEST => {
            WireRequestV1::DurableSaveAsLookup(DurableSaveAsLookupV1 {
                operation: decode_operation_ref(&mut decoder)?,
                client: client_id(read_u64(&mut decoder)?)?,
                request: request_id(read_u64(&mut decoder)?)?,
                intent: decode_durable_save_as_intent_payload(&mut decoder, limits)?,
            })
        }
        DURABLE_SAVE_AS_RETIRE_REQUEST => {
            WireRequestV1::DurableSaveAsRetirement(DurableSaveAsRetirementV1 {
                operation: decode_operation_ref(&mut decoder)?,
                client: client_id(read_u64(&mut decoder)?)?,
                request: request_id(read_u64(&mut decoder)?)?,
                intent: decode_durable_save_as_intent_payload(&mut decoder, limits)?,
            })
        }
        _ => return Err(WireDecodeError::UnknownTag),
    };
    finish_decode(&decoder)?;
    Ok(request)
}

pub fn encode_reply_v1(
    reply: &WireReplyV1,
    limits: WireLimits,
) -> Result<Vec<u8>, WireEncodeError> {
    validate_reply(reply, limits)?;
    let length = reply_length(reply)?;
    let mut encoder = Encoder::new(length, limits)?;
    encoder.u8(VERSION_V1)?;
    match reply {
        WireReplyV1::Connected(client) => {
            encoder.u8(CONNECTED_REPLY)?;
            encoder.u64(client.get())?;
        }
        WireReplyV1::ConnectRefused(refusal) => {
            encoder.u8(CONNECT_REFUSED_REPLY)?;
            encoder.u8(refusal_tag(*refusal))?;
        }
        WireReplyV1::Executed(reply) => {
            encoder.u8(EXECUTED_REPLY)?;
            encode_protocol_reply(&mut encoder, reply)?;
        }
        WireReplyV1::Disconnected => encoder.u8(DISCONNECTED_REPLY)?,
        WireReplyV1::DisconnectRefused(refusal) => {
            encoder.u8(DISCONNECT_REFUSED_REPLY)?;
            encoder.u8(refusal_tag(*refusal))?;
        }
        WireReplyV1::DurableExecuted(outcome) => {
            encoder.u8(DURABLE_EXECUTED_REPLY)?;
            encode_durable_execute_outcome_fields(&mut encoder, outcome)?;
        }
        WireReplyV1::DurableReceiptLookedUp(outcome) => {
            encoder.u8(DURABLE_RECEIPT_LOOKED_UP_REPLY)?;
            encode_durable_lookup_outcome_fields(&mut encoder, outcome)?;
        }
        WireReplyV1::DurableReceiptRetired(outcome) => {
            encoder.u8(DURABLE_RECEIPT_RETIRED_REPLY)?;
            encoder.u8(match outcome {
                DurableReceiptRetirementOutcomeV1::Retired => 1,
                DurableReceiptRetirementOutcomeV1::AbsentOrForeign => 2,
                DurableReceiptRetirementOutcomeV1::RequestConflict => 3,
                DurableReceiptRetirementOutcomeV1::OutcomeUnknown => 4,
                DurableReceiptRetirementOutcomeV1::Unavailable => 5,
            })?;
        }
        WireReplyV1::DurableReceiptStatus(status) => {
            encoder.u8(DURABLE_RECEIPT_STATUS_REPLY)?;
            encoder.u8(match status {
                DurableReceiptStatusV1::Pending => 1,
                DurableReceiptStatusV1::Complete => 2,
                DurableReceiptStatusV1::Retired => 3,
                DurableReceiptStatusV1::AbsentOrForeign => 4,
                DurableReceiptStatusV1::Unavailable => 5,
            })?;
        }
        WireReplyV1::DurableSaveAsSubmitted(outcome) => {
            encoder.u8(DURABLE_SAVE_AS_SUBMITTED_REPLY)?;
            encode_durable_save_as_submit_outcome_fields(&mut encoder, outcome, limits)?;
        }
        WireReplyV1::DurableSaveAsLookedUp(outcome) => {
            encoder.u8(DURABLE_SAVE_AS_LOOKED_UP_REPLY)?;
            encode_durable_save_as_lookup_outcome_fields(&mut encoder, outcome, limits)?;
        }
        WireReplyV1::DurableSaveAsRetired(outcome) => {
            encoder.u8(DURABLE_SAVE_AS_RETIRED_REPLY)?;
            encoder.u8(match outcome {
                DurableReceiptRetirementOutcomeV1::Retired => 1,
                DurableReceiptRetirementOutcomeV1::AbsentOrForeign => 2,
                DurableReceiptRetirementOutcomeV1::RequestConflict => 3,
                DurableReceiptRetirementOutcomeV1::OutcomeUnknown => 4,
                DurableReceiptRetirementOutcomeV1::Unavailable => 5,
            })?;
        }
    }
    encoder.finish(length)
}

pub fn decode_reply_v1(input: &[u8], limits: WireLimits) -> Result<WireReplyV1, WireDecodeError> {
    let mut decoder = decoder(input, limits)?;
    read_version(&mut decoder)?;
    let reply = match read_u8(&mut decoder)? {
        CONNECTED_REPLY => WireReplyV1::Connected(client_id(read_u64(&mut decoder)?)?),
        CONNECT_REFUSED_REPLY => {
            WireReplyV1::ConnectRefused(decode_refusal(read_u8(&mut decoder)?)?)
        }
        EXECUTED_REPLY => WireReplyV1::Executed(decode_protocol_reply(&mut decoder, limits)?),
        DISCONNECTED_REPLY => WireReplyV1::Disconnected,
        DISCONNECT_REFUSED_REPLY => {
            WireReplyV1::DisconnectRefused(decode_refusal(read_u8(&mut decoder)?)?)
        }
        DURABLE_EXECUTED_REPLY => {
            WireReplyV1::DurableExecuted(decode_durable_execute_outcome_fields(&mut decoder)?)
        }
        DURABLE_RECEIPT_LOOKED_UP_REPLY => {
            WireReplyV1::DurableReceiptLookedUp(decode_durable_lookup_outcome_fields(&mut decoder)?)
        }
        DURABLE_RECEIPT_RETIRED_REPLY => {
            WireReplyV1::DurableReceiptRetired(match read_u8(&mut decoder)? {
                1 => DurableReceiptRetirementOutcomeV1::Retired,
                2 => DurableReceiptRetirementOutcomeV1::AbsentOrForeign,
                3 => DurableReceiptRetirementOutcomeV1::RequestConflict,
                4 => DurableReceiptRetirementOutcomeV1::OutcomeUnknown,
                5 => DurableReceiptRetirementOutcomeV1::Unavailable,
                _ => return Err(WireDecodeError::UnknownTag),
            })
        }
        DURABLE_RECEIPT_STATUS_REPLY => {
            WireReplyV1::DurableReceiptStatus(match read_u8(&mut decoder)? {
                1 => DurableReceiptStatusV1::Pending,
                2 => DurableReceiptStatusV1::Complete,
                3 => DurableReceiptStatusV1::Retired,
                4 => DurableReceiptStatusV1::AbsentOrForeign,
                5 => DurableReceiptStatusV1::Unavailable,
                _ => return Err(WireDecodeError::UnknownTag),
            })
        }
        DURABLE_SAVE_AS_SUBMITTED_REPLY => WireReplyV1::DurableSaveAsSubmitted(
            decode_durable_save_as_submit_outcome_fields(&mut decoder, limits)?,
        ),
        DURABLE_SAVE_AS_LOOKED_UP_REPLY => WireReplyV1::DurableSaveAsLookedUp(
            decode_durable_save_as_lookup_outcome_fields(&mut decoder, limits)?,
        ),
        DURABLE_SAVE_AS_RETIRED_REPLY => {
            WireReplyV1::DurableSaveAsRetired(match read_u8(&mut decoder)? {
                1 => DurableReceiptRetirementOutcomeV1::Retired,
                2 => DurableReceiptRetirementOutcomeV1::AbsentOrForeign,
                3 => DurableReceiptRetirementOutcomeV1::RequestConflict,
                4 => DurableReceiptRetirementOutcomeV1::OutcomeUnknown,
                5 => DurableReceiptRetirementOutcomeV1::Unavailable,
                _ => return Err(WireDecodeError::UnknownTag),
            })
        }
        _ => return Err(WireDecodeError::UnknownTag),
    };
    finish_decode(&decoder)?;
    Ok(reply)
}

fn push_packed_u32(output: &mut Vec<u8>, value: usize) -> Result<(), WireEncodeError> {
    output.extend_from_slice(
        &u32::try_from(value)
            .map_err(|_| WireEncodeError::LengthOverflow)?
            .to_be_bytes(),
    );
    Ok(())
}

fn push_packed_text(output: &mut Vec<u8>, value: &str) -> Result<(), WireEncodeError> {
    push_packed_u32(output, value.len())?;
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_workspace_edit_payload(
    documents: &[WorkspaceDocumentEditV1],
) -> Result<Vec<u8>, WireEncodeError> {
    if documents.is_empty() || documents.len() > MAX_WORKSPACE_EDIT_DOCUMENTS_V1 {
        return Err(WireEncodeError::Limit);
    }
    let mut length = 4usize;
    for document in documents {
        if document.replacements.is_empty()
            || document.replacements.len() > MAX_WORKSPACE_EDIT_REPLACEMENTS_V1
        {
            return Err(WireEncodeError::Limit);
        }
        length = length.checked_add(28).ok_or(WireEncodeError::Limit)?;
        for replacement in &document.replacements {
            length = length
                .checked_add(20)
                .and_then(|value| value.checked_add(replacement.inserted.len()))
                .filter(|value| *value <= MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1)
                .ok_or(WireEncodeError::Limit)?;
        }
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| WireEncodeError::Capacity)?;
    push_packed_u32(&mut output, documents.len())?;
    for document in documents {
        output.extend_from_slice(&document.document.workspace().get().to_be_bytes());
        output.extend_from_slice(&document.document.get().to_be_bytes());
        output.extend_from_slice(&document.expected_revision.to_be_bytes());
        push_packed_u32(&mut output, document.replacements.len())?;
        for replacement in &document.replacements {
            output.extend_from_slice(&replacement.start.to_be_bytes());
            output.extend_from_slice(&replacement.end.to_be_bytes());
            push_packed_text(&mut output, &replacement.inserted)?;
        }
    }
    Ok(output)
}

fn encode_workspace_edit_receipt_payload(
    receipt: &WorkspaceEditReceiptV1,
) -> Result<Vec<u8>, WireEncodeError> {
    if receipt.documents.len() > MAX_WORKSPACE_EDIT_DOCUMENTS_V1 {
        return Err(WireEncodeError::Limit);
    }
    let mut length = 20usize;
    for document in &receipt.documents {
        length = length
            .checked_add(28)
            .and_then(|value| value.checked_add(document.snapshot.text.len()))
            .filter(|value| *value <= MAX_WORKSPACE_EDIT_PAYLOAD_BYTES_V1)
            .ok_or(WireEncodeError::Limit)?;
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| WireEncodeError::Capacity)?;
    output.extend_from_slice(&receipt.workspace.get().to_be_bytes());
    output.extend_from_slice(&receipt.generation.to_be_bytes());
    push_packed_u32(&mut output, receipt.documents.len())?;
    for document in &receipt.documents {
        if document.document.workspace() != receipt.workspace {
            return Err(WireEncodeError::IdentityMismatch);
        }
        output.extend_from_slice(&document.document.get().to_be_bytes());
        output.extend_from_slice(&document.snapshot.witness.get().to_be_bytes());
        output.extend_from_slice(&document.snapshot.revision.to_be_bytes());
        push_packed_text(&mut output, &document.snapshot.text)?;
    }
    Ok(output)
}

struct PackedCursor<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> PackedCursor<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], WireDecodeError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(WireDecodeError::Limit)?;
        let value = self
            .input
            .get(self.offset..end)
            .ok_or(WireDecodeError::Malformed)?;
        self.offset = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, WireDecodeError> {
        Ok(u32::from_be_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| WireDecodeError::Malformed)?,
        ))
    }

    fn u64(&mut self) -> Result<u64, WireDecodeError> {
        Ok(u64::from_be_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| WireDecodeError::Malformed)?,
        ))
    }

    fn text(&mut self) -> Result<String, WireDecodeError> {
        let length = usize::try_from(self.u32()?).map_err(|_| WireDecodeError::Limit)?;
        let bytes = self.take(length)?;
        let text = std::str::from_utf8(bytes).map_err(|_| WireDecodeError::Malformed)?;
        let mut owned = String::new();
        owned
            .try_reserve_exact(length)
            .map_err(|_| WireDecodeError::Capacity)?;
        owned.push_str(text);
        Ok(owned)
    }

    fn finish(self) -> Result<(), WireDecodeError> {
        if self.offset == self.input.len() {
            Ok(())
        } else {
            Err(WireDecodeError::Malformed)
        }
    }
}

fn packed_count(cursor: &mut PackedCursor<'_>) -> Result<usize, WireDecodeError> {
    usize::try_from(cursor.u32()?).map_err(|_| WireDecodeError::Limit)
}

fn decode_workspace_edit_payload(
    payload: &[u8],
) -> Result<Vec<WorkspaceDocumentEditV1>, WireDecodeError> {
    let mut cursor = PackedCursor::new(payload);
    let count = packed_count(&mut cursor)?;
    if count == 0 || count > MAX_WORKSPACE_EDIT_DOCUMENTS_V1 {
        return Err(WireDecodeError::Limit);
    }
    let mut documents = Vec::new();
    documents
        .try_reserve_exact(count)
        .map_err(|_| WireDecodeError::Capacity)?;
    for _ in 0..count {
        let workspace = workspace_id(cursor.u64()?)?;
        let document = workspace_document_id(workspace, cursor.u64()?)?;
        let expected_revision = cursor.u64()?;
        let replacement_count = packed_count(&mut cursor)?;
        if replacement_count == 0 || replacement_count > MAX_WORKSPACE_EDIT_REPLACEMENTS_V1 {
            return Err(WireDecodeError::Limit);
        }
        let mut replacements = Vec::new();
        replacements
            .try_reserve_exact(replacement_count)
            .map_err(|_| WireDecodeError::Capacity)?;
        for _ in 0..replacement_count {
            replacements.push(WorkspaceReplacementV1 {
                start: cursor.u64()?,
                end: cursor.u64()?,
                inserted: cursor.text()?,
            });
        }
        documents.push(WorkspaceDocumentEditV1 {
            document,
            expected_revision,
            replacements,
        });
    }
    cursor.finish()?;
    Ok(documents)
}

fn decode_workspace_edit_receipt_payload(
    payload: &[u8],
) -> Result<WorkspaceEditReceiptV1, WireDecodeError> {
    let mut cursor = PackedCursor::new(payload);
    let workspace = workspace_id(cursor.u64()?)?;
    let generation = cursor.u64()?;
    let count = packed_count(&mut cursor)?;
    if count > MAX_WORKSPACE_EDIT_DOCUMENTS_V1 {
        return Err(WireDecodeError::Limit);
    }
    let mut documents = Vec::new();
    documents
        .try_reserve_exact(count)
        .map_err(|_| WireDecodeError::Capacity)?;
    for _ in 0..count {
        let document = workspace_document_id(workspace, cursor.u64()?)?;
        documents.push(WorkspaceEditedDocumentV1 {
            document,
            snapshot: SnapshotV1 {
                witness: witness(cursor.u64()?)?,
                revision: cursor.u64()?,
                text: Arc::from(cursor.text()?),
            },
        });
    }
    cursor.finish()?;
    Ok(WorkspaceEditReceiptV1 {
        workspace,
        generation,
        documents,
    })
}

fn encode_command(encoder: &mut Encoder, command: &CommandV1) -> Result<(), WireEncodeError> {
    match command {
        CommandV1::WorkspaceInventory => encoder.u8(3),
        CommandV1::ResolveLocator { locator } => {
            encoder.u8(18)?;
            encoder.text(locator)
        }
        CommandV1::WorkspaceChanges { after, page_size } => {
            encoder.u8(13)?;
            match after {
                None => encoder.u8(0)?,
                Some(cursor) => {
                    encoder.u8(1)?;
                    encoder.u64(cursor.workspace().get())?;
                    encoder.u64(cursor.epoch())?;
                    encoder.u64(cursor.revision())?;
                }
            }
            encoder.u64(u64::from(*page_size))
        }
        CommandV1::WorkspaceInventoryPage { cursor, page_size } => {
            encoder.u8(10)?;
            encoder.u8(u8::from(cursor.is_some()))?;
            if let Some(cursor) = cursor {
                encoder.u64(cursor.workspace().get())?;
                encoder.u64(cursor.generation())?;
                encoder.u64(cursor.offset())?;
            }
            encoder.u64(u64::from(*page_size))
        }
        CommandV1::OpenView => encoder.u8(8),
        CommandV1::CloseView => encoder.u8(9),
        CommandV1::AddFile { locator } => {
            encoder.u8(4)?;
            encoder.text(locator)
        }
        CommandV1::AddDirectory { locator } => {
            encoder.u8(5)?;
            encoder.text(locator)
        }
        CommandV1::RemoveDocument { document } => {
            encoder.u8(11)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())
        }
        CommandV1::RemoveRoot { root } => {
            encoder.u8(12)?;
            encoder.u64(root.workspace().get())?;
            encoder.u64(root.get())
        }
        CommandV1::SelectDocument { document } => {
            encoder.u8(6)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())
        }
        CommandV1::ObserveDocumentVersion { document } => {
            encoder.u8(19)?;
            encoder.u64(document.workspace().get())?;
            encoder.u64(document.get())
        }
        CommandV1::Save { expected } => {
            encoder.u8(7)?;
            encoder.u64(expected.get())
        }
        CommandV1::ReplaceAndSave {
            expected,
            start,
            end,
            inserted,
        } => {
            encoder.u8(14)?;
            encoder.u64(expected.get())?;
            encoder.u64(*start)?;
            encoder.u64(*end)?;
            encoder.text(inserted)
        }
        CommandV1::Snapshot => encoder.u8(1),
        CommandV1::Replace {
            expected,
            start,
            end,
            inserted,
        } => {
            encoder.u8(2)?;
            encoder.u64(expected.get())?;
            encoder.u64(*start)?;
            encoder.u64(*end)?;
            encoder.text(inserted)
        }
        CommandV1::WorkspaceEdit { documents } => {
            encoder.u8(17)?;
            encoder.bytes(&encode_workspace_edit_payload(documents)?)
        }
    }
}

fn decode_command(decoder: &mut Decoder<'_>) -> Result<CommandV1, WireDecodeError> {
    match read_u8(decoder)? {
        3 => Ok(CommandV1::WorkspaceInventory),
        18 => Ok(CommandV1::ResolveLocator {
            locator: read_text(decoder)?,
        }),
        13 => {
            let after = match read_u8(decoder)? {
                0 => None,
                1 => Some(
                    WorkspaceChangeCursorV1::new(
                        workspace_id(read_u64(decoder)?)?,
                        read_u64(decoder)?,
                        read_u64(decoder)?,
                    )
                    .ok_or(WireDecodeError::InvalidIdentity)?,
                ),
                _ => return Err(WireDecodeError::Malformed),
            };
            let page_size =
                u16::try_from(read_u64(decoder)?).map_err(|_| WireDecodeError::Malformed)?;
            Ok(CommandV1::WorkspaceChanges { after, page_size })
        }
        10 => {
            let cursor = match read_u8(decoder)? {
                0 => None,
                1 => {
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    Some(
                        WorkspaceInventoryCursorV1::new(
                            workspace,
                            read_u64(decoder)?,
                            read_u64(decoder)?,
                        )
                        .ok_or(WireDecodeError::InvalidIdentity)?,
                    )
                }
                _ => return Err(WireDecodeError::Malformed),
            };
            let page_size =
                u16::try_from(read_u64(decoder)?).map_err(|_| WireDecodeError::Malformed)?;
            Ok(CommandV1::WorkspaceInventoryPage { cursor, page_size })
        }
        8 => Ok(CommandV1::OpenView),
        9 => Ok(CommandV1::CloseView),
        4 => Ok(CommandV1::AddFile {
            locator: read_text(decoder)?,
        }),
        5 => Ok(CommandV1::AddDirectory {
            locator: read_text(decoder)?,
        }),
        11 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Ok(CommandV1::RemoveDocument {
                document: workspace_document_id(workspace, read_u64(decoder)?)?,
            })
        }
        12 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Ok(CommandV1::RemoveRoot {
                root: workspace_root_id(workspace, read_u64(decoder)?)?,
            })
        }
        6 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Ok(CommandV1::SelectDocument {
                document: workspace_document_id(workspace, read_u64(decoder)?)?,
            })
        }
        19 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Ok(CommandV1::ObserveDocumentVersion {
                document: workspace_document_id(workspace, read_u64(decoder)?)?,
            })
        }
        7 => Ok(CommandV1::Save {
            expected: witness(read_u64(decoder)?)?,
        }),
        14 => Ok(CommandV1::ReplaceAndSave {
            expected: witness(read_u64(decoder)?)?,
            start: read_u64(decoder)?,
            end: read_u64(decoder)?,
            inserted: read_text(decoder)?,
        }),
        1 => Ok(CommandV1::Snapshot),
        2 => Ok(CommandV1::Replace {
            expected: witness(read_u64(decoder)?)?,
            start: read_u64(decoder)?,
            end: read_u64(decoder)?,
            inserted: read_text(decoder)?,
        }),
        17 => Ok(CommandV1::WorkspaceEdit {
            documents: decode_workspace_edit_payload(decoder.read_bytes().map_err(map_decode)?)?,
        }),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_protocol_reply(encoder: &mut Encoder, reply: &ReplyV1) -> Result<(), WireEncodeError> {
    match reply {
        ReplyV1::Accepted(outcome) => {
            encoder.u8(1)?;
            match outcome {
                OutcomeV1::WorkspaceInventory(inventory) => {
                    encoder.u8(3)?;
                    encode_inventory(encoder, inventory)
                }
                OutcomeV1::LocatorResolved(resolution) => {
                    encoder.u8(18)?;
                    encoder.u64(resolution.workspace.get())?;
                    encoder.u8(u8::from(resolution.document.is_some()))?;
                    if let Some(document) = resolution.document {
                        encoder.u64(document.get())?;
                    }
                    encoder.u8(u8::from(resolution.root.is_some()))?;
                    if let Some(root) = &resolution.root {
                        encoder.u64(root.id.get())?;
                        encoder.u8(match root.kind {
                            WorkspaceRootKindV1::File => 1,
                            WorkspaceRootKindV1::Directory => 2,
                        })?;
                    }
                    Ok(())
                }
                OutcomeV1::WorkspaceChanges(changes) => {
                    encoder.u8(13)?;
                    encode_workspace_changes(encoder, changes)
                }
                OutcomeV1::WorkspaceInventoryPage(page) => {
                    encoder.u8(10)?;
                    encode_inventory_page(encoder, page)
                }
                OutcomeV1::ViewOpened(receipt) => {
                    encoder.u8(8)?;
                    encoder.u8(match receipt.status {
                        ViewOpenStatusV1::Opened => 1,
                        ViewOpenStatusV1::AlreadyOpen => 2,
                    })?;
                    encoder.u64(receipt.document.workspace().get())?;
                    encoder.u64(receipt.document.get())
                }
                OutcomeV1::ViewClosed(status) => {
                    encoder.u8(9)?;
                    encoder.u8(match status {
                        ViewCloseStatusV1::Closed => 1,
                        ViewCloseStatusV1::AlreadyClosed => 2,
                    })
                }
                OutcomeV1::FileAdded(inventory) => {
                    encoder.u8(4)?;
                    encode_inventory(encoder, inventory)
                }
                OutcomeV1::DirectoryAdded(inventory) => {
                    encoder.u8(5)?;
                    encode_inventory(encoder, inventory)
                }
                OutcomeV1::FileAdmitted(receipt) => {
                    encoder.u8(15)?;
                    encode_file_admission(encoder, receipt)
                }
                OutcomeV1::DirectoryAdmitted(receipt) => {
                    encoder.u8(16)?;
                    encode_directory_admission(encoder, receipt)
                }
                OutcomeV1::DocumentRemoved(receipt) => {
                    encoder.u8(11)?;
                    encoder.u64(receipt.document.workspace().get())?;
                    encoder.u64(receipt.document.get())?;
                    encode_membership_removal_status(encoder, receipt.status)
                }
                OutcomeV1::RootRemoved(receipt) => {
                    encoder.u8(12)?;
                    encoder.u64(receipt.root.workspace().get())?;
                    encoder.u64(receipt.root.get())?;
                    encode_membership_removal_status(encoder, receipt.status)
                }
                OutcomeV1::DocumentSelected(document) => {
                    encoder.u8(6)?;
                    encoder.u64(document.workspace().get())?;
                    encoder.u64(document.get())
                }
                OutcomeV1::DocumentVersionObserved(observation) => {
                    encoder.u8(19)?;
                    encoder.u64(observation.document.workspace().get())?;
                    encoder.u64(observation.document.get())?;
                    encoder.u64(observation.revision)
                }
                OutcomeV1::Saved(receipt) => {
                    encoder.u8(7)?;
                    encode_save_receipt(encoder, receipt)
                }
                OutcomeV1::ReplacedAndSaved(receipt) => {
                    encoder.u8(14)?;
                    encode_snapshot(encoder, &receipt.snapshot)?;
                    encode_edit_persistence(encoder, receipt.persistence)
                }
                OutcomeV1::Snapshot(snapshot) => {
                    encoder.u8(1)?;
                    encode_snapshot(encoder, snapshot)
                }
                OutcomeV1::Replaced(snapshot) => {
                    encoder.u8(2)?;
                    encode_snapshot(encoder, snapshot)
                }
                OutcomeV1::WorkspaceEdited(receipt) => {
                    encoder.u8(17)?;
                    encoder.bytes(&encode_workspace_edit_receipt_payload(receipt)?)
                }
            }
        }
        ReplyV1::Refused(refusal) => {
            encoder.u8(2)?;
            encoder.u8(refusal_tag(*refusal))
        }
    }
}

fn decode_protocol_reply(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<ReplyV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => {
            let outcome = match read_u8(decoder)? {
                3 => OutcomeV1::WorkspaceInventory(decode_inventory(decoder, limits)?),
                18 => {
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    let document = match read_u8(decoder)? {
                        0 => None,
                        1 => Some(workspace_document_id(workspace, read_u64(decoder)?)?),
                        _ => return Err(WireDecodeError::Malformed),
                    };
                    let root = match read_u8(decoder)? {
                        0 => None,
                        1 => Some(WorkspaceRootV1 {
                            id: workspace_root_id(workspace, read_u64(decoder)?)?,
                            kind: match read_u8(decoder)? {
                                1 => WorkspaceRootKindV1::File,
                                2 => WorkspaceRootKindV1::Directory,
                                _ => return Err(WireDecodeError::Malformed),
                            },
                        }),
                        _ => return Err(WireDecodeError::Malformed),
                    };
                    OutcomeV1::LocatorResolved(WorkspaceLocatorResolutionV1 {
                        workspace,
                        document,
                        root,
                    })
                }
                13 => OutcomeV1::WorkspaceChanges(decode_workspace_changes(decoder, limits)?),
                10 => OutcomeV1::WorkspaceInventoryPage(decode_inventory_page(decoder, limits)?),
                8 => {
                    let status = match read_u8(decoder)? {
                        1 => ViewOpenStatusV1::Opened,
                        2 => ViewOpenStatusV1::AlreadyOpen,
                        _ => return Err(WireDecodeError::Malformed),
                    };
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    OutcomeV1::ViewOpened(ViewOpenReceiptV1 {
                        document: workspace_document_id(workspace, read_u64(decoder)?)?,
                        status,
                    })
                }
                9 => OutcomeV1::ViewClosed(match read_u8(decoder)? {
                    1 => ViewCloseStatusV1::Closed,
                    2 => ViewCloseStatusV1::AlreadyClosed,
                    _ => return Err(WireDecodeError::Malformed),
                }),
                4 => OutcomeV1::FileAdded(decode_inventory(decoder, limits)?),
                5 => OutcomeV1::DirectoryAdded(decode_inventory(decoder, limits)?),
                15 => OutcomeV1::FileAdmitted(decode_file_admission(decoder)?),
                16 => OutcomeV1::DirectoryAdmitted(decode_directory_admission(decoder)?),
                11 => {
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    OutcomeV1::DocumentRemoved(DocumentRemovalReceiptV1 {
                        document: workspace_document_id(workspace, read_u64(decoder)?)?,
                        status: decode_membership_removal_status(decoder)?,
                    })
                }
                12 => {
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    OutcomeV1::RootRemoved(RootRemovalReceiptV1 {
                        root: workspace_root_id(workspace, read_u64(decoder)?)?,
                        status: decode_membership_removal_status(decoder)?,
                    })
                }
                6 => {
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    OutcomeV1::DocumentSelected(workspace_document_id(
                        workspace,
                        read_u64(decoder)?,
                    )?)
                }
                19 => {
                    let workspace = workspace_id(read_u64(decoder)?)?;
                    OutcomeV1::DocumentVersionObserved(DocumentVersionObservationV1 {
                        document: workspace_document_id(workspace, read_u64(decoder)?)?,
                        revision: read_u64(decoder)?,
                    })
                }
                7 => OutcomeV1::Saved(decode_save_receipt(decoder)?),
                14 => OutcomeV1::ReplacedAndSaved(ReplaceAndSaveReceiptV1 {
                    snapshot: decode_snapshot(decoder)?,
                    persistence: decode_edit_persistence(decoder)?,
                }),
                1 => OutcomeV1::Snapshot(decode_snapshot(decoder)?),
                2 => OutcomeV1::Replaced(decode_snapshot(decoder)?),
                17 => OutcomeV1::WorkspaceEdited(decode_workspace_edit_receipt_payload(
                    decoder.read_bytes().map_err(map_decode)?,
                )?),
                _ => return Err(WireDecodeError::UnknownTag),
            };
            Ok(ReplyV1::Accepted(outcome))
        }
        2 => Ok(ReplyV1::Refused(decode_refusal(read_u8(decoder)?)?)),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_inventory(
    encoder: &mut Encoder,
    inventory: &WorkspaceInventoryV1,
) -> Result<(), WireEncodeError> {
    encode_inventory_parts(
        encoder,
        inventory.workspace,
        &inventory.documents,
        &inventory.roots,
    )
}

fn encode_workspace_changes(
    encoder: &mut Encoder,
    changes: &WorkspaceChangesV1,
) -> Result<(), WireEncodeError> {
    match changes {
        WorkspaceChangesV1::Page(page) => {
            encoder.u8(1)?;
            encoder.u64(page.workspace.get())?;
            encoder.u64(page.observed_head)?;
            encoder.u64(
                u64::try_from(page.changes.len()).map_err(|_| WireEncodeError::LengthOverflow)?,
            )?;
            for change in &page.changes {
                encoder.u64(change.revision())?;
                encode_workspace_change_kind(encoder, change.kind)?;
            }
            encoder.u64(page.next.workspace().get())?;
            encoder.u64(page.next.epoch())?;
            encoder.u64(page.next.revision())
        }
        WorkspaceChangesV1::Gap(gap) => {
            encoder.u8(2)?;
            encoder.u64(gap.workspace.get())?;
            encoder.u64(gap.epoch())?;
            encoder.u64(gap.requested_after)?;
            encoder.u64(gap.oldest_available)?;
            encoder.u64(gap.observed_head)
        }
    }
}

fn encode_workspace_change_kind(
    encoder: &mut Encoder,
    kind: WorkspaceChangeKindV1,
) -> Result<(), WireEncodeError> {
    let (tag, workspace, local, source_revision) = match kind {
        WorkspaceChangeKindV1::DocumentAdded(id) => (1, id.workspace(), id.get(), None),
        WorkspaceChangeKindV1::DocumentRemoved(id) => (2, id.workspace(), id.get(), None),
        WorkspaceChangeKindV1::RootAdded(id) => (3, id.workspace(), id.get(), None),
        WorkspaceChangeKindV1::RootRemoved(id) => (4, id.workspace(), id.get(), None),
        WorkspaceChangeKindV1::DocumentChanged {
            document,
            source_revision,
        } => (
            5,
            document.workspace(),
            document.get(),
            Some(source_revision),
        ),
        WorkspaceChangeKindV1::DocumentBindingChanged(id) => (6, id.workspace(), id.get(), None),
    };
    encoder.u8(tag)?;
    encoder.u64(workspace.get())?;
    encoder.u64(local)?;
    if let Some(revision) = source_revision {
        encoder.u64(revision)?;
    }
    Ok(())
}

fn decode_workspace_changes(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<WorkspaceChangesV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            let observed_head = read_u64(decoder)?;
            let count = bounded_count(
                read_u64(decoder)?,
                usize::from(MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1),
            )?;
            let mut changes = Vec::new();
            changes
                .try_reserve_exact(count)
                .map_err(|_| WireDecodeError::Capacity)?;
            for _ in 0..count {
                let revision = read_u64(decoder)?;
                let kind = decode_workspace_change_kind(decoder)?;
                let change = WorkspaceChangeV1::new(revision, kind)
                    .ok_or(WireDecodeError::InvalidIdentity)?;
                if change.revision() > observed_head {
                    return Err(WireDecodeError::Malformed);
                }
                changes.push(change);
            }
            let next_workspace = workspace_id(read_u64(decoder)?)?;
            let next = WorkspaceChangeCursorV1::new(
                next_workspace,
                read_u64(decoder)?,
                read_u64(decoder)?,
            )
            .ok_or(WireDecodeError::InvalidIdentity)?;
            if next_workspace != workspace
                || changes
                    .last()
                    .is_some_and(|change| change.revision() != next.revision())
            {
                return Err(WireDecodeError::Malformed);
            }
            if changes.is_empty() && next.revision() > observed_head {
                return Err(WireDecodeError::Malformed);
            }
            let _ = limits;
            Ok(WorkspaceChangesV1::Page(WorkspaceChangePageV1 {
                workspace,
                observed_head,
                changes,
                next,
            }))
        }
        2 => {
            let gap = WorkspaceChangeGapV1::new(
                workspace_id(read_u64(decoder)?)?,
                read_u64(decoder)?,
                read_u64(decoder)?,
                read_u64(decoder)?,
                read_u64(decoder)?,
            )
            .ok_or(WireDecodeError::InvalidIdentity)?;
            if gap.oldest_available == 0
                || gap.requested_after.saturating_add(1) >= gap.oldest_available
                || gap.oldest_available > gap.observed_head
            {
                return Err(WireDecodeError::Malformed);
            }
            Ok(WorkspaceChangesV1::Gap(gap))
        }
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn decode_workspace_change_kind(
    decoder: &mut Decoder<'_>,
) -> Result<WorkspaceChangeKindV1, WireDecodeError> {
    let tag = read_u8(decoder)?;
    let workspace = workspace_id(read_u64(decoder)?)?;
    let local = read_u64(decoder)?;
    match tag {
        1 => Ok(WorkspaceChangeKindV1::DocumentAdded(workspace_document_id(
            workspace, local,
        )?)),
        2 => Ok(WorkspaceChangeKindV1::DocumentRemoved(
            workspace_document_id(workspace, local)?,
        )),
        3 => Ok(WorkspaceChangeKindV1::RootAdded(workspace_root_id(
            workspace, local,
        )?)),
        4 => Ok(WorkspaceChangeKindV1::RootRemoved(workspace_root_id(
            workspace, local,
        )?)),
        5 => Ok(WorkspaceChangeKindV1::DocumentChanged {
            document: workspace_document_id(workspace, local)?,
            source_revision: read_u64(decoder)?,
        }),
        6 => Ok(WorkspaceChangeKindV1::DocumentBindingChanged(
            workspace_document_id(workspace, local)?,
        )),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn encode_inventory_parts(
    encoder: &mut Encoder,
    workspace: WorkspaceIdV1,
    documents: &[WorkspaceDocumentIdV1],
    roots: &[WorkspaceRootV1],
) -> Result<(), WireEncodeError> {
    encoder.u64(workspace.get())?;
    encoder.u64(u64::try_from(documents.len()).map_err(|_| WireEncodeError::LengthOverflow)?)?;
    for document in documents {
        encoder.u64(document.get())?;
    }
    encoder.u64(u64::try_from(roots.len()).map_err(|_| WireEncodeError::LengthOverflow)?)?;
    for root in roots {
        encoder.u64(root.id.get())?;
        encoder.u8(match root.kind {
            WorkspaceRootKindV1::File => 1,
            WorkspaceRootKindV1::Directory => 2,
        })?;
    }
    Ok(())
}

fn decode_inventory(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<WorkspaceInventoryV1, WireDecodeError> {
    let workspace = workspace_id(read_u64(decoder)?)?;
    let document_count = bounded_count(read_u64(decoder)?, limits.max_fields)?;
    let mut documents = Vec::new();
    documents
        .try_reserve_exact(document_count)
        .map_err(|_| WireDecodeError::Capacity)?;
    for _ in 0..document_count {
        documents.push(workspace_document_id(workspace, read_u64(decoder)?)?);
    }
    let root_count = bounded_count(read_u64(decoder)?, limits.max_fields)?;
    if document_count
        .checked_add(root_count.checked_mul(2).ok_or(WireDecodeError::Limit)?)
        .filter(|fields| {
            limits
                .max_fields
                .checked_sub(7)
                .is_some_and(|available| *fields <= available)
        })
        .is_none()
    {
        return Err(WireDecodeError::Limit);
    }
    let mut roots = Vec::new();
    roots
        .try_reserve_exact(root_count)
        .map_err(|_| WireDecodeError::Capacity)?;
    for _ in 0..root_count {
        let id = workspace_root_id(workspace, read_u64(decoder)?)?;
        let kind = match read_u8(decoder)? {
            1 => WorkspaceRootKindV1::File,
            2 => WorkspaceRootKindV1::Directory,
            _ => return Err(WireDecodeError::UnknownTag),
        };
        roots.push(WorkspaceRootV1 { id, kind });
    }
    Ok(WorkspaceInventoryV1 {
        workspace,
        documents,
        roots,
    })
}

fn encode_inventory_page(
    encoder: &mut Encoder,
    page: &WorkspaceInventoryPageV1,
) -> Result<(), WireEncodeError> {
    let entries = page
        .documents
        .len()
        .checked_add(page.roots.len())
        .ok_or(WireEncodeError::LengthOverflow)?;
    if entries > usize::from(MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1) {
        return Err(WireEncodeError::Limit);
    }
    encode_inventory_parts(encoder, page.workspace, &page.documents, &page.roots)?;
    encoder.u8(u8::from(page.next.is_some()))?;
    if let Some(cursor) = page.next {
        encoder.u64(cursor.workspace().get())?;
        encoder.u64(cursor.generation())?;
        encoder.u64(cursor.offset())?;
    }
    Ok(())
}

fn decode_inventory_page(
    decoder: &mut Decoder<'_>,
    limits: WireLimits,
) -> Result<WorkspaceInventoryPageV1, WireDecodeError> {
    let inventory = decode_inventory(decoder, limits)?;
    let entries = inventory
        .documents
        .len()
        .checked_add(inventory.roots.len())
        .ok_or(WireDecodeError::Limit)?;
    if entries > usize::from(MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1) {
        return Err(WireDecodeError::Limit);
    }
    let next = match read_u8(decoder)? {
        0 => None,
        1 => {
            let workspace = workspace_id(read_u64(decoder)?)?;
            Some(
                WorkspaceInventoryCursorV1::new(workspace, read_u64(decoder)?, read_u64(decoder)?)
                    .ok_or(WireDecodeError::InvalidIdentity)?,
            )
        }
        _ => return Err(WireDecodeError::Malformed),
    };
    if next.is_some_and(|cursor| cursor.workspace() != inventory.workspace) {
        return Err(WireDecodeError::InvalidIdentity);
    }
    Ok(WorkspaceInventoryPageV1 {
        workspace: inventory.workspace,
        documents: inventory.documents,
        roots: inventory.roots,
        next,
    })
}

fn encode_snapshot(encoder: &mut Encoder, snapshot: &SnapshotV1) -> Result<(), WireEncodeError> {
    encoder.u64(snapshot.witness.get())?;
    encoder.u64(snapshot.revision)?;
    encoder.text(&snapshot.text)
}

fn decode_snapshot(decoder: &mut Decoder<'_>) -> Result<SnapshotV1, WireDecodeError> {
    Ok(SnapshotV1 {
        witness: witness(read_u64(decoder)?)?,
        revision: read_u64(decoder)?,
        text: Arc::from(read_text(decoder)?),
    })
}

fn encode_membership_removal_status(
    encoder: &mut Encoder,
    status: MembershipRemovalStatusV1,
) -> Result<(), WireEncodeError> {
    encoder.u8(match status {
        MembershipRemovalStatusV1::Removed => 1,
        MembershipRemovalStatusV1::AlreadyRemoved => 2,
    })
}

fn decode_membership_removal_status(
    decoder: &mut Decoder<'_>,
) -> Result<MembershipRemovalStatusV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(MembershipRemovalStatusV1::Removed),
        2 => Ok(MembershipRemovalStatusV1::AlreadyRemoved),
        _ => Err(WireDecodeError::Malformed),
    }
}

fn encode_file_admission(
    encoder: &mut Encoder,
    receipt: &FileAdmissionReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u8(match receipt.status {
        FileAdmissionStatusV1::Added => 1,
        FileAdmissionStatusV1::AlreadyPresent => 2,
    })?;
    encoder.u64(receipt.document.workspace().get())?;
    encoder.u64(receipt.document.get())?;
    encoder.u8(u8::from(receipt.root.is_some()))?;
    if let Some(root) = receipt.root {
        encoder.u64(root.get())?;
    }
    Ok(())
}

fn decode_file_admission(
    decoder: &mut Decoder<'_>,
) -> Result<FileAdmissionReceiptV1, WireDecodeError> {
    let status = match read_u8(decoder)? {
        1 => FileAdmissionStatusV1::Added,
        2 => FileAdmissionStatusV1::AlreadyPresent,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let workspace = workspace_id(read_u64(decoder)?)?;
    let document = workspace_document_id(workspace, read_u64(decoder)?)?;
    let root = match read_u8(decoder)? {
        0 => None,
        1 => Some(workspace_root_id(workspace, read_u64(decoder)?)?),
        _ => return Err(WireDecodeError::Malformed),
    };
    if matches!(status, FileAdmissionStatusV1::Added) != root.is_some() {
        return Err(WireDecodeError::Malformed);
    }
    Ok(FileAdmissionReceiptV1 {
        document,
        root,
        status,
    })
}

fn encode_directory_admission(
    encoder: &mut Encoder,
    receipt: &DirectoryAdmissionReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u64(receipt.root.workspace().get())?;
    encoder.u64(receipt.root.get())?;
    encoder.u64(receipt.added)?;
    encoder.u64(receipt.joined)?;
    let byte_length = receipt
        .added_documents
        .len()
        .checked_mul(8)
        .ok_or(WireEncodeError::LengthOverflow)?;
    let mut packed = Vec::new();
    packed
        .try_reserve_exact(byte_length)
        .map_err(|_| WireEncodeError::Capacity)?;
    for document in &receipt.added_documents {
        packed.extend_from_slice(&document.get().to_be_bytes());
    }
    encoder.bytes(&packed)
}

fn decode_directory_admission(
    decoder: &mut Decoder<'_>,
) -> Result<DirectoryAdmissionReceiptV1, WireDecodeError> {
    let workspace = workspace_id(read_u64(decoder)?)?;
    let root = workspace_root_id(workspace, read_u64(decoder)?)?;
    let added = read_u64(decoder)?;
    let joined = read_u64(decoder)?;
    let packed = decoder.read_bytes().map_err(map_decode)?;
    let count = usize::try_from(added).map_err(|_| WireDecodeError::Limit)?;
    if count.checked_mul(8) != Some(packed.len()) {
        return Err(WireDecodeError::Malformed);
    }
    let mut added_documents = Vec::new();
    added_documents
        .try_reserve_exact(count)
        .map_err(|_| WireDecodeError::Capacity)?;
    for bytes in packed.chunks_exact(8) {
        let local = u64::from_be_bytes(bytes.try_into().map_err(|_| WireDecodeError::Malformed)?);
        added_documents.push(workspace_document_id(workspace, local)?);
    }
    if added_documents
        .iter()
        .enumerate()
        .any(|(index, id)| added_documents[..index].contains(id))
    {
        return Err(WireDecodeError::Malformed);
    }
    Ok(DirectoryAdmissionReceiptV1 {
        root,
        added,
        joined,
        added_documents,
    })
}

fn encode_save_receipt(
    encoder: &mut Encoder,
    receipt: &SaveReceiptV1,
) -> Result<(), WireEncodeError> {
    encoder.u8(match receipt.application {
        SaveApplicationV1::Unknown => 1,
        SaveApplicationV1::NotWritten => 2,
        SaveApplicationV1::Written => 3,
    })?;
    encoder.u8(match receipt.synchronization {
        SaveSynchronizationV1::Unknown => 1,
        SaveSynchronizationV1::NotRequested => 2,
        SaveSynchronizationV1::Succeeded => 3,
        SaveSynchronizationV1::Failed => 4,
    })?;
    for value in [
        receipt.no_future_effects,
        receipt.quiescent,
        receipt.adapter_failure,
        receipt.cleanup_failed,
        receipt.baseline_published,
        receipt.outcome_unknown,
    ] {
        encoder.u8(u8::from(value))?;
    }
    Ok(())
}

fn decode_save_receipt(decoder: &mut Decoder<'_>) -> Result<SaveReceiptV1, WireDecodeError> {
    let application = match read_u8(decoder)? {
        1 => SaveApplicationV1::Unknown,
        2 => SaveApplicationV1::NotWritten,
        3 => SaveApplicationV1::Written,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let synchronization = match read_u8(decoder)? {
        1 => SaveSynchronizationV1::Unknown,
        2 => SaveSynchronizationV1::NotRequested,
        3 => SaveSynchronizationV1::Succeeded,
        4 => SaveSynchronizationV1::Failed,
        _ => return Err(WireDecodeError::UnknownTag),
    };
    let mut flag = || match read_u8(decoder)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(WireDecodeError::Malformed),
    };
    Ok(SaveReceiptV1 {
        application,
        synchronization,
        no_future_effects: flag()?,
        quiescent: flag()?,
        adapter_failure: flag()?,
        cleanup_failed: flag()?,
        baseline_published: flag()?,
        outcome_unknown: flag()?,
    })
}

fn encode_edit_persistence(
    encoder: &mut Encoder,
    persistence: EditPersistenceV1,
) -> Result<(), WireEncodeError> {
    match persistence {
        EditPersistenceV1::Durable(receipt) => {
            encoder.u8(1)?;
            encode_save_receipt(encoder, &receipt)
        }
        EditPersistenceV1::NotDurable(receipt) => {
            encoder.u8(2)?;
            encode_save_receipt(encoder, &receipt)
        }
        EditPersistenceV1::Unknown(receipt) => {
            encoder.u8(3)?;
            encode_save_receipt(encoder, &receipt)
        }
        EditPersistenceV1::NotAttempted(refusal) => {
            encoder.u8(4)?;
            encoder.u8(refusal_tag(refusal))
        }
    }
}

fn decode_edit_persistence(
    decoder: &mut Decoder<'_>,
) -> Result<EditPersistenceV1, WireDecodeError> {
    match read_u8(decoder)? {
        1 => Ok(EditPersistenceV1::Durable(decode_save_receipt(decoder)?)),
        2 => Ok(EditPersistenceV1::NotDurable(decode_save_receipt(decoder)?)),
        3 => Ok(EditPersistenceV1::Unknown(decode_save_receipt(decoder)?)),
        4 => Ok(EditPersistenceV1::NotAttempted(decode_refusal(read_u8(
            decoder,
        )?)?)),
        _ => Err(WireDecodeError::UnknownTag),
    }
}

fn request_length(request: &WireRequestV1) -> Result<usize, WireEncodeError> {
    match request {
        WireRequestV1::Connect => Ok(2),
        WireRequestV1::Disconnect(_) => Ok(10),
        WireRequestV1::DurableExecute(request) => durable_intent_length(&request.intent)?
            .checked_add(33)
            .ok_or(WireEncodeError::LengthOverflow),
        WireRequestV1::DurableReceiptLookup(lookup) => durable_intent_length(&lookup.intent)?
            .checked_add(33)
            .ok_or(WireEncodeError::LengthOverflow),
        WireRequestV1::DurableReceiptRetirement(retirement) => {
            durable_intent_length(&retirement.intent)?
                .checked_add(33)
                .ok_or(WireEncodeError::LengthOverflow)
        }
        WireRequestV1::DurableReceiptStatus(_) => Ok(34),
        WireRequestV1::DurableSaveAsSubmit(request) => {
            durable_save_as_request_length(&request.intent)
        }
        WireRequestV1::DurableSaveAsLookup(lookup) => {
            durable_save_as_request_length(&lookup.intent)
        }
        WireRequestV1::DurableSaveAsRetirement(retirement) => {
            durable_save_as_request_length(&retirement.intent)
        }
        WireRequestV1::Execute(request) => match &request.command {
            CommandV1::WorkspaceInventory | CommandV1::OpenView | CommandV1::CloseView => Ok(19),
            CommandV1::WorkspaceChanges { after, .. } => Ok(if after.is_some() { 52 } else { 28 }),
            CommandV1::WorkspaceInventoryPage { cursor, .. } => {
                Ok(if cursor.is_some() { 52 } else { 28 })
            }
            CommandV1::ResolveLocator { locator }
            | CommandV1::AddFile { locator }
            | CommandV1::AddDirectory { locator } => {
                check_wire_text_length(locator.len())?;
                checked_add(23, locator.len())
            }
            CommandV1::RemoveDocument { .. } | CommandV1::RemoveRoot { .. } => Ok(35),
            CommandV1::SelectDocument { .. } | CommandV1::ObserveDocumentVersion { .. } => Ok(35),
            CommandV1::Save { .. } => Ok(27),
            CommandV1::Snapshot => Ok(19),
            CommandV1::Replace { inserted, .. } | CommandV1::ReplaceAndSave { inserted, .. } => {
                check_wire_text_length(inserted.len())?;
                checked_add(47, inserted.len())
            }
            CommandV1::WorkspaceEdit { documents } => {
                let payload = encode_workspace_edit_payload(documents)?;
                checked_add(23, payload.len())
            }
        },
    }
}

fn reply_length(reply: &WireReplyV1) -> Result<usize, WireEncodeError> {
    match reply {
        WireReplyV1::Connected(_) => Ok(10),
        WireReplyV1::ConnectRefused(_) => Ok(3),
        WireReplyV1::Executed(ReplyV1::Refused(_)) => Ok(4),
        WireReplyV1::Disconnected => Ok(2),
        WireReplyV1::DisconnectRefused(_) => Ok(3),
        WireReplyV1::DurableExecuted(outcome) => durable_outcome_length(outcome)?
            .checked_add(2)
            .ok_or(WireEncodeError::LengthOverflow),
        WireReplyV1::DurableReceiptLookedUp(outcome) => durable_lookup_outcome_length(outcome)?
            .checked_add(2)
            .ok_or(WireEncodeError::LengthOverflow),
        WireReplyV1::DurableReceiptRetired(_) => Ok(3),
        WireReplyV1::DurableReceiptStatus(_) => Ok(3),
        WireReplyV1::DurableSaveAsSubmitted(outcome) => {
            durable_save_as_submit_outcome_length(outcome)?
                .checked_add(2)
                .ok_or(WireEncodeError::LengthOverflow)
        }
        WireReplyV1::DurableSaveAsLookedUp(outcome) => {
            durable_save_as_lookup_outcome_length(outcome)?
                .checked_add(2)
                .ok_or(WireEncodeError::LengthOverflow)
        }
        WireReplyV1::DurableSaveAsRetired(_) => Ok(3),
        WireReplyV1::Executed(ReplyV1::Accepted(outcome)) => match outcome {
            OutcomeV1::WorkspaceInventory(inventory)
            | OutcomeV1::FileAdded(inventory)
            | OutcomeV1::DirectoryAdded(inventory) => inventory_length(inventory),
            OutcomeV1::LocatorResolved(resolution) => Ok(
                match (resolution.document.is_some(), resolution.root.is_some()) {
                    (false, false) => 14,
                    (true, false) => 22,
                    (false, true) => 23,
                    (true, true) => 31,
                },
            ),
            OutcomeV1::FileAdmitted(receipt) => Ok(if receipt.root.is_some() { 30 } else { 22 }),
            OutcomeV1::DirectoryAdmitted(receipt) => receipt
                .added_documents
                .len()
                .checked_mul(8)
                .and_then(|bytes| bytes.checked_add(40))
                .ok_or(WireEncodeError::LengthOverflow),
            OutcomeV1::WorkspaceChanges(changes) => workspace_changes_length(changes),
            OutcomeV1::WorkspaceInventoryPage(page) => inventory_page_length(page),
            OutcomeV1::ViewOpened(_) => Ok(21),
            OutcomeV1::ViewClosed(_) => Ok(5),
            OutcomeV1::DocumentRemoved(_) => Ok(21),
            OutcomeV1::RootRemoved(_) => Ok(21),
            OutcomeV1::DocumentSelected(_) => Ok(20),
            OutcomeV1::DocumentVersionObserved(_) => Ok(28),
            OutcomeV1::Saved(_) => Ok(12),
            OutcomeV1::ReplacedAndSaved(receipt) => {
                check_wire_text_length(receipt.snapshot.text.len())?;
                checked_add(
                    match receipt.persistence {
                        EditPersistenceV1::NotAttempted(_) => 26,
                        _ => 33,
                    },
                    receipt.snapshot.text.len(),
                )
            }
            OutcomeV1::Snapshot(snapshot) | OutcomeV1::Replaced(snapshot) => {
                check_wire_text_length(snapshot.text.len())?;
                checked_add(24, snapshot.text.len())
            }
            OutcomeV1::WorkspaceEdited(receipt) => {
                let payload = encode_workspace_edit_receipt_payload(receipt)?;
                checked_add(8, payload.len())
            }
        },
    }
}

fn validate_request(request: &WireRequestV1, limits: WireLimits) -> Result<(), WireEncodeError> {
    let fields = match request {
        WireRequestV1::Connect => 2,
        WireRequestV1::Disconnect(_) => 3,
        WireRequestV1::DurableExecute(request) => 6_usize
            .checked_add(validate_durable_intent(&request.intent, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireRequestV1::DurableReceiptLookup(lookup) => 6_usize
            .checked_add(validate_durable_intent(&lookup.intent, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireRequestV1::DurableReceiptRetirement(retirement) => 6_usize
            .checked_add(validate_durable_intent(&retirement.intent, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireRequestV1::DurableReceiptStatus(_) => 5,
        WireRequestV1::DurableSaveAsSubmit(request) => {
            validate_durable_save_as_request(&request.intent, limits)?
        }
        WireRequestV1::DurableSaveAsLookup(lookup) => {
            validate_durable_save_as_request(&lookup.intent, limits)?
        }
        WireRequestV1::DurableSaveAsRetirement(retirement) => {
            validate_durable_save_as_request(&retirement.intent, limits)?
        }
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::WorkspaceInventory,
            ..
        })
        | WireRequestV1::Execute(RequestV1 {
            command: CommandV1::OpenView | CommandV1::CloseView,
            ..
        })
        | WireRequestV1::Execute(RequestV1 {
            command: CommandV1::Snapshot,
            ..
        }) => 5,
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::WorkspaceChanges { after, .. },
            ..
        }) => {
            if after.is_some() {
                10
            } else {
                7
            }
        }
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::WorkspaceInventoryPage { cursor, .. },
            ..
        }) => {
            if cursor.is_some() {
                10
            } else {
                7
            }
        }
        WireRequestV1::Execute(RequestV1 {
            command:
                CommandV1::ResolveLocator { locator }
                | CommandV1::AddFile { locator }
                | CommandV1::AddDirectory { locator },
            ..
        }) => {
            check_text_limit(locator.len(), limits)?;
            6
        }
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::SelectDocument { .. } | CommandV1::ObserveDocumentVersion { .. },
            ..
        })
        | WireRequestV1::Execute(RequestV1 {
            command: CommandV1::RemoveDocument { .. } | CommandV1::RemoveRoot { .. },
            ..
        }) => 7,
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::Save { .. },
            ..
        }) => 6,
        WireRequestV1::Execute(RequestV1 {
            command:
                CommandV1::Replace { inserted, .. } | CommandV1::ReplaceAndSave { inserted, .. },
            ..
        }) => {
            check_text_limit(inserted.len(), limits)?;
            9
        }
        WireRequestV1::Execute(RequestV1 {
            command: CommandV1::WorkspaceEdit { documents },
            ..
        }) => {
            let payload = encode_workspace_edit_payload(documents)?;
            check_text_limit(payload.len(), limits)?;
            6
        }
    };
    check_field_limit(fields, limits)
}

fn validate_reply(reply: &WireReplyV1, limits: WireLimits) -> Result<(), WireEncodeError> {
    let fields = match reply {
        WireReplyV1::Connected(_)
        | WireReplyV1::ConnectRefused(_)
        | WireReplyV1::DisconnectRefused(_) => 3,
        WireReplyV1::Disconnected => 2,
        WireReplyV1::DurableExecuted(outcome) => 2_usize
            .checked_add(validate_durable_outcome(outcome, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireReplyV1::DurableReceiptLookedUp(outcome) => 2_usize
            .checked_add(validate_durable_lookup_outcome(outcome, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireReplyV1::DurableReceiptRetired(_) => 3,
        WireReplyV1::DurableReceiptStatus(_) => 3,
        WireReplyV1::DurableSaveAsSubmitted(outcome) => 2_usize
            .checked_add(validate_durable_save_as_submit_outcome(outcome, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireReplyV1::DurableSaveAsLookedUp(outcome) => 2_usize
            .checked_add(validate_durable_save_as_lookup_outcome(outcome, limits)?)
            .ok_or(WireEncodeError::LengthOverflow)?,
        WireReplyV1::DurableSaveAsRetired(_) => 3,
        WireReplyV1::Executed(ReplyV1::Refused(_)) => 4,
        WireReplyV1::Executed(ReplyV1::Accepted(outcome)) => match outcome {
            OutcomeV1::WorkspaceInventory(inventory)
            | OutcomeV1::FileAdded(inventory)
            | OutcomeV1::DirectoryAdded(inventory) => {
                if inventory
                    .documents
                    .iter()
                    .any(|id| id.workspace() != inventory.workspace)
                    || inventory
                        .roots
                        .iter()
                        .any(|root| root.id.workspace() != inventory.workspace)
                {
                    return Err(WireEncodeError::IdentityMismatch);
                }
                let fields = inventory
                    .documents
                    .len()
                    .checked_add(
                        inventory
                            .roots
                            .len()
                            .checked_mul(2)
                            .ok_or(WireEncodeError::LengthOverflow)?,
                    )
                    .and_then(|entries| entries.checked_add(7))
                    .ok_or(WireEncodeError::LengthOverflow)?;
                check_field_limit(fields, limits)?;
                fields
            }
            OutcomeV1::LocatorResolved(resolution) => {
                if resolution
                    .document
                    .is_some_and(|document| document.workspace() != resolution.workspace)
                    || resolution
                        .root
                        .as_ref()
                        .is_some_and(|root| root.id.workspace() != resolution.workspace)
                {
                    return Err(WireEncodeError::IdentityMismatch);
                }
                7 + usize::from(resolution.document.is_some())
                    + usize::from(resolution.root.is_some()) * 2
            }
            OutcomeV1::FileAdmitted(receipt) => {
                if receipt
                    .root
                    .is_some_and(|root| root.workspace() != receipt.document.workspace())
                    || (matches!(receipt.status, FileAdmissionStatusV1::Added)
                        != receipt.root.is_some())
                {
                    return Err(WireEncodeError::IdentityMismatch);
                }
                if receipt.root.is_some() { 9 } else { 8 }
            }
            OutcomeV1::DirectoryAdmitted(receipt) => {
                let count = u64::try_from(receipt.added_documents.len())
                    .map_err(|_| WireEncodeError::LengthOverflow)?;
                let byte_length = receipt
                    .added_documents
                    .len()
                    .checked_mul(8)
                    .ok_or(WireEncodeError::LengthOverflow)?;
                if byte_length > limits.max_text_bytes {
                    return Err(WireEncodeError::Limit);
                }
                if count != receipt.added
                    || receipt
                        .added_documents
                        .iter()
                        .any(|document| document.workspace() != receipt.root.workspace())
                    || receipt
                        .added_documents
                        .iter()
                        .enumerate()
                        .any(|(index, id)| receipt.added_documents[..index].contains(id))
                {
                    return Err(WireEncodeError::IdentityMismatch);
                }
                9
            }
            OutcomeV1::WorkspaceChanges(changes) => match changes {
                WorkspaceChangesV1::Page(page) => {
                    if page.workspace != page.next.workspace()
                        || page.next.epoch() == 0
                        || page.changes.len() > usize::from(MAX_WORKSPACE_CHANGE_PAGE_SIZE_V1)
                        || page.changes.iter().any(|change| {
                            change.revision() > page.observed_head
                                || match change.kind {
                                    WorkspaceChangeKindV1::DocumentAdded(id)
                                    | WorkspaceChangeKindV1::DocumentRemoved(id)
                                    | WorkspaceChangeKindV1::DocumentChanged {
                                        document: id, ..
                                    }
                                    | WorkspaceChangeKindV1::DocumentBindingChanged(id) => {
                                        id.workspace() != page.workspace
                                    }
                                    WorkspaceChangeKindV1::RootAdded(id)
                                    | WorkspaceChangeKindV1::RootRemoved(id) => {
                                        id.workspace() != page.workspace
                                    }
                                }
                        })
                        || page
                            .changes
                            .last()
                            .is_some_and(|change| change.revision() != page.next.revision())
                        || (page.changes.is_empty() && page.next.revision() > page.observed_head)
                    {
                        return Err(WireEncodeError::IdentityMismatch);
                    }
                    11 + page
                        .changes
                        .iter()
                        .map(|change| match change.kind {
                            WorkspaceChangeKindV1::DocumentChanged { .. } => 5,
                            _ => 4,
                        })
                        .sum::<usize>()
                }
                WorkspaceChangesV1::Gap(gap) => {
                    if gap.oldest_available == 0
                        || gap.requested_after.saturating_add(1) >= gap.oldest_available
                        || gap.oldest_available > gap.observed_head
                    {
                        return Err(WireEncodeError::IdentityMismatch);
                    }
                    10
                }
            },
            OutcomeV1::WorkspaceInventoryPage(page) => {
                if page
                    .documents
                    .iter()
                    .any(|id| id.workspace() != page.workspace)
                    || page
                        .roots
                        .iter()
                        .any(|root| root.id.workspace() != page.workspace)
                    || page
                        .next
                        .is_some_and(|cursor| cursor.workspace() != page.workspace)
                {
                    return Err(WireEncodeError::IdentityMismatch);
                }
                let fields = page
                    .documents
                    .len()
                    .checked_add(
                        page.roots
                            .len()
                            .checked_mul(2)
                            .ok_or(WireEncodeError::LengthOverflow)?,
                    )
                    .and_then(|entries| {
                        entries.checked_add(if page.next.is_some() { 11 } else { 8 })
                    })
                    .ok_or(WireEncodeError::LengthOverflow)?;
                check_field_limit(fields, limits)?;
                fields
            }
            OutcomeV1::ViewOpened(_) => 7,
            OutcomeV1::ViewClosed(_) => 5,
            OutcomeV1::DocumentRemoved(_) => 8,
            OutcomeV1::RootRemoved(_) => 8,
            OutcomeV1::DocumentSelected(_) => 6,
            OutcomeV1::DocumentVersionObserved(_) => 7,
            OutcomeV1::Saved(_) => 12,
            OutcomeV1::ReplacedAndSaved(receipt) => {
                check_text_limit(receipt.snapshot.text.len(), limits)?;
                match receipt.persistence {
                    EditPersistenceV1::NotAttempted(_) => 9,
                    _ => 16,
                }
            }
            OutcomeV1::Snapshot(snapshot) | OutcomeV1::Replaced(snapshot) => {
                check_text_limit(snapshot.text.len(), limits)?;
                7
            }
            OutcomeV1::WorkspaceEdited(receipt) => {
                let payload = encode_workspace_edit_receipt_payload(receipt)?;
                check_text_limit(payload.len(), limits)?;
                5
            }
        },
    };
    check_field_limit(fields, limits)
}

fn validate_durable_intent(
    intent: &DurableEffectIntentV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    match intent {
        DurableEffectIntentV1::AddFile { locator }
        | DurableEffectIntentV1::AddDirectory { locator } => {
            check_text_limit(locator.len(), limits)?;
            Ok(2)
        }
        DurableEffectIntentV1::RemoveDocument { .. } | DurableEffectIntentV1::RemoveRoot { .. } => {
            Ok(3)
        }
        DurableEffectIntentV1::SaveDocument { .. } => Ok(4),
        DurableEffectIntentV1::ReplaceDocumentAndSave { inserted, .. } => {
            check_text_limit(inserted.len(), limits)?;
            Ok(7)
        }
        DurableEffectIntentV1::ExportDocumentCopyExclusive { destination, .. } => {
            check_text_limit(destination.len(), limits)?;
            Ok(5)
        }
    }
}

fn validate_durable_save_as_request(
    intent: &DurableSaveAsIntentV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    encode_durable_save_as_intent_v1(intent, limits)?;
    Ok(7)
}

fn validate_durable_save_as_state(
    state: &DurableSaveAsStateV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    if !state.is_congruent() {
        return Err(WireEncodeError::IdentityMismatch);
    }
    match state {
        DurableSaveAsStateV1::Pending | DurableSaveAsStateV1::Retired => Ok(1),
        DurableSaveAsStateV1::Fenced(receipt) | DurableSaveAsStateV1::Complete(receipt) => {
            encode_durable_save_as_receipt_v1(*receipt, limits)?;
            Ok(2)
        }
    }
}

fn validate_durable_save_as_submit_outcome(
    outcome: &DurableSaveAsSubmitOutcomeV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableSaveAsSubmitOutcomeV1::Observed(state) => 1_usize
            .checked_add(validate_durable_save_as_state(state, limits)?)
            .ok_or(WireEncodeError::LengthOverflow),
        DurableSaveAsSubmitOutcomeV1::Refused(_) => Ok(2),
    }
}

fn validate_durable_save_as_lookup_outcome(
    outcome: &DurableSaveAsLookupOutcomeV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableSaveAsLookupOutcomeV1::Observed(state) => 1_usize
            .checked_add(validate_durable_save_as_state(state, limits)?)
            .ok_or(WireEncodeError::LengthOverflow),
        DurableSaveAsLookupOutcomeV1::RequestConflict
        | DurableSaveAsLookupOutcomeV1::OutcomeUnknown
        | DurableSaveAsLookupOutcomeV1::AbsentOrForeign
        | DurableSaveAsLookupOutcomeV1::Unavailable => Ok(1),
    }
}

fn validate_durable_outcome(
    outcome: &DurableExecuteOutcomeV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableExecuteOutcomeV1::Refused(_) => Ok(2),
        DurableExecuteOutcomeV1::Retired => Ok(1),
        DurableExecuteOutcomeV1::Completed(receipt) => 1_usize
            .checked_add(validate_durable_receipt(receipt, limits)?)
            .ok_or(WireEncodeError::LengthOverflow),
    }
}

fn validate_durable_lookup_outcome(
    outcome: &DurableReceiptLookupOutcomeV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableReceiptLookupOutcomeV1::Complete(outcome) => 1_usize
            .checked_add(validate_durable_outcome(outcome, limits)?)
            .ok_or(WireEncodeError::LengthOverflow),
        DurableReceiptLookupOutcomeV1::RequestConflict
        | DurableReceiptLookupOutcomeV1::OutcomeUnknown
        | DurableReceiptLookupOutcomeV1::AbsentOrForeign
        | DurableReceiptLookupOutcomeV1::Unavailable => Ok(1),
    }
}

fn validate_durable_receipt(
    receipt: &DurableEffectReceiptV1,
    limits: WireLimits,
) -> Result<usize, WireEncodeError> {
    match receipt {
        DurableEffectReceiptV1::FileAdmitted(receipt) => {
            if receipt
                .root
                .is_some_and(|root| root.workspace() != receipt.document.workspace())
                || (matches!(receipt.status, FileAdmissionStatusV1::Added)
                    != receipt.root.is_some())
            {
                return Err(WireEncodeError::IdentityMismatch);
            }
            Ok(if receipt.root.is_some() { 6 } else { 5 })
        }
        DurableEffectReceiptV1::DirectoryAdmitted(receipt) => {
            let count = u64::try_from(receipt.added_documents.len())
                .map_err(|_| WireEncodeError::LengthOverflow)?;
            let byte_length = receipt
                .added_documents
                .len()
                .checked_mul(8)
                .ok_or(WireEncodeError::LengthOverflow)?;
            if byte_length > limits.max_text_bytes {
                return Err(WireEncodeError::Limit);
            }
            if count != receipt.added
                || receipt
                    .added_documents
                    .iter()
                    .any(|document| document.workspace() != receipt.root.workspace())
                || receipt
                    .added_documents
                    .iter()
                    .enumerate()
                    .any(|(index, id)| receipt.added_documents[..index].contains(id))
            {
                return Err(WireEncodeError::IdentityMismatch);
            }
            Ok(6)
        }
        DurableEffectReceiptV1::DocumentRemoved(_) | DurableEffectReceiptV1::RootRemoved(_) => {
            Ok(4)
        }
        DurableEffectReceiptV1::DocumentSaved(_) => Ok(7),
        DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt) => {
            check_text_limit(receipt.text.len(), limits)?;
            Ok(match receipt.persistence {
                EditPersistenceV1::NotAttempted(_) => 7,
                _ => 9,
            })
        }
        DurableEffectReceiptV1::DocumentCopyExportedExclusive(_) => Ok(7),
    }
}

fn durable_outcome_length(outcome: &DurableExecuteOutcomeV1) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableExecuteOutcomeV1::Refused(_) => Ok(2),
        DurableExecuteOutcomeV1::Retired => Ok(1),
        DurableExecuteOutcomeV1::Completed(receipt) => durable_receipt_length(receipt)?
            .checked_add(1)
            .ok_or(WireEncodeError::LengthOverflow),
    }
}

fn durable_save_as_request_length(
    intent: &DurableSaveAsIntentV1,
) -> Result<usize, WireEncodeError> {
    check_wire_text_length(intent.destination().len())?;
    checked_add(68, intent.destination().len())
}

fn durable_save_as_state_length(state: &DurableSaveAsStateV1) -> Result<usize, WireEncodeError> {
    if !state.is_congruent() {
        return Err(WireEncodeError::IdentityMismatch);
    }
    Ok(match state {
        DurableSaveAsStateV1::Pending | DurableSaveAsStateV1::Retired => 1,
        DurableSaveAsStateV1::Fenced(_) | DurableSaveAsStateV1::Complete(_) => 36,
    })
}

fn durable_save_as_submit_outcome_length(
    outcome: &DurableSaveAsSubmitOutcomeV1,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableSaveAsSubmitOutcomeV1::Observed(state) => durable_save_as_state_length(state)?
            .checked_add(1)
            .ok_or(WireEncodeError::LengthOverflow),
        DurableSaveAsSubmitOutcomeV1::Refused(_) => Ok(2),
    }
}

fn durable_save_as_lookup_outcome_length(
    outcome: &DurableSaveAsLookupOutcomeV1,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableSaveAsLookupOutcomeV1::Observed(state) => durable_save_as_state_length(state)?
            .checked_add(1)
            .ok_or(WireEncodeError::LengthOverflow),
        DurableSaveAsLookupOutcomeV1::RequestConflict
        | DurableSaveAsLookupOutcomeV1::OutcomeUnknown
        | DurableSaveAsLookupOutcomeV1::AbsentOrForeign
        | DurableSaveAsLookupOutcomeV1::Unavailable => Ok(1),
    }
}

fn durable_lookup_outcome_length(
    outcome: &DurableReceiptLookupOutcomeV1,
) -> Result<usize, WireEncodeError> {
    match outcome {
        DurableReceiptLookupOutcomeV1::Complete(outcome) => durable_outcome_length(outcome)?
            .checked_add(1)
            .ok_or(WireEncodeError::LengthOverflow),
        DurableReceiptLookupOutcomeV1::RequestConflict
        | DurableReceiptLookupOutcomeV1::OutcomeUnknown
        | DurableReceiptLookupOutcomeV1::AbsentOrForeign
        | DurableReceiptLookupOutcomeV1::Unavailable => Ok(1),
    }
}

fn durable_receipt_length(receipt: &DurableEffectReceiptV1) -> Result<usize, WireEncodeError> {
    match receipt {
        DurableEffectReceiptV1::FileAdmitted(receipt) => {
            Ok(if receipt.root.is_some() { 27 } else { 19 })
        }
        DurableEffectReceiptV1::DirectoryAdmitted(receipt) => receipt
            .added_documents
            .len()
            .checked_mul(8)
            .and_then(|bytes| bytes.checked_add(37))
            .ok_or(WireEncodeError::LengthOverflow),
        DurableEffectReceiptV1::DocumentRemoved(_) | DurableEffectReceiptV1::RootRemoved(_) => {
            Ok(18)
        }
        DurableEffectReceiptV1::DocumentSaved(_) => Ok(28),
        DurableEffectReceiptV1::DocumentReplacedAndSaved(receipt) => {
            check_wire_text_length(receipt.text.len())?;
            checked_add(
                match receipt.persistence {
                    EditPersistenceV1::NotAttempted(_) => 31,
                    _ => 33,
                },
                receipt.text.len(),
            )
        }
        DurableEffectReceiptV1::DocumentCopyExportedExclusive(_) => Ok(28),
    }
}

fn check_text_limit(length: usize, limits: WireLimits) -> Result<(), WireEncodeError> {
    check_wire_text_length(length)?;
    if length > limits.max_text_bytes {
        return Err(WireEncodeError::Limit);
    }
    Ok(())
}

fn check_wire_text_length(length: usize) -> Result<(), WireEncodeError> {
    u32::try_from(length)
        .map(|_| ())
        .map_err(|_| WireEncodeError::LengthOverflow)
}

fn check_field_limit(fields: usize, limits: WireLimits) -> Result<(), WireEncodeError> {
    if fields > limits.max_fields {
        return Err(WireEncodeError::Limit);
    }
    Ok(())
}

fn checked_add(base: usize, length: usize) -> Result<usize, WireEncodeError> {
    base.checked_add(length)
        .ok_or(WireEncodeError::LengthOverflow)
}

fn inventory_length(inventory: &WorkspaceInventoryV1) -> Result<usize, WireEncodeError> {
    let documents = inventory
        .documents
        .len()
        .checked_mul(8)
        .ok_or(WireEncodeError::LengthOverflow)?;
    let roots = inventory
        .roots
        .len()
        .checked_mul(9)
        .ok_or(WireEncodeError::LengthOverflow)?;
    checked_add(28, documents)?
        .checked_add(roots)
        .ok_or(WireEncodeError::LengthOverflow)
}

fn inventory_page_length(page: &WorkspaceInventoryPageV1) -> Result<usize, WireEncodeError> {
    inventory_parts_length(page.documents.len(), page.roots.len())?
        .checked_add(if page.next.is_some() { 25 } else { 1 })
        .ok_or(WireEncodeError::LengthOverflow)
}

fn workspace_changes_length(changes: &WorkspaceChangesV1) -> Result<usize, WireEncodeError> {
    match changes {
        WorkspaceChangesV1::Gap(_) => Ok(45),
        WorkspaceChangesV1::Page(page) => {
            let events = page.changes.iter().try_fold(0usize, |total, change| {
                total
                    .checked_add(match change.kind {
                        WorkspaceChangeKindV1::DocumentChanged { .. } => 33,
                        _ => 25,
                    })
                    .ok_or(WireEncodeError::LengthOverflow)
            })?;
            checked_add(53, events)
        }
    }
}

fn inventory_parts_length(documents: usize, roots: usize) -> Result<usize, WireEncodeError> {
    let documents = documents
        .checked_mul(8)
        .ok_or(WireEncodeError::LengthOverflow)?;
    let roots = roots
        .checked_mul(9)
        .ok_or(WireEncodeError::LengthOverflow)?;
    checked_add(28, documents)?
        .checked_add(roots)
        .ok_or(WireEncodeError::LengthOverflow)
}

fn bounded_count(value: u64, maximum: usize) -> Result<usize, WireDecodeError> {
    let value = usize::try_from(value).map_err(|_| WireDecodeError::Limit)?;
    (value <= maximum)
        .then_some(value)
        .ok_or(WireDecodeError::Limit)
}

struct Encoder {
    output: Vec<u8>,
    fields: usize,
    limits: WireLimits,
}

impl Encoder {
    fn new(length: usize, limits: WireLimits) -> Result<Self, WireEncodeError> {
        if length > limits.max_frame_bytes {
            return Err(WireEncodeError::Limit);
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(length)
            .map_err(|_| WireEncodeError::Capacity)?;
        Ok(Self {
            output,
            fields: 0,
            limits,
        })
    }

    fn u8(&mut self, value: u8) -> Result<(), WireEncodeError> {
        self.field()?;
        self.output.push(value);
        Ok(())
    }

    fn u64(&mut self, value: u64) -> Result<(), WireEncodeError> {
        self.field()?;
        self.output.extend_from_slice(&value.to_be_bytes());
        Ok(())
    }

    fn text(&mut self, value: &str) -> Result<(), WireEncodeError> {
        self.field()?;
        if value.len() > self.limits.max_text_bytes {
            return Err(WireEncodeError::Limit);
        }
        encode_bytes(&mut self.output, value.as_bytes())
            .map_err(|_| WireEncodeError::LengthOverflow)
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), WireEncodeError> {
        self.field()?;
        if value.len() > self.limits.max_text_bytes {
            return Err(WireEncodeError::Limit);
        }
        encode_bytes(&mut self.output, value).map_err(|_| WireEncodeError::LengthOverflow)
    }

    fn field(&mut self) -> Result<(), WireEncodeError> {
        if self.fields >= self.limits.max_fields {
            return Err(WireEncodeError::Limit);
        }
        self.fields += 1;
        Ok(())
    }

    fn finish(self, expected: usize) -> Result<Vec<u8>, WireEncodeError> {
        if self.output.len() != expected || self.output.len() > self.limits.max_frame_bytes {
            return Err(WireEncodeError::LengthOverflow);
        }
        Ok(self.output)
    }
}

fn decoder(input: &[u8], limits: WireLimits) -> Result<Decoder<'_>, WireDecodeError> {
    Decoder::new(
        input,
        DecodeLimits {
            max_input_bytes: limits.max_frame_bytes,
            max_field_bytes: limits.max_text_bytes,
            max_fields: limits.max_fields,
        },
    )
    .map_err(map_decode)
}

fn read_version(decoder: &mut Decoder<'_>) -> Result<(), WireDecodeError> {
    if read_u8(decoder)? != VERSION_V1 {
        return Err(WireDecodeError::UnsupportedVersion);
    }
    Ok(())
}

fn read_u8(decoder: &mut Decoder<'_>) -> Result<u8, WireDecodeError> {
    decoder.read_u8().map_err(map_decode)
}

fn read_u64(decoder: &mut Decoder<'_>) -> Result<u64, WireDecodeError> {
    decoder.read_u64_be().map_err(map_decode)
}

fn read_text(decoder: &mut Decoder<'_>) -> Result<String, WireDecodeError> {
    let bytes = decoder.read_bytes().map_err(map_decode)?;
    let value = std::str::from_utf8(bytes).map_err(|_| WireDecodeError::InvalidUtf8)?;
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| WireDecodeError::Capacity)?;
    text.push_str(value);
    Ok(text)
}

fn finish_decode(decoder: &Decoder<'_>) -> Result<(), WireDecodeError> {
    if !decoder.is_finished() {
        return Err(WireDecodeError::TrailingData);
    }
    Ok(())
}

fn map_decode(error: DecodeError) -> WireDecodeError {
    match error {
        DecodeError::InputLimit | DecodeError::FieldLimit | DecodeError::FieldCountLimit => {
            WireDecodeError::Limit
        }
        DecodeError::Truncated | DecodeError::LengthOverflow => WireDecodeError::Malformed,
    }
}

fn client_id(value: u64) -> Result<ClientId, WireDecodeError> {
    ClientId::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn request_id(value: u64) -> Result<RequestId, WireDecodeError> {
    RequestId::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn witness(value: u64) -> Result<VersionWitness, WireDecodeError> {
    VersionWitness::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn workspace_id(value: u64) -> Result<WorkspaceIdV1, WireDecodeError> {
    WorkspaceIdV1::new(value).ok_or(WireDecodeError::InvalidIdentity)
}

fn workspace_document_id(
    workspace: WorkspaceIdV1,
    value: u64,
) -> Result<WorkspaceDocumentIdV1, WireDecodeError> {
    WorkspaceDocumentIdV1::new(workspace, value).ok_or(WireDecodeError::InvalidIdentity)
}

fn workspace_root_id(
    workspace: WorkspaceIdV1,
    value: u64,
) -> Result<WorkspaceRootIdV1, WireDecodeError> {
    WorkspaceRootIdV1::new(workspace, value).ok_or(WireDecodeError::InvalidIdentity)
}

fn refusal_tag(refusal: RefusalV1) -> u8 {
    match refusal {
        RefusalV1::AuthorityDenied => 20,
        RefusalV1::AuthorityUnavailable => 21,
        RefusalV1::UnknownClient => 1,
        RefusalV1::ClientCapacity => 2,
        RefusalV1::ClientLifetimeCapacity => 17,
        RefusalV1::RequestCapacity => 3,
        RefusalV1::RequestConflict => 4,
        RefusalV1::WitnessCapacity => 5,
        RefusalV1::UnknownWitness => 6,
        RefusalV1::UnknownDocument => 19,
        RefusalV1::ForeignInventoryCursor => 22,
        RefusalV1::StaleInventoryCursor => 23,
        RefusalV1::InvalidInventoryCursor => 24,
        RefusalV1::UnknownRoot => 25,
        RefusalV1::DocumentInUse => 26,
        RefusalV1::ForeignChangeCursor => 27,
        RefusalV1::InvalidChangeCursor => 28,
        RefusalV1::NoActiveDocument => 18,
        RefusalV1::Stale => 7,
        RefusalV1::InvalidRange => 8,
        RefusalV1::InputLimit => 9,
        RefusalV1::SnapshotLimit => 10,
        RefusalV1::EditorClosed => 11,
        RefusalV1::EditorCapacity => 12,
        RefusalV1::EditorConflict => 13,
        RefusalV1::EditorIncomplete => 14,
        RefusalV1::EditorUnavailable => 15,
        RefusalV1::CounterExhausted => 16,
    }
}

fn decode_refusal(tag: u8) -> Result<RefusalV1, WireDecodeError> {
    match tag {
        20 => Ok(RefusalV1::AuthorityDenied),
        21 => Ok(RefusalV1::AuthorityUnavailable),
        1 => Ok(RefusalV1::UnknownClient),
        2 => Ok(RefusalV1::ClientCapacity),
        17 => Ok(RefusalV1::ClientLifetimeCapacity),
        3 => Ok(RefusalV1::RequestCapacity),
        4 => Ok(RefusalV1::RequestConflict),
        5 => Ok(RefusalV1::WitnessCapacity),
        6 => Ok(RefusalV1::UnknownWitness),
        19 => Ok(RefusalV1::UnknownDocument),
        22 => Ok(RefusalV1::ForeignInventoryCursor),
        23 => Ok(RefusalV1::StaleInventoryCursor),
        24 => Ok(RefusalV1::InvalidInventoryCursor),
        25 => Ok(RefusalV1::UnknownRoot),
        26 => Ok(RefusalV1::DocumentInUse),
        27 => Ok(RefusalV1::ForeignChangeCursor),
        28 => Ok(RefusalV1::InvalidChangeCursor),
        18 => Ok(RefusalV1::NoActiveDocument),
        7 => Ok(RefusalV1::Stale),
        8 => Ok(RefusalV1::InvalidRange),
        9 => Ok(RefusalV1::InputLimit),
        10 => Ok(RefusalV1::SnapshotLimit),
        11 => Ok(RefusalV1::EditorClosed),
        12 => Ok(RefusalV1::EditorCapacity),
        13 => Ok(RefusalV1::EditorConflict),
        14 => Ok(RefusalV1::EditorIncomplete),
        15 => Ok(RefusalV1::EditorUnavailable),
        16 => Ok(RefusalV1::CounterExhausted),
        _ => Err(WireDecodeError::UnknownTag),
    }
}
