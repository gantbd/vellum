use super::*;
use crate::coordinator::serialization::tests::{context, facts, limits, reads, request, setup};
fn profile() -> SerializationBasisLimitsV1 {
    SerializationBasisLimitsV1 {
        serialization: limits(),
        max_contexts: 2,
        max_issued_contexts: 4,
        max_context_bytes: 1024,
        max_held_bytes: 16384,
    }
}
fn control() -> SerializationPoolControlV1 {
    SerializationPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn basis<'a>(f: &'a TrustedSerializationFactsV1<'a>) -> TrustedSerializationBasisFactsV1<'a> {
    TrustedSerializationBasisFactsV1 {
        native: f,
        owner_control_allowed: Some(true),
        basis_allowed: Some(true),
        host_basis_allowed: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn ready() -> (SourceCoordinator, Recorder, DocumentVersion) {
    let (mut o, mut r, d) = setup();
    o.configure_retention_budget(16384, &mut r).unwrap();
    o.configure_serialization_bases(profile(), &control(), &mut r)
        .unwrap();
    (o, r, d)
}
#[test]
fn shutdown_inventory_serialization_basis_keeps_actual_document_and_parent_reservation() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, mut r, d) = ready();
    let c = context();
    let f = facts(&c, &d.document);
    let _basis = o
        .capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
        .unwrap();
    let before = (o.serialization_basis_usage(), o.retention_usage());
    let cut = inspect(&o, &mut r);
    let view = inspect_read(&o, &cut, &mut r);
    let row = view
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::SerializationBases)
        .unwrap();
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.reserved_bytes, Some(before.0.reserved_pool_bytes));
    assert!(view.dependencies.iter().any(|e| e.dependent
        == ShutdownInventoryTargetV1::Owner(row.owner.clone())
        && e.required == ShutdownInventoryTargetV1::Document(d.document.clone())));
    assert_eq!((o.serialization_basis_usage(), o.retention_usage()), before);
}

#[test]
fn shutdown_preserves_existing_serialization_basis_service_but_refuses_new_obligations() {
    let (mut o, mut r, d) = setup();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_serialization_bases(profile(), &control(), &mut r)
        .unwrap();
    let c = context();
    let f = facts(&c, &d.document);
    let original = o
        .capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
        .unwrap();
    let dependencies = reads();
    let plan = o
        .prepare_source_serialization(&request(&d.document, &dependencies), &c, &f, &mut r)
        .unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let usage = o.serialization_basis_usage();
    let funding = o.retention_usage();
    assert_eq!(
        o.capture_serialization_basis(&d.document, &c, &basis(&f), &mut r),
        Err(Error::Unavailable)
    );
    assert!(matches!(
        o.prepare_source_serialization(&request(&d.document, &dependencies), &c, &f, &mut r),
        Err(Error::Unavailable)
    ));
    assert_eq!(
        o.inspect_serialization_basis(&original, &c, &basis(&f), &mut r),
        SerializationBasisAvailabilityV1::Current
    );
    assert_eq!(o.serialization_basis_usage(), usage);
    assert_eq!(o.retention_usage(), funding);
    assert_eq!(
        o.commit_source_serialization(plan, &c, &f, &mut r),
        Err(Error::Unavailable)
    );
    assert!(o.stop_serialization_basis(&original, &c, &release(&c), &mut r));
    assert!(o.retire_serialization_basis(&original, &c, &release(&c), &mut r));
    o.retire_serialization_bases(&control(), &mut r).unwrap();
    assert!(o.serialization_basis_usage().closed);
    assert!(!o.is_faulted());
}

