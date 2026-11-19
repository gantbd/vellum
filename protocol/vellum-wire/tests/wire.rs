use std::sync::Arc;
use vellum_protocol::{
    ClientId, CommandV1, DirectoryAdmissionReceiptV1, DocumentRemovalReceiptV1,
    DocumentVersionObservationV1, DurableEffectIntentV1, DurableEffectReceiptV1,
    DurableExecuteOutcomeV1, DurableExecuteRequestV1, DurableExportCopyFailureV1,
    DurableExportCopyPersistenceV1, DurableExportCopyReceiptV1, DurableOperationRefV1,
    DurableReceiptLookupOutcomeV1, DurableReceiptLookupV1, DurableReceiptRetirementOutcomeV1,
    DurableReceiptRetirementV1, DurableReceiptStatusRequestV1, DurableReceiptStatusV1,
    DurableReplaceAndSaveReceiptV1, DurableSaveAsBindingUnknownReasonV1, DurableSaveAsBindingV1,
    DurableSaveAsDestinationEffectV1, DurableSaveAsFailureV1, DurableSaveAsFenceReasonV1,
    DurableSaveAsFenceSetV1, DurableSaveAsIntentV1, DurableSaveAsLifecycleV1,
    DurableSaveAsLookupOutcomeV1, DurableSaveAsLookupV1, DurableSaveAsNotActivatedReasonV1,
    DurableSaveAsNotAttemptedReasonV1, DurableSaveAsNotDurableReasonV1, DurableSaveAsPersistenceV1,
    DurableSaveAsPhysicalUnknownReasonV1, DurableSaveAsReceiptV1, DurableSaveAsRetirementV1,
    DurableSaveAsStateV1, DurableSaveAsSubmitOutcomeV1, DurableSaveAsSubmitRequestV1,
    DurableSaveReceiptV1, EditPersistenceV1, FileAdmissionReceiptV1, FileAdmissionStatusV1,
    MembershipRemovalStatusV1, OutcomeV1, RefusalV1, ReplaceAndSaveReceiptV1, ReplyV1, RequestId,
    RequestV1, RootRemovalReceiptV1, SaveApplicationV1, SaveReceiptV1, SaveSynchronizationV1,
    SnapshotV1, VersionWitness, ViewCloseStatusV1, ViewOpenReceiptV1, ViewOpenStatusV1,
    WorkspaceChangeCursorV1, WorkspaceChangeKindV1, WorkspaceChangePageV1, WorkspaceChangeV1,
    WorkspaceChangesV1, WorkspaceDocumentEditV1, WorkspaceDocumentIdV1, WorkspaceEditReceiptV1,
    WorkspaceEditedDocumentV1, WorkspaceIdV1, WorkspaceInventoryCursorV1, WorkspaceInventoryPageV1,
    WorkspaceInventoryV1, WorkspaceLocatorResolutionV1, WorkspaceReplacementV1, WorkspaceRootIdV1,
    WorkspaceRootKindV1, WorkspaceRootV1,
};
use vellum_wire::{
    WireDecodeError, WireEncodeError, WireLimits, WireReplyV1, WireRequestV1,
    decode_durable_intent_v1, decode_durable_outcome_v1, decode_durable_save_as_intent_v1,
    decode_durable_save_as_receipt_v1, decode_reply_v1, decode_request_v1,
    encode_durable_intent_v1, encode_durable_outcome_v1, encode_durable_save_as_intent_v1,
    encode_durable_save_as_receipt_v1, encode_reply_v1, encode_request_v1,
};

fn fixture(source: &str) -> Vec<u8> {
    source
        .split_ascii_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

fn limits() -> WireLimits {
    WireLimits {
        max_frame_bytes: 256,
        max_text_bytes: 64,
        max_fields: 16,
    }
}

#[test]
fn workspace_edit_request_and_receipt_round_trip_as_bounded_packed_values() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 9).unwrap();
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(1).unwrap(),
        request: RequestId::new(2).unwrap(),
        command: CommandV1::WorkspaceEdit {
            documents: vec![WorkspaceDocumentEditV1 {
                document,
                expected_revision: 11,
                replacements: vec![WorkspaceReplacementV1 {
                    start: 1,
                    end: 3,
                    inserted: "XY".into(),
                }],
            }],
        },
    });
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);
    assert!(!format!("{request:?}").contains("XY"));

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(
        WorkspaceEditReceiptV1 {
            workspace,
            generation: 13,
            documents: vec![WorkspaceEditedDocumentV1 {
                document,
                snapshot: SnapshotV1 {
                    witness: VersionWitness::new(17).unwrap(),
                    revision: 19,
                    text: Arc::from("aXYd"),
                },
            }],
        },
    )));
    let encoded = encode_reply_v1(&reply, limits()).unwrap();
    assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    assert!(!format!("{reply:?}").contains("aXYd"));

    let oversized = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(1).unwrap(),
        request: RequestId::new(3).unwrap(),
        command: CommandV1::WorkspaceEdit {
            documents: (0..17)
                .map(|_| WorkspaceDocumentEditV1 {
                    document,
                    expected_revision: 11,
                    replacements: vec![WorkspaceReplacementV1 {
                        start: 0,
                        end: 0,
                        inserted: String::new(),
                    }],
                })
                .collect(),
        },
    });
    assert_eq!(
        encode_request_v1(&oversized, limits()),
        Err(WireEncodeError::Limit)
    );
}

#[test]
fn durable_intent_golden_is_transport_identity_free_and_exact() {
    let workspace = WorkspaceIdV1::new(7).unwrap();
    let intent = DurableEffectIntentV1::ReplaceDocumentAndSave {
        document: WorkspaceDocumentIdV1::new(workspace, 9).unwrap(),
        expected_revision: 11,
        start: 2,
        end: 4,
        inserted: "xy".into(),
    };
    let encoded = encode_durable_intent_v1(&intent, limits()).unwrap();
    assert_eq!(
        encoded,
        fixture(
            "01 06 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00 09 \
             00 00 00 00 00 00 00 0b 00 00 00 00 00 00 00 02 \
             00 00 00 00 00 00 00 04 00 00 00 02 78 79"
        )
    );
    assert_eq!(
        decode_durable_intent_v1(&encoded, limits()).unwrap(),
        intent
    );
    let mut changed = encoded;
    *changed.last_mut().unwrap() = b'z';
    assert_ne!(
        decode_durable_intent_v1(&changed, limits()).unwrap(),
        intent
    );
}

#[test]
fn durable_exclusive_export_intent_is_exact_and_debug_redacts_destination() {
    let document = WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(7).unwrap(), 9).unwrap();
    let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document,
        expected_revision: 11,
        destination: "/tmp/new.rs".into(),
    };
    let encoded = encode_durable_intent_v1(&intent, limits()).unwrap();
    assert_eq!(
        encoded,
        fixture(
            "01 07 00 00 00 00 00 00 00 07 00 00 00 00 00 00 00 09 \
             00 00 00 00 00 00 00 0b 00 00 00 0b 2f 74 6d 70 2f 6e \
             65 77 2e 72 73"
        )
    );
    assert_eq!(
        decode_durable_intent_v1(&encoded, limits()).unwrap(),
        intent
    );
    let debug = format!("{intent:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("/tmp/new.rs"));
}

