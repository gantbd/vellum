//! Public native shutdown: actual save/disposition through final control retirement.
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Recorder};
use vellum_engine::resource::*;
use vellum_engine::source::Replacement;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)] // This scenario uses only the bound source/annotation pair.
mod fixture;

fn coverage() -> TrustedDisposalCoverageV1 {
    TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: None,
        source_discard_allowed: Some(true),
        annotation_retention_ready: None,
        annotation_discard_allowed: Some(true),
        retention_complete: Some(true),
        effects_retained: Some(true),
        history_retained: Some(true),
        snapshots_retained: Some(true),
        providers_retained: Some(true),
        views_detachable: Some(true),
        memberships_detachable: Some(true),
        observer_scopes_fenced: Some(true),
        binding_coverage_complete: Some(true),
    }
}

#[test]
fn host_can_settle_both_original_saves_dispose_and_report_still_retained_effects() {
    let mut f = fixture::Fixture::new();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    let requirements = SaveRequirements {
        protection: Protection::BestEffort,
        acknowledgment: Acknowledgment::Ordinary,
    };
    let pair = f.pair().unwrap();
    let source_save = f
        .owner
        .queue_pinned_save_with_pair(&f.doc, requirements, pair, &mut f.recorder)
        .unwrap();
    let encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    let encoded = encoder.seal(snapshot, b"[]".to_vec()).unwrap();
    let pair = f.pair();
    let annotation_save = f
        .owner
        .queue_annotation_save(
            encoded,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            requirements,
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let c = SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "native".into(),
        recipient: "host".into(),
        purpose: "shutdown".into(),
        destination: "host".into(),
        profile: "native".into(),
        continuity: "local".into(),
    };
    let control = TrustedShutdownFenceFactsV1 {
        context: &c,
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        owner_control_allowed: Some(true),
        admission_fence_allowed: Some(true),
        resources_ready: Some(true),
    };
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let limits = ShutdownInventoryLimitsV1 {
        max_documents: 8,
        max_writes: 8,
        max_artifacts: 8,
        max_bindings: 8,
        max_reads: 8,
        max_owners: 64,
        max_attachments: 8,
        max_dependencies: 64,
        max_work: 256,
        max_capture_bytes: 65536,
    };
    let basis = f
        .owner
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 65536,
                max_capture_bytes: 131072,
            },
            &control,
            &mut f.recorder,
        )
        .unwrap();
    let inventory = f
        .owner
        .capture_shutdown_inventory_v1(&basis, limits, &auth, &mut f.recorder)
        .unwrap();
    let rows = f
        .owner
        .read_shutdown_inventory_v1(&inventory, &auth, &mut f.recorder)
        .unwrap();
    let choices: Vec<_> = rows
        .documents
        .iter()
        .map(|row| ShutdownDocumentDecisionV1 {
            expected: row,
            source: Some(ShutdownDocumentChoiceV1::Save),
            annotations: Some(ShutdownDocumentChoiceV1::Save),
        })
        .collect();
    let ShutdownDecisionCaptureV1::Captured(decisions) = f
        .owner
        .capture_shutdown_decisions_v1(&inventory, &choices, 65536, &auth, &mut f.recorder)
        .unwrap()
    else {
        panic!("complete choices")
    };
    let accepted_bindings = vec![ShutdownSaveBindingsV1 {
        document: f.doc.document.clone(),
        source: Some(source_save.clone()),
        annotations: Some(annotation_save.clone()),
    }];
    let ShutdownAcceptanceV1::Accepted(fence) = f
        .owner
        .accept_shutdown_disposition_v1(
            &inventory,
            &decisions,
            &accepted_bindings,
            262144,
            &auth,
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!("original saves admitted")
    };
    let original = f
        .owner
        .capture_shutdown_progress_v1(&fence, limits, &auth, &mut f.recorder)
        .unwrap();
    let accepted_charge = f
        .owner
        .read_shutdown_progress_v1(&original, &auth, &mut f.recorder)
        .unwrap()
        .owners
        .iter()
        .find(|owner| owner.owner.role() == ShutdownOwnerRoleV1::ShutdownControl)
        .unwrap()
        .additional_reserved_bytes
        .unwrap();
    let document = f.doc.document.clone();
    let request = ShutdownDocumentDispositionRequestV1 {
        document: &document,
        source_save: Some(&source_save),
        annotation_save: Some(&annotation_save),
    };
    let mut allowed = coverage();
    allowed.source_retention_ready = Some(true);
    allowed.annotation_retention_ready = Some(true);
    allowed.source_discard_allowed = None;
    allowed.annotation_discard_allowed = None;
    assert!(
        f.owner
            .dispose_shutdown_document_v1(
                &inventory,
                &decisions,
                &request,
                &auth,
                &allowed,
                &mut f.recorder
            )
            .is_err()
    );
    let mut r = Recorder::try_with_capacity(256)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    let mut cancellation = f
        .owner
        .begin_shutdown_cancellation_v1(&fence, &control, &mut r)
        .unwrap();
    for _ in 0..8 {
        let progress = f
            .owner
            .advance_shutdown_cancellation_v1(&mut cancellation, 1, &control, &mut r)
            .unwrap();
        if progress.status == ShutdownCancellationStatusV1::FinishedScan {
            break;
        }
    }
    assert_eq!(
        cancellation.progress().status,
        ShutdownCancellationStatusV1::FinishedScan
    );
    assert_eq!(cancellation.progress().stopped_queued_saves, 0);
    assert_eq!(
        f.owner.save_outcome(&source_save, &mut r).unwrap().phase,
        SavePhase::Queued
    );
    assert_eq!(
        f.owner
            .save_outcome(&annotation_save, &mut r)
            .unwrap()
            .phase,
        SavePhase::Queued
    );
    let pair = f.pair().unwrap();
    let source_dispatch = f
        .owner
        .dispatch_save_with_pair(
            &source_save,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    let annotation_dispatch = f
        .owner
        .dispatch_save_with_pair(
            &annotation_save,
            f.port.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let wait_facts = TrustedShutdownWaitFactsV1 {
        inventory: &auth,
        clock_valid: Some(true),
    };
    let wait_request = ShutdownWaitRequestV1 {
        clock: ShutdownWaitClockV1 { epoch: 1, tick: 10 },
        budget_ticks: 1,
        report_limits: limits,
    };
    f.owner
        .start_shutdown_wait_v1(&fence, wait_request, &wait_facts, &mut r)
        .unwrap();
    let wait_held = f.owner.retention_usage();
    let resource_before_cutoff = f.owner.resource_usage();
    assert_eq!(
        f.owner.advance_shutdown_wait_v1(
            &fence,
            ShutdownWaitEventV1::Tick(ShutdownWaitClockV1 { epoch: 1, tick: 11 }),
            &wait_facts,
            &mut r
        ),
        Ok(ShutdownWaitStatusV1::Cutoff {
            reason: ShutdownWaitCutoffReasonV1::BudgetEnded,
            remaining_ticks: Some(0),
            report: ShutdownCutoffReportStateV1::Available,
        })
    );
    assert_eq!(f.owner.retention_usage(), wait_held);
    assert_eq!(f.owner.resource_usage(), resource_before_cutoff);
    let cutoff = f
        .owner
        .read_shutdown_wait_cutoff_v1(&fence, &auth, &mut r)
        .unwrap();
    assert_eq!(cutoff.writes.len(), 2);
    let control_owner = cutoff
        .owners
        .iter()
        .find(|owner| owner.owner.role() == ShutdownOwnerRoleV1::ShutdownControl)
        .unwrap();
    assert_eq!(control_owner.state, ShutdownOwnerStateV1::Active);
    assert_eq!(control_owner.retained_records, Some(3));
    assert_eq!(
        control_owner.additional_reserved_bytes,
        Some(accepted_charge + wait_request.required_retained_bytes().unwrap())
    );
    assert!(!control_owner.retirement_supported); // Wait retirement is not fence retirement.
    assert!(
        cutoff
            .writes
            .iter()
            .all(|w| w.application == Application::Unknown && !w.quiescent && !w.no_future_effects)
    );
    for (dispatch, length) in [(&source_dispatch, 4), (&annotation_dispatch, 2)] {
        f.owner
            .record_save_evidence(
                f.port
                    .result(
                        dispatch,
                        Application::Written,
                        length,
                        Synchronization::NotRequested,
                        Some(b"v1"),
                    )
                    .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
    }
    let pair = f.pair().unwrap();
    f.owner
        .publish_save_baseline_with_pair(&source_save, pair, &mut f.recorder)
        .unwrap();
    let pair = f.pair();
    f.owner
        .install_annotation_save_baseline(&annotation_save, pair, &mut f.recorder)
        .unwrap();
    let generation = f.owner.generation();
    for invalid in [
        ShutdownDocumentDispositionRequestV1 {
            document: &document,
            source_save: Some(&annotation_save),
            annotation_save: Some(&source_save),
        },
        ShutdownDocumentDispositionRequestV1 {
            document: &document,
            source_save: Some(&source_save),
            annotation_save: None,
        },
    ] {
        assert!(
            f.owner
                .dispose_shutdown_document_v1(
                    &inventory, &decisions, &invalid, &auth, &allowed, &mut r
                )
                .is_err()
        );
        assert_eq!(f.owner.generation(), generation);
    }
    assert_eq!(
        f.owner
            .dispose_accepted_shutdown_document_v1(&fence, &document, &auth, &allowed, &mut r)
            .unwrap()
            .phase,
        LifecyclePhase::DisposedRetained
    );
    f.owner.stop_annotation_read_admission_v1(&mut r);
    f.owner
        .retire_annotation_reader_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    let current = f
        .owner
        .capture_shutdown_progress_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    let now = f
        .owner
        .read_shutdown_progress_v1(&current, &auth, &mut r)
        .unwrap();
    assert_eq!(
        now.documents[0].lifecycle.phase,
        LifecyclePhase::DisposedRetained
    );
    assert_eq!(now.writes.len(), 2);
    assert!(
        now.writes
            .iter()
            .all(|w| w.application == Application::Written && !w.quiescent && !w.no_future_effects)
    );
    assert!(now.coverage.all_owner_obligations);
    assert!(now.owners.iter().any(|owner| owner.owner.role()
        == ShutdownOwnerRoleV1::AnnotationReader
        && owner.state == ShutdownOwnerStateV1::Closed));
    assert!(now.owners.iter().any(
        |owner| owner.owner.role() == ShutdownOwnerRoleV1::SharedWriter
            && owner.state == ShutdownOwnerStateV1::Active
    ));
    let old = f
        .owner
        .read_shutdown_progress_v1(&original, &auth, &mut r)
        .unwrap();
    assert!(
        old.writes
            .iter()
            .all(|w| w.application == Application::Unknown)
    );
    assert_eq!(old.documents[0].lifecycle.phase, LifecyclePhase::Live);
    assert_eq!(
        f.owner.retire_save(&source_save, &mut r),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner
            .retire_shutdown_writer_v1(&fence, &control, Some(true), Some(true), &mut r),
        Err(ResourceError::Retained)
    );
    for dispatch in [&source_dispatch, &annotation_dispatch] {
        for evidence in [
            f.port.no_future_effects(dispatch).unwrap(),
            f.port.quiescence(dispatch).unwrap(),
        ] {
            f.owner.record_save_evidence(evidence, &mut r).unwrap();
        }
    }
    drop(source_dispatch);
    drop(annotation_dispatch);
    let state = f.owner.lifecycle_state(&document, &mut r).unwrap();
    f.owner
        .release_disposed_source_binding(&document, state, &mut r)
        .unwrap();
    let state = f.owner.lifecycle_state(&document, &mut r).unwrap();
    f.owner
        .release_disposed_annotation_binding(&document, state, &mut r)
        .unwrap();
    let state = f.owner.lifecycle_state(&document, &mut r).unwrap();
    f.owner
        .release_disposed_provenance(&document, state, &mut r)
        .unwrap();
    f.owner.retire_save(&source_save, &mut r).unwrap();
    f.owner.retire_save(&annotation_save, &mut r).unwrap();
    f.owner.stop_source_save_admission_v1(&mut r);
    f.owner.stop_annotation_save_admission(&mut r);
    let held = f.owner.retention_usage();
    f.owner
        .retire_shutdown_writer_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(f.owner.retention_usage(), held);
    let closed = f
        .owner
        .capture_shutdown_progress_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    let closed = f
        .owner
        .read_shutdown_progress_v1(&closed, &auth, &mut r)
        .unwrap();
    assert!(closed.writes.is_empty() && closed.artifacts.is_empty() && closed.bindings.is_empty());
    let writer = closed
        .owners
        .iter()
        .find(|owner| owner.owner.role() == ShutdownOwnerRoleV1::SharedWriter)
        .unwrap();
    assert_eq!(writer.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(writer.issued_slots, None);
    assert!(closed.coverage.all_owner_obligations);
    let metadata = closed
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::ResourceMetadata)
        .unwrap();
    assert_eq!(metadata.state, ShutdownOwnerStateV1::Active);
    assert!(metadata.retained_records.unwrap() >= 2); // Original provider and encoder survive writer retirement.
    let root = closed
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::SemanticRoot)
        .unwrap();
    assert_eq!(root.state, ShutdownOwnerStateV1::Active);
    assert_eq!(
        root.held_bytes,
        Some(f.owner.semantic_root_usage(&mut r).unwrap().total_bytes)
    );
    // The original cutoff survives late persistence, disposal and producer
    // retirement unchanged, and retains no producer/payload pin.
    let cutoff = f
        .owner
        .read_shutdown_wait_cutoff_v1(&fence, &auth, &mut r)
        .unwrap();
    assert_eq!(cutoff.writes.len(), 2);
    assert!(
        cutoff
            .writes
            .iter()
            .all(|w| w.application == Application::Unknown)
    );
    assert!(cutoff.coverage.all_owner_obligations);
    let before_release = f.owner.retention_usage();
    f.owner
        .retire_shutdown_wait_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(
        f.owner.retention_usage().reserved,
        before_release.reserved - wait_request.required_retained_bytes().unwrap()
    );
    let retained_control = f
        .owner
        .capture_shutdown_progress_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    let retained_control = f
        .owner
        .read_shutdown_progress_v1(&retained_control, &auth, &mut r)
        .unwrap();
    let control_owner = retained_control
        .owners
        .iter()
        .find(|owner| owner.owner.role() == ShutdownOwnerRoleV1::ShutdownControl)
        .unwrap();
    assert_eq!(control_owner.state, ShutdownOwnerStateV1::Active);
    assert_eq!(control_owner.retained_records, Some(2));
    assert_eq!(
        control_owner.additional_reserved_bytes,
        Some(accepted_charge)
    );
    let before_plan_release = f.owner.retention_usage();
    f.owner
        .retire_shutdown_disposition_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(
        f.owner.retention_usage().reserved,
        before_plan_release.reserved - accepted_charge
    );
    // Keeping the host's original handles cannot revive retired consent, even
    // by retrying the raw fence or bypassing the engine-owned plan driver.
    assert_eq!(
        f.owner
            .install_shutdown_fence_v1(&basis, &control, &mut r)
            .unwrap(),
        fence
    );
    assert!(matches!(
        f.owner.accept_shutdown_disposition_v1(
            &inventory,
            &decisions,
            &accepted_bindings,
            262144,
            &auth,
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    ));
    assert!(matches!(
        f.owner.dispose_shutdown_document_v1(
            &inventory,
            &decisions,
            &request,
            &auth,
            &coverage(),
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    ));
    let report = f
        .owner
        .capture_shutdown_progress_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    let report = f
        .owner
        .read_shutdown_progress_v1(&report, &auth, &mut r)
        .unwrap();
    let control_owner = report
        .owners
        .iter()
        .find(|owner| owner.owner.role() == ShutdownOwnerRoleV1::ShutdownControl)
        .unwrap();
    assert_eq!(control_owner.retained_records, Some(1));
    assert_eq!(control_owner.additional_reserved_bytes, Some(0));
    assert!(report.coverage.all_owner_obligations);
    assert_eq!(
        f.owner
            .finish_shutdown_attempt_v1(&fence, limits, &auth, &mut r),
        Ok(ShutdownAttemptStatusV1::UnresolvedRetained)
    );
    f.owner
        .retire_shutdown_attempt_report_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(
        f.owner
            .retire_shutdown_control_v1(&fence, limits, &auth, &mut r),
        Err(CoordinatorError::Unavailable),
        "disposed provenance is not final native payload reclamation"
    );
    let state = f.owner.lifecycle_state(&document, &mut r).unwrap();
    f.owner
        .reclaim_disposed_document_v1(
            &document,
            state,
            &TrustedDocumentReclamationV1 {
                authorized: Some(true),
                authored_disposition_ready: Some(true),
                session_references_released: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            },
            &mut r,
        )
        .unwrap();
    f.owner
        .retire_shutdown_control_v1(&fence, limits, &auth, &mut r)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
    assert_eq!(
        f.owner.create(b"not reopened", &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
    for boundary in [
        Boundary::ShutdownCancellationStart,
        Boundary::ShutdownCancellationStep,
        Boundary::ShutdownSaveReadiness,
        Boundary::ShutdownDocumentDisposition,
        Boundary::ShutdownAcceptedDocumentDisposition,
        Boundary::ShutdownProgressCapture,
        Boundary::ShutdownProgressRelease,
        Boundary::AnnotationReaderRetirement,
        Boundary::ShutdownWriterRetirement,
        Boundary::ShutdownWaitStart,
        Boundary::ShutdownWaitAdvance,
        Boundary::ShutdownWaitCutoffRelease,
        Boundary::ShutdownWaitRetirement,
        Boundary::ShutdownDispositionRetirement,
    ] {
        assert!(events.iter().any(|e| e.boundary == boundary
            && e.disposition == Disposition::Completed
            && e.service_time.is_some()));
    }
    assert!(!f.owner.is_faulted());
}

#[test]
fn host_can_acknowledge_fence_and_dispose_without_ui_or_substrate_and_observe_native_timing() {
    for mode in 0..4 {
        let mut o = SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 2,
                max_participants: 2,
                max_staged_bytes: 128,
                source: vellum_engine::source::Limits {
                    max_source_bytes: 64,
                    max_edits: 4,
                    max_edit_payload_bytes: 64,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 4,
                max_retained: 4,
                max_intent_bytes: 4096,
            },
        );
        let mut setup = Recorder::disabled();
        o.configure_retention_budget(1048576, &mut setup).unwrap();
        let a = o.create(b"PRIVATE_PROSE", &mut setup).unwrap();
        let b = o.create(b"PRIVATE_CODE", &mut setup).unwrap();
        let pending = o.issue_invocation(&mut setup).unwrap();
        o.submit(
            &pending,
            &SourceIntent {
                writes: vec![DocumentEdits {
                    expected: a.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 13,
                        expected: "PRIVATE_PROSE".into(),
                        inserted: "not published".into(),
                    }],
                }],
                reads: vec![],
            },
            &mut setup,
        )
        .unwrap();
        let c = SettingsExecutionContextV1 {
            authority: "host".into(),
            engine: "native".into(),
            recipient: "host".into(),
            purpose: "shutdown".into(),
            destination: "host".into(),
            profile: "native".into(),
            continuity: "local".into(),
        };
        let control = TrustedShutdownFenceFactsV1 {
            context: &c,
            authorized: Some(true),
            scope_current: Some(true),
            continuous: Some(true),
            owner_control_allowed: Some(true),
            admission_fence_allowed: Some(true),
            resources_ready: Some(true),
        };
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let basis = o
            .capture_shutdown_fence_basis_v1(
                ShutdownFenceLimitsV1 {
                    max_context_bytes: 1024,
                    max_root_bytes: 65536,
                    max_capture_bytes: 131072,
                },
                &control,
                &mut setup,
            )
            .unwrap();
        let inventory = o
            .capture_shutdown_inventory_v1(
                &basis,
                ShutdownInventoryLimitsV1 {
                    max_documents: 2,
                    max_writes: 0,
                    max_artifacts: 0,
                    max_bindings: 0,
                    max_reads: 0,
                    max_owners: 64,
                    max_attachments: 0,
                    max_dependencies: 8,
                    max_work: 64,
                    max_capture_bytes: 65536,
                },
                &f,
                &mut setup,
            )
            .unwrap();
        let missing = o
            .capture_shutdown_decisions_v1(&inventory, &[], 65536, &f, &mut setup)
            .unwrap();
        let ShutdownDecisionCaptureV1::NeedsDecision(needs) = missing else {
            panic!("explicit choices required")
        };
        assert_eq!(needs.len(), 2);
        assert_eq!(o.lifecycle_usage().live, 2);
        let rows = o
            .read_shutdown_inventory_v1(&inventory, &f, &mut setup)
            .unwrap();
        assert!(rows.coverage.all_owner_obligations);
        assert_eq!(rows.invocations.len(), 1);
        assert_eq!(rows.invocations[0].invocation, pending);
        assert_eq!(rows.invocations[0].state, Ok(InvocationState::Pending));
        let input: Vec<_> = rows
            .documents
            .iter()
            .map(|d| ShutdownDocumentDecisionV1 {
                expected: d,
                source: Some(ShutdownDocumentChoiceV1::Discard),
                annotations: Some(ShutdownDocumentChoiceV1::Discard),
            })
            .collect();
        let ShutdownDecisionCaptureV1::Captured(decisions) = o
            .capture_shutdown_decisions_v1(&inventory, &input, 65536, &f, &mut setup)
            .unwrap()
        else {
            panic!("complete choices")
        };
        let fence = o
            .install_shutdown_fence_v1(&basis, &control, &mut setup)
            .unwrap();
        let held = o.retention_usage();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let generation = o.generation();
        o.stop_shutdown_invocation_v1(&fence, &pending, &control, &mut r)
            .unwrap();
        o.stop_shutdown_invocation_v1(&fence, &pending, &control, &mut r)
            .unwrap();
        assert_eq!(o.generation(), generation);
        assert!(matches!(
            o.invocation_outcome(&pending, &mut setup),
            Ok(InvocationOutcome::StoppedBeforePublication)
        ));
        assert_eq!(o.read(&a.document, &mut setup).unwrap().version, a);
        // The original report remains pending; current state and protected
        // retirement come from the actual original ledger, not that old cut.
        assert_eq!(
            o.read_shutdown_inventory_v1(&inventory, &f, &mut setup)
                .unwrap()
                .invocations[0]
                .state,
            Ok(InvocationState::Pending)
        );
        o.forget_invocation_outcome(&pending, &mut setup).unwrap();
        o.retire_invocation(&pending, &mut setup).unwrap();
        for d in [&a, &b] {
            let state = o
                .discard_shutdown_document_v1(
                    &inventory,
                    &decisions,
                    &d.document,
                    &f,
                    &coverage(),
                    &mut r,
                )
                .unwrap();
            assert_eq!(state.phase, LifecyclePhase::DisposedRetained);
            assert_eq!(o.lifecycle_state(&d.document, &mut setup).unwrap(), state);
        }
        assert_eq!(o.lifecycle_usage().live, 0);
        assert_eq!(o.lifecycle_usage().disposed_retained, 2);
        assert_eq!(o.retention_usage(), held);
        assert_eq!(
            o.install_shutdown_fence_v1(&basis, &control, &mut setup)
                .unwrap(),
            fence
        );
        assert_eq!(
            o.create(b"new", &mut setup),
            Err(CoordinatorError::Unavailable)
        );
        let mut completed = 0;
        let mut stopped = 0;
        while let Some(e) = r.pop_front() {
            assert!(!format!("{e:?}").contains("PRIVATE"));
            if e.boundary == Boundary::ShutdownDocumentDisposition {
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode >= 2);
                completed += 1;
            }
            if e.boundary == Boundary::ShutdownInvocationStop {
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode >= 2);
                stopped += 1;
            }
        }
        assert_eq!(completed, if mode == 1 || mode == 2 { 2 } else { 0 });
        assert_eq!(stopped, if mode == 1 || mode == 2 { 2 } else { 0 });
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
        assert!(!o.is_faulted());
    }
}
