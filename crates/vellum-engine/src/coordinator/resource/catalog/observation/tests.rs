use super::*;

#[test]
fn writer_identity_metadata_is_observed_only_with_descriptor_permission() {
    for permitted in [false, true] {
        let (mut o, mut r, mut s) = fixture(limits());
        s.write_descriptors = permitted;
        let opened = open(&mut o, &s, None, &mut r);
        let writer_flag = |projection: &ResourceRegistryProjectionV1| {
            let writer = projection
                .rows()
                .iter()
                .find(|row| row.table == ResourceCatalogTableV1::Write)
                .unwrap();
            assert_eq!(writer.descriptor.is_some(), permitted);
            assert!(
                projection
                    .rows()
                    .iter()
                    .filter(|row| row.table == ResourceCatalogTableV1::Read)
                    .all(|row| row.identity_known.is_none())
            );
            writer.identity_known
        };
        assert_eq!(writer_flag(&opened.projection), permitted.then_some(false));
        let (root, _) = o.resources.catalog.current().unwrap();
        let row = root
            .rows
            .iter()
            .find(|row| row.address.table == ResourceCatalogTableV1::Write)
            .unwrap();
        let mut configuration = row.configuration.copy().unwrap();
        configuration.identity_known = true;
        let change = ResourceCatalogChangeV1 {
            address: row.address.copy().unwrap(),
            expected: CatalogExpectationV1::Present {
                generation: row.generation,
            },
            action: ResourceCatalogActionV1::Install(configuration),
        };
        let catalog_facts = super::super::tests::facts();
        let prepared = o
            .prepare_resource_catalog_v1(&[change], &catalog_facts, &mut r)
            .unwrap();
        o.commit_resource_catalog_v1(prepared, &catalog_facts, &mut r)
            .unwrap();
        let packet = snapshot(&mut o, &opened.stream, &s, None, &mut r);
        let current = facts(&s);
        let sampled = o
            .sample_resource_registry_observation_v1(&opened.stream, &s, &packet, &current, &mut r)
            .unwrap();
        assert_eq!(
            sampled,
            if permitted {
                ResourceRegistrySampleV1::Queued
            } else {
                ResourceRegistrySampleV1::Unchanged
            }
        );
        if permitted {
            let cert = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
            let ResourceRegistryDeliveryV1::Projection(event) = o
                .release_resource_registry_event_v1(
                    &opened.stream,
                    &s,
                    Some(&cert),
                    &current,
                    &mut r,
                )
                .unwrap()
            else {
                panic!("authorized writer change must be delivered")
            };
            assert_eq!(writer_flag(&event.projection), Some(true));
        }
        // Previously released metadata is immutable, not a live catalog view.
        assert_eq!(writer_flag(&opened.projection), permitted.then_some(false));
    }
}
#[test]
fn publication_admission_fence_blocks_new_registry_owner_without_spending_funding() {
    let (mut o, mut r, _) = fixture_with_observation(limits(), false);
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let debt = o.retention_usage();
    assert_eq!(
        o.configure_resource_registry_observation_v1("a", "e", limits(), &control(), &mut r),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(o.retention_usage(), debt);
    assert!(matches!(o.registry_observations, State::Unconfigured));
}
#[test]
fn shutdown_inventory_reports_real_registry_observer_and_packet_slots_without_consuming_them() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let _packet = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    let before = o.resource_registry_observation_usage_v1().unwrap();
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::RegistryObservation, &[]);
    assert!(row.retained_records.unwrap() >= 2);
    assert_eq!(
        row.reserved_bytes,
        Some(limits().required_pool_bytes().unwrap())
    );
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
}

#[test]
fn heterogeneous_event_registry_parent_pressure_preserves_exact_head_and_certificate() {
    let (mut temporary, mut tr, ts) = fixture(limits());
    let original = open(&mut temporary, &ts, None, &mut tr);
    let frame = projection::frame_bytes(&original.projection).unwrap();
    let mut l = limits();
    l.max_reply_bytes = frame;
    let (mut o, mut r, s) = fixture(l);
    let opened = open(&mut o, &s, None, &mut r);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    let f = facts(&s);
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let p = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
    let before = o.resource_registry_observation_usage_v1().unwrap();
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::ResourceRegistry(&opened.stream),
            ObservationEventPortV1::ResourceRegistry {
                stream: &opened.stream,
                scope: &s,
                certificate: Some(&p),
                facts: &f
            },
            &mut r
        ),
        Err(ObservationEventErrorV1::ResourceRegistry(
            ResourceError::Limit
        ))
    ));
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
    let ResourceRegistryDeliveryV1::Projection(event) = o
        .release_resource_registry_event_v1(&opened.stream, &s, Some(&p), &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(event.before, opened.generation);
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().queued,
        0
    );
}

