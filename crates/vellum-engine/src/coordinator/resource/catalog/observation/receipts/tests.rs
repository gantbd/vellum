use super::super::tests::{addresses, change, cleanup, facts, fixture, limits, rows};
use super::*;
#[test]
fn publication_admission_fence_registry_preserves_existing_receipt_replay_and_cleanup() {
    let (mut o, mut r, s, command, original) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &command,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let op = ResourceRegistryCommandOperationV1::Establish;
    let admission = TrustedResourceRegistryCommandAdmissionV1 {
        current: &f,
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    };
    let pending = o
        .issue_resource_registry_command_v1(&op, &s, &admission, &mut r)
        .unwrap();
    let a = addresses(&o);
    let rows = rows(&a, None);
    let snapshot = TrustedResourceRegistrySnapshotFactsV1 {
        current: &f,
        rows: &rows,
        capture_policy_current: Some(true),
        visibility_complete: Some(true),
        order_attested: Some(true),
    };
    let p = o
        .prepare_resource_registry_command_snapshot_v1(&pending, &s, &snapshot, &mut r)
        .unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let before = (
        o.resource_registry_observation_usage_v1().unwrap(),
        o.resource_registry_receipt_usage_v1().unwrap(),
        o.retention_usage(),
    );
    assert_eq!(
        o.issue_resource_registry_command_v1(&op, &s, &admission, &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(matches!(
        o.open_resource_registry_observation_v1(&s, &snapshot, &mut r),
        Err(ResourceError::Unavailable)
    ));
    assert_eq!(
        o.open_resource_registry_receipt_v1(
            &command,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r
        ),
        Err(ResourceError::Unavailable)
    );
    let mut q = ResourceRegistryCommandRequestV1 {
        scope: &s,
        operation: &op,
        preparation: Some(&p),
        current: Some(&f),
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    };
    assert_eq!(
        o.request_resource_registry_command_v1(&pending, &q, &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(
        !o.resource_registry_command_facts_v1(&pending, &s, &mut r)
            .unwrap()
            .native_started
    );
    assert_eq!(
        (
            o.resource_registry_observation_usage_v1().unwrap(),
            o.resource_registry_receipt_usage_v1().unwrap(),
            o.retention_usage()
        ),
        before
    );
    q.preparation = None;
    q.current = None;
    assert!(matches!(
        o.request_resource_registry_command_v1(&command, &q, &mut r),
        Ok(ResourceRegistryCommandReplyV1::ObserveExisting { .. })
    ));
    let packet = certificate(
        &mut o,
        &context,
        &command,
        &s,
        &original,
        (true, true),
        &mut r,
    );
    let reply = o
        .release_resource_registry_receipt_v1(&context, &command, &s, &packet, &rf, &mut r)
        .unwrap();
    assert!(Arc::ptr_eq(&reply.receipt, &original));
    o.faulted = true;
    o.stop_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_command_v1(&command, &cleanup(&s), &mut r)
        .unwrap();
}

#[test]
fn stop_v3_registry_and_receipt_keep_commands_and_retention_independent() {
    for typed in [false, true] {
        let (mut o, mut r, s, cmd, result) = setup();
        let f = facts(&s);
        let rf = receipt_facts(&f);
        let one = o
            .open_resource_registry_receipt_v1(
                &cmd,
                &s,
                ResourceRegistryReceiptCeilingV1 { allowed: true },
                &rf,
                Some(true),
                &mut r,
            )
            .unwrap();
        let two = o
            .open_resource_registry_receipt_v1(
                &cmd,
                &s,
                ResourceRegistryReceiptCeilingV1 { allowed: true },
                &rf,
                Some(true),
                &mut r,
            )
            .unwrap();
        let debt = o.retention_usage();
        let before = o.resource_registry_observation_usage_v1().unwrap();
        o.faulted = true;
        for (target, allowed, expected) in [
            (&two, Some(true), false),
            (&one, None, false),
            (&one, Some(false), false),
            (&one, Some(true), true),
            (&one, Some(true), true),
        ] {
            let mut control = cleanup(&s);
            control.owner_control_allowed = allowed;
            let outcome = crate::coordinator::observation_control::request_v3(
                ObservationStopTargetV3::RegistryReceipt(target),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v3(
                        DecodedObservationStopRequestV3 {
                            envelope,
                            native: ObservationStopPortV3::RegistryReceipt {
                                context: &one,
                                control: &control,
                            },
                        },
                        &mut r,
                    )
                },
            );
            assert_eq!(outcome.is_ok(), expected);
        }
        assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 1);
        assert_eq!(
            o.resource_registry_observation_usage_v1().unwrap().stopped,
            0
        );
        assert!(
            o.retire_resource_registry_command_v1(&cmd, &cleanup(&s), &mut r)
                .is_err()
        );
        for allowed in [None, Some(false), Some(true), Some(true)] {
            let mut control = cleanup(&s);
            control.owner_control_allowed = allowed;
            let outcome = crate::coordinator::observation_control::request_v3(
                ObservationStopTargetV3::ResourceRegistry(&result.stream),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v3(
                        DecodedObservationStopRequestV3 {
                            envelope,
                            native: ObservationStopPortV3::ResourceRegistry {
                                stream: &result.stream,
                                control: &control,
                            },
                        },
                        &mut r,
                    )
                },
            );
            assert_eq!(outcome.is_ok(), allowed == Some(true));
        }
        assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 1);
        let after = o.resource_registry_observation_usage_v1().unwrap();
        assert_eq!(after.streams, 1);
        assert_eq!(after.stopped, 1);
        assert_eq!(after.preparations, before.preparations);
        assert_eq!(after.issued_lifetime, before.issued_lifetime);
        assert_eq!(o.retention_usage(), debt);
        assert!(o.faulted);
    }
}
pub(in crate::coordinator) fn setup() -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistryObservationScopeV1,
    ResourceRegistryCommandIdV1,
    Arc<ResourceRegistryBaselineV1>,
) {
    let mut l = limits();
    l.max_packet_bytes = 65536;
    l.max_held_bytes = 2097152;
    l.max_preparations = 8;
    let (mut o, mut r, s) = fixture(l);
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let command = o
        .issue_resource_registry_command_v1(
            &op,
            &s,
            &TrustedResourceRegistryCommandAdmissionV1 {
                current: &f,
                owner_control_allowed: Some(true),
                baseline_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let a = addresses(&o);
    let rows = rows(&a, None);
    let packet = o
        .prepare_resource_registry_command_snapshot_v1(
            &command,
            &s,
            &TrustedResourceRegistrySnapshotFactsV1 {
                current: &f,
                rows: &rows,
                capture_policy_current: Some(true),
                visibility_complete: Some(true),
                order_attested: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.request_resource_registry_command_v1(
        &command,
        &ResourceRegistryCommandRequestV1 {
            operation: &op,
            scope: &s,
            preparation: Some(&packet),
            current: Some(&f),
            owner_control_allowed: Some(true),
            baseline_allowed: Some(true),
        },
        &mut r,
    )
    .unwrap();
    let result = o
        .resource_registry_command_facts_v1(&command, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    (o, r, s, command, result)
}
pub(in crate::coordinator) fn receipt_facts<'a>(
    f: &'a TrustedResourceRegistryObservationFactsV1<'a>,
) -> TrustedResourceRegistryReceiptFactsV1<'a> {
    TrustedResourceRegistryReceiptFactsV1 {
        current: f,
        receipt_allowed: Some(true),
        receipt_grant_active: Some(true),
        authorization_current: Some(true),
    }
}
pub(in crate::coordinator) fn certificate(
    o: &mut SourceCoordinator,
    context: &ResourceRegistryReceiptContextIdV1,
    cmd: &ResourceRegistryCommandIdV1,
    s: &ResourceRegistryObservationScopeV1,
    result: &ResourceRegistryBaselineV1,
    rights: (bool, bool),
    r: &mut Recorder,
) -> ResourceRegistryPreparationIdV1 {
    let f = facts(s);
    let rf = receipt_facts(&f);
    let rows: Vec<_> = result
        .projection
        .rows()
        .iter()
        .map(|row| ResourceRegistryHistoricalRowFactsV1 {
            reference: &row.reference,
            membership_allowed: rights.0,
            descriptor_allowed: rights.1,
        })
        .collect();
    let basis = o
        .resource_registry_receipt_authorization_basis_v1(context, cmd, s, r)
        .unwrap()
        .basis;
    o.prepare_resource_registry_receipt_v1(
        context,
        cmd,
        s,
        &TrustedResourceRegistryReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &rf,
            rows: &rows,
            historical_release_attested: Some(true),
        },
        r,
    )
    .unwrap()
}
#[test]
fn registry_receipt_classification_cannot_cross_a_native_root_change() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let basis = o
        .resource_registry_receipt_authorization_basis_v1(&context, &cmd, &s, &mut r)
        .unwrap()
        .basis;
    let classified: Vec<_> = result
        .projection
        .rows()
        .iter()
        .map(|row| ResourceRegistryHistoricalRowFactsV1 {
            reference: &row.reference,
            membership_allowed: true,
            descriptor_allowed: true,
        })
        .collect();
    o.create(b"classification basis changed", &mut r).unwrap();
    assert!(
        o.prepare_resource_registry_receipt_v1(
            &context,
            &cmd,
            &s,
            &TrustedResourceRegistryReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &rf,
                rows: &classified,
                historical_release_attested: Some(true),
            },
            &mut r,
        )
        .is_err()
    );
    assert_eq!(
        o.resource_registry_receipt_usage_v1().unwrap().certificates,
        0
    );
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
}