#[test]
fn durable_exclusive_export_receipt_is_locator_free_and_algebraically_closed() {
    let document = WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(3).unwrap(), 5).unwrap();
    assert!(
        DurableExportCopyReceiptV1::try_new(
            document,
            8,
            DurableExportCopyPersistenceV1::Durable,
            Some(DurableExportCopyFailureV1::Io),
            false,
        )
        .is_none()
    );
    assert!(
        DurableExportCopyReceiptV1::try_new(
            document,
            8,
            DurableExportCopyPersistenceV1::Unknown,
            None,
            false,
        )
        .is_none()
    );

    let receipt = DurableExportCopyReceiptV1::try_new(
        document,
        8,
        DurableExportCopyPersistenceV1::Unknown,
        Some(DurableExportCopyFailureV1::Io),
        true,
    )
    .unwrap();
    let outcome = DurableExecuteOutcomeV1::Completed(
        DurableEffectReceiptV1::DocumentCopyExportedExclusive(receipt),
    );
    let encoded = encode_durable_outcome_v1(&outcome, limits()).unwrap();
    assert_eq!(
        encoded,
        fixture(
            "01 01 07 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00 05 \
             00 00 00 00 00 00 00 08 03 05 01"
        )
    );
    assert_eq!(
        decode_durable_outcome_v1(&encoded, limits()).unwrap(),
        outcome
    );
    let debug = format!("{outcome:?}");
    assert!(!debug.contains("/tmp/new.rs"));

    let mut missing_failure = encoded.clone();
    let failure = missing_failure.len() - 2;
    missing_failure[failure] = 0;
    assert_eq!(
        decode_durable_outcome_v1(&missing_failure, limits()),
        Err(WireDecodeError::Malformed)
    );

    let durable = DurableExportCopyReceiptV1::try_new(
        document,
        8,
        DurableExportCopyPersistenceV1::Durable,
        None,
        false,
    )
    .unwrap();
    let durable = encode_durable_outcome_v1(
        &DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DocumentCopyExportedExclusive(
            durable,
        )),
        limits(),
    )
    .unwrap();
    let mut impossible_failure = durable;
    let failure = impossible_failure.len() - 2;
    impossible_failure[failure] = 5;
    assert_eq!(
        decode_durable_outcome_v1(&impossible_failure, limits()),
        Err(WireDecodeError::Malformed)
    );
}

#[test]
fn durable_save_as_intent_is_canonical_redacted_and_effect_explicit() {
    let document = WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(3).unwrap(), 5).unwrap();
    let overwrite = DurableSaveAsIntentV1::try_new(
        document,
        8,
        "/work/new.rs".into(),
        DurableSaveAsDestinationEffectV1::OverwriteExisting,
    )
    .unwrap();
    let encoded = encode_durable_save_as_intent_v1(&overwrite, limits()).unwrap();
    assert_eq!(
        encoded,
        fixture(
            "01 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00 05 \
             00 00 00 00 00 00 00 08 01 00 00 00 0c 2f 77 6f 72 \
             6b 2f 6e 65 77 2e 72 73"
        )
    );
    assert_eq!(
        decode_durable_save_as_intent_v1(&encoded, limits()).unwrap(),
        overwrite
    );
    assert!(!format!("{overwrite:?}").contains("/work/new.rs"));

    let exclusive = DurableSaveAsIntentV1::try_new(
        document,
        8,
        "/work/new.rs".into(),
        DurableSaveAsDestinationEffectV1::CreateExclusive,
    )
    .unwrap();
    let exclusive = encode_durable_save_as_intent_v1(&exclusive, limits()).unwrap();
    assert_eq!(exclusive[25], 2);
    assert_eq!(
        decode_durable_save_as_intent_v1(&exclusive, limits())
            .unwrap()
            .effect(),
        DurableSaveAsDestinationEffectV1::CreateExclusive
    );

    let mut unknown_effect = encoded.clone();
    unknown_effect[25] = 3;
    assert_eq!(
        decode_durable_save_as_intent_v1(&unknown_effect, limits()),
        Err(WireDecodeError::UnknownTag)
    );
    let empty = fixture(
        "01 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00 05 \
         00 00 00 00 00 00 00 08 01 00 00 00 00",
    );
    assert_eq!(
        decode_durable_save_as_intent_v1(&empty, limits()),
        Err(WireDecodeError::Malformed)
    );
}