#[test]
fn heterogeneous_event_registry_delivers_authorized_native_head_and_gap_without_aliasing() {
    let (mut o, mut r, s) = fixture(limits());
    let one = open(&mut o, &s, None, &mut r);
    let two = open(&mut o, &s, None, &mut r);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &one.stream, &s, None, &mut r);
    let f = facts(&s);
    o.sample_resource_registry_observation_v1(&one.stream, &s, &p, &f, &mut r)
        .unwrap();
    let p = certificate(&mut o, &one.stream, &s, true, true, &mut r);
    let mut lost = facts(&s);
    lost.authorized = Some(false);
    let before = o.resource_registry_observation_usage_v1().unwrap();
    assert!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::ResourceRegistry(&two.stream),
            ObservationEventPortV1::ResourceRegistry {
                stream: &one.stream,
                scope: &s,
                certificate: Some(&p),
                facts: &lost
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
    let ObservationEventDeliveryV1::ResourceRegistry(ResourceRegistryDeliveryV1::Projection(event)) =
        o.release_observation_event_v1(
            ObservationEventTargetV1::ResourceRegistry(&one.stream),
            ObservationEventPortV1::ResourceRegistry {
                stream: &one.stream,
                scope: &s,
                certificate: Some(&p),
                facts: &f,
            },
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(event.before, one.generation);
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::ResourceRegistry(&one.stream),
            ObservationEventPortV1::ResourceRegistry {
                stream: &one.stream,
                scope: &s,
                certificate: None,
                facts: &f
            },
            &mut r
        ),
        Ok(ObservationEventDeliveryV1::ResourceRegistry(
            ResourceRegistryDeliveryV1::Empty
        ))
    ));
    change(&mut o, 0, false, false, &mut r);
    let next = snapshot(&mut o, &one.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&one.stream, &s, &next, &f, &mut r)
        .unwrap();
    let denied = certificate(&mut o, &one.stream, &s, false, false, &mut r);
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::ResourceRegistry(&one.stream),
            ObservationEventPortV1::ResourceRegistry {
                stream: &one.stream,
                scope: &s,
                certificate: Some(&denied),
                facts: &f
            },
            &mut r
        ),
        Ok(ObservationEventDeliveryV1::ResourceRegistry(
            ResourceRegistryDeliveryV1::Gap
        ))
    ));
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::ResourceRegistry(&one.stream),
            ObservationEventPortV1::ResourceRegistry {
                stream: &one.stream,
                scope: &s,
                certificate: None,
                facts: &f
            },
            &mut r
        ),
        Ok(ObservationEventDeliveryV1::ResourceRegistry(
            ResourceRegistryDeliveryV1::Gap
        ))
    ));
}

#[test]
fn registry_observation_historical_classification_input_cannot_be_reused_after_root_change() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let old = o
        .inspect_resource_registry_head_v1(&opened.stream, &mut r)
        .unwrap()
        .unwrap();
    let address = o
        .resource_registry_historical_address_v1(
            &old.head,
            &old.projection.rows()[0].reference,
            &mut r,
        )
        .unwrap();
    assert_eq!(address.table, old.projection.rows()[0].table);
    let rows: Vec<_> = old
        .projection
        .rows()
        .iter()
        .map(|r| ResourceRegistryHistoricalRowFactsV1 {
            reference: &r.reference,
            membership_allowed: true,
            descriptor_allowed: true,
        })
        .collect();
    change(&mut o, 0, true, false, &mut r);
    assert!(
        o.resource_registry_historical_address_v1(
            &old.head,
            &old.projection.rows()[0].reference,
            &mut r
        )
        .is_err()
    );
    assert!(
        o.prepare_resource_registry_historical_release_v1(
            &opened.stream,
            &s,
            &old.head,
            &TrustedResourceRegistryHistoricalFactsV1 {
                current: &f,
                rows: &rows,
                historical_release_attested: Some(true)
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .preparations,
        0
    );
}

#[test]
fn registry_observation_logical_slot_reinstall_keeps_reference_but_new_incarnation_does_not() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let original = opened
        .projection
        .rows()
        .iter()
        .find(|r| r.table == ResourceCatalogTableV1::Read)
        .unwrap()
        .reference
        .clone();
    for incarnation in [0, 1] {
        let index = o
            .resources
            .catalog
            .current()
            .unwrap()
            .0
            .rows
            .iter()
            .position(|r| r.address.table == ResourceCatalogTableV1::Read)
            .unwrap();
        change(&mut o, index, false, true, &mut r);
        let f = facts(&s);
        let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
        o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
        let cert = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r)
            .unwrap();
        let mut address = super::super::tests::address(ResourceCatalogTableV1::Read, "read");
        address.provider.incarnation = incarnation;
        let command = ResourceCatalogChangeV1 {
            address,
            expected: CatalogExpectationV1::Absent,
            action: ResourceCatalogActionV1::Install(super::super::tests::config(
                ResourceCatalogTableV1::Read,
                "reinstalled private key",
            )),
        };
        let cf = super::super::tests::facts();
        let p = o
            .prepare_resource_catalog_v1(&[command], &cf, &mut r)
            .unwrap();
        o.commit_resource_catalog_v1(p, &cf, &mut r).unwrap();
        let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
        o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
        let cert = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
        let ResourceRegistryDeliveryV1::Projection(event) = o
            .release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r)
            .unwrap()
        else {
            panic!()
        };
        let reference = &event
            .projection
            .rows()
            .iter()
            .find(|r| r.table == ResourceCatalogTableV1::Read)
            .unwrap()
            .reference;
        assert_eq!(reference == &original, incarnation == 0);
    }
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().bindings,
        3
    );
}