#[test]
fn registry_receipt_optional_capture_preserves_results_and_owner_charges() {
    use crate::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 64] {
            let (mut o, _, s, cmd, result) = setup();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let root = o.native_root_identity().unwrap();
            let debt = o.retention_usage();
            let f = facts(&s);
            let rf = receipt_facts(&f);
            let context = o
                .open_resource_registry_receipt_v1(
                    &cmd,
                    &s,
                    ResourceRegistryReceiptCeilingV1 { allowed: true },
                    &rf,
                    Some(true),
                    &mut r,
                )
                .unwrap();
            let packet = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
            let reply = o
                .release_resource_registry_receipt_v1(&context, &cmd, &s, &packet, &rf, &mut r)
                .unwrap();
            assert!(Arc::ptr_eq(&reply.receipt, &result));
            o.validate_resource_registry_receipt_reply_v1(&context, &cmd, &s, &reply, &mut r)
                .unwrap();
            o.stop_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
                .unwrap();
            o.retire_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
                .unwrap();
            assert_eq!(o.native_root_identity().unwrap(), root);
            assert_eq!(o.retention_usage(), debt);
            let usage = o.resource_registry_receipt_usage_v1().unwrap();
            assert_eq!(usage, ResourceRegistryReceiptUsageV1::default());
            if capacity == 64 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                for boundary in [
                    Boundary::RegistryReceiptAdmission,
                    Boundary::RegistryReceiptHostBasis,
                    Boundary::RegistryReceiptPreparation,
                    Boundary::RegistryReceiptRelease,
                    Boundary::RegistryReceiptReplyValidation,
                    Boundary::RegistryReceiptStop,
                    Boundary::RegistryReceiptRetirement,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}

#[test]
fn registry_receipt_foreign_basis_cannot_apply_loss_or_allocate_a_certificate() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let one = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let two = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let basis = o
        .resource_registry_receipt_authorization_basis_v1(&one, &cmd, &s, &mut r)
        .unwrap()
        .basis;
    let mut denied = receipt_facts(&f);
    denied.receipt_grant_active = Some(false);
    assert!(
        o.prepare_resource_registry_receipt_v1(
            &two,
            &cmd,
            &s,
            &TrustedResourceRegistryReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &denied,
                rows: &[],
                historical_release_attested: Some(true),
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    assert_eq!(
        o.resource_registry_receipt_usage_v1().unwrap().certificates,
        0
    );
    let packet = certificate(&mut o, &two, &cmd, &s, &result, (true, true), &mut r);
    assert!(
        o.release_resource_registry_receipt_v1(&two, &cmd, &s, &packet, &rf, &mut r)
            .is_ok()
    );
}

#[test]
fn registry_receipt_context_holds_actual_command_claim_even_after_stopping() {
    let (mut o, mut r, s, cmd, _) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let mut denied = cleanup(&s);
    denied.owner_control_allowed = Some(false);
    assert!(matches!(
        o.retire_resource_registry_command_v1(&cmd, &denied, &mut r),
        Err(ResourceError::Unavailable)
    ));
    assert!(
        o.retire_resource_registry_command_v1(&cmd, &cleanup(&s), &mut r)
            .is_err()
    );
    o.stop_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
        .unwrap();
    assert!(
        o.retire_resource_registry_command_v1(&cmd, &cleanup(&s), &mut r)
            .is_err()
    );
    o.retire_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_command_v1(&cmd, &cleanup(&s), &mut r)
        .unwrap();
}
#[test]
fn registry_receipt_releases_original_complete_result_after_live_stream_retirement_and_slot_removal()
 {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    change(&mut o, 0, false, true, &mut r);
    o.stop_resource_registry_observation_v1(&result.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_observation_v1(&result.stream, &cleanup(&s), &mut r)
        .unwrap();
    let cert = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    let reply = o
        .release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &rf, &mut r)
        .unwrap();
    assert!(Arc::ptr_eq(&reply.receipt, &result));
    assert_eq!(reply.status(), "operation-receipt");
    assert_eq!(reply.facet(), "registry-baseline");
    assert_eq!(reply.receipt.projection.rows().len(), 2);
    assert!(
        o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &rf, &mut r)
            .is_err()
    );
}

#[test]
fn registry_receipt_requires_explicit_ceiling_and_admission_not_generic_stream_control() {
    let (mut o, mut r, s, cmd, _) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let before = o.resource_registry_observation_usage_v1().unwrap();
    for admit in [None, Some(false)] {
        assert!(
            o.open_resource_registry_receipt_v1(
                &cmd,
                &s,
                ResourceRegistryReceiptCeilingV1 { allowed: true },
                &rf,
                admit,
                &mut r
            )
            .is_err()
        );
    }
    assert!(
        o.open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: false },
            &rf,
            Some(true),
            &mut r
        )
        .is_err()
    );
    let mut denied = receipt_facts(&f);
    denied.receipt_grant_active = None;
    assert!(
        o.open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &denied,
            Some(true),
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
}