#[test]
fn durable_save_as_receipt_codec_preserves_the_closed_product() {
    let document = WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(3).unwrap(), 5).unwrap();
    let physical = DurableSaveAsFenceSetV1::try_from_bits(
        1 << DurableSaveAsFenceReasonV1::PhysicalUncertainty as u8,
    )
    .unwrap();
    let binding = DurableSaveAsFenceSetV1::try_from_bits(
        1 << DurableSaveAsFenceReasonV1::BindingUncertainty as u8,
    )
    .unwrap();
    let retained =
        DurableSaveAsFenceSetV1::try_from_bits(1 << DurableSaveAsFenceReasonV1::RetainedTask as u8)
            .unwrap();
    let receipts = [
        DurableSaveAsReceiptV1::try_new(
            document,
            8,
            DurableSaveAsPersistenceV1::Durable,
            DurableSaveAsBindingV1::Activated,
            None,
            DurableSaveAsLifecycleV1::TerminalReplayable,
        )
        .unwrap(),
        DurableSaveAsReceiptV1::try_new(
            document,
            8,
            DurableSaveAsPersistenceV1::Durable,
            DurableSaveAsBindingV1::NotActivated,
            Some(DurableSaveAsFailureV1::NotActivated(
                DurableSaveAsNotActivatedReasonV1::DestinationChanged,
            )),
            DurableSaveAsLifecycleV1::Fenced(retained),
        )
        .unwrap(),
        DurableSaveAsReceiptV1::try_new(
            document,
            8,
            DurableSaveAsPersistenceV1::Durable,
            DurableSaveAsBindingV1::Unknown,
            Some(DurableSaveAsFailureV1::BindingUnknown(
                DurableSaveAsBindingUnknownReasonV1::TaskStateUnavailable,
            )),
            DurableSaveAsLifecycleV1::Fenced(binding),
        )
        .unwrap(),
        DurableSaveAsReceiptV1::try_new(
            document,
            8,
            DurableSaveAsPersistenceV1::NotDurable,
            DurableSaveAsBindingV1::NotActivated,
            Some(DurableSaveAsFailureV1::NotDurable(
                DurableSaveAsNotDurableReasonV1::Io,
            )),
            DurableSaveAsLifecycleV1::TerminalReplayable,
        )
        .unwrap(),
        DurableSaveAsReceiptV1::try_new(
            document,
            8,
            DurableSaveAsPersistenceV1::Unknown,
            DurableSaveAsBindingV1::NotActivated,
            Some(DurableSaveAsFailureV1::PhysicalUnknown(
                DurableSaveAsPhysicalUnknownReasonV1::Cleanup,
            )),
            DurableSaveAsLifecycleV1::Fenced(physical),
        )
        .unwrap(),
        DurableSaveAsReceiptV1::try_new(
            document,
            8,
            DurableSaveAsPersistenceV1::NotAttempted,
            DurableSaveAsBindingV1::NotActivated,
            Some(DurableSaveAsFailureV1::NotAttempted(
                DurableSaveAsNotAttemptedReasonV1::AuthorityDenied,
            )),
            DurableSaveAsLifecycleV1::TerminalReplayable,
        )
        .unwrap(),
    ];

    assert_eq!(
        encode_durable_save_as_receipt_v1(receipts[0], limits()).unwrap(),
        fixture(
            "01 00 00 00 00 00 00 00 03 00 00 00 00 00 00 00 05 \
             00 00 00 00 00 00 00 08 01 01 00 00 01 00"
        )
    );

    for receipt in receipts {
        let encoded = encode_durable_save_as_receipt_v1(receipt, limits()).unwrap();
        assert_eq!(encoded.len(), 31);
        assert_eq!(
            decode_durable_save_as_receipt_v1(&encoded, limits()).unwrap(),
            receipt
        );
        assert!(!format!("{receipt:?}").contains('/'));
    }

    let mut unknown_as_terminal = encode_durable_save_as_receipt_v1(receipts[2], limits()).unwrap();
    unknown_as_terminal[29] = 1;
    unknown_as_terminal[30] = 0;
    assert_eq!(
        decode_durable_save_as_receipt_v1(&unknown_as_terminal, limits()),
        Err(WireDecodeError::Malformed)
    );
    let mut trailing = encode_durable_save_as_receipt_v1(receipts[0], limits()).unwrap();
    trailing.push(0);
    assert_eq!(
        decode_durable_save_as_receipt_v1(&trailing, limits()),
        Err(WireDecodeError::TrailingData)
    );
}

#[test]
fn durable_save_as_operation_surface_is_canonical_closed_and_redacted() {
    let workspace = WorkspaceIdV1::new(3).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 5).unwrap();
    let intent = DurableSaveAsIntentV1::try_new(
        document,
        8,
        "/private/new-binding".into(),
        DurableSaveAsDestinationEffectV1::CreateExclusive,
    )
    .unwrap();
    let client = ClientId::new(7).unwrap();
    let request = RequestId::new(9).unwrap();
    let operation = DurableOperationRefV1::new([11; 16]).unwrap();

    let requests = [
        WireRequestV1::DurableSaveAsSubmit(DurableSaveAsSubmitRequestV1 {
            operation,
            client,
            request,
            intent: intent.clone(),
        }),
        WireRequestV1::DurableSaveAsLookup(DurableSaveAsLookupV1 {
            operation,
            client,
            request,
            intent: intent.clone(),
        }),
        WireRequestV1::DurableSaveAsRetirement(DurableSaveAsRetirementV1 {
            operation,
            client,
            request,
            intent: intent.clone(),
        }),
    ];
    for (request, tag) in requests.into_iter().zip([0x08, 0x09, 0x0a]) {
        assert!(!format!("{request:?}").contains("/private"));
        let encoded = encode_request_v1(&request, limits()).unwrap();
        assert_eq!(encoded[1], tag);
        assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);
    }

    let terminal = DurableSaveAsReceiptV1::try_new(
        document,
        8,
        DurableSaveAsPersistenceV1::Durable,
        DurableSaveAsBindingV1::Activated,
        None,
        DurableSaveAsLifecycleV1::TerminalReplayable,
    )
    .unwrap();
    let physical = DurableSaveAsFenceSetV1::try_from_bits(
        1 << DurableSaveAsFenceReasonV1::PhysicalUncertainty as u8,
    )
    .unwrap();
    let fenced = DurableSaveAsReceiptV1::try_new(
        document,
        8,
        DurableSaveAsPersistenceV1::Unknown,
        DurableSaveAsBindingV1::NotActivated,
        Some(DurableSaveAsFailureV1::PhysicalUnknown(
            DurableSaveAsPhysicalUnknownReasonV1::EvidenceIncomplete,
        )),
        DurableSaveAsLifecycleV1::Fenced(physical),
    )
    .unwrap();
    let replies = [
        WireReplyV1::DurableSaveAsSubmitted(DurableSaveAsSubmitOutcomeV1::Observed(
            DurableSaveAsStateV1::Pending,
        )),
        WireReplyV1::DurableSaveAsSubmitted(DurableSaveAsSubmitOutcomeV1::Refused(
            RefusalV1::AuthorityDenied,
        )),
        WireReplyV1::DurableSaveAsLookedUp(DurableSaveAsLookupOutcomeV1::Observed(
            DurableSaveAsStateV1::Fenced(fenced),
        )),
        WireReplyV1::DurableSaveAsLookedUp(DurableSaveAsLookupOutcomeV1::Observed(
            DurableSaveAsStateV1::Complete(terminal),
        )),
        WireReplyV1::DurableSaveAsLookedUp(DurableSaveAsLookupOutcomeV1::OutcomeUnknown),
        WireReplyV1::DurableSaveAsRetired(DurableReceiptRetirementOutcomeV1::Retired),
    ];
    for reply in replies {
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }

    assert_eq!(
        encode_reply_v1(
            &WireReplyV1::DurableSaveAsLookedUp(DurableSaveAsLookupOutcomeV1::Observed(
                DurableSaveAsStateV1::Fenced(terminal),
            )),
            limits(),
        ),
        Err(WireEncodeError::IdentityMismatch)
    );
    let mut malformed = encode_reply_v1(
        &WireReplyV1::DurableSaveAsLookedUp(DurableSaveAsLookupOutcomeV1::Observed(
            DurableSaveAsStateV1::Complete(terminal),
        )),
        limits(),
    )
    .unwrap();
    malformed[3] = 2;
    assert_eq!(
        decode_reply_v1(&malformed, limits()),
        Err(WireDecodeError::Malformed)
    );
}

fn operation(value: u8) -> DurableOperationRefV1 {
    DurableOperationRefV1::new([value; 16]).unwrap()
}

fn durable_save_receipt() -> SaveReceiptV1 {
    SaveReceiptV1 {
        application: SaveApplicationV1::Written,
        synchronization: SaveSynchronizationV1::Succeeded,
        no_future_effects: true,
        quiescent: true,
        adapter_failure: false,
        cleanup_failed: false,
        baseline_published: true,
        outcome_unknown: false,
    }
}