#[test]
fn registry_observation_foreign_packet_or_head_claim_cannot_apply_attributed_loss() {
    let (mut o, mut r, s) = fixture(limits());
    let local = open(&mut o, &s, None, &mut r);
    let other = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    change(&mut o, 0, false, false, &mut r);
    for id in [&local.stream, &other.stream] {
        let p = snapshot(&mut o, id, &s, None, &mut r);
        o.sample_resource_registry_observation_v1(id, &s, &p, &f, &mut r)
            .unwrap();
    }
    let foreign_head = o
        .inspect_resource_registry_head_v1(&other.stream, &mut r)
        .unwrap()
        .unwrap();
    let rows: Vec<_> = foreign_head
        .projection
        .rows()
        .iter()
        .map(|r| ResourceRegistryHistoricalRowFactsV1 {
            reference: &r.reference,
            membership_allowed: true,
            descriptor_allowed: true,
        })
        .collect();
    let mut denied = facts(&s);
    denied.authorized = Some(false);
    assert!(
        o.prepare_resource_registry_historical_release_v1(
            &local.stream,
            &s,
            &foreign_head.head,
            &TrustedResourceRegistryHistoricalFactsV1 {
                current: &denied,
                rows: &rows,
                historical_release_attested: Some(true)
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().stopped,
        0
    );
    let foreign_certificate = certificate(&mut o, &other.stream, &s, true, true, &mut r);
    assert!(
        o.release_resource_registry_event_v1(
            &local.stream,
            &s,
            Some(&foreign_certificate),
            &denied,
            &mut r
        )
        .is_err()
    );
    let foreign_packet = snapshot(&mut o, &other.stream, &s, None, &mut r);
    assert!(
        o.sample_resource_registry_observation_v1(
            &local.stream,
            &s,
            &foreign_packet,
            &denied,
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().stopped,
        0
    );
}

#[test]
fn registry_observation_projection_pressure_does_not_reject_published_catalog_changes() {
    let mut l = limits();
    l.max_bindings = 2;
    let (mut o, mut r, s) = fixture(l);
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    let address = super::super::tests::address(ResourceCatalogTableV1::Read, "extra");
    let change = ResourceCatalogChangeV1 {
        address,
        expected: CatalogExpectationV1::Absent,
        action: ResourceCatalogActionV1::Install(super::super::tests::config(
            ResourceCatalogTableV1::Read,
            "another private key",
        )),
    };
    let cf = super::super::tests::facts();
    let p = o
        .prepare_resource_catalog_v1(&[change], &cf, &mut r)
        .unwrap();
    o.commit_resource_catalog_v1(p, &cf, &mut r).unwrap();
    assert_eq!(o.resources.catalog.current().unwrap().0.rows.len(), 3);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    let issued = o
        .resource_registry_observation_usage_v1()
        .unwrap()
        .issued_lifetime;
    assert_eq!(
        o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(ResourceRegistrySampleV1::Gap)
    );
    let u = o.resource_registry_observation_usage_v1().unwrap();
    assert_eq!(u.bindings, 2);
    assert_eq!(u.issued_lifetime, issued);
    let hidden = o
        .resources
        .catalog
        .current()
        .unwrap()
        .0
        .rows
        .iter()
        .position(|r| r.address.resource.id == "extra")
        .unwrap();
    let p = snapshot(&mut o, &opened.stream, &s, Some(hidden), &mut r);
    let fresh = o
        .resync_resource_registry_observation_v1(&opened.stream, &s, &p, &f, Some(true), &mut r)
        .unwrap();
    for row in fresh.projection.rows() {
        assert!(
            opened
                .projection
                .rows()
                .iter()
                .any(|old| old.reference == row.reference)
        );
    }
}

#[test]
fn registry_observation_foreign_handles_and_wrong_claims_cannot_apply_loss() {
    let (mut a, mut ar, s) = fixture(limits());
    let (mut b, mut br, bs) = fixture(limits());
    let local = open(&mut a, &s, None, &mut ar);
    let foreign = open(&mut b, &bs, None, &mut br);
    assert_ne!(local.stream, foreign.stream);
    assert_ne!(local.generation, foreign.generation);
    assert_ne!(
        local.projection.rows()[0].reference,
        foreign.projection.rows()[0].reference
    );
    assert!(
        a.stop_resource_registry_observation_v1(&foreign.stream, &cleanup(&s), &mut ar)
            .is_err()
    );
    let mut wrong = s.clone();
    wrong.context.recipient = "other".into();
    let mut f = facts(&s);
    f.authorized = Some(false);
    assert!(
        a.release_resource_registry_event_v1(&local.stream, &wrong, None, &f, &mut ar)
            .is_err()
    );
    assert_eq!(
        a.resource_registry_observation_usage_v1().unwrap().stopped,
        0
    );
    assert!(
        a.release_resource_registry_event_v1(&local.stream, &s, None, &f, &mut ar)
            .is_err()
    );
    assert_eq!(
        a.resource_registry_observation_usage_v1().unwrap().stopped,
        1
    );
    f.authorized = Some(true);
    assert!(
        a.release_resource_registry_event_v1(&local.stream, &s, None, &f, &mut ar)
            .is_err()
    );
}

#[test]
fn registry_observation_preparation_does_not_wait_for_reply_and_release_pressure_keeps_exact_certificate()
 {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    assert!(matches!(
        o.release_resource_registry_event_v1(&opened.stream, &s, None, &f, &mut r),
        Ok(ResourceRegistryDeliveryV1::Empty)
    ));
    change(&mut o, 0, false, false, &mut r);
    let a = addresses(&o);
    let rows = rows(&a, None);
    let mut waiting = facts(&s);
    waiting.reply_ready = Some(false);
    waiting.transfer_ready = Some(false);
    let p = o
        .prepare_resource_registry_snapshot_v1(
            &opened.stream,
            &s,
            &TrustedResourceRegistrySnapshotFactsV1 {
                current: &waiting,
                rows: &rows,
                capture_policy_current: Some(true),
                visibility_complete: Some(true),
                order_attested: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &waiting, &mut r)
        .unwrap();
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, None, &f, &mut r)
            .is_err()
    );
    let head = o
        .inspect_resource_registry_head_v1(&opened.stream, &mut r)
        .unwrap()
        .unwrap();
    let rows: Vec<_> = head
        .projection
        .rows()
        .iter()
        .map(|row| ResourceRegistryHistoricalRowFactsV1 {
            reference: &row.reference,
            membership_allowed: true,
            descriptor_allowed: true,
        })
        .collect();
    let cert = o
        .prepare_resource_registry_historical_release_v1(
            &opened.stream,
            &s,
            &head.head,
            &TrustedResourceRegistryHistoricalFactsV1 {
                current: &waiting,
                rows: &rows,
                historical_release_attested: Some(true),
            },
            &mut r,
        )
        .unwrap();
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &waiting, &mut r)
            .is_err()
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().queued,
        1
    );
    assert!(matches!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r),
        Ok(ResourceRegistryDeliveryV1::Projection(_))
    ));
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r)
            .is_err()
    );
}