#[test]
fn registry_receipt_row_denial_never_filters_or_changes_command_result_or_live_queue() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    change(&mut o, 0, false, false, &mut r);
    let p = super::super::tests::snapshot(&mut o, &result.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&result.stream, &s, &p, &f, &mut r)
        .unwrap();
    for rights in [(false, false), (true, false)] {
        let cert = certificate(&mut o, &context, &cmd, &s, &result, rights, &mut r);
        let before = o.resource_registry_observation_usage_v1().unwrap();
        assert!(
            o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &rf, &mut r)
                .is_err()
        );
        assert_eq!(
            o.resource_registry_observation_usage_v1().unwrap().queued,
            before.queued
        );
        assert_eq!(before.queued, 1);
        assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
        assert!(Arc::ptr_eq(
            &result,
            o.resource_registry_command_facts_v1(&cmd, &s, &mut r)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
        ));
    }
    let cert = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    assert!(Arc::ptr_eq(
        &o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &rf, &mut r)
            .unwrap()
            .receipt,
        &result
    ));
}

#[test]
fn registry_receipt_stale_root_and_foreign_context_certificate_cannot_authorize_or_apply_loss() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let one = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let two = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let cert = certificate(&mut o, &one, &cmd, &s, &result, (true, true), &mut r);
    let mut denied = receipt_facts(&f);
    denied.receipt_grant_active = Some(false);
    assert!(
        o.release_resource_registry_receipt_v1(&two, &cmd, &s, &cert, &denied, &mut r)
            .is_err()
    );
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    o.create(b"new current engine root", &mut r).unwrap();
    assert!(
        o.release_resource_registry_receipt_v1(&one, &cmd, &s, &cert, &rf, &mut r)
            .is_err()
    );
    assert_eq!(
        o.resource_registry_receipt_usage_v1().unwrap().certificates,
        1
    );
    o.retire_resource_registry_preparation_v1(&cert, &cleanup(&s), &mut r)
        .unwrap();
    let fresh = certificate(&mut o, &one, &cmd, &s, &result, (true, true), &mut r);
    assert!(
        o.release_resource_registry_receipt_v1(&one, &cmd, &s, &fresh, &rf, &mut r)
            .is_ok()
    );
}