#[test]
fn durable_envelopes_are_additive_exact_and_reject_a_zero_operation_reference() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let intent = DurableEffectIntentV1::SaveDocument {
        document: WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
        expected_revision: 17,
    };
    let execute = WireRequestV1::DurableExecute(DurableExecuteRequestV1 {
        operation: operation(1),
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        intent: intent.clone(),
    });
    let encoded = encode_request_v1(&execute, limits()).unwrap();
    assert_eq!(
        encoded,
        fixture(
            "01 04 01 01 01 01 01 01 01 01 01 01 01 01 01 01 01 01 \
             00 00 00 00 00 00 00 07 00 00 00 00 00 00 00 09 05 \
             00 00 00 00 00 00 00 0b 00 00 00 00 00 00 00 0d \
             00 00 00 00 00 00 00 11"
        )
    );
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), execute);
    for end in 0..encoded.len() {
        assert!(decode_request_v1(&encoded[..end], limits()).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert_eq!(
        decode_request_v1(&trailing, limits()),
        Err(WireDecodeError::TrailingData)
    );

    let lookup = WireRequestV1::DurableReceiptLookup(DurableReceiptLookupV1 {
        operation: operation(1),
        client: ClientId::new(19).unwrap(),
        request: RequestId::new(23).unwrap(),
        intent,
    });
    let encoded_lookup = encode_request_v1(&lookup, limits()).unwrap();
    assert_eq!(encoded_lookup[0..2], [1, 5]);
    assert_eq!(
        decode_request_v1(&encoded_lookup, limits()).unwrap(),
        lookup
    );

    let mut zero = encoded;
    zero[2..18].fill(0);
    assert_eq!(
        decode_request_v1(&zero, limits()),
        Err(WireDecodeError::InvalidIdentity)
    );
}

#[test]
fn durable_status_is_content_free_fixed_width_and_closed() {
    let request = WireRequestV1::DurableReceiptStatus(DurableReceiptStatusRequestV1 {
        operation: operation(0x31),
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
    });
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(encoded.len(), 34);
    assert_eq!(encoded[0..2], [1, 7]);
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);
    for status in [
        DurableReceiptStatusV1::Pending,
        DurableReceiptStatusV1::Complete,
        DurableReceiptStatusV1::Retired,
        DurableReceiptStatusV1::AbsentOrForeign,
        DurableReceiptStatusV1::Unavailable,
    ] {
        let reply = WireReplyV1::DurableReceiptStatus(status);
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(encoded.len(), 3);
        assert_eq!(encoded[0..2], [1, 0x89]);
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }
    assert_eq!(
        decode_reply_v1(&[1, 0x89, 6], limits()),
        Err(WireDecodeError::UnknownTag)
    );
}

#[test]
fn durable_retirement_and_absorbing_execute_outcome_have_exact_additive_tags() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let request = WireRequestV1::DurableReceiptRetirement(DurableReceiptRetirementV1 {
        operation: operation(2),
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        intent: DurableEffectIntentV1::SaveDocument {
            document: WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
            expected_revision: 17,
        },
    });
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(encoded[0..2], [1, 6]);
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);
    for end in 0..encoded.len() {
        assert!(decode_request_v1(&encoded[..end], limits()).is_err());
    }

    for (outcome, tag) in [
        (DurableReceiptRetirementOutcomeV1::Retired, 1),
        (DurableReceiptRetirementOutcomeV1::AbsentOrForeign, 2),
        (DurableReceiptRetirementOutcomeV1::RequestConflict, 3),
        (DurableReceiptRetirementOutcomeV1::OutcomeUnknown, 4),
        (DurableReceiptRetirementOutcomeV1::Unavailable, 5),
    ] {
        let reply = WireReplyV1::DurableReceiptRetired(outcome);
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(encoded, vec![1, 0x88, tag]);
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }
    assert_eq!(
        encode_durable_outcome_v1(&DurableExecuteOutcomeV1::Retired, limits()).unwrap(),
        vec![1, 3]
    );
    assert_eq!(
        decode_reply_v1(&[1, 0x88, 0xff], limits()),
        Err(WireDecodeError::UnknownTag)
    );
}

#[test]
fn durable_outcomes_are_stable_witness_free_and_bounded() {
    let document = WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(3).unwrap(), 5).unwrap();
    let replacement = DurableExecuteOutcomeV1::Completed(
        DurableEffectReceiptV1::DocumentReplacedAndSaved(DurableReplaceAndSaveReceiptV1 {
            document,
            source_revision: 8,
            text: Arc::from("private stable receipt"),
            persistence: EditPersistenceV1::Durable(durable_save_receipt()),
        }),
    );
    let encoded = encode_durable_outcome_v1(&replacement, limits()).unwrap();
    assert_eq!(
        decode_durable_outcome_v1(&encoded, limits()).unwrap(),
        replacement
    );
    assert!(!format!("{replacement:?}").contains("private stable receipt"));
    let mut invalid_flags = encoded.clone();
    *invalid_flags.last_mut().unwrap() |= 0x80;
    assert_eq!(
        decode_durable_outcome_v1(&invalid_flags, limits()),
        Err(WireDecodeError::Malformed)
    );

    for outcome in [
        DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DocumentSaved(
            DurableSaveReceiptV1 {
                document,
                source_revision: 7,
                persistence: durable_save_receipt(),
            },
        )),
        DurableExecuteOutcomeV1::Refused(RefusalV1::Stale),
    ] {
        let encoded = encode_durable_outcome_v1(&outcome, limits()).unwrap();
        assert_eq!(
            decode_durable_outcome_v1(&encoded, limits()).unwrap(),
            outcome
        );
    }

    let execute_reply = WireReplyV1::DurableExecuted(replacement.clone());
    let encoded_reply = encode_reply_v1(&execute_reply, limits()).unwrap();
    assert_eq!(encoded_reply[0..2], [1, 0x86]);
    assert_eq!(
        decode_reply_v1(&encoded_reply, limits()).unwrap(),
        execute_reply
    );

    for lookup in [
        DurableReceiptLookupOutcomeV1::Complete(replacement.clone()),
        DurableReceiptLookupOutcomeV1::RequestConflict,
        DurableReceiptLookupOutcomeV1::OutcomeUnknown,
        DurableReceiptLookupOutcomeV1::AbsentOrForeign,
        DurableReceiptLookupOutcomeV1::Unavailable,
    ] {
        let reply = WireReplyV1::DurableReceiptLookedUp(lookup);
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(encoded[0..2], [1, 0x87]);
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }

    let tight = WireLimits {
        max_fields: 11,
        ..limits()
    };
    assert_eq!(
        encode_reply_v1(&WireReplyV1::DurableExecuted(replacement), tight),
        Err(WireEncodeError::Limit)
    );
    assert_eq!(
        decode_reply_v1(&encoded_reply, tight),
        Err(WireDecodeError::Limit)
    );
}

