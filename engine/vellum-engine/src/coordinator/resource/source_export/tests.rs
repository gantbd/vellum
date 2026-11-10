use super::*;
fn control() -> TrustedSourceExportControlV1 {
    TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    }
}
pub(in crate::coordinator::resource) fn facts() -> TrustedSourceExportFactsV1 {
    TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: Some(true),
        overwrite_allowed: Some(true),
    }
}
fn options() -> SourceExportOptionsV1 {
    SourceExportOptionsV1 {
        purpose: SourceExportPurposeV1::CopyExport,
        requirements: SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        },
        pair: None,
    }
}
#[test]
fn source_admission_fence_blocks_copy_and_saveas_without_issuing_and_discharges_only_preparation() {
    for purpose in [
        SourceExportPurposeV1::CopyExport,
        SourceExportPurposeV1::SaveAs,
    ] {
        let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
        let s = o.source_serialization(&d.document, &mut r).unwrap();
        let prepared = o.prepare_source_artifact(&d, s, &mut r).unwrap();
        o.stop_source_save_admission_v1(&mut r);
        let before = o.resource_usage();
        let mut intent = options();
        intent.purpose = purpose;
        assert_eq!(
            o.queue_source_export(
                &prepared,
                port.read_result(b"out", b"v0", b"x")
                    .unwrap()
                    .into_source_export_destination(),
                intent,
                &facts(),
                &mut r
            ),
            Err(ResourceError::Unavailable)
        );
        // The existing export admission contract consumes its original codec
        // preparation on safe refusal; this is not task/artifact issuance.
        let mut expected = before;
        expected.held_artifact_bytes -= expected.prepared_source_bytes;
        expected.prepared_sources = 0;
        expected.prepared_source_bytes = 0;
        assert_eq!(o.resource_usage(), expected);
        assert_eq!(
            o.discard_source_artifact_preparation(&prepared, &mut r),
            Err(ResourceError::Unavailable)
        );
        assert!(!o.is_faulted());
    }
}
#[test]
fn exhausted_publication_counter_refuses_before_issuance_and_releases_only_owned_preparation() {
    let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
    let s = o.source_serialization(&d.document, &mut r).unwrap();
    let p = o.prepare_source_artifact(&d, s, &mut r).unwrap();
    o.generation = u64::MAX;
    assert_eq!(
        o.queue_source_export(
            &p,
            port.read_result(b"out", b"v0", b"x")
                .unwrap()
                .into_source_export_destination(),
            options(),
            &facts(),
            &mut r
        ),
        Err(ResourceError::VersionExhausted)
    );
    let u = o.resource_usage();
    assert_eq!(
        (
            u.issued_lifetime,
            u.issued_artifacts_lifetime,
            u.prepared_sources,
            u.source_export_reservations,
            u.held_artifact_bytes
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(o.generation, u64::MAX);
    assert!(!o.is_faulted());
}
#[test]
fn unexpected_reservation_target_drift_fences_before_dispatch_without_a_not_written_fiction() {
    let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
    let peer = o.create(b"peer", &mut r).unwrap();
    let s = o.source_serialization(&d.document, &mut r).unwrap();
    let p = o.prepare_source_artifact(&d, s, &mut r).unwrap();
    let id = o
        .queue_source_export(
            &p,
            port.read_result(b"out", b"v0", b"x")
                .unwrap()
                .into_source_export_destination(),
            options(),
            &facts(),
            &mut r,
        )
        .unwrap();
    o.resources.source_export_reservations[0].document = peer.document;
    assert_eq!(
        o.validate_export_retirement(&id, o.resource_task(&id).unwrap()),
        Err(ResourceError::Faulted)
    );
    let generation = o.generation;
    assert_eq!(
        o.dispatch_source_export(
            &id,
            port.read_result(b"out", b"v0", b"x")
                .unwrap()
                .into_source_export_destination(),
            None,
            &facts(),
            &mut r
        )
        .unwrap_err(),
        ResourceError::Faulted
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation, generation);
    let out = o.save_outcome(&id, &mut r).unwrap();
    assert_eq!(out.application, Application::Unknown);
    assert_eq!(out.phase, SavePhase::Queued);
    assert!(!out.no_future_effects);
    assert!(o.resources.tasks[id.slot].as_ref().unwrap().nonce.is_none());
    assert_eq!(o.resource_usage().source_export_reservations, 1);
}

#[test]
fn faulted_root_cannot_publish_reservation_release_even_for_stopped_safe_work() {
    let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
    let s = o.source_serialization(&d.document, &mut r).unwrap();
    let p = o.prepare_source_artifact(&d, s, &mut r).unwrap();
    let id = o
        .queue_source_export(
            &p,
            port.read_result(b"out", b"v0", b"x")
                .unwrap()
                .into_source_export_destination(),
            options(),
            &facts(),
            &mut r,
        )
        .unwrap();
    o.stop_save(&id, &mut r).unwrap();
    o.faulted = true;
    let before = o.resource_usage();
    let generation = o.generation;
    assert_eq!(
        o.release_source_export_reservation(&id, &control(), &mut r),
        Err(ResourceError::Faulted)
    );
    assert_eq!(o.resource_usage(), before);
    assert_eq!(o.generation, generation);
    assert_eq!(
        o.save_outcome(&id, &mut r)
            .unwrap()
            .source_export_reservation_released,
        Some(false)
    );
    assert!(o.is_faulted());
}

pub(in crate::coordinator::resource) fn ready_activation() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    ProviderPort,
    SaveId,
) {
    ready_activation_with_quiescence(false)
}
pub(in crate::coordinator::resource) fn ready_activation_with_quiescence(
    quiescent: bool,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    ProviderPort,
    SaveId,
) {
    let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
    let s = o.source_serialization(&d.document, &mut r).unwrap();
    let p = o.prepare_source_artifact(&d, s, &mut r).unwrap();
    let id = o
        .queue_source_export(
            &p,
            port.read_result(b"out", b"o0", b"OLD")
                .unwrap()
                .into_source_export_destination(),
            SourceExportOptionsV1 {
                purpose: SourceExportPurposeV1::SaveAs,
                ..options()
            },
            &facts(),
            &mut r,
        )
        .unwrap();
    let dispatch = o
        .dispatch_source_export(
            &id,
            port.read_result(b"out", b"o0", b"OLD")
                .unwrap()
                .into_source_export_destination(),
            None,
            &facts(),
            &mut r,
        )
        .unwrap();
    o.record_save_evidence(
        port.result(
            &dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::NotRequested,
            Some(b"o1"),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.record_save_evidence(port.no_future_effects(&dispatch).unwrap(), &mut r)
        .unwrap();
    if quiescent {
        o.record_save_evidence(port.quiescence(&dispatch).unwrap(), &mut r)
            .unwrap();
    }
    (o, r, d, port, id)
}
pub(in crate::coordinator::resource) fn activation() -> TrustedSourceSaveAsFactsV1 {
    TrustedSourceSaveAsFactsV1 {
        control: control(),
        probe_attested: Some(true),
        resource_continuity_attested: Some(true),
        create_allowed: Some(true),
        overwrite_allowed: Some(true),
    }
}
#[test]
fn source_activation_counts_exact_archive_and_baseline_root_bytes_before_publication() {
    let (mut probe, mut r, _, port, id) = ready_activation();
    probe
        .activate_source_save_as(
            &id,
            port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            None,
            1,
            &activation(),
            &mut r,
        )
        .unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    for delta in [-1, 0, 1] {
        let (mut o, mut r, d, port, id) = ready_activation();
        o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
            .unwrap();
        let generation = o.generation;
        let before = o.semantic_root_usage(&mut r).unwrap();
        let result = o.activate_source_save_as(
            &id,
            port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            None,
            1,
            &activation(),
            &mut r,
        );
        if delta < 0 {
            assert_eq!(result, Err(ResourceError::Limit));
            assert_eq!(o.generation, generation);
            assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
            assert_eq!(o.resource_usage().source_export_reservations, 1);
        } else {
            assert_eq!(result, Ok(true));
            assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
            let history = &o.documents[d.document.slot]
                .as_ref()
                .unwrap()
                .retained_bindings
                .source_history;
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].identity.key.0.as_ref(), b"key");
            assert_eq!(history[0].baseline.as_ref().unwrap().bytes.0.as_ref(), b"A");
            assert_eq!(
                history[0]
                    .baseline
                    .as_ref()
                    .unwrap()
                    .validator
                    .as_ref()
                    .unwrap()
                    .0
                    .as_ref(),
                b"v0"
            );
            assert_eq!(history[0].generation, 0);
        }
    }
}
#[test]
fn activation_exhaustion_fault_or_reservation_drift_never_installs_a_partial_binding() {
    for mode in ["counter", "fault", "reservation", "original"] {
        let (mut o, mut r, d, port, id) = ready_activation();
        match mode {
            "counter" => o.generation = u64::MAX,
            "fault" => o.faulted = true,
            "reservation" => {
                o.resources.source_export_reservations[0].identity.key =
                    Blob(Arc::from(b"other".as_slice()))
            }
            "original" => o.resources.bindings[0].generation += 1,
            _ => unreachable!(),
        }
        let generation = o.generation;
        let expected = match mode {
            "counter" => ResourceError::VersionExhausted,
            "original" => ResourceError::Stale,
            _ => ResourceError::Faulted,
        };
        assert_eq!(
            o.activate_source_save_as(
                &id,
                port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
                None,
                1,
                &activation(),
                &mut r
            ),
            Err(expected)
        );
        assert_eq!(o.generation, generation);
        assert_eq!(o.resources.bindings[0].identity.key.0.as_ref(), b"key");
        assert!(
            o.documents[d.document.slot]
                .as_ref()
                .unwrap()
                .retained_bindings
                .source_history
                .is_empty()
        );
        assert_eq!(
            o.resource_task(&id).unwrap().knowledge.data.application,
            Application::Written
        );
        assert!(
            !o.resource_task(&id)
                .unwrap()
                .source_export
                .as_ref()
                .unwrap()
                .binding_applied
        );
    }
}
fn history_release() -> TrustedSourceBindingHistoryReleaseV1 {
    TrustedSourceBindingHistoryReleaseV1 {
        control: control(),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
        binding_history_release_allowed: Some(true),
        history_evidence_released: Some(true),
        snapshot_evidence_released: Some(true),
        read_evidence_released: Some(true),
        write_evidence_released: Some(true),
        provider_evidence_released: Some(true),
    }
}
#[test]
fn history_release_counter_fault_and_archive_drift_are_atomic_but_empty_needs_no_counter() {
    for mode in ["counter", "fault", "drift", "empty", "root"] {
        let (mut o, mut r, d, port, id) = ready_activation();
        o.activate_source_save_as(
            &id,
            port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            None,
            1,
            &activation(),
            &mut r,
        )
        .unwrap();
        match mode {
            "counter" => o.generation = u64::MAX,
            "fault" => o.faulted = true,
            "drift" => {
                o.documents[d.document.slot]
                    .as_mut()
                    .unwrap()
                    .retained_bindings
                    .source_history[0]
                    .document = o.create(b"peer", &mut r).unwrap().document
            }
            "empty" => {
                o.documents[d.document.slot]
                    .as_mut()
                    .unwrap()
                    .retained_bindings
                    .source_history
                    .clear();
                o.generation = u64::MAX;
            }
            "root" => o.root_byte_ceiling = Some(0),
            _ => unreachable!(),
        }
        let generation = o.generation;
        let rows = o.resource_usage().source_binding_history_entries;
        let result = o.release_source_binding_history(
            &d.document,
            Some(1),
            None,
            1,
            &history_release(),
            &mut r,
        );
        assert_eq!(
            result,
            match mode {
                "counter" => Err(ResourceError::VersionExhausted),
                "empty" => Ok(false),
                "root" => Err(ResourceError::Limit),
                _ => Err(ResourceError::Faulted),
            }
        );
        assert_eq!(o.generation, generation);
        assert_eq!(o.resource_usage().source_binding_history_entries, rows);
    }
}
#[test]
fn shrinking_association_history_preserves_an_unrelated_real_baseline_promise_counter_floor() {
    let (mut o, mut r, claim, port) = super::super::serialization_tests::pending_root_fixture();
    let d = o
        .load_resource(port.read_result(b"other", b"x0", b"A").unwrap(), &mut r)
        .unwrap();
    let s = o.source_serialization(&d.document, &mut r).unwrap();
    let p = o.prepare_source_artifact(&d, s, &mut r).unwrap();
    let id = o
        .queue_source_export(
            &p,
            port.read_result(b"out", b"o0", b"OLD")
                .unwrap()
                .into_source_export_destination(),
            SourceExportOptionsV1 {
                purpose: SourceExportPurposeV1::SaveAs,
                ..options()
            },
            &facts(),
            &mut r,
        )
        .unwrap();
    let dispatch = o
        .dispatch_source_export(
            &id,
            port.read_result(b"out", b"o0", b"OLD")
                .unwrap()
                .into_source_export_destination(),
            None,
            &facts(),
            &mut r,
        )
        .unwrap();
    for e in [
        port.result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"o1"),
        )
        .unwrap(),
        port.no_future_effects(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, &mut r).unwrap();
    }
    o.activate_source_save_as(
        &id,
        port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
        None,
        1,
        &activation(),
        &mut r,
    )
    .unwrap();
    o.generation = u64::MAX - 1;
    let before = o.semantic_root_capacity_usage(&mut r).unwrap();
    assert_eq!(
        o.release_source_binding_history(&d.document, Some(1), None, 1, &history_release(), &mut r),
        Err(ResourceError::VersionExhausted)
    );
    assert_eq!(o.semantic_root_capacity_usage(&mut r).unwrap(), before);
    assert_eq!(o.generation, u64::MAX - 1);
    assert_eq!(o.resource_usage().source_binding_history_entries, 1);
    assert_eq!(
        o.resource_task(&claim)
            .unwrap()
            .baseline_claim
            .as_ref()
            .unwrap()
            .state,
        BaselineCapacityState::Pending
    );
}

