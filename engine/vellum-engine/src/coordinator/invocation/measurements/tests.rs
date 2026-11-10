use super::*;
#[test]
fn shutdown_inventory_reports_actual_invocation_measurement_owner_until_native_retirement() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::InvocationMeasurements, &[]);
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(row.retained_records, Some(0)); // No issued managed entries; still an owner.
    assert!(row.reserved_bytes.unwrap() > 0);
    assert!(o.stop_invocation_measurements_v1(Some(true), &mut r));
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::InvocationMeasurements, &[]);
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert!(row.admission_stopped);
    assert!(o.retire_invocation_measurements_v1(&control(), &mut r));
    let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::InvocationMeasurements, &[]);
    assert_eq!(row.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(row.reserved_bytes, None);
}
mod pending_age;
fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 65536,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(200_000, &mut r).unwrap();
    (o, r)
}
fn profile(maximum: u64) -> InvocationMeasurementLimitsV1 {
    InvocationMeasurementLimitsV1 {
        values: m::ValueLimitsV1 {
            max_integer: maximum,
            max_seconds: 0.0,
        },
        max_pool_bytes: InvocationMeasurementLimitsV1::required_pool_bytes(16).unwrap(),
    }
}
fn enable(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_invocation_measurements_v1(profile(100), Some(true), r)
        .unwrap();
}
fn control() -> InvocationMeasurementControlV1 {
    InvocationMeasurementControlV1 {
        owner_control_allowed: Some(true),
        delivery_drained: Some(true),
        retention_released: Some(true),
    }
}
fn intent(
    o: &SourceCoordinator,
    d: &DocumentId,
    from: &str,
    to: &str,
    r: &mut Recorder,
) -> SourceIntent {
    SourceIntent {
        writes: vec![DocumentEdits {
            expected: o.read(d, r).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: from.into(),
                inserted: to.into(),
            }],
        }],
        reads: vec![],
    }
}
fn total(p: &m::MeasurementPointV1) -> u64 {
    match p.state() {
        m::PointStateV1::Observed {
            value: m::ValueV1::Counter(v),
            ..
        } => v.total,
        _ => panic!("observed counter"),
    }
}
fn totals(o: &mut SourceCoordinator, r: &mut Recorder) -> [u64; 7] {
    let p = o.invocation_measurements_v1(r).unwrap().unwrap();
    let mut v = [0; 7];
    for (i, p) in p.dispositions.iter().enumerate() {
        v[i] = total(p);
    }
    v[6] = total(&p.admissions);
    v
}
#[test]
fn service_timing_does_not_admit_foreign_or_unsubmitted_tickets_or_change_the_cohort() {
    let (mut o, _) = owner();
    let (mut peer, _) = owner();
    let mut r = timed_service_recorder();
    enable(&mut o, &mut r);
    let own = o.issue_invocation(&mut r).unwrap();
    let foreign = peer.issue_invocation(&mut r).unwrap();
    let held = o.retention_usage().reserved;
    let generation = o.generation();
    assert_eq!(o.run(&foreign, &mut r), Err(InvocationError::Unavailable));
    assert_eq!(o.run(&own, &mut r), Err(InvocationError::NotSubmitted));
    assert_eq!(
        o.invocation_state(&own, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(
        peer.invocation_state(&foreign, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(totals(&mut o, &mut r), [0; 7]);
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
    assert_service_markers(
        &mut r,
        &[(Boundary::InvocationExecution, Disposition::Refused); 2],
    );
}

#[test]
fn issuance_first_admission_pending_replay_and_terminal_replay_are_distinct() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [0; 7]);
    o.submit(&id, &q, &mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 0, 1]);
    o.run(&id, &mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [1, 0, 0, 0, 0, 0, 1]);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [1, 0, 0, 0, 0, 0, 1]);
}
#[test]
fn noop_known_preparation_refusal_and_stop_are_local_terminal_dispositions() {
    let (mut o, _) = owner();
    let mut r = timed_service_recorder();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let stale = intent(&o, &d, "A", "x", &mut r);
    for q in [
        intent(&o, &d, "A", "A", &mut r),
        intent(&o, &d, "A", "a", &mut r),
        stale,
    ] {
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit(&id, &q, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
    }
    let id = o.issue_invocation(&mut r).unwrap();
    let q = intent(&o, &d, "a", "b", &mut r);
    o.submit(&id, &q, &mut r).unwrap();
    o.stop_invocation(&id, &mut r).unwrap();
    o.stop_invocation(&id, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [1, 1, 1, 1, 0, 0, 4]);
    // A successful service call can settle a rejected or stopped job. These
    // attempt timings must not relabel the authoritative disposition counters.
    assert_service_markers(
        &mut r,
        &[
            (Boundary::InvocationExecution, Disposition::Completed),
            (Boundary::InvocationExecution, Disposition::Completed),
            (Boundary::InvocationExecution, Disposition::Completed),
            (Boundary::InvocationReplay, Disposition::Completed),
        ],
    );
}
#[test]
fn activation_excludes_old_admitted_work_but_includes_previously_issued_unbound_tickets() {
    let (mut o, mut r) = owner();
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let old = o.issue_invocation(&mut r).unwrap();
    o.submit(&old, &q, &mut r).unwrap();
    let issued = o.issue_invocation(&mut r).unwrap();
    enable(&mut o, &mut r);
    o.run(&old, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [0; 7]);
    let q = intent(&o, &d, "a", "b", &mut r);
    o.submit(&issued, &q, &mut r).unwrap();
    o.run(&issued, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [1, 0, 0, 0, 0, 0, 1]);
}
#[test]
fn mismatched_or_over_budget_offers_are_not_new_logical_invocation_dispositions() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    let other = intent(&o, &d, "A", "b", &mut r);
    assert_eq!(
        o.submit(&id, &other, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 0, 1]);
    let new = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = 0;
    assert_eq!(o.submit(&new, &q, &mut r), Err(InvocationError::Limit));
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(
        o.invocation_state(&new, &mut r),
        Ok(InvocationState::Issued)
    );
}
#[test]
fn retained_faulted_execution_census_is_once_only_and_never_guessed_nonpublication() {
    let (mut o, _) = owner();
    let mut r = timed_service_recorder();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    o.publication_fault_after = Some(1);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err());
    // An unwound service episode has no endpoint; do not fabricate a completed
    // duration, recover the publisher, or replay publication to obtain one.
    assert_service_markers(&mut r, &[]);
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
    assert_service_markers(
        &mut r,
        &[(Boundary::InvocationExecution, Disposition::Unknown)],
    );
    assert!(o.is_faulted());
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 1, 1]);
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 1, 1]);
    assert_eq!(
        o.retire_invocation(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.forget_invocation_outcome(&id, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.documents[d.slot].as_ref().unwrap().source.text(), "a");
    assert!(o.stop_invocation_measurements_v1(Some(true), &mut r));
    assert!(o.retire_invocation_measurements_v1(&control(), &mut r));
    assert!(o.is_faulted());
}
#[test]
fn returned_integrity_fault_remains_a_faulted_census_not_terminal_refusal() {
    let (mut o, _) = owner();
    let mut r = timed_service_recorder();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    let Phase::Pending {
        prepared: Ok(p), ..
    } = &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
    else {
        panic!()
    };
    p.candidates[0].edits[0].inserted = "wrong".into();
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 1, 1]);
    assert_service_markers(
        &mut r,
        &[(Boundary::InvocationExecution, Disposition::Unknown)],
    );
}
#[test]
fn settings_and_relation_jobs_use_the_same_transitions_without_source_publication() {
    use crate::settings::*;
    let (mut o, _) = owner();
    let mut r = timed_service_recorder();
    o.install_initial_settings(
        crate::coordinator::settings::settings_test_fixture(&mut r),
        &mut r,
    )
    .unwrap();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 8,
            max_request_nodes: 64,
            max_request_depth: 16,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    o.configure_invocation_measurements_v2(
        InvocationMeasurementLimitsV2 {
            values: profile(100).values,
            max_pool_bytes: InvocationMeasurementLimitsV2::required_pool_bytes(
                16,
                crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            )
            .unwrap(),
            timing: crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
        },
        Some(true),
        &mut r,
    )
    .unwrap();
    let request = |generation| SettingChangeV1 {
        layer: SettingIdentityV1 {
            authority: "engine".into(),
            id: "target".into(),
            incarnation: 1,
        },
        expected_generation: generation,
        key: "flag".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
    };
    for generation in [1, 2] {
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_change(&id, &[request(generation)], &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
    }
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_relation(&id, &RelationCommandV1::CreateWorkspace, &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [1, 1, 1, 0, 0, 0, 3]);
    let Some(InvocationResponsivenessV1::Observed(ages)) =
        o.invocation_responsiveness_v1(&mut r).unwrap()
    else {
        panic!("native timed cohort");
    };
    assert_eq!((ages.admissions, ages.pending, ages.terminal), (3, 0, 3));
    assert!(ages.max_known_terminal_duration.is_some());
    assert_service_markers(
        &mut r,
        &[(Boundary::InvocationExecution, Disposition::Completed); 3],
    );
}
#[test]
fn failed_profile_admission_does_not_charge_root_or_parent_or_disable_jobs() {
    let (mut o, mut r) = owner();
    let root = o.semantic_root_usage(&mut r).unwrap();
    let p = profile(100);
    assert_eq!(
        o.configure_invocation_measurements_v1(
            InvocationMeasurementLimitsV1 {
                max_pool_bytes: p.max_pool_bytes - 1,
                ..p
            },
            Some(true),
            &mut r
        ),
        Err(Error::Limit)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    let mut hold = o.retention.reserve(200_000 - p.max_pool_bytes + 1).unwrap();
    assert_eq!(
        o.configure_invocation_measurements_v1(p, Some(true), &mut r),
        Err(Error::Limit)
    );
    o.retention.release(&mut hold).unwrap();
    enable(&mut o, &mut r);
    assert_eq!(o.retention_usage().reserved, p.max_pool_bytes);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), root);
    assert!(InvocationMeasurementLimitsV1::required_pool_bytes(usize::MAX).is_err());
}
#[test]
fn saturation_and_snapshot_ordinal_exhaustion_do_not_reject_queued_work() {
    let (mut o, mut r) = owner();
    o.configure_invocation_measurements_v1(profile(1), Some(true), &mut r)
        .unwrap();
    let first = o.invocation_measurements_v1(&mut r).unwrap().unwrap();
    let d = o.create(b"A", &mut r).unwrap().document;
    for (from, to) in [("A", "a"), ("a", "b")] {
        let q = intent(&o, &d, from, to, &mut r);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit(&id, &q, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
    }
    let second = o.invocation_measurements_v1(&mut r).unwrap().unwrap();
    assert_eq!(
        m::counter_delta_v1(&first.admissions, &second.admissions, Some(true)),
        None
    );
    assert!(matches!(
        second.admissions.state(),
        m::PointStateV1::Observed {
            value: m::ValueV1::Counter(m::CounterV1 {
                total: 1,
                exact: false
            }),
            ..
        }
    ));
    assert_eq!(o.invocation_measurements_v1(&mut r), Err(Error::Exhausted));
    let q = intent(&o, &d, "b", "c", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(o.read(&d, &mut r).unwrap().source.text(), "c");
    assert!(!o.is_faulted());
}
#[test]
fn stopped_retired_observer_does_not_cancel_pending_jobs_or_rearm() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    assert!(o.stop_invocation_measurements_v1(Some(true), &mut r));
    let p = o.invocation_measurements_v1(&mut r).unwrap().unwrap();
    assert_eq!(p.admissions.state(), m::PointStateV1::Off);
    let mut c = control();
    c.delivery_drained = None;
    assert!(!o.retire_invocation_measurements_v1(&c, &mut r));
    c.delivery_drained = Some(true);
    assert!(o.retire_invocation_measurements_v1(&c, &mut r));
    assert!(
        !o.invocations.slots[id.slot]
            .as_ref()
            .unwrap()
            .measurement_cohort
    );
    assert_eq!(o.retention_usage().reserved, 0);
    o.run(&id, &mut r).unwrap();
    assert_eq!(o.read(&d, &mut r).unwrap().source.text(), "a");
    assert_eq!(
        o.configure_invocation_measurements_v1(profile(100), Some(true), &mut r),
        Err(Error::Unavailable)
    );
    assert!(!o.retire_invocation_measurements_v1(&c, &mut r));
}
#[test]
fn snapshot_preflights_all_series_and_never_resets_existing_configuration() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    assert_eq!(
        o.configure_invocation_measurements_v1(profile(100), Some(true), &mut r),
        Ok(false)
    );
    assert_eq!(
        o.configure_invocation_measurements_v1(profile(101), Some(true), &mut r),
        Err(Error::Unavailable)
    );
    let State::Active(p) = &mut o.invocation_measurements else {
        panic!()
    };
    for _ in 0..=100 {
        p.series[6].observe(m::PointStateV1::Off).unwrap();
    }
    assert_eq!(o.invocation_measurements_v1(&mut r), Err(Error::Exhausted));
    let State::Active(p) = &mut o.invocation_measurements else {
        panic!()
    };
    assert!(p.series[0].can_observe());
    assert_eq!(
        p.series[0]
            .observe(m::PointStateV1::Off)
            .unwrap()
            .sequence(),
        0
    );
}

#[test]
fn nonconsuming_control_protocol_fault_is_not_a_second_original_job_disposition() {
    let (mut o, mut r) = owner();
    enable(&mut o, &mut r);
    let d = o.create(b"A", &mut r).unwrap().document;
    let q = intent(&o, &d, "A", "a", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit(&id, &q, &mut r).unwrap();
    // Context-free source jobs cannot receive scoped stop intent. Inject that
    // private protocol surprise: service refuses before consuming its candidate.
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .stop_requested = true;
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
    assert!(matches!(
        o.invocations.slots[id.slot].as_ref().unwrap().phase,
        Phase::Pending { .. }
    ));
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 0, 0, 0, 1]);
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(totals(&mut o, &mut r), [0, 0, 0, 1, 0, 0, 1]);
    assert!(o.is_faulted());
    assert_eq!(o.documents[d.slot].as_ref().unwrap().source.text(), "A");
}