#[test]
fn golden_connect_frames_are_canonical() {
    let request = WireRequestV1::Connect;
    let request_fixture = fixture(include_str!("fixtures/connect-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Connected(ClientId::new(7).unwrap());
    let reply_fixture = fixture(include_str!("fixtures/connect-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
}

#[test]
fn golden_semantic_frames_preserve_utf8_and_redacted_protocol_values() {
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::Replace {
            expected: VersionWitness::new(11).unwrap(),
            start: 1,
            end: 3,
            inserted: "é".into(),
        },
    });
    let request_fixture = fixture(include_str!("fixtures/replace-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::Snapshot(SnapshotV1 {
        witness: VersionWitness::new(13).unwrap(),
        revision: 5,
        text: Arc::from("alpha"),
    })));
    let reply_fixture = fixture(include_str!("fixtures/snapshot-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
}

#[test]
fn golden_inventory_frames_are_content_free_and_canonical() {
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::WorkspaceInventory,
    });
    let request_fixture = fixture(include_str!("fixtures/inventory-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let inventory = WorkspaceInventoryV1 {
        workspace: WorkspaceIdV1::new(11).unwrap(),
        documents: vec![
            WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(11).unwrap(), 13).unwrap(),
            WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(11).unwrap(), 17).unwrap(),
        ],
        roots: vec![
            WorkspaceRootV1 {
                id: WorkspaceRootIdV1::new(WorkspaceIdV1::new(11).unwrap(), 19).unwrap(),
                kind: WorkspaceRootKindV1::File,
            },
            WorkspaceRootV1 {
                id: WorkspaceRootIdV1::new(WorkspaceIdV1::new(11).unwrap(), 23).unwrap(),
                kind: WorkspaceRootKindV1::Directory,
            },
        ],
    };
    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)));
    let reply_fixture = fixture(include_str!("fixtures/inventory-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
    assert!(!format!("{reply:?}").contains('/'));
}

#[test]
fn golden_inventory_page_frames_bind_cursor_and_encode_end_canonically() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let cursor = WorkspaceInventoryCursorV1::new(workspace, 5, 1).unwrap();
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::WorkspaceInventoryPage {
            cursor: Some(cursor),
            page_size: 2,
        },
    });
    let request_fixture = fixture(include_str!("fixtures/inventory-page-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
        WorkspaceInventoryPageV1 {
            workspace,
            documents: vec![],
            roots: vec![WorkspaceRootV1 {
                id: WorkspaceRootIdV1::new(workspace, 19).unwrap(),
                kind: WorkspaceRootKindV1::Directory,
            }],
            next: None,
        },
    )));
    let reply_fixture = fixture(include_str!("fixtures/inventory-page-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
    assert!(!format!("{request:?}{reply:?}").contains("11"));
}

#[test]
fn golden_change_feed_frames_are_bounded_content_free_and_canonical() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::WorkspaceChanges {
            after: Some(WorkspaceChangeCursorV1::new(workspace, 3, 5).unwrap()),
            page_size: 1,
        },
    });
    let request_fixture = fixture(include_str!("fixtures/change-feed-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(
        WorkspaceChangesV1::Page(WorkspaceChangePageV1 {
            workspace,
            observed_head: 7,
            changes: vec![
                WorkspaceChangeV1::new(
                    7,
                    WorkspaceChangeKindV1::DocumentChanged {
                        document: WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
                        source_revision: 4,
                    },
                )
                .unwrap(),
            ],
            next: WorkspaceChangeCursorV1::new(workspace, 3, 7).unwrap(),
        }),
    )));
    let reply_fixture = fixture(include_str!("fixtures/change-feed-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
    assert!(!format!("{request:?}").contains("private"));
}

#[test]
fn binding_change_round_trips_without_a_locator_or_source_revision() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 13).unwrap();
    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(
        WorkspaceChangesV1::Page(WorkspaceChangePageV1 {
            workspace,
            observed_head: 1,
            changes: vec![
                WorkspaceChangeV1::new(1, WorkspaceChangeKindV1::DocumentBindingChanged(document))
                    .unwrap(),
            ],
            next: WorkspaceChangeCursorV1::new(workspace, 3, 1).unwrap(),
        }),
    )));

    let encoded = encode_reply_v1(&reply, limits()).unwrap();
    assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    assert!(!format!("{reply:?}").contains("/"));
}

#[test]
fn change_feed_decode_rejects_oversized_and_mismatched_pages() {
    let mut oversized = fixture(include_str!("fixtures/change-feed-reply-v1.hex"));
    oversized[28] = 2;
    assert_eq!(
        decode_reply_v1(&oversized, limits()),
        Err(WireDecodeError::Limit)
    );

    let mut foreign_next = fixture(include_str!("fixtures/change-feed-reply-v1.hex"));
    let next_workspace_last_byte = foreign_next.len() - 17;
    foreign_next[next_workspace_last_byte] = 12;
    assert_eq!(
        decode_reply_v1(&foreign_next, limits()),
        Err(WireDecodeError::Malformed)
    );

    let mut zero_epoch_request = fixture(include_str!("fixtures/change-feed-request-v1.hex"));
    zero_epoch_request[28..36].fill(0);
    assert_eq!(
        decode_request_v1(&zero_epoch_request, limits()),
        Err(WireDecodeError::InvalidIdentity)
    );
}

#[test]
fn golden_workspace_mutation_frames_are_canonical_and_debug_redacted() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let cases = [
        (
            CommandV1::AddFile {
                locator: "a.txt".into(),
            },
            include_str!("fixtures/add-file-request-v1.hex"),
        ),
        (
            CommandV1::AddDirectory {
                locator: "src".into(),
            },
            include_str!("fixtures/add-directory-request-v1.hex"),
        ),
        (
            CommandV1::SelectDocument {
                document: WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
            },
            include_str!("fixtures/select-document-request-v1.hex"),
        ),
    ];
    for (command, expected) in cases {
        let request = WireRequestV1::Execute(RequestV1 {
            client: ClientId::new(7).unwrap(),
            request: RequestId::new(9).unwrap(),
            command,
        });
        let expected = fixture(expected);
        assert_eq!(encode_request_v1(&request, limits()).unwrap(), expected);
        assert_eq!(decode_request_v1(&expected, limits()).unwrap(), request);
    }
    let debug = format!(
        "{:?}",
        WireRequestV1::Execute(RequestV1 {
            client: ClientId::new(7).unwrap(),
            request: RequestId::new(9).unwrap(),
            command: CommandV1::AddFile {
                locator: "private-locator".into(),
            },
        })
    );
    assert!(!debug.contains("private-locator"));
}