#[test]
fn registry_observation_capture_modes_preserve_events_and_lifetime_ceiling_gaps_without_refill() {
    use crate::instrumentation::NativeTimingProfileV1;
    for capacity in [0, 1, 64] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            let (mut o, mut setup, s) = fixture(limits());
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let opened = open(&mut o, &s, None, &mut r);
            let f = facts(&s);
            change(&mut o, 0, false, false, &mut setup);
            let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
            o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
                .unwrap();
            let cert = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
            let ResourceRegistryDeliveryV1::Projection(event) = o
                .release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r)
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(event.before, opened.generation);
            assert_eq!(event.projection.rows().len(), 2);
            if capacity == 64 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::RegistryObservationRelease
                            && e.service_time.is_some()
                                == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
                );
            }
        }
    }
    let mut l = limits();
    l.max_issued = 4;
    let (mut o, mut r, s) = fixture(l);
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    assert_eq!(
        o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(ResourceRegistrySampleV1::Gap)
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .issued_lifetime,
        4
    );
    assert!(snapshot_attempt(&mut o, &opened.stream, &s, &mut r).is_err());
    o.stop_resource_registry_observation_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_observation_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .issued_lifetime,
        4
    );
}
fn snapshot_attempt(
    o: &mut SourceCoordinator,
    id: &ResourceRegistryStreamIdV1,
    s: &ResourceRegistryObservationScopeV1,
    r: &mut Recorder,
) -> Result<ResourceRegistryPreparationIdV1> {
    let a = addresses(o);
    let rows = rows(&a, None);
    let f = facts(s);
    o.prepare_resource_registry_snapshot_v1(
        id,
        s,
        &TrustedResourceRegistrySnapshotFactsV1 {
            current: &f,
            rows: &rows,
            capture_policy_current: Some(true),
            visibility_complete: Some(true),
            order_attested: Some(true),
        },
        r,
    )
}
pub(super) fn limits() -> ResourceRegistryObservationLimitsV1 {
    ResourceRegistryObservationLimitsV1 {
        max_streams: 2,
        max_preparations: 4,
        max_rows: 8,
        max_bindings: 8,
        max_issued: 128,
        max_root_bytes: 1048576,
        max_profile_bytes: 1024,
        max_scope_bytes: 2048,
        max_tag_bytes: 128,
        max_packet_bytes: 8192,
        max_projection_bytes: 8192,
        max_binding_bytes: 4096,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_held_bytes: 1048576,
    }
}
pub(super) fn control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn cleanup(
    s: &ResourceRegistryObservationScopeV1,
) -> ResourceRegistryObservationControlV1<'_> {
    ResourceRegistryObservationControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn fixture(
    l: ResourceRegistryObservationLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistryObservationScopeV1,
) {
    fixture_with_observation(l, true)
}
fn fixture_with_observation(
    l: ResourceRegistryObservationLimitsV1,
    configured: bool,
) -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistryObservationScopeV1,
) {
    let mut o = super::super::tests::bare();
    let mut r = Recorder::disabled();
    let rows = [ResourceCatalogTableV1::Read, ResourceCatalogTableV1::Write].map(|table| {
        ResourceCatalogRowV1 {
            address: super::super::tests::address(
                table,
                if table == ResourceCatalogTableV1::Read {
                    "read"
                } else {
                    "write"
                },
            ),
            generation: 0,
            configuration: super::super::tests::config(table, "PRIVATE_KEY"),
        }
    });
    o.configure_resource_catalog_v1(
        &rows,
        super::super::tests::limits(),
        &super::super::tests::facts(),
        &mut r,
    )
    .unwrap();
    o.configure_retention_budget(2097152, &mut r).unwrap();
    if configured {
        o.configure_resource_registry_observation_v1("a", "e", l, &control(), &mut r)
            .unwrap();
    }
    (
        o,
        r,
        ResourceRegistryObservationScopeV1 {
            context: SettingsExecutionContextV1 {
                authority: "a".into(),
                engine: "e".into(),
                recipient: "r".into(),
                purpose: "p".into(),
                destination: "d".into(),
                profile: RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1.into(),
                continuity: "c".into(),
            },
            policy: "policy".into(),
            read_descriptors: true,
            write_descriptors: true,
        },
    )
}
pub(in crate::coordinator) fn facts(
    s: &ResourceRegistryObservationScopeV1,
) -> TrustedResourceRegistryObservationFactsV1<'_> {
    TrustedResourceRegistryObservationFactsV1 {
        scope: s,
        native_profile: super::super::tests::limits(),
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        projection_allowed: Some(true),
        slot_identity_allowed: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
pub(super) fn addresses(o: &SourceCoordinator) -> Vec<ResourceCatalogAddressV1> {
    o.resources
        .catalog
        .current()
        .unwrap()
        .0
        .rows
        .iter()
        .map(|r| r.address.copy().unwrap())
        .collect()
}
pub(super) fn rows<'a>(
    a: &'a [ResourceCatalogAddressV1],
    hidden: Option<usize>,
) -> Vec<ResourceRegistrySnapshotRowFactsV1<'a>> {
    let mut rank = 0;
    a.iter()
        .enumerate()
        .map(|(i, address)| {
            let visible = hidden != Some(i);
            let visible_order = if visible {
                let n = rank;
                rank += 1;
                Some(n)
            } else {
                None
            };
            ResourceRegistrySnapshotRowFactsV1 {
                address,
                captured_visible: visible,
                current_visible: visible,
                captured_descriptor: visible,
                current_descriptor: visible,
                visible_order,
            }
        })
        .collect()
}
pub(super) fn open(
    o: &mut SourceCoordinator,
    s: &ResourceRegistryObservationScopeV1,
    hidden: Option<usize>,
    r: &mut Recorder,
) -> ResourceRegistryBaselineV1 {
    let a = addresses(o);
    let rows = rows(&a, hidden);
    let f = facts(s);
    o.open_resource_registry_observation_v1(
        s,
        &TrustedResourceRegistrySnapshotFactsV1 {
            current: &f,
            rows: &rows,
            capture_policy_current: Some(true),
            visibility_complete: Some(true),
            order_attested: Some(true),
        },
        r,
    )
    .unwrap()
}
pub(super) fn snapshot(
    o: &mut SourceCoordinator,
    id: &ResourceRegistryStreamIdV1,
    s: &ResourceRegistryObservationScopeV1,
    hidden: Option<usize>,
    r: &mut Recorder,
) -> ResourceRegistryPreparationIdV1 {
    let a = addresses(o);
    let rows = rows(&a, hidden);
    let f = facts(s);
    o.prepare_resource_registry_snapshot_v1(
        id,
        s,
        &TrustedResourceRegistrySnapshotFactsV1 {
            current: &f,
            rows: &rows,
            capture_policy_current: Some(true),
            visibility_complete: Some(true),
            order_attested: Some(true),
        },
        r,
    )
    .unwrap()
}
fn certificate(
    o: &mut SourceCoordinator,
    id: &ResourceRegistryStreamIdV1,
    s: &ResourceRegistryObservationScopeV1,
    member: bool,
    descriptor: bool,
    r: &mut Recorder,
) -> ResourceRegistryPreparationIdV1 {
    let head = o.inspect_resource_registry_head_v1(id, r).unwrap().unwrap();
    let rows: Vec<_> = head
        .projection
        .rows()
        .iter()
        .map(|row| ResourceRegistryHistoricalRowFactsV1 {
            reference: &row.reference,
            membership_allowed: member,
            descriptor_allowed: descriptor,
        })
        .collect();
    let f = facts(s);
    o.prepare_resource_registry_historical_release_v1(
        id,
        s,
        &head.head,
        &TrustedResourceRegistryHistoricalFactsV1 {
            current: &f,
            rows: &rows,
            historical_release_attested: Some(true),
        },
        r,
    )
    .unwrap()
}
pub(super) fn change(
    o: &mut SourceCoordinator,
    index: usize,
    private_only: bool,
    remove: bool,
    r: &mut Recorder,
) {
    let (root, _) = o.resources.catalog.current().unwrap();
    let row = &root.rows[index];
    let mut config = row.configuration.copy().unwrap();
    if private_only {
        config.identity_key.push('!');
        config.locator.push('!');
    } else {
        match &mut config.capabilities {
            ResourceCatalogCapabilitiesV1::StrictUtf8OneBom { atomic_read, .. }
            | ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read } => {
                *atomic_read = !*atomic_read
            }
            ResourceCatalogCapabilitiesV1::Write { procedure, .. } => procedure.push('!'),
        }
    }
    let change = ResourceCatalogChangeV1 {
        address: row.address.copy().unwrap(),
        expected: CatalogExpectationV1::Present {
            generation: row.generation,
        },
        action: if remove {
            ResourceCatalogActionV1::Remove
        } else {
            ResourceCatalogActionV1::Install(config)
        },
    };
    let f = super::super::tests::facts();
    let p = o.prepare_resource_catalog_v1(&[change], &f, r).unwrap();
    o.commit_resource_catalog_v1(p, &f, r).unwrap();
}