pub(in crate::coordinator::resource) fn rebind_facts() -> TrustedSourceRebindFactsV1 {
    TrustedSourceRebindFactsV1 {
        control: control(),
        destination_allowed: Some(true),
        rebind_allowed: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
        ordinary_bindings_current: Some(true),
        ordinary_identity_current: Some(true),
    }
}
pub(in crate::coordinator::resource) fn rebind_options(
    expected: Option<u64>,
    max: usize,
) -> SourceRebindOptionsV1 {
    SourceRebindOptionsV1 {
        expected_binding: expected,
        max_binding_history: max,
        pair: None,
    }
}
#[test]
fn rebind_accounts_exact_new_row_growth_and_preserves_the_archived_baseline_or_absence() {
    for scratch in [false, true] {
        let ready = || {
            let (mut o, mut r, loaded, port) = super::super::serialization_tests::fixture();
            let d = if scratch {
                o.create(b"scratch", &mut r).unwrap()
            } else {
                loaded
            };
            (o, r, d, port)
        };
        let (mut probe, mut r, d, port) = ready();
        let expected = if scratch { None } else { Some(0) };
        probe
            .rebind_source_document(
                &d.document,
                port.source_binding_target(b"out").unwrap(),
                rebind_options(expected, 1),
                &rebind_facts(),
                &mut r,
            )
            .unwrap();
        let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
        for delta in [-1, 0, 1] {
            let (mut o, mut r, d, port) = ready();
            o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
                .unwrap();
            let before = o.semantic_root_usage(&mut r).unwrap();
            let generation = o.generation;
            let result = o.rebind_source_document(
                &d.document,
                port.source_binding_target(b"out").unwrap(),
                rebind_options(expected, 1),
                &rebind_facts(),
                &mut r,
            );
            if delta < 0 {
                assert_eq!(result, Err(ResourceError::Limit));
                assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
                assert_eq!(o.generation, generation);
            } else {
                assert_eq!(result, Ok(true));
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
                let binding = o
                    .resources
                    .bindings
                    .iter()
                    .find(|b| b.document == d.document)
                    .unwrap();
                assert!(binding.baseline.is_none());
                let history = &o
                    .document(&d.document)
                    .unwrap()
                    .retained_bindings
                    .source_history;
                assert_eq!(history.len(), usize::from(!scratch));
                if !scratch {
                    assert_eq!(history[0].baseline.as_ref().unwrap().bytes.0.as_ref(), b"A");
                }
            }
        }
    }
}
#[test]
fn rebind_exhaustion_and_faults_refuse_atomically_and_same_context_needs_no_counter() {
    for mode in ["publication", "binding", "fault", "same"] {
        let (mut o, mut r, d, port) = super::super::serialization_tests::fixture();
        o.generation = u64::MAX;
        if mode == "binding" || mode == "same" {
            o.resources.bindings[0].generation = u64::MAX;
        }
        if mode == "fault" {
            o.faulted = true;
        }
        let generation = o.generation;
        let key = if mode == "same" {
            b"key".as_slice()
        } else {
            b"out".as_slice()
        };
        let expected = if mode == "binding" || mode == "same" {
            Some(u64::MAX)
        } else {
            Some(0)
        };
        assert_eq!(
            o.rebind_source_document(
                &d.document,
                port.source_binding_target(key).unwrap(),
                rebind_options(expected, 1),
                &rebind_facts(),
                &mut r
            ),
            match mode {
                "same" => Ok(false),
                "fault" => Err(ResourceError::Faulted),
                _ => Err(ResourceError::VersionExhausted),
            }
        );
        assert_eq!(o.generation, generation);
        assert_eq!(o.resources.bindings[0].identity.key.0.as_ref(), b"key");
        assert!(o.resources.bindings[0].baseline.is_some());
        assert_eq!(o.resource_usage().source_binding_history_entries, 0);
        assert_eq!(o.resource_usage().issued_lifetime, 0);
    }
}
#[test]
fn rebind_cannot_transfer_a_real_pending_annotation_baseline_promise_to_a_new_source_context() {
    let (mut o, mut r, claim, port) = super::super::serialization_tests::pending_root_fixture();
    let d = o.resource_task(&claim).unwrap().captured.clone();
    let target = port.source_binding_target(b"out").unwrap();
    let pair = o
        .source_rebind_pair_check(&d.document, &target, &mut r)
        .unwrap()
        .map(|c| {
            port.attest_ordinary_pair(&c, ResourceRelation::Distinct)
                .unwrap()
        });
    let before = o.semantic_root_capacity_usage(&mut r).unwrap();
    let generation = o.generation;
    assert_eq!(
        o.rebind_source_document(
            &d.document,
            target,
            SourceRebindOptionsV1 {
                pair,
                ..rebind_options(Some(0), 1)
            },
            &rebind_facts(),
            &mut r
        ),
        Err(ResourceError::Retained)
    );
    assert_eq!(o.generation, generation);
    assert_eq!(o.semantic_root_capacity_usage(&mut r).unwrap(), before);
    assert_eq!(
        o.resource_task(&claim)
            .unwrap()
            .baseline_claim
            .as_ref()
            .unwrap()
            .state,
        BaselineCapacityState::Pending
    );
}