#[test]
fn golden_exact_admission_receipts_are_canonical_bounded_and_workspace_scoped() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let file = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::FileAdmitted(
        FileAdmissionReceiptV1 {
            document: WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
            root: Some(WorkspaceRootIdV1::new(workspace, 17).unwrap()),
            status: FileAdmissionStatusV1::Added,
        },
    )));
    let file_fixture = fixture(include_str!("fixtures/file-admission-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&file, limits()).unwrap(), file_fixture);
    assert_eq!(decode_reply_v1(&file_fixture, limits()).unwrap(), file);

    let directory = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(
        DirectoryAdmissionReceiptV1 {
            root: WorkspaceRootIdV1::new(workspace, 17).unwrap(),
            added: 2,
            joined: 1,
            added_documents: vec![
                WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
                WorkspaceDocumentIdV1::new(workspace, 14).unwrap(),
            ],
        },
    )));
    let directory_fixture = fixture(include_str!("fixtures/directory-admission-reply-v1.hex"));
    assert_eq!(
        encode_reply_v1(&directory, limits()).unwrap(),
        directory_fixture
    );
    assert_eq!(
        decode_reply_v1(&directory_fixture, limits()).unwrap(),
        directory
    );
}

#[test]
fn exact_admission_receipts_reject_incoherent_or_malformed_identity_sets() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 13).unwrap();
    let root = WorkspaceRootIdV1::new(workspace, 17).unwrap();
    let missing_added_root = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::FileAdmitted(
        FileAdmissionReceiptV1 {
            document,
            root: None,
            status: FileAdmissionStatusV1::Added,
        },
    )));
    assert_eq!(
        encode_reply_v1(&missing_added_root, limits()),
        Err(WireEncodeError::IdentityMismatch)
    );
    let wrong_count = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(
        DirectoryAdmissionReceiptV1 {
            root,
            added: 2,
            joined: 0,
            added_documents: vec![document],
        },
    )));
    assert_eq!(
        encode_reply_v1(&wrong_count, limits()),
        Err(WireEncodeError::IdentityMismatch)
    );
    let duplicate = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(
        DirectoryAdmissionReceiptV1 {
            root,
            added: 2,
            joined: 0,
            added_documents: vec![document, document],
        },
    )));
    assert_eq!(
        encode_reply_v1(&duplicate, limits()),
        Err(WireEncodeError::IdentityMismatch)
    );

    let mut malformed = fixture(include_str!("fixtures/directory-admission-reply-v1.hex"));
    malformed[27] = 3;
    assert_eq!(
        decode_reply_v1(&malformed, limits()),
        Err(WireDecodeError::Malformed)
    );
    let mut zero_identity = fixture(include_str!("fixtures/directory-admission-reply-v1.hex"));
    zero_identity[48..56].fill(0);
    assert_eq!(
        decode_reply_v1(&zero_identity, limits()),
        Err(WireDecodeError::InvalidIdentity)
    );
}

#[test]
fn golden_membership_removal_frames_are_canonical_and_privacy_minimized() {
    let workspace = WorkspaceIdV1::new(11).unwrap();
    let document = WorkspaceDocumentIdV1::new(workspace, 13).unwrap();
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::RemoveDocument { document },
    });
    let request_fixture = fixture(include_str!("fixtures/remove-document-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::DocumentRemoved(
        DocumentRemovalReceiptV1 {
            document,
            status: MembershipRemovalStatusV1::Removed,
        },
    )));
    let reply_fixture = fixture(include_str!("fixtures/remove-document-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
    assert_eq!(reply_fixture.len(), 21);
}

#[test]
fn golden_view_lifecycle_frames_are_canonical_and_closed() {
    for (command, source) in [
        (
            CommandV1::OpenView,
            include_str!("fixtures/open-view-request-v1.hex"),
        ),
        (
            CommandV1::CloseView,
            include_str!("fixtures/close-view-request-v1.hex"),
        ),
    ] {
        let request = WireRequestV1::Execute(RequestV1 {
            client: ClientId::new(7).unwrap(),
            request: RequestId::new(9).unwrap(),
            command,
        });
        let bytes = fixture(source);
        assert_eq!(encode_request_v1(&request, limits()).unwrap(), bytes);
        assert_eq!(decode_request_v1(&bytes, limits()).unwrap(), request);
    }

    let workspace = WorkspaceIdV1::new(11).unwrap();
    let opened = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::ViewOpened(
        ViewOpenReceiptV1 {
            document: WorkspaceDocumentIdV1::new(workspace, 13).unwrap(),
            status: ViewOpenStatusV1::Opened,
        },
    )));
    let opened_bytes = fixture(include_str!("fixtures/open-view-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&opened, limits()).unwrap(), opened_bytes);
    assert_eq!(decode_reply_v1(&opened_bytes, limits()).unwrap(), opened);

    let closed = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::ViewClosed(
        ViewCloseStatusV1::Closed,
    )));
    let closed_bytes = fixture(include_str!("fixtures/close-view-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&closed, limits()).unwrap(), closed_bytes);
    assert_eq!(decode_reply_v1(&closed_bytes, limits()).unwrap(), closed);
}

#[test]
fn golden_save_frames_preserve_closed_receipt_facts() {
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::Save {
            expected: VersionWitness::new(11).unwrap(),
        },
    });
    let request_fixture = fixture(include_str!("fixtures/save-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::Saved(SaveReceiptV1 {
        application: SaveApplicationV1::Written,
        synchronization: SaveSynchronizationV1::Succeeded,
        no_future_effects: true,
        quiescent: true,
        adapter_failure: false,
        cleanup_failed: false,
        baseline_published: true,
        outcome_unknown: false,
    })));
    let reply_fixture = fixture(include_str!("fixtures/save-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);
}

#[test]
fn golden_replace_and_save_frames_preserve_partial_and_durable_evidence() {
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(7).unwrap(),
        request: RequestId::new(9).unwrap(),
        command: CommandV1::ReplaceAndSave {
            expected: VersionWitness::new(11).unwrap(),
            start: 1,
            end: 3,
            inserted: "é".into(),
        },
    });
    let request_fixture = fixture(include_str!("fixtures/replace-and-save-request-v1.hex"));
    assert_eq!(
        encode_request_v1(&request, limits()).unwrap(),
        request_fixture
    );
    assert_eq!(
        decode_request_v1(&request_fixture, limits()).unwrap(),
        request
    );

    let save = SaveReceiptV1 {
        application: SaveApplicationV1::Written,
        synchronization: SaveSynchronizationV1::Succeeded,
        no_future_effects: true,
        quiescent: true,
        adapter_failure: false,
        cleanup_failed: false,
        baseline_published: true,
        outcome_unknown: false,
    };
    let reply = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(
        ReplaceAndSaveReceiptV1 {
            snapshot: SnapshotV1 {
                witness: VersionWitness::new(13).unwrap(),
                revision: 5,
                text: Arc::from("alpha"),
            },
            persistence: EditPersistenceV1::Durable(save),
        },
    )));
    let reply_fixture = fixture(include_str!("fixtures/replace-and-save-reply-v1.hex"));
    assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), reply_fixture);
    assert_eq!(decode_reply_v1(&reply_fixture, limits()).unwrap(), reply);

    for persistence in [
        EditPersistenceV1::NotDurable(save),
        EditPersistenceV1::Unknown(SaveReceiptV1 {
            application: SaveApplicationV1::Unknown,
            synchronization: SaveSynchronizationV1::Unknown,
            no_future_effects: false,
            quiescent: false,
            adapter_failure: true,
            cleanup_failed: false,
            baseline_published: false,
            outcome_unknown: true,
        }),
        EditPersistenceV1::NotAttempted(RefusalV1::EditorCapacity),
    ] {
        let value = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(
            ReplaceAndSaveReceiptV1 {
                snapshot: SnapshotV1 {
                    witness: VersionWitness::new(13).unwrap(),
                    revision: 5,
                    text: Arc::from("alpha"),
                },
                persistence,
            },
        )));
        let encoded = encode_reply_v1(&value, limits()).unwrap();
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), value);
    }
}