#[test]
fn registry_observation_intersects_both_policies_and_class_ceilings_before_disclosure() {
    let (mut o, mut r, mut s) = fixture(limits());
    s.read_descriptors = false;
    let a = addresses(&o);
    let mut rows = rows(&a, None);
    rows[1].captured_descriptor = false;
    // Opposite native order is explicitly approved.
    rows[0].visible_order = Some(1);
    rows[1].visible_order = Some(0);
    let f = facts(&s);
    let opened = o
        .open_resource_registry_observation_v1(
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
    assert_eq!(opened.projection.coverage(), "captured-policy-current");
    assert_eq!(
        opened.projection.rows()[0].table,
        ResourceCatalogTableV1::Write
    );
    assert!(
        opened
            .projection
            .rows()
            .iter()
            .all(|r| r.descriptor.is_none())
    );
    let mut s2 = s.clone();
    s2.context.recipient = "other".into();
    let f2 = facts(&s2);
    rows[0].captured_visible = false;
    rows[0].captured_descriptor = false;
    rows[0].visible_order = None;
    rows[1].visible_order = Some(0);
    let narrowed = o
        .open_resource_registry_observation_v1(
            &s2,
            &TrustedResourceRegistrySnapshotFactsV1 {
                current: &f2,
                rows: &rows,
                capture_policy_current: Some(true),
                visibility_complete: Some(true),
                order_attested: Some(true),
            },
            &mut r,
        )
        .unwrap();
    assert_eq!(narrowed.projection.rows().len(), 1);
}

#[test]
fn registry_observation_private_or_hidden_changes_do_not_advance_projection() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, Some(1), &mut r);
    let f = facts(&s);
    let reference = opened.projection.rows()[0].reference.clone();
    change(&mut o, 0, true, false, &mut r);
    change(&mut o, 1, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, Some(1), &mut r);
    let before = o.resource_registry_observation_usage_v1().unwrap();
    assert_eq!(
        o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(ResourceRegistrySampleV1::Unchanged)
    );
    let after = o.resource_registry_observation_usage_v1().unwrap();
    assert_eq!(after.issued_lifetime, before.issued_lifetime);
    assert_eq!(after.bindings, before.bindings);
    assert_eq!(after.queued, 0);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, Some(1), &mut r);
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let cert = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
    let ResourceRegistryDeliveryV1::Projection(e) = o
        .release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(e.before, opened.generation);
    assert_eq!(e.projection.rows()[0].reference, reference);
}