#[test]
fn shutdown_rejects_late_serialization_configuration_without_resetting_matching_registration() {
    let (mut o, mut r, _) = setup();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let funding = o.retention_usage();
    assert_eq!(
        o.configure_source_serialization("engine", "local", limits(), &mut r),
        Ok(false)
    );
    assert_eq!(
        o.configure_serialization_bases(profile(), &control(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.retention_usage(), funding);
    assert_eq!(
        o.serialization_basis_usage(),
        SerializationBasisUsageV1::default()
    );
}
#[test]
fn shutdown_refuses_unchanged_serialization_plan_publication_and_first_registration() {
    for configured in [false, true] {
        let (mut o, mut r, d) = setup();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = context();
        let f = facts(&c, &d.document);
        let dependencies = reads();
        let mut req = request(&d.document, &dependencies);
        req.serialization.bom = false;
        let plan = o
            .prepare_source_serialization(&req, &c, &f, &mut r)
            .unwrap();
        if !configured {
            o.serialization_profile = None;
        }
        crate::coordinator::shutdown::tests::install(&mut o, &mut r);
        let root = o.native_root_identity().unwrap();
        let funding = o.retention_usage();
        assert_eq!(
            o.configure_source_serialization("engine", "local", limits(), &mut r),
            if configured {
                Ok(false)
            } else {
                Err(Error::Unavailable)
            }
        );
        if configured {
            assert_eq!(
                o.commit_source_serialization(plan, &c, &f, &mut r),
                Err(Error::Unavailable)
            );
        }
        assert_eq!(o.native_root_identity().unwrap(), root);
        assert_eq!(o.retention_usage(), funding);
        assert!(!o.is_faulted());
    }
}
#[test]
fn document_context_pins_count_closed_bases_until_actual_retirement() {
    let (mut o, mut r, d) = ready();
    let peer = o.create(b"peer", &mut r).unwrap();
    let c = context();
    let f = facts(&c, &d.document);
    let b = o
        .capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
        .unwrap();
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r)
            .unwrap()
            .serialization_bases,
        1
    );
    assert_eq!(
        o.document_context_pins_v1(&peer.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
    assert!(o.stop_serialization_basis(&b, &c, &release(&c), &mut r));
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r)
            .unwrap()
            .serialization_bases,
        1
    );
    assert!(o.retire_serialization_basis(&b, &c, &release(&c), &mut r));
    assert_eq!(
        o.document_context_pins_v1(&d.document, &mut r)
            .unwrap()
            .total(),
        Ok(0)
    );
}
#[test]
fn pool_and_actual_parent_bounds_include_lifetime_slots_and_refund_only_once() {
    let p = profile();
    let charge = p.required_pool_bytes().unwrap();
    for exact in [false, true] {
        for parent in [false, true] {
            let (mut o, mut r, _) = setup();
            o.configure_retention_budget(
                if parent && !exact { charge - 1 } else { charge },
                &mut r,
            )
            .unwrap();
            let p = SerializationBasisLimitsV1 {
                max_held_bytes: if !parent && !exact {
                    charge - 1
                } else {
                    charge
                },
                ..p
            };
            assert_eq!(
                o.configure_serialization_bases(p, &control(), &mut r)
                    .is_ok(),
                exact
            );
            assert_eq!(o.retention_usage().reserved, if exact { charge } else { 0 });
            assert!(!o.is_faulted());
            if exact {
                assert_eq!(
                    o.configure_serialization_bases(p, &control(), &mut r),
                    Err(Error::Unavailable)
                );
                o.retire_serialization_bases(&control(), &mut r).unwrap();
                assert_eq!(o.retention_usage().reserved, 0);
                assert_eq!(
                    o.retire_serialization_bases(&control(), &mut r),
                    Err(Error::Unavailable)
                );
                assert_eq!(
                    o.configure_serialization_bases(p, &control(), &mut r),
                    Err(Error::Unavailable)
                );
            }
        }
    }
    for p in [
        SerializationBasisLimitsV1 {
            max_contexts: usize::MAX,
            ..profile()
        },
        SerializationBasisLimitsV1 {
            max_issued_contexts: usize::MAX,
            ..profile()
        },
    ] {
        assert_eq!(p.required_pool_bytes(), Err(Error::Limit));
    }
}
#[test]
fn metadata_only_capture_uses_no_registry_source_snapshot_or_publication_and_issuance_never_refills()
 {
    let (mut o, mut r, d) = ready();
    let c = context();
    let f = facts(&c, &d.document);
    let g = basis(&f);
    assert!(o.settings.is_none());
    let publication = o.generation;
    let a = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    assert_eq!(
        o.capture_serialization_basis(&d.document, &c, &g, &mut r),
        Err(Error::Limit)
    );
    assert_eq!(o.generation, publication);
    assert_eq!(
        o.inspect_serialization_basis(&a, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Current
    );
    assert!(o.retire_serialization_basis(&a, &c, &release(&c), &mut r));
    assert!(o.retire_serialization_basis(&b, &c, &release(&c), &mut r));
    for _ in 0..2 {
        let b = o
            .capture_serialization_basis(&d.document, &c, &g, &mut r)
            .unwrap();
        assert!(o.retire_serialization_basis(&b, &c, &release(&c), &mut r));
    }
    assert_eq!(
        o.capture_serialization_basis(&d.document, &c, &g, &mut r),
        Err(Error::Limit)
    );
    assert_eq!(o.serialization_basis_usage().issued_lifetime, 4);
    assert_eq!(o.serialization_basis_usage().retained, 0);
}
#[test]
fn bound_recognized_loss_is_absorbing_but_unknown_readiness_does_not_close() {
    for field in 0..11 {
        for value in [None, Some(false)] {
            let (mut o, mut r, d) = ready();
            let c = context();
            let mut f = facts(&c, &d.document);
            let b = o
                .capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
                .unwrap();
            match field {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.interpretation_current = value,
                5 => f.settings_writable = value,
                6 => f.write_allowed = value,
                7 => f.not_stopped = value,
                8 => f.resources_ready = value,
                _ => {}
            }
            let mut g = basis(&f);
            if field == 9 {
                g.basis_allowed = value;
            }
            if field == 10 {
                g.owner_control_allowed = value;
            }
            assert_eq!(
                o.inspect_serialization_basis(&b, &c, &g, &mut r),
                SerializationBasisAvailabilityV1::Unavailable
            );
            let f = facts(&c, &d.document);
            let lost = value == Some(false) && (field <= 6 || field == 9);
            assert_eq!(
                o.inspect_serialization_basis(&b, &c, &basis(&f), &mut r),
                if lost {
                    SerializationBasisAvailabilityV1::Unavailable
                } else {
                    SerializationBasisAvailabilityV1::Current
                }
            );
        }
    }
}
#[test]
fn foreign_context_profile_target_and_unknown_grant_cannot_poison_a_basis() {
    for field in 0..10 {
        let (mut o, mut r, d) = ready();
        let c = context();
        let f = facts(&c, &d.document);
        let b = o
            .capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
            .unwrap();
        let other = o.create(b"other", &mut r).unwrap();
        let mut wrong = c.clone();
        match field {
            0 => wrong.authority = "other".into(),
            1 => wrong.engine = "other".into(),
            2 => wrong.recipient = "other".into(),
            3 => wrong.purpose = "other".into(),
            4 => wrong.destination = "other".into(),
            5 => wrong.profile = "other".into(),
            6 => wrong.continuity = "other".into(),
            _ => {}
        }
        let mut bad = facts(
            &wrong,
            if field == 7 {
                &other.document
            } else {
                &d.document
            },
        );
        if field == 8 {
            bad.profile.max_work += 1;
        }
        if field == 9 {
            bad.write_allowed = None;
        }
        let mut g = basis(&bad);
        g.basis_allowed = Some(false);
        assert_eq!(
            o.inspect_serialization_basis(&b, &wrong, &g, &mut r),
            SerializationBasisAvailabilityV1::Unavailable
        );
        assert_eq!(
            o.inspect_serialization_basis(&b, &c, &basis(&f), &mut r),
            SerializationBasisAvailabilityV1::Current
        );
    }
    let (mut x, mut r, d) = ready();
    let c = context();
    let f = facts(&c, &d.document);
    let b = x
        .capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
        .unwrap();
    let (mut y, mut yr, _) = ready();
    assert_eq!(
        y.inspect_serialization_basis(&b, &c, &basis(&f), &mut yr),
        SerializationBasisAvailabilityV1::Unavailable
    );
    assert!(!y.retire_serialization_basis(&b, &c, &release(&c), &mut yr));
    assert_eq!(
        x.inspect_serialization_basis(&b, &c, &basis(&f), &mut r),
        SerializationBasisAvailabilityV1::Current
    );
}
#[test]
fn separately_authorized_recipient_can_capture_without_matching_opening_recipient() {
    let (mut o, mut r, d) = ready();
    let mut c = context();
    c.recipient = "other".into();
    c.destination = "other-client".into();
    let f = facts(&c, &d.document);
    let g = basis(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    assert_eq!(
        o.inspect_serialization_basis(&b, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Current
    );
}
#[test]
fn source_progress_commutes_but_format_change_and_disposal_close_condition() {
    for dispose in [false, true] {
        let (mut o, mut r, d) = ready();
        let c = context();
        let f = facts(&c, &d.document);
        let g = basis(&f);
        let b = o
            .capture_serialization_basis(&d.document, &c, &g, &mut r)
            .unwrap();
        let p = o
            .prepare(
                &[DocumentEdits {
                    expected: d.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "A".into(),
                        inserted: "B".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(p, &mut r).unwrap();
        assert_eq!(
            o.inspect_serialization_basis(&b, &c, &g, &mut r),
            SerializationBasisAvailabilityV1::Current
        );
        if dispose {
            o.dispose(&d.document, &mut r).unwrap();
        } else {
            let reads = reads();
            let p = o
                .prepare_source_serialization(&request(&d.document, &reads), &c, &f, &mut r)
                .unwrap();
            o.commit_source_serialization(p, &c, &f, &mut r).unwrap();
        }
        assert_eq!(
            o.inspect_serialization_basis(&b, &c, &g, &mut r),
            SerializationBasisAvailabilityV1::Unavailable
        );
        assert!(o.retire_serialization_basis(&b, &c, &release(&c), &mut r));
    }
}
#[test]
fn stop_preserves_charge_and_protected_drain_works_after_fault_without_reopening_pool() {
    let (mut o, mut r, d) = ready();
    let c = context();
    let f = facts(&c, &d.document);
    let g = basis(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    let held = o.serialization_basis_usage();
    assert!(o.stop_serialization_basis(&b, &c, &release(&c), &mut r));
    assert_eq!(o.serialization_basis_usage(), held);
    o.faulted = true;
    o.generation = u64::MAX;
    let mut control = release(&c);
    control.delivery_drained = None;
    assert!(!o.retire_serialization_basis(&b, &c, &control, &mut r));
    assert_eq!(
        o.retire_serialization_bases(&super::tests::control(), &mut r),
        Err(Error::Unavailable)
    );
    assert!(o.retire_serialization_basis(&b, &c, &release(&c), &mut r));
    o.retire_serialization_bases(&super::tests::control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.is_faulted());
    assert_eq!(o.serialization_basis_usage().issued_lifetime, 1);
    assert!(o.serialization_basis_usage().closed);
}
#[test]
fn complete_context_ceiling_is_exact_and_invalid_native_state_faults_without_issuance() {
    let c = context();
    let bytes = labels(&c)
        .into_iter()
        .fold(std::mem::size_of::<Record>(), |n, s| n + s.len());
    for exact in [false, true] {
        let (mut o, mut r, d) = setup();
        o.configure_retention_budget(16384, &mut r).unwrap();
        o.configure_serialization_bases(
            SerializationBasisLimitsV1 {
                max_context_bytes: bytes - usize::from(!exact),
                ..profile()
            },
            &control(),
            &mut r,
        )
        .unwrap();
        let f = facts(&c, &d.document);
        assert_eq!(
            o.capture_serialization_basis(&d.document, &c, &basis(&f), &mut r)
                .is_ok(),
            exact
        );
        assert_eq!(
            o.serialization_basis_usage().issued_lifetime,
            usize::from(exact)
        );
    }
    let (mut o, mut r, d) = ready();
    o.documents[d.document.slot]
        .as_mut()
        .unwrap()
        .serialization
        .generation = 0;
    let f = facts(&c, &d.document);
    assert_eq!(
        o.capture_serialization_basis(&d.document, &c, &basis(&f), &mut r),
        Err(Error::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.serialization_basis_usage().issued_lifetime, 0);
}
#[test]
fn shared_parent_reservations_compete_and_corrupt_reservation_faults_without_publishing() {
    let charge = profile().required_pool_bytes().unwrap();
    let (mut o, mut r, d) = setup();
    o.configure_retention_budget(charge, &mut r).unwrap();
    let mut other = o.retention.reserve(1).unwrap();
    assert_eq!(
        o.configure_serialization_bases(profile(), &control(), &mut r),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage().reserved, 1);
    o.retention.release(&mut other).unwrap();
    o.configure_serialization_bases(profile(), &control(), &mut r)
        .unwrap();
    let c = context();
    let f = facts(&c, &d.document);
    let g = basis(&f);
    let b = o
        .capture_serialization_basis(&d.document, &c, &g, &mut r)
        .unwrap();
    // A valid reservation from another owner must not authenticate this pool.
    let (mut other, mut other_r, _) = setup();
    other
        .configure_retention_budget(charge, &mut other_r)
        .unwrap();
    let reservation = other.retention.reserve(charge).unwrap();
    let BasisPool::Active(p) = &mut o.serialization_bases else {
        unreachable!()
    };
    p.reservation = reservation;
    let generation = o.generation;
    assert_eq!(
        o.inspect_serialization_basis(&b, &c, &g, &mut r),
        SerializationBasisAvailabilityV1::Unavailable
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation, generation);
    assert_eq!(o.retention_usage().reserved, charge);
    assert_eq!(o.serialization_basis_usage().retained, 1);
}