#[test]
fn registry_receipt_real_grant_loss_is_absorbing_but_command_control_and_retention_survive() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let cert = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    let mut revoked = receipt_facts(&f);
    revoked.receipt_grant_active = Some(false);
    assert!(
        o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &revoked, &mut r)
            .is_err()
    );
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 1);
    assert!(
        o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &rf, &mut r)
            .is_err()
    );
    assert!(
        o.retire_resource_registry_command_v1(&cmd, &cleanup(&s), &mut r)
            .is_err()
    );
    let op = ResourceRegistryCommandOperationV1::Establish;
    assert!(matches!(
        o.request_resource_registry_command_v1(
            &cmd,
            &ResourceRegistryCommandRequestV1 {
                operation: &op,
                scope: &s,
                preparation: None,
                current: None,
                owner_control_allowed: Some(true),
                baseline_allowed: None
            },
            &mut r
        ),
        Ok(ResourceRegistryCommandReplyV1::ObserveExisting { .. })
    ));
}

#[test]
fn registry_receipt_authorization_invalidation_and_reply_pressure_preserve_exact_result() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let cert = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    let mut stale = receipt_facts(&f);
    stale.authorization_current = Some(false);
    assert!(
        o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &stale, &mut r)
            .is_err()
    );
    assert!(
        o.release_resource_registry_receipt_v1(&context, &cmd, &s, &cert, &rf, &mut r)
            .is_err()
    );
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    o.retire_resource_registry_preparation_v1(&cert, &cleanup(&s), &mut r)
        .unwrap();
    let fresh = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    let mut wait = facts(&s);
    wait.reply_ready = Some(false);
    let waiting = receipt_facts(&wait);
    assert!(
        o.release_resource_registry_receipt_v1(&context, &cmd, &s, &fresh, &waiting, &mut r)
            .is_err()
    );
    assert_eq!(
        o.resource_registry_receipt_usage_v1().unwrap().certificates,
        1
    );
    assert!(Arc::ptr_eq(
        &o.release_resource_registry_receipt_v1(&context, &cmd, &s, &fresh, &rf, &mut r)
            .unwrap()
            .receipt,
        &result
    ));
}

#[test]
fn registry_receipt_protocol_correlation_quarantines_only_context_and_protected_cleanup_is_independent()
 {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let cert = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    let fake = ResourceRegistryReceiptReplyV1 {
        receipt: Arc::new(ResourceRegistryBaselineV1 {
            stream: result.stream.clone(),
            generation: result.generation.clone(),
            projection: result.projection.clone(),
        }),
    };
    assert!(matches!(
        o.validate_resource_registry_receipt_reply_v1(&context, &cmd, &s, &fake, &mut r),
        Err(ResourceError::Faulted)
    ));
    assert!(!o.is_faulted());
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 1);
    assert!(
        o.retire_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
            .is_err()
    );
    o.faulted = true;
    o.retire_resource_registry_preparation_v1(&cert, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_receipt_v1(&context, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_command_v1(&cmd, &cleanup(&s), &mut r)
        .unwrap();
    assert!(o.is_faulted());
}