#[test]
fn registry_observation_historical_release_requires_exact_old_head_not_current_membership() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let stale = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
    change(&mut o, 0, false, true, &mut r);
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&stale), &f, &mut r)
            .is_err()
    );
    let fresh = certificate(&mut o, &opened.stream, &s, true, true, &mut r);
    let ResourceRegistryDeliveryV1::Projection(e) = o
        .release_resource_registry_event_v1(&opened.stream, &s, Some(&fresh), &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(e.projection.rows().len(), 2);
    assert!(o.resources.catalog.current().unwrap().0.rows.len() < 2);
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&fresh), &f, &mut r)
            .is_err()
    ); // A spent certificate is not an empty-delivery credential.
}

#[test]
fn registry_observation_certificate_cannot_survive_same_root_resync_into_another_head() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let old = certificate(&mut o, &opened.stream, &s, false, false, &mut r);
    let root = o.resources.catalog.stamp().unwrap();
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    o.resync_resource_registry_observation_v1(&opened.stream, &s, &p, &f, Some(true), &mut r)
        .unwrap();
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&old), &f, &mut r)
            .is_err()
    );
    assert_eq!(o.resources.catalog.stamp().unwrap(), root);
    let a = addresses(&o);
    let mut rows = rows(&a, None);
    rows[0].visible_order = Some(1);
    rows[1].visible_order = Some(0);
    let p = o
        .prepare_resource_registry_snapshot_v1(
            &opened.stream,
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
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    assert_eq!(o.resources.catalog.stamp().unwrap(), root);
    assert!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&old), &f, &mut r)
            .is_err()
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().gapped,
        0
    );
}

