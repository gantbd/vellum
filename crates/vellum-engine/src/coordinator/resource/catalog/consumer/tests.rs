use super::super::tests::{address, config, facts, limits};
use super::*;

pub(super) fn binding() -> TrustedCatalogSourceReadBindingV1 {
    TrustedCatalogSourceReadBindingV1 {
        profile: limits(),
        expected_generation: 0,
        max_binding_bytes: 16384,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        identity_allowed: Some(true),
        read_allowed: Some(true),
        provider_available: Some(true),
        correspondence_current: Some(true),
        resources_ready: Some(true),
    }
}
fn setup() -> (
    SourceCoordinator,
    Recorder,
    ProviderPort,
    ResourceCatalogAddressV1,
) {
    let (mut o, mut r, _, p) = crate::coordinator::resource::serialization_tests::fixture();
    let a = address(ResourceCatalogTableV1::Read, "source");
    o.configure_resource_catalog_v1(
        &[ResourceCatalogRowV1 {
            address: a.copy().unwrap(),
            generation: 0,
            configuration: config(ResourceCatalogTableV1::Read, "catalog-key"),
        }],
        limits(),
        &facts(),
        &mut r,
    )
    .unwrap();
    (o, r, p, a)
}
#[test]
fn every_host_fact_is_strict_and_refusal_does_not_fault_the_owner() {
    for index in 0..9 {
        for value in [None, Some(false)] {
            let (mut o, mut r, p, a) = setup();
            let mut f = binding();
            let flags = [
                &mut f.authorized,
                &mut f.scope,
                &mut f.continuous,
                &mut f.live,
                &mut f.identity_allowed,
                &mut f.read_allowed,
                &mut f.provider_available,
                &mut f.correspondence_current,
                &mut f.resources_ready,
            ];
            *flags.into_iter().nth(index).unwrap() = value;
            assert!(matches!(
                o.bind_catalog_source_read_target_v1(
                    p.source_read_target_v1(b"native").unwrap(),
                    &a,
                    &f,
                    &mut r
                ),
                Err(ResourceError::Unavailable)
            ));
            assert!(!o.faulted);
        }
    }
}

#[test]
fn correspondence_binding_is_inert_and_capture_modes_do_not_activate_native_work() {
    use crate::instrumentation::NativeTimingProfileV1;
    for mode in 0..4 {
        let (mut o, mut setup, p, a) = setup();
        let before = (o.generation(), o.resource_usage(), o.retention_usage());
        let root = o.native_root_identity().unwrap();
        let target = p.source_read_target_v1(b"native").unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let sentinel = crate::instrumentation::Observation {
            boundary: Boundary::Control,
            disposition: crate::instrumentation::Disposition::Completed,
            service_time: None,
        };
        if mode == 3 {
            r.record(sentinel);
        }
        let bound = o
            .bind_catalog_source_read_target_v1(target, &a, &binding(), &mut r)
            .unwrap();
        assert_eq!(
            (o.generation(), o.resource_usage(), o.retention_usage()),
            before
        );
        assert_eq!(o.native_root_identity().unwrap(), root);
        let capture = bound.catalog.as_ref().unwrap();
        assert_eq!(capture.row.generation, 0);
        assert_eq!(capture.row.address, a);
        assert!(Arc::ptr_eq(&capture.registration, &bound.registration));
        assert_eq!(
            o.source_read_catalog_current(
                Some(capture),
                &bound.registration,
                &bound.identity,
                false,
                &mut setup
            ),
            Ok(())
        );
        drop(bound);
        assert_eq!(
            (o.generation(), o.resource_usage(), o.retention_usage()),
            before
        );
        if mode == 0 {
            assert_eq!((r.queued(), r.dropped()), (0, 0));
        } else if mode == 3 {
            assert_eq!(r.pop_front(), Some(sentinel));
            assert!(r.dropped() > 0);
        } else {
            let event = r.pop_front().unwrap();
            assert_eq!(event.boundary, Boundary::CatalogSourceReadBinding);
            assert_eq!(
                event.disposition,
                crate::instrumentation::Disposition::Completed
            );
            assert_eq!(event.service_time.is_some(), mode == 2);
            assert!(r.pop_front().is_none());
        }
        assert!(!o.is_faulted());
    }
}
#[test]
fn exact_capture_bound_precedes_issuance_and_existing_binding_cannot_be_reinterpreted() {
    let (mut o, mut r, p, a) = setup();
    let t = o
        .bind_catalog_source_read_target_v1(
            p.source_read_target_v1(b"native").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    let bytes = t.catalog.as_ref().unwrap().native_bytes().unwrap();
    let mut f = binding();
    f.max_binding_bytes = bytes - 1;
    assert!(matches!(
        o.bind_catalog_source_read_target_v1(
            p.source_read_target_v1(b"native").unwrap(),
            &a,
            &f,
            &mut r
        ),
        Err(ResourceError::Limit)
    ));
    f.max_binding_bytes = bytes;
    o.bind_catalog_source_read_target_v1(
        p.source_read_target_v1(b"native").unwrap(),
        &a,
        &f,
        &mut r,
    )
    .unwrap();
    assert!(matches!(
        o.bind_catalog_source_read_target_v1(t, &a, &binding(), &mut r),
        Err(ResourceError::Unavailable)
    ));
}
#[test]
fn unknown_identity_is_not_promoted_and_static_integrity_does_not_depend_on_current_rows() {
    let (mut o, mut r, p, a) = setup();
    let t = o
        .bind_catalog_source_read_target_v1(
            p.source_read_target_v1(b"native").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.source_read_catalog_current(
            t.catalog.as_ref(),
            &t.registration,
            &t.identity,
            false,
            &mut r
        ),
        Ok(())
    );
    assert_eq!(
        o.source_read_catalog_current(
            t.catalog.as_ref(),
            &t.registration,
            &t.identity,
            true,
            &mut r
        ),
        Err(ResourceError::Unavailable)
    );
    let plan = o
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: a,
                expected: CatalogExpectationV1::Present { generation: 0 },
                action: ResourceCatalogActionV1::Remove,
            }],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(plan, &facts(), &mut r)
        .unwrap();
    assert!(
        t.catalog
            .as_ref()
            .unwrap()
            .intact(&o.namespace, &t.registration, &t.identity)
            .is_ok()
    );
    assert_eq!(
        o.source_read_catalog_current(
            t.catalog.as_ref(),
            &t.registration,
            &t.identity,
            false,
            &mut r
        ),
        Err(ResourceError::Stale)
    );
    let other = p.source_read_target_v1(b"different").unwrap();
    assert_eq!(
        t.catalog
            .as_ref()
            .unwrap()
            .intact(&o.namespace, &other.registration, &other.identity),
        Err(ResourceError::Faulted)
    );
}