#[test]
fn golden_authority_refusals_are_closed_and_canonical() {
    for (refusal, bytes) in [
        (
            RefusalV1::AuthorityDenied,
            fixture(include_str!("fixtures/authority-denied-reply-v1.hex")),
        ),
        (
            RefusalV1::AuthorityUnavailable,
            fixture(include_str!("fixtures/authority-unavailable-reply-v1.hex")),
        ),
    ] {
        let reply = WireReplyV1::Executed(ReplyV1::Refused(refusal));
        assert_eq!(encode_reply_v1(&reply, limits()).unwrap(), bytes);
        assert_eq!(decode_reply_v1(&bytes, limits()).unwrap(), reply);
    }
}

#[test]
fn every_truncation_and_any_trailing_byte_refuses() {
    let encoded = fixture(include_str!("fixtures/replace-request-v1.hex"));
    for end in 0..encoded.len() {
        assert!(decode_request_v1(&encoded[..end], limits()).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert_eq!(
        decode_request_v1(&trailing, limits()),
        Err(WireDecodeError::TrailingData)
    );
}

#[test]
fn bounds_versions_tags_identities_and_utf8_are_closed_refusals() {
    let encoded = fixture(include_str!("fixtures/replace-request-v1.hex"));
    let mut frame_limit = limits();
    frame_limit.max_frame_bytes = encoded.len() - 1;
    assert_eq!(
        decode_request_v1(&encoded, frame_limit),
        Err(WireDecodeError::Limit)
    );

    let mut text_limit = limits();
    text_limit.max_text_bytes = 1;
    assert_eq!(
        decode_request_v1(&encoded, text_limit),
        Err(WireDecodeError::Limit)
    );
    assert_eq!(
        encode_request_v1(
            &WireRequestV1::Execute(RequestV1 {
                client: ClientId::new(1).unwrap(),
                request: RequestId::new(1).unwrap(),
                command: CommandV1::Replace {
                    expected: VersionWitness::new(1).unwrap(),
                    start: 0,
                    end: 0,
                    inserted: "too long".into(),
                },
            }),
            WireLimits {
                max_text_bytes: 1,
                ..limits()
            },
        ),
        Err(WireEncodeError::Limit)
    );

    let workspace = WorkspaceIdV1::new(1).unwrap();
    let foreign = WorkspaceIdV1::new(2).unwrap();
    assert_eq!(
        encode_reply_v1(
            &WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
                WorkspaceInventoryPageV1 {
                    workspace,
                    documents: vec![],
                    roots: vec![],
                    next: Some(WorkspaceInventoryCursorV1::new(foreign, 1, 1).unwrap(),),
                }
            ),)),
            limits(),
        ),
        Err(WireEncodeError::IdentityMismatch)
    );
    let cursor = WorkspaceInventoryCursorV1::new(workspace, 1, 3).unwrap();
    let two_roots = WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(
        WorkspaceInventoryPageV1 {
            workspace,
            documents: vec![],
            roots: vec![
                WorkspaceRootV1 {
                    id: WorkspaceRootIdV1::new(workspace, 1).unwrap(),
                    kind: WorkspaceRootKindV1::Directory,
                },
                WorkspaceRootV1 {
                    id: WorkspaceRootIdV1::new(workspace, 2).unwrap(),
                    kind: WorkspaceRootKindV1::Directory,
                },
            ],
            next: Some(cursor),
        },
    )));
    assert!(encode_reply_v1(&two_roots, limits()).is_ok());
    let three_documents = WireReplyV1::Executed(ReplyV1::Accepted(
        OutcomeV1::WorkspaceInventoryPage(WorkspaceInventoryPageV1 {
            workspace,
            documents: (1..=3)
                .map(|local| WorkspaceDocumentIdV1::new(workspace, local).unwrap())
                .collect(),
            roots: vec![],
            next: Some(cursor),
        }),
    ));
    assert_eq!(
        encode_reply_v1(&three_documents, limits()),
        Err(WireEncodeError::Limit)
    );
    let mut oversized_page = fixture(include_str!("fixtures/inventory-page-reply-v1.hex"));
    oversized_page[20..28].copy_from_slice(&3_u64.to_be_bytes());
    oversized_page.pop();
    oversized_page.extend_from_slice(&20_u64.to_be_bytes());
    oversized_page.push(2);
    oversized_page.extend_from_slice(&21_u64.to_be_bytes());
    oversized_page.push(2);
    oversized_page.push(0);
    assert_eq!(
        decode_reply_v1(
            &oversized_page,
            WireLimits {
                max_fields: 64,
                ..limits()
            },
        ),
        Err(WireDecodeError::Limit)
    );
    assert_eq!(
        encode_request_v1(
            &WireRequestV1::Connect,
            WireLimits {
                max_fields: 1,
                ..limits()
            },
        ),
        Err(WireEncodeError::Limit)
    );

    let mut unsupported = encoded.clone();
    unsupported[0] = 2;
    assert_eq!(
        decode_request_v1(&unsupported, limits()),
        Err(WireDecodeError::UnsupportedVersion)
    );
    let mut unknown_kind = encoded.clone();
    unknown_kind[1] = 127;
    assert_eq!(
        decode_request_v1(&unknown_kind, limits()),
        Err(WireDecodeError::UnknownTag)
    );
    let mut zero_client = encoded.clone();
    zero_client[2..10].fill(0);
    assert_eq!(
        decode_request_v1(&zero_client, limits()),
        Err(WireDecodeError::InvalidIdentity)
    );
    let mut invalid_utf8 = encoded;
    let last = invalid_utf8.len() - 1;
    invalid_utf8[last] = 0xff;
    assert_eq!(
        decode_request_v1(&invalid_utf8, limits()),
        Err(WireDecodeError::InvalidUtf8)
    );
}

