use super::super::tests::{facts, limits, policy, pool, setup};
use super::*;
fn decoder_limits() -> NativeOperationDecoderLimitsV1 {
    NativeOperationDecoderLimitsV1 {
        input: CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 256,
            max_depth: 8,
        },
        max_held_bytes: 262144,
    }
}
fn control(c: &SettingsExecutionContextV1) -> NativeOperationObservationControlV1<'_> {
    NativeOperationObservationControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn envelope<T>(
    task: &SaveId,
    id: &NativeOperationObservationIdV1,
    c: &SettingsExecutionContextV1,
    run: impl FnOnce(&NativeOperationEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use NativeOperationEnvelopeValueV1 as V;
    let args = [
        (
            "contract",
            V::Data(InertValueV1::Text("native-operation-observation/1")),
        ),
        ("operation", V::Data(InertValueV1::Text("status"))),
        ("task", V::Task(NativeOperationTaskV1::Save(task))),
        ("context", V::Context(id)),
        ("profile", V::Data(InertValueV1::Text(&c.profile))),
    ];
    let header = [
        ("family", V::Data(InertValueV1::Number(5.0))),
        ("variant", V::Data(InertValueV1::Text("status"))),
        ("version", V::Data(InertValueV1::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        ("route", V::Data(InertValueV1::Text("typed"))),
        ("request", V::Object(&header)),
    ];
    run(&V::Object(&outer))
}
#[test]
fn decoder_exact_declared_and_actual_shared_funding_is_prepaid_once_before_enablement() {
    for field in 0..2 {
        for delta in [-1, 0, 1] {
            let mut d = decoder_limits();
            let charge = d.reserved_bytes().unwrap();
            if field == 0 {
                d.max_held_bytes = charge.checked_add_signed(delta).unwrap();
            }
            let budget = if field == 1 {
                (pool().required_pool_bytes().unwrap() + limits().max_held_bytes + charge)
                    .checked_add_signed(delta)
                    .unwrap()
            } else {
                16777216
            };
            let (mut o, mut r, task, c) = setup(budget);
            o.configure_native_operation_observation_v1(pool(), &mut r)
                .unwrap();
            let id = o
                .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
                .unwrap();
            let before = o.retention_usage();
            let result = o.enable_native_operation_decoder_v1(&id, d, &control(&c), &mut r);
            if delta < 0 {
                assert_eq!(result, Err(Error::Limit));
                assert_eq!(o.retention_usage(), before);
            } else {
                result.unwrap();
                assert_eq!(o.retention_usage().reserved, before.reserved + charge);
                let funded = o.retention_usage();
                let cut = crate::coordinator::shutdown::tests::inspect(&o, &mut r);
                let cut = crate::coordinator::shutdown::tests::inspect_read(&o, &cut, &mut r);
                assert!(cut.coverage.all_owner_obligations);
                assert_eq!(cut.funding.parent_reserved_bytes, funded.reserved);
                let owner = cut
                    .owners
                    .iter()
                    .find(|row| row.owner.role() == ShutdownOwnerRoleV1::OperationObservers)
                    .unwrap();
                assert_eq!(
                    owner.additional_reserved_bytes,
                    Some(limits().max_held_bytes + charge)
                );
                o.enable_native_operation_decoder_v1(&id, d, &control(&c), &mut r)
                    .unwrap();
                assert_eq!(o.retention_usage(), funded);
                envelope(&task, &id, &c, |e| {
                    o.query_decoded_native_operation_v1(
                        &id,
                        e,
                        &policy(&c),
                        &facts(&c),
                        None,
                        &mut r,
                    )
                })
                .unwrap();
                assert_eq!(o.retention_usage(), funded);
            }
        }
    }
}
#[test]
fn complete_native_argument_byte_node_and_depth_bounds_are_exact_without_traversing_handles() {
    let (mut o, mut r, task, c) = setup(16777216);
    o.configure_native_operation_observation_v1(pool(), &mut r)
        .unwrap();
    let id = o
        .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
        .unwrap();
    envelope(&task, &id, &c, |e| {
        let baseline =
            normalize_native_operation_envelope_v1(e, &c, decoder_limits().input, 16384, &mut r)
                .unwrap();
        for field in 0..3 {
            let exact = match field {
                0 => baseline
                    .capture_usage()
                    .iter()
                    .map(|u| u.peak_bytes)
                    .max()
                    .unwrap()
                    .max(baseline.owned_bytes()),
                1 => baseline
                    .capture_usage()
                    .iter()
                    .map(|u| u.nodes)
                    .max()
                    .unwrap(),
                _ => baseline
                    .capture_usage()
                    .iter()
                    .map(|u| u.depth)
                    .max()
                    .unwrap(),
            };
            for delta in [-1, 0, 1] {
                let mut cap = decoder_limits().input;
                let bound = exact.checked_add_signed(delta).unwrap();
                match field {
                    0 => cap.max_bytes = bound,
                    1 => cap.max_nodes = bound,
                    _ => cap.max_depth = bound,
                }
                let result =
                    normalize_native_operation_envelope_v1(e, &c, cap, cap.max_bytes, &mut r);
                if delta < 0 {
                    assert_eq!(result, Err(Error::Limit));
                } else {
                    result.unwrap();
                }
            }
        }
    });
}
#[test]
fn decoder_busy_and_local_fault_are_separate_from_native_receipt_fault_and_preserve_protected_control()
 {
    for native_fault in [false, true] {
        let (mut o, mut r, task, c) = setup(16777216);
        o.configure_native_operation_observation_v1(pool(), &mut r)
            .unwrap();
        let id = o
            .open_native_save_observation_v1(&task, &policy(&c), limits(), &facts(&c), &mut r)
            .unwrap();
        o.enable_native_operation_decoder_v1(&id, decoder_limits(), &control(&c), &mut r)
            .unwrap();
        let State::Active(d) = &mut o.observation_row_mut(&id).unwrap().decoder else {
            unreachable!()
        };
        d.busy = true;
        assert_eq!(
            o.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &policy(&c),
                &facts(&c),
                &mut r
            ),
            Err(Error::Unavailable)
        );
        assert_eq!(
            o.retire_native_operation_observation_v1(&id, &control(&c), &mut r),
            Err(Error::Unavailable)
        );
        let State::Active(d) = &mut o.observation_row_mut(&id).unwrap().decoder else {
            unreachable!()
        };
        d.busy = false;
        d.panic = !native_fault;
        if native_fault {
            o.observation_row_mut(&id).unwrap().fault = Some(Fault::WrongKind);
        }
        let funded = o.retention_usage();
        let work = o.resource_usage();
        assert_eq!(
            envelope(&task, &id, &c, |e| o.query_decoded_native_operation_v1(
                &id,
                e,
                &policy(&c),
                &facts(&c),
                None,
                &mut r
            )),
            Err(Error::Faulted)
        );
        assert_eq!(
            o.native_operation_decoder_usage_v1(&id).unwrap().faulted,
            !native_fault
        );
        assert_eq!(
            o.native_operation_observation_usage_v1(&id)
                .unwrap()
                .receipt_faulted,
            native_fault
        );
        assert!(!o.is_faulted());
        assert_eq!(o.retention_usage(), funded);
        assert_eq!(o.resource_usage(), work);
        o.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::MinimalControlStatus,
            &policy(&c),
            &facts(&c),
            &mut r,
        )
        .unwrap();
        o.retire_native_operation_observation_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.resource_usage(), work);
    }
}