#[test]
fn registry_observation_table_only_frames_still_require_membership_but_not_descriptor_permission() {
    for member in [false, true] {
        let (mut o, mut r, mut s) = fixture(limits());
        s.read_descriptors = false;
        s.write_descriptors = false;
        let opened = open(&mut o, &s, None, &mut r);
        let f = facts(&s);
        change(&mut o, 0, false, true, &mut r);
        let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
        o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
        let cert = certificate(&mut o, &opened.stream, &s, member, false, &mut r);
        let result = o
            .release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r)
            .unwrap();
        assert_eq!(
            matches!(result, ResourceRegistryDeliveryV1::Projection(_)),
            member
        );
        assert_eq!(
            o.resource_registry_observation_usage_v1().unwrap().gapped,
            usize::from(!member)
        );
    }
}

#[test]
fn registry_observation_incomplete_duplicate_or_bad_order_snapshots_admit_nothing() {
    let (mut o, mut r, s) = fixture(limits());
    let a = addresses(&o);
    let f = facts(&s);
    for kind in 0..4 {
        let mut rows = rows(&a, None);
        match kind {
            0 => {
                rows.pop();
            }
            1 => rows[1].address = rows[0].address,
            2 => rows[1].visible_order = Some(0),
            _ => rows[0].captured_visible = false,
        }
        assert!(
            o.open_resource_registry_observation_v1(
                &s,
                &TrustedResourceRegistrySnapshotFactsV1 {
                    current: &f,
                    rows: &rows,
                    capture_policy_current: Some(true),
                    visibility_complete: Some(true),
                    order_attested: Some(true)
                },
                &mut r
            )
            .is_err()
        );
        assert_eq!(
            o.resource_registry_observation_usage_v1().unwrap().streams,
            0
        );
        assert_eq!(
            o.resource_registry_observation_usage_v1()
                .unwrap()
                .issued_lifetime,
            0
        );
    }
}