#[test]
fn owned_recovery_preserves_original_catalog_capture_after_withdrawal_without_relaunch() {
    let (mut o, mut r, p, a) = setup();
    o.configure_retention_budget(1048576, &mut r).unwrap();
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
            max_capture_bytes: 16384,
            max_reply_bytes: 512,
            max_label_bytes: 64,
            max_store_bytes: 262144,
        },
        &mut r,
    )
    .unwrap();
    let context = crate::coordinator::serialization::tests::context();
    let f = TrustedSourceReadOperationFactsV1 {
        context: &context,
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
    let raw = p.source_read_target_v1(b"native").unwrap();
    let target = o
        .bind_catalog_source_read_target_v1(
            p.source_read_target_v1(b"native").unwrap(),
            &a,
            &binding(),
            &mut r,
        )
        .unwrap();
    let request = OwnedSourceReadRequestV1 {
        target: &target,
        strength: ReadStrength::CapturedRead,
        origin: SourceReadOperationOriginV1 {
            operation: b"op",
            effect: b"read",
            attempt: b"one",
        },
    };
    let first = o
        .request_owned_source_read_v1(&request, &f, &mut r)
        .unwrap();
    let usage = o.source_reader_usage_v1().unwrap();
    let plan = o
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: a,
                expected: CatalogExpectationV1::Present { generation: 0 },
                action: ResourceCatalogActionV1::Remove,
            }],
            &facts(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(plan, &facts(), &mut r)
        .unwrap();
    let again = o
        .request_owned_source_read_v1(&request, &f, &mut r)
        .unwrap();
    assert_eq!(
        again.status,
        SourceReadOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(again.task, first.task);
    assert_eq!(o.source_reader_usage_v1().unwrap(), usage);
    let unbound = OwnedSourceReadRequestV1 {
        target: &raw,
        ..request
    };
    assert_eq!(
        o.request_owned_source_read_v1(&unbound, &f, &mut r),
        Err(ResourceError::Unavailable)
    );
    assert!(matches!(
        o.submit_source_read_v1(&first.task, &mut r),
        Err(ResourceError::Stale)
    ));
}

#[test]
fn reader_charges_the_complete_binding_and_refuses_before_spending_when_it_does_not_fit() {
    let mut total = 0;
    for bounded in [false, true] {
        let (mut o, mut r, p, a) = setup();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let target = o
            .bind_catalog_source_read_target_v1(
                p.source_read_target_v1(b"native").unwrap(),
                &a,
                &binding(),
                &mut r,
            )
            .unwrap();
        let capture = target.catalog.as_ref().unwrap().native_bytes().unwrap();
        o.configure_source_reader_v1(
            SourceReaderLimitsV1 {
                max_live: 2,
                max_issued: 4,
                max_input_bytes: 128,
                max_chunks: 8,
                max_held_bytes: if bounded { total - 1 } else { 65536 },
                max_store_bytes: 131072,
            },
            &mut r,
        )
        .unwrap();
        let result = o.admit_source_read_v1(target, ReadStrength::CapturedRead, &mut r);
        let usage = o.source_reader_usage_v1().unwrap();
        if bounded {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(usage.issued, 0);
            assert_eq!(usage.held_bytes, 0);
        } else {
            result.unwrap();
            total = usage.held_bytes;
            assert!(total > capture);
        }
    }
}
