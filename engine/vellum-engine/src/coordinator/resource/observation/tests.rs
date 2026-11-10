use super::*;
#[test]
fn shutdown_inventory_operation_observer_reports_actual_context_and_verified_pool_reservation() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r, task, c) = setup(4194304);
    o.configure_native_operation_observation_v1(pool(), &mut r)
        .unwrap();
    let observer = o
        .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
        .unwrap();
    let before = (
        o.native_operation_observation_pool_usage_v1().unwrap(),
        o.retention_usage(),
    );
    let cut = inspect(&o, &mut r);
    let view = inspect_read(&o, &cut, &mut r);
    let row = view
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::OperationObservers)
        .unwrap();
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.reserved_bytes, Some(before.0.reserved_pool_bytes));
    assert_eq!(
        row.additional_reserved_bytes,
        Some(
            o.native_operation_observation_usage_v1(&observer)
                .unwrap()
                .reserved_ceiling_bytes
        )
    );
    assert_eq!(
        (
            o.native_operation_observation_pool_usage_v1().unwrap(),
            o.retention_usage()
        ),
        before
    );
}
#[test]
fn shutdown_fences_new_observation_storage_and_contexts_but_keeps_original_queries_and_cleanup() {
    for configured in [false, true] {
        let (mut o, mut r, task, c) = setup(4194304);
        let original = if configured {
            o.configure_native_operation_observation_v1(pool(), &mut r)
                .unwrap();
            Some(
                o.open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
                    .unwrap(),
            )
        } else {
            None
        };
        crate::coordinator::shutdown::tests::install(&mut o, &mut r);
        let held = o.retention_usage();
        assert_eq!(
            o.configure_native_operation_observation_v1(pool(), &mut r),
            if configured {
                Ok(())
            } else {
                Err(Error::Unavailable)
            }
        );
        assert_eq!(
            o.open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r),
            Err(Error::Unavailable)
        );
        assert_eq!(o.retention_usage(), held);
        if let Some(id) = original {
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::Status,
                &policy(&c),
                &facts(&c),
                &mut r,
            )
            .unwrap();
            let control = NativeOperationObservationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            o.stop_native_operation_observation_v1(&id, &control, &mut r)
                .unwrap();
            // Queries may issue references; discharge those through the actual
            // record before whole-pool retirement in their existing tests.
            assert!(!o.resource_task(&task).unwrap().stop_requested);
        }
    }
}
#[test]
fn versioned_stop_delegates_native_operation_without_cancelling_producer() {
    for typed in [false, true] {
        let (mut o, mut r, task, c) = setup(4194304);
        o.configure_native_operation_observation_v1(pool(), &mut r)
            .unwrap();
        let a = o
            .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
            .unwrap();
        let b = o
            .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
            .unwrap();
        let ctl = NativeOperationObservationControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: None,
            delivery_drained: None,
        };
        let debt = o.retention_usage();
        for target in [&b, &a, &a] {
            let result = crate::coordinator::observation_control::request(
                ObservationStopTargetV2::NativeOperation(target),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v2(
                        DecodedObservationStopRequestV2 {
                            envelope,
                            native: ObservationStopPortV2::Existing(
                                ObservationStopPortV1::NativeOperation {
                                    observer: &a,
                                    control: &ctl,
                                },
                            ),
                        },
                        &mut r,
                    )
                },
            );
            assert_eq!(result.is_ok(), target == &a);
            assert_eq!(
                o.native_operation_observation_usage_v1(&a).unwrap().live,
                target == &b
            );
            assert!(o.native_operation_observation_usage_v1(&b).unwrap().live);
            assert!(!o.resource_task(&task).unwrap().stop_requested);
            assert_eq!(o.retention_usage(), debt);
        }
    }
}
#[test]
fn heterogeneous_stop_preserves_native_producer_and_observer_debt_on_denial_and_success() {
    for allowed in [None, Some(false), Some(true)] {
        for faulted in [false, true] {
            let (mut o, mut r, task, c) = setup(4194304);
            o.configure_native_operation_observation_v1(pool(), &mut r)
                .unwrap();
            let observer = o
                .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
                .unwrap();
            let debt = o.retention_usage();
            let usage = o.resource_usage();
            o.faulted = faulted;
            let control = NativeOperationObservationControlV1 {
                context: &c,
                owner_control_allowed: allowed,
                retention_released: None,
                delivery_drained: Some(false),
            };
            let result = o.stop_observation_facade_v1(
                ObservationStopPortV1::NativeOperation {
                    observer: &observer,
                    control: &control,
                },
                &mut r,
            );
            assert_eq!(result.is_ok(), allowed == Some(true));
            assert_eq!(
                o.native_operation_observation_usage_v1(&observer)
                    .unwrap()
                    .live,
                allowed != Some(true)
            );
            assert!(!o.resource_task(&task).unwrap().stop_requested);
            assert_eq!(o.is_faulted(), faulted);
            assert_eq!(o.resource_usage(), usage);
            assert_eq!(o.retention_usage(), debt);
            let (mut foreign, mut fr, _, _) = setup(4194304);
            assert!(
                foreign
                    .stop_observation_facade_v1(
                        ObservationStopPortV1::NativeOperation {
                            observer: &observer,
                            control: &control
                        },
                        &mut fr
                    )
                    .is_err()
            );
        }
    }
}
fn source_fixture() -> (
    SourceCoordinator,
    SourceReadIdV1,
    ProviderPort,
    Recorder,
    SettingsExecutionContextV1,
) {
    let (mut o, mut r, _, p) = super::super::serialization_tests::fixture();
    o.configure_retention_budget(4194304, &mut r).unwrap();
    o.configure_source_reader_v1(
        SourceReaderLimitsV1 {
            max_live: 2,
            max_issued: 4,
            max_input_bytes: 128,
            max_chunks: 8,
            max_held_bytes: 65536,
            max_store_bytes: 131072,
        },
        &mut r,
    )
    .unwrap();
    o.configure_source_read_operations_v1(
        SourceReadOperationLimitsV1 {
            max_capture_bytes: 4096,
            max_reply_bytes: 512,
            max_label_bytes: 64,
            max_store_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let c = crate::coordinator::serialization::tests::context();
    let target = p.source_read_target_v1(b"new").unwrap();
    let req = OwnedSourceReadRequestV1 {
        origin: SourceReadOperationOriginV1 {
            operation: b"load",
            effect: b"read",
            attempt: b"one",
        },
        target: &target,
        strength: ReadStrength::CapturedRead,
    };
    let facts = TrustedSourceReadOperationFactsV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        provider_available: Some(true),
        launch_allowed: Some(true),
    };
    let task = o
        .request_owned_source_read_v1(&req, &facts, &mut r)
        .unwrap()
        .task;
    o.configure_native_operation_observation_v1(pool(), &mut r)
        .unwrap();
    (o, task, p, r, c)
}
#[test]
fn source_read_reply_envelope_funds_two_complete_epoch_token_end_witnesses() {
    for delta in [-1, 0, 1] {
        let (mut o, task, p, mut r, c) = source_fixture();
        let dispatch = o.submit_source_read_v1(&task, &mut r).unwrap();
        let max = p.registration.limits.max_validator_bytes;
        let first_epoch = vec![b'e'; max];
        let first_token = vec![b't'; max];
        let second_epoch = vec![b'f'; max];
        let second_token = vec![b'u'; max];
        let first = SourceReadVersionV1::EpochToken(ResourceVersionV1 {
            epoch: &first_epoch,
            token: &first_token,
        });
        let second = SourceReadVersionV1::EpochToken(ResourceVersionV1 {
            epoch: &second_epoch,
            token: &second_token,
        });
        for e in [
            p.source_read_start_v1(&dispatch, first).unwrap(),
            p.source_read_end_v1(&dispatch, first, 0, 0).unwrap(),
            p.source_read_end_v1(&dispatch, second, 0, 0).unwrap(),
        ] {
            o.record_source_read_v1(e, &mut r).unwrap();
        }
        let mut l = limits();
        l.max_context_bytes = input_bytes(&c, &policy(&c)).unwrap();
        let exact = l.max_context_bytes.max(
            size_of::<NativeOperationWaitReplyV1>()
                + size_of::<NativeSourceReadReceiptV1>()
                + 4 * max,
        );
        l.max_reply_bytes = exact.checked_add_signed(delta).unwrap();
        let funded = o.retention_usage();
        let opened =
            o.open_native_source_read_observation_v1(&task, &policy(&c), l, &facts(&c), &mut r);
        if delta < 0 {
            assert_eq!(opened, Err(Error::Limit));
            assert_eq!(o.retention_usage(), funded);
        } else {
            let id = opened.unwrap();
            let NativeOperationObservationReplyV1::Receipt {
                receipt: NativeOperationReceiptV1::SourceLoad(value),
                ..
            } = o
                .query_native_operation_observation_v1(
                    &id,
                    NativeOperationObservationQueryV1::RetainedOutcome,
                    &policy(&c),
                    &facts(&c),
                    &mut r,
                )
                .unwrap()
            else {
                panic!("source receipt")
            };
            assert!(value.contested);
            assert_eq!(value.witnesses.iter().flatten().count(), 2);
        }
    }
}
#[test]
fn source_read_result_faults_preserve_minimal_control_and_actual_producer_and_parent_debt() {
    for invalid in [false, true] {
        let (mut o, task, p, mut r, c) = source_fixture();
        let dispatch = o.submit_source_read_v1(&task, &mut r).unwrap();
        for e in [
            p.source_read_start_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"))
                .unwrap(),
            p.source_read_end_v1(&dispatch, SourceReadVersionV1::Opaque(b"v"), 0, 0)
                .unwrap(),
        ] {
            o.record_source_read_v1(e, &mut r).unwrap();
        }
        let id = o
            .open_native_source_read_observation_v1(
                &task,
                &policy(&c),
                limits(),
                &facts(&c),
                &mut r,
            )
            .unwrap();
        if invalid {
            o.test_corrupt_source_terminal(&task);
        } else {
            o.observation_row_mut(&id).unwrap().fault = Some(Fault::WrongKind);
        }
        let funded = o.retention_usage();
        let producer = o.source_reader_usage_v1().unwrap();
        let parent = o.source_read_operation_usage_v1().unwrap();
        assert_eq!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &policy(&c),
                &facts(&c),
                &mut r
            ),
            Err(Error::Faulted)
        );
        assert!(
            o.native_operation_observation_usage_v1(&id)
                .unwrap()
                .receipt_faulted
        );
        assert!(matches!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &policy(&c),
                &facts(&c),
                &mut r
            )
            .unwrap(),
            NativeOperationObservationReplyV1::Status { receipt: None, .. }
        ));
        assert!(!o.is_faulted());
        assert_eq!(o.retention_usage(), funded);
        assert_eq!(o.source_reader_usage_v1().unwrap(), producer);
        assert_eq!(o.source_read_operation_usage_v1().unwrap(), parent);
    }
}
#[test]
fn read_attachment_reply_envelope_and_shared_funding_are_checked_before_exposure() {
    for field in 0..2 {
        for delta in [-1, 0, 1] {
            let (mut o, task, _, mut r) = super::super::read::tests::checkpointed_read_mode(true);
            let c = crate::coordinator::serialization::tests::context();
            o.configure_native_operation_observation_v1(pool(), &mut r)
                .unwrap();
            let mut l = limits();
            l.max_context_bytes = input_bytes(&c, &policy(&c)).unwrap();
            let exact = size_of::<NativeOperationWaitReplyV1>()
                + size_of::<SaveEvidence>().max(size_of::<NativeAnnotationReadReceiptV1>())
                + 2 * o.resources.limits.unwrap().max_validator_bytes;
            l.max_reply_bytes = l.max_context_bytes.max(exact);
            if field == 0 {
                l.max_reply_bytes = l.max_reply_bytes.checked_add_signed(delta).unwrap();
            }
            let funding = o.retention_usage();
            if field == 1 {
                l.max_held_bytes = (funding.maximum.unwrap() - funding.reserved)
                    .checked_add_signed(-delta)
                    .unwrap();
            }
            let resources = o.resource_usage();
            let out = o.open_native_annotation_read_observation_v1(
                &task,
                &policy(&c),
                l,
                &facts(&c),
                &mut r,
            );
            if delta < 0 {
                assert_eq!(out, Err(Error::Limit));
                assert_eq!(
                    o.native_operation_observation_pool_usage_v1()
                        .unwrap()
                        .issued_lifetime,
                    0
                );
                assert_eq!(o.retention_usage().reserved, funding.reserved);
            } else {
                let id = out.unwrap();
                o.query_native_operation_observation_v1(
                    &id,
                    NativeOperationObservationQueryV1::RetainedOutcome,
                    &policy(&c),
                    &facts(&c),
                    &mut r,
                )
                .unwrap();
            }
            assert_eq!(o.resource_usage(), resources);
        }
    }
}
#[test]
fn read_result_faults_quarantine_only_the_observer_and_keep_independent_minimal_control() {
    for invalid_metadata in [false, true] {
        let (mut o, task, _, mut r) = super::super::read::tests::checkpointed_read_mode(true);
        let c = crate::coordinator::serialization::tests::context();
        o.configure_native_operation_observation_v1(pool(), &mut r)
            .unwrap();
        let id = o
            .open_native_annotation_read_observation_v1(
                &task,
                &policy(&c),
                limits(),
                &facts(&c),
                &mut r,
            )
            .unwrap();
        if invalid_metadata {
            o.test_corrupt_read_terminal(&task);
        } else {
            o.observation_row_mut(&id).unwrap().fault = Some(Fault::WrongKind);
        }
        let before = o.resource_usage();
        let funded = o.retention_usage();
        let generation = o.generation();
        assert_eq!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &policy(&c),
                &facts(&c),
                &mut r
            ),
            Err(Error::Faulted)
        );
        let usage = o.native_operation_observation_usage_v1(&id).unwrap();
        assert!(usage.receipt_faulted && !usage.busy);
        assert!(matches!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &policy(&c),
                &facts(&c),
                &mut r
            )
            .unwrap(),
            NativeOperationObservationReplyV1::Status { receipt: None, .. }
        ));
        assert!(!o.is_faulted());
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.retention_usage(), funded);
        assert_eq!(o.generation(), generation);
    }
}
pub(super) fn limits() -> NativeOperationObservationLimitsV1 {
    NativeOperationObservationLimitsV1 {
        max_waiters: 2,
        max_issued: 8,
        max_wait_steps: 8,
        max_context_bytes: 8192,
        max_wait_bytes: 512,
        max_reference_bytes: 128,
        max_reply_bytes: 16384,
        max_control_reply_bytes: 1024,
        max_held_bytes: 262144,
        max_label_bytes: 64,
    }
}
pub(super) fn policy(c: &SettingsExecutionContextV1) -> NativeOperationObservationPolicyV1 {
    NativeOperationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 1,
        grant: true,
        publication: false,
        control: true,
        effect: true,
        reasons: true,
        details: true,
        recovery: true,
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> NativeOperationObservationFactsV1<'_> {
    NativeOperationObservationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
        control_allowed: Some(true),
        control_ready: Some(true),
        ordinary_ready: Some(true),
        operation_receipt_admission_allowed: Some(true),
        operation_receipt_allowed: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        disposition_readable: Some(true),
        read_allowed: Some(true),
        reply_ready: Some(true),
        wait_capacity: Some(true),
        wait_clock_valid: Some(true),
        owner_control_allowed: Some(true),
    }
}
pub(super) fn setup(
    budget: usize,
) -> (
    SourceCoordinator,
    Recorder,
    SaveId,
    SettingsExecutionContextV1,
) {
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
    o.configure_save_operations_v1(
        SaveOperationLimitsV1 {
            max_issued: 8,
            max_label_bytes: 64,
            max_capture_bytes: 8192,
            max_reply_bytes: 1024,
            max_store_bytes: 131072,
        },
        &mut r,
    )
    .unwrap();
    let id = super::super::operations::test_admit_source_operation(&mut o, &mut r, &d, &p)
        .unwrap()
        .task;
    o.configure_retention_budget(budget, &mut r).unwrap();
    (
        o,
        r,
        id,
        crate::coordinator::serialization::tests::context(),
    )
}
pub(super) fn pool() -> NativeOperationObservationPoolLimitsV1 {
    NativeOperationObservationPoolLimitsV1 {
        max_contexts: 8,
        max_issued: 8,
        max_pool_bytes: 65536,
    }
}
#[test]
fn exact_native_context_reply_reference_wait_and_shared_ceiling_bounds_precede_attachment() {
    let l = limits();
    assert_eq!(
        l.declared_held_bytes().unwrap(),
        5 * l.max_reply_bytes
            + 2 * l.max_control_reply_bytes
            + 3 * l.max_context_bytes
            + (l.max_waiters + 1) * l.max_wait_bytes
            + (l.max_issued + 4) * l.max_reference_bytes
    );
    for field in 0..7 {
        for delta in [-1, 0, 1] {
            let (mut o, mut r, task, c) = setup(16777216);
            o.configure_native_operation_observation_v1(pool(), &mut r)
                .unwrap();
            let p = policy(&c);
            let mut l = limits();
            let exact = match field {
                0 => l.declared_held_bytes().unwrap(),
                1 => input_bytes(&c, &p).unwrap(),
                2 => size_of::<NativeOperationObservationReplyV1>(),
                3 => size_of::<Reference>() + size_of::<NativeOperationProgressIdV1>(),
                4 => size_of::<Waiter>() + size_of::<NativeOperationWaitIdV1>(),
                5 => pool().required_pool_bytes().unwrap(),
                _ => l.max_context_bytes.max(
                    size_of::<NativeOperationWaitReplyV1>()
                        + size_of::<SaveEvidence>().max(size_of::<NativeAnnotationReadReceiptV1>())
                        + 128,
                ),
            };
            let bound = exact.checked_add_signed(delta).unwrap();
            match field {
                0 => l.max_held_bytes = bound,
                1 => l.max_context_bytes = bound,
                2 => l.max_control_reply_bytes = bound,
                3 => l.max_reference_bytes = bound,
                4 => l.max_wait_bytes = bound,
                5 => {}
                _ => l.max_reply_bytes = bound,
            }
            if field == 5 {
                let (mut o, mut r, _, _) = setup(16777216);
                let mut pool = pool();
                pool.max_pool_bytes = bound;
                let before = o.retention_usage();
                let out = o.configure_native_operation_observation_v1(pool, &mut r);
                if delta < 0 {
                    assert_eq!(out, Err(Error::Limit));
                    assert_eq!(o.retention_usage(), before);
                } else {
                    out.unwrap();
                }
            } else {
                let before = o.retention_usage();
                let resources = o.resource_usage();
                let out = o.open_native_save_observation_v1(&task, &p, l, &facts(&c), &mut r);
                if delta < 0 {
                    assert_eq!(out, Err(Error::Limit));
                    assert_eq!(o.retention_usage(), before);
                    assert_eq!(o.resource_usage(), resources);
                } else {
                    out.unwrap();
                    assert_eq!(
                        o.retention_usage().reserved,
                        before.reserved + l.max_held_bytes
                    );
                }
            }
        }
    }
    let (mut o, mut r, task, c) =
        setup(pool().required_pool_bytes().unwrap() + limits().max_held_bytes - 1);
    o.configure_native_operation_observation_v1(pool(), &mut r)
        .unwrap();
    let before = o.retention_usage();
    assert_eq!(
        o.open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage(), before);
    assert!(o.observation_pool().unwrap().rows.is_empty());
    let mut small = limits();
    small.max_held_bytes = small.declared_held_bytes().unwrap();
    o.open_native_save_observation_v1(&task, &policy(&c), small, &facts(&c), &mut r)
        .unwrap();
    let mut overflow = limits();
    overflow.max_issued = usize::MAX;
    assert_eq!(overflow.declared_held_bytes(), Err(Error::Limit));
}
#[test]
fn result_faults_quarantine_only_the_receipt_path_and_preserve_producer_and_funding() {
    for fault in [Fault::Panic, Fault::WrongKind] {
        let (mut o, mut r, task, c) = setup(16777216);
        o.configure_native_operation_observation_v1(pool(), &mut r)
            .unwrap();
        let p = policy(&c);
        let id = o
            .open_native_save_observation_v1(&task, &p, limits(), &facts(&c), &mut r)
            .unwrap();
        o.observation_row_mut(&id).unwrap().fault = Some(fault);
        let before = o.resource_usage();
        let funding = o.retention_usage();
        let generation = o.generation;
        assert_eq!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &p,
                &facts(&c),
                &mut r
            ),
            Err(Error::Faulted)
        );
        let usage = o.native_operation_observation_usage_v1(&id).unwrap();
        assert!(!usage.busy);
        assert!(usage.receipt_faulted);
        assert!(!o.is_faulted());
        let minimal = o
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &facts(&c),
                &mut r,
            )
            .unwrap();
        assert!(matches!(
            minimal,
            NativeOperationObservationReplyV1::Status { receipt: None, .. }
        ));
        assert!(matches!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::Status,
                &p,
                &facts(&c),
                &mut r
            )
            .unwrap(),
            NativeOperationObservationReplyV1::Status { receipt: None, .. }
        ));
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.retention_usage(), funding);
        assert_eq!(o.generation, generation);
    }
}
#[test]
fn busy_unowned_references_and_reply_bound_drift_refuse_without_forged_progress_or_refunds() {
    let (mut o, mut r, task, c) = setup(16777216);
    o.configure_native_operation_observation_v1(pool(), &mut r)
        .unwrap();
    let p = policy(&c);
    let id = o
        .open_native_save_observation_v1(&task, &p, limits(), &facts(&c), &mut r)
        .unwrap();
    o.observation_row_mut(&id).unwrap().busy = true;
    assert_eq!(
        o.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::Status,
            &p,
            &facts(&c),
            &mut r
        ),
        Err(Error::Unavailable)
    );
    assert!(!o.observation_row(&id).unwrap().receipt_faulted);
    o.observation_row_mut(&id).unwrap().busy = false;
    let unowned = NativeOperationProgressIdV1 {
        owner: o.namespace.clone(),
        context: id.marker.clone(),
        marker: Arc::new(()),
    };
    assert_eq!(
        o.start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: &unowned,
                budget: 1,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 0 }
            },
            &p,
            &facts(&c),
            &mut r
        ),
        Err(Error::Unavailable)
    );
    assert_eq!(
        o.native_operation_observation_usage_v1(&id)
            .unwrap()
            .ordinary_issued,
        0
    );
    // Corrupt the retained protected reply bound; delivery must still fit it.
    let record = o.observation_row_mut(&id).unwrap();
    record.limits.max_control_reply_bytes = 0;
    let funding = o.retention_usage();
    assert_eq!(
        o.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::MinimalControlStatus,
            &p,
            &facts(&c),
            &mut r
        ),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage(), funding);
    assert_eq!(
        o.native_operation_observation_usage_v1(&id)
            .unwrap()
            .control_issued,
        0
    );
}