#[test]
fn registry_observation_denied_descriptor_is_gap_not_silent_narrowing() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let head = o
        .inspect_resource_registry_head_v1(&opened.stream, &mut r)
        .unwrap()
        .unwrap();
    let malformed: Vec<_> = head
        .projection
        .rows()
        .iter()
        .map(|row| ResourceRegistryHistoricalRowFactsV1 {
            reference: &row.reference,
            membership_allowed: false,
            descriptor_allowed: true,
        })
        .collect();
    let before = o.resource_registry_observation_usage_v1().unwrap();
    assert!(
        o.prepare_resource_registry_historical_release_v1(
            &opened.stream,
            &s,
            &head.head,
            &TrustedResourceRegistryHistoricalFactsV1 {
                current: &f,
                rows: &malformed,
                historical_release_attested: Some(true)
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
    let cert = certificate(&mut o, &opened.stream, &s, true, false, &mut r);
    assert!(matches!(
        o.release_resource_registry_event_v1(&opened.stream, &s, Some(&cert), &f, &mut r),
        Ok(ResourceRegistryDeliveryV1::Gap)
    ));
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().queued,
        0
    );
    let p = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    assert!(
        o.resync_resource_registry_observation_v1(&opened.stream, &s, &p, &f, None, &mut r)
            .is_err()
    );
    let fresh = o
        .resync_resource_registry_observation_v1(&opened.stream, &s, &p, &f, Some(true), &mut r)
        .unwrap();
    assert_ne!(fresh.generation, opened.generation);
}

#[test]
fn registry_observation_stop_and_independent_packet_retirement_survive_fault_without_refill() {
    let (mut o, mut r, s) = fixture(limits());
    let opened = open(&mut o, &s, None, &mut r);
    let pending = snapshot(&mut o, &opened.stream, &s, None, &mut r);
    let debt = o.retention_usage();
    let issued = o
        .resource_registry_observation_usage_v1()
        .unwrap()
        .issued_lifetime;
    o.faulted = true;
    o.resources.catalog.faulted = true;
    o.stop_resource_registry_observation_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), debt);
    assert!(o.is_faulted());
    o.retire_resource_registry_observation_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    assert!(
        o.retire_resource_registry_observation_pool_v1(&control(), &mut r)
            .is_err()
    );
    o.retire_resource_registry_preparation_v1(&pending, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_observation_pool_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .issued_lifetime,
        issued
    );
    o.faulted = false;
    o.resources.catalog.faulted = false;
    assert!(
        o.configure_resource_registry_observation_v1("a", "e", limits(), &control(), &mut r)
            .is_err()
    );
}
#[test]
fn registry_observation_parent_arithmetic_cannot_wrap() {
    let l = ResourceRegistryObservationLimitsV1 {
        max_streams: usize::MAX,
        max_preparations: 2,
        max_rows: 8,
        max_bindings: 8,
        max_issued: 64,
        max_root_bytes: 1048576,
        max_profile_bytes: 1024,
        max_scope_bytes: 2048,
        max_tag_bytes: 128,
        max_packet_bytes: 8192,
        max_projection_bytes: 8192,
        max_binding_bytes: 4096,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_held_bytes: usize::MAX,
    };
    assert_eq!(l.required_pool_bytes(), Err(ResourceError::Limit));
}