#[test]
fn every_closed_reply_variant_round_trips_and_debug_redacts_content() {
    let refusals = [
        RefusalV1::AuthorityDenied,
        RefusalV1::AuthorityUnavailable,
        RefusalV1::UnknownClient,
        RefusalV1::ClientCapacity,
        RefusalV1::ClientLifetimeCapacity,
        RefusalV1::RequestCapacity,
        RefusalV1::RequestConflict,
        RefusalV1::WitnessCapacity,
        RefusalV1::UnknownWitness,
        RefusalV1::UnknownDocument,
        RefusalV1::UnknownRoot,
        RefusalV1::DocumentInUse,
        RefusalV1::ForeignInventoryCursor,
        RefusalV1::StaleInventoryCursor,
        RefusalV1::InvalidInventoryCursor,
        RefusalV1::ForeignChangeCursor,
        RefusalV1::InvalidChangeCursor,
        RefusalV1::NoActiveDocument,
        RefusalV1::Stale,
        RefusalV1::InvalidRange,
        RefusalV1::InputLimit,
        RefusalV1::SnapshotLimit,
        RefusalV1::EditorClosed,
        RefusalV1::EditorCapacity,
        RefusalV1::EditorConflict,
        RefusalV1::EditorIncomplete,
        RefusalV1::EditorUnavailable,
        RefusalV1::CounterExhausted,
    ];
    for refusal in refusals {
        for reply in [
            WireReplyV1::ConnectRefused(refusal),
            WireReplyV1::Executed(ReplyV1::Refused(refusal)),
        ] {
            let encoded = encode_reply_v1(&reply, limits()).unwrap();
            assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
        }
    }

    for outcome in [
        OutcomeV1::WorkspaceInventory(WorkspaceInventoryV1 {
            workspace: WorkspaceIdV1::new(1).unwrap(),
            documents: vec![],
            roots: vec![],
        }),
        OutcomeV1::LocatorResolved(WorkspaceLocatorResolutionV1 {
            workspace: WorkspaceIdV1::new(1).unwrap(),
            document: Some(WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap()),
            root: Some(WorkspaceRootV1 {
                id: WorkspaceRootIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
                kind: WorkspaceRootKindV1::File,
            }),
        }),
        OutcomeV1::WorkspaceInventoryPage(WorkspaceInventoryPageV1 {
            workspace: WorkspaceIdV1::new(1).unwrap(),
            documents: vec![],
            roots: vec![],
            next: None,
        }),
        OutcomeV1::FileAdded(WorkspaceInventoryV1 {
            workspace: WorkspaceIdV1::new(1).unwrap(),
            documents: vec![],
            roots: vec![],
        }),
        OutcomeV1::DirectoryAdded(WorkspaceInventoryV1 {
            workspace: WorkspaceIdV1::new(1).unwrap(),
            documents: vec![],
            roots: vec![],
        }),
        OutcomeV1::FileAdmitted(FileAdmissionReceiptV1 {
            document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
            root: None,
            status: FileAdmissionStatusV1::AlreadyPresent,
        }),
        OutcomeV1::DirectoryAdmitted(DirectoryAdmissionReceiptV1 {
            root: WorkspaceRootIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
            added: 0,
            joined: 1,
            added_documents: vec![],
        }),
        OutcomeV1::ViewOpened(ViewOpenReceiptV1 {
            document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
            status: ViewOpenStatusV1::AlreadyOpen,
        }),
        OutcomeV1::ViewClosed(ViewCloseStatusV1::AlreadyClosed),
        OutcomeV1::DocumentRemoved(DocumentRemovalReceiptV1 {
            document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
            status: MembershipRemovalStatusV1::AlreadyRemoved,
        }),
        OutcomeV1::RootRemoved(RootRemovalReceiptV1 {
            root: WorkspaceRootIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
            status: MembershipRemovalStatusV1::AlreadyRemoved,
        }),
        OutcomeV1::DocumentSelected(
            WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
        ),
        OutcomeV1::DocumentVersionObserved(DocumentVersionObservationV1 {
            document: WorkspaceDocumentIdV1::new(WorkspaceIdV1::new(1).unwrap(), 1).unwrap(),
            revision: 7,
        }),
        OutcomeV1::Saved(SaveReceiptV1 {
            application: SaveApplicationV1::Unknown,
            synchronization: SaveSynchronizationV1::Unknown,
            no_future_effects: false,
            quiescent: false,
            adapter_failure: true,
            cleanup_failed: true,
            baseline_published: false,
            outcome_unknown: true,
        }),
        OutcomeV1::ReplacedAndSaved(ReplaceAndSaveReceiptV1 {
            snapshot: SnapshotV1 {
                witness: VersionWitness::new(1).unwrap(),
                revision: 2,
                text: Arc::from("private durable replacement"),
            },
            persistence: EditPersistenceV1::NotAttempted(RefusalV1::EditorCapacity),
        }),
        OutcomeV1::Snapshot(SnapshotV1 {
            witness: VersionWitness::new(1).unwrap(),
            revision: 2,
            text: Arc::from("private snapshot"),
        }),
        OutcomeV1::Replaced(SnapshotV1 {
            witness: VersionWitness::new(3).unwrap(),
            revision: 4,
            text: Arc::from("private replacement"),
        }),
    ] {
        let reply = WireReplyV1::Executed(ReplyV1::Accepted(outcome));
        let debug = format!("{reply:?}");
        assert!(!debug.contains("private"));
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }

    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(1).unwrap(),
        request: RequestId::new(1).unwrap(),
        command: CommandV1::Snapshot,
    });
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);

    let workspace = WorkspaceIdV1::new(1).unwrap();
    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(1).unwrap(),
        request: RequestId::new(3).unwrap(),
        command: CommandV1::ObserveDocumentVersion {
            document: WorkspaceDocumentIdV1::new(workspace, 2).unwrap(),
        },
    });
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);

    let request = WireRequestV1::Execute(RequestV1 {
        client: ClientId::new(1).unwrap(),
        request: RequestId::new(2).unwrap(),
        command: CommandV1::ResolveLocator {
            locator: "/private/exact/member".into(),
        },
    });
    assert!(!format!("{request:?}").contains("/private/exact/member"));
    let encoded = encode_request_v1(&request, limits()).unwrap();
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), request);

    let disconnect = WireRequestV1::Disconnect(ClientId::new(7).unwrap());
    let encoded = encode_request_v1(&disconnect, limits()).unwrap();
    assert_eq!(decode_request_v1(&encoded, limits()).unwrap(), disconnect);
    for reply in [
        WireReplyV1::Disconnected,
        WireReplyV1::DisconnectRefused(RefusalV1::UnknownClient),
    ] {
        let encoded = encode_reply_v1(&reply, limits()).unwrap();
        assert_eq!(decode_reply_v1(&encoded, limits()).unwrap(), reply);
    }
}