#[test]
fn queued_source_anchor_drift_fences_retirement_without_erasing_tasks_or_artifacts() {
    for mode in ["origin", "generation", "payload", "serial"] {
        let (mut o, mut r, d, port, id) = ready_activation();
        o.activate_source_save_as(
            &id,
            port.source_save_as_probe(b"out", Some(b"o1")).unwrap(),
            None,
            1,
            &activation(),
            &mut r,
        )
        .unwrap();
        let consumer = o
            .queue_pinned_save(&d, options().requirements, &mut r)
            .unwrap();
        match mode {
            "origin" => {
                let WriteCondition::Present(condition) =
                    &mut o.resources.tasks[consumer.slot].as_mut().unwrap().condition
                else {
                    panic!("present source condition")
                };
                condition.anchor = Some(SaveId {
                    owner: o.namespace.clone(),
                    slot: usize::MAX,
                });
            }
            "generation" => {
                o.resources.tasks[consumer.slot]
                    .as_mut()
                    .unwrap()
                    .binding_generation += 1
            }
            "payload" => {
                let WriteCondition::Present(condition) =
                    &mut o.resources.tasks[consumer.slot].as_mut().unwrap().condition
                else {
                    panic!("present source condition")
                };
                condition.bytes = Blob(Arc::from(b"drift".as_slice()));
            }
            "serial" => o.resources.tasks[id.slot].as_mut().unwrap().dispatch_serial = None,
            _ => unreachable!(),
        }
        let usage = o.resource_usage();
        let generation = o.generation;
        assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.resource_usage(), usage);
        assert_eq!(o.generation, generation);
        assert!(o.resource_task(&id).is_ok());
        assert!(
            o.resource_artifact(&o.resource_task(&id).unwrap().artifact)
                .is_ok()
        );
    }
}
