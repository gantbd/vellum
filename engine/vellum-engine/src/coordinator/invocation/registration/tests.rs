use super::*;
use std::cell::Cell;

#[test]
fn registration_service_rechecks_interpretation_profile_result_bound_and_native_authority() {
    for drift in 0..4 {
        let (mut o, mut r) = setup();
        let c = context();
        let id = submit(
            &mut o,
            &mut r,
            &c,
            &[change("r", CatalogExpectationV1::Absent)],
        );
        let mut n = native();
        if drift == 3 {
            n.authorized = None;
        }
        let mut f = host(&c, &n);
        match drift {
            0 => f.interpretation_current = None,
            1 => f.profile.max_relations += 1,
            2 => f.max_result_bytes += 1,
            _ => {}
        }
        let generation = o.generation();
        o.run_resource_registration_v1(&id, &c, &f, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::ResourceCatalogRejected(
                ResourceError::Unavailable
            ))
        ));
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
    }
}

#[test]
fn another_jobs_sealed_registration_result_cannot_be_delivered_for_this_ticket() {
    let (mut o, mut r) = setup();
    let c = context();
    let a = submit(
        &mut o,
        &mut r,
        &c,
        &[change("a", CatalogExpectationV1::Absent)],
    );
    let b = submit(
        &mut o,
        &mut r,
        &c,
        &[change("b", CatalogExpectationV1::Absent)],
    );
    for id in [&a, &b] {
        o.run_resource_registration_v1(id, &c, &host(&c, &native()), &mut r)
            .unwrap();
    }
    let other = std::mem::replace(
        &mut o.invocations.slots[b.slot].as_mut().unwrap().phase,
        Phase::Terminal(None),
    );
    o.invocations.slots[a.slot].as_mut().unwrap().phase = other;
    assert!(matches!(
        o.retained_resource_registration_result_v1(&a, &release(&c), &mut r),
        Err(InvocationError::Faulted)
    ));
    assert!(!o.is_faulted());
}

#[test]
fn registration_jobs_feed_real_once_only_admission_and_terminal_disposition_cohorts() {
    use crate::instrumentation::measurement as m;
    let (mut o, mut r) = setup();
    let c = context();
    o.configure_retention_budget(200000, &mut r).unwrap();
    o.configure_invocation_measurements_v1(
        InvocationMeasurementLimitsV1 {
            values: m::ValueLimitsV1 {
                max_integer: 100,
                max_seconds: 0.0,
            },
            max_pool_bytes: InvocationMeasurementLimitsV1::required_pool_bytes(64).unwrap(),
        },
        Some(true),
        &mut r,
    )
    .unwrap();
    let a = submit(
        &mut o,
        &mut r,
        &c,
        &[change("r", CatalogExpectationV1::Absent)],
    );
    o.run_resource_registration_v1(&a, &c, &host(&c, &native()), &mut r)
        .unwrap();
    let b = submit(
        &mut o,
        &mut r,
        &c,
        &[change("r", CatalogExpectationV1::Present { generation: 1 })],
    );
    o.run_resource_registration_v1(&b, &c, &host(&c, &native()), &mut r)
        .unwrap();
    let late = [change("x", CatalogExpectationV1::Absent)];
    let d = submit(&mut o, &mut r, &c, &late);
    let p = o
        .prepare_resource_catalog_v1(&late, &native(), &mut r)
        .unwrap();
    o.commit_resource_catalog_v1(p, &native(), &mut r).unwrap();
    o.run_resource_registration_v1(&d, &c, &host(&c, &native()), &mut r)
        .unwrap();
    let stopped = submit(
        &mut o,
        &mut r,
        &c,
        &[change("y", CatalogExpectationV1::Absent)],
    );
    o.stop_invocation(&stopped, &mut r).unwrap();
    for id in [&a, &b, &d, &stopped] {
        o.run_resource_registration_v1(id, &c, &host(&c, &native()), &mut r)
            .unwrap();
    }
    let points = o.invocation_measurements_v1(&mut r).unwrap().unwrap();
    let total = |p: &m::MeasurementPointV1| {
        let m::PointStateV1::Observed {
            value: m::ValueV1::Counter(v),
            ..
        } = p.state()
        else {
            panic!("counter")
        };
        v.total
    };
    assert_eq!(total(&points.admissions), 4);
    assert_eq!(
        points.dispositions.each_ref().map(total),
        [1, 1, 1, 1, 0, 0]
    );
}
thread_local! {static BEFORE: Cell<bool> = const {Cell::new(false)}; static AFTER: Cell<bool> = const {Cell::new(false)};}
pub(super) fn panic_before_stage() -> bool {
    BEFORE.with(|v| v.replace(false))
}
pub(super) fn panic_after_publication() -> bool {
    AFTER.with(|v| v.replace(false))
}
pub(super) fn inject_fault(after: bool) {
    if after {
        AFTER.with(|v| v.set(true));
    } else {
        BEFORE.with(|v| v.set(true));
    }
}
pub(super) fn profile() -> ResourceCatalogLimitsV1 {
    ResourceCatalogLimitsV1 {
        max_relations: 8,
        max_tag_bytes: 128,
        max_request_bytes: 16384,
        max_result_bytes: 16384,
        max_plan_bytes: 131072,
    }
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "register".into(),
        destination: "host".into(),
        continuity: "c".into(),
        profile: NATIVE_RESOURCE_REGISTRATION_PROFILE_V1.into(),
    }
}
pub(super) fn native() -> TrustedResourceCatalogFactsV1 {
    TrustedResourceCatalogFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        registration_allowed: Some(true),
        identity_allowed: Some(true),
        registry_coverage_complete: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        owner_control_allowed: Some(true),
        preparation_reserved: Some(true),
    }
}
fn host<'a>(
    c: &'a SettingsExecutionContextV1,
    n: &'a TrustedResourceCatalogFactsV1,
) -> TrustedResourceRegistrationInvocationV1<'a> {
    TrustedResourceRegistrationInvocationV1 {
        context: c,
        profile: profile(),
        max_result_bytes: 8192,
        interpretation_current: Some(true),
        host_result_allowed: Some(true),
        native: n,
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedResourceRegistrationResultV1<'_> {
    TrustedResourceRegistrationResultV1 {
        context: c,
        max_result_bytes: 8192,
        outcome_readable: Some(true),
        registration_result_readable: Some(true),
        registration_result_context_current: Some(true),
        host_result_allowed: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
pub(super) fn change(id: &str, expected: CatalogExpectationV1) -> ResourceCatalogChangeV1 {
    ResourceCatalogChangeV1 {
        address: ResourceCatalogAddressV1 {
            table: ResourceCatalogTableV1::Read,
            provider: ResourceCatalogIdentityV1 {
                authority: "a".into(),
                id: "provider".into(),
                incarnation: 1,
            },
            resource: ResourceCatalogIdentityV1 {
                authority: "a".into(),
                id: id.into(),
                incarnation: 1,
            },
        },
        expected,
        action: ResourceCatalogActionV1::Install(ResourceCatalogConfigurationV1 {
            identity_domain: "domain".into(),
            identity_key: "private-key".into(),
            identity_known: false,
            locator: "private-locator".into(),
            capabilities: ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read: false },
        }),
    }
}
pub(super) fn setup() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 16,
            max_participants: 8,
            max_staged_bytes: 8192,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 64,
            max_retained: 32,
            max_intent_bytes: 32768,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_resource_catalog_v1(&[], profile(), &native(), &mut r)
        .unwrap();
    (o, r)
}
fn submit(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    c: &SettingsExecutionContextV1,
    changes: &[ResourceCatalogChangeV1],
) -> InvocationId {
    let id = o.issue_invocation(r).unwrap();
    o.submit_resource_registration_v1(&id, changes, c, &host(c, &native()), r)
        .unwrap();
    id
}

#[test]
fn registration_jobs_reprepare_at_service_preserving_unrelated_edits_and_using_the_current_shared_clock()
 {
    let (mut o, mut r) = setup();
    let c = context();
    let batch = [change("first", CatalogExpectationV1::Absent)];
    let id = submit(&mut o, &mut r, &c, &batch);
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
    let source = o.create(b"prose survives", &mut r).unwrap();
    let p = o
        .prepare_resource_catalog_v1(
            &[change("other", CatalogExpectationV1::Absent)],
            &native(),
            &mut r,
        )
        .unwrap();
    o.commit_resource_catalog_v1(p, &native(), &mut r).unwrap();
    let before = o.generation();
    o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r)
        .unwrap();
    assert_eq!(o.generation(), before + 1);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "prose survives"
    );
    let result = o
        .retained_resource_registration_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    assert_eq!(result.invocation, id);
    assert_eq!(result.result.registrations[0].generation, Some(2));
    let generation = o.generation();
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &c, &host(&c, &native()), &mut r),
        Ok(InvocationState::Finished)
    );
    o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .unwrap()
            .result
            .registrations[0]
            .generation,
        Some(2)
    );
}

#[test]
fn same_slot_conflict_is_a_once_only_rejection_not_an_old_root_install_or_retry() {
    let (mut o, mut r) = setup();
    let c = context();
    let batch = [change("r", CatalogExpectationV1::Absent)];
    let id = submit(&mut o, &mut r, &c, &batch);
    let p = o
        .prepare_resource_catalog_v1(&batch, &native(), &mut r)
        .unwrap();
    o.commit_resource_catalog_v1(p, &native(), &mut r).unwrap();
    let generation = o.generation();
    o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::ResourceCatalogRejected(
            ResourceError::Stale
        ))
    ));
    assert_eq!(o.generation(), generation);
    assert!(matches!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    ));
    assert_eq!(
        o.submit_resource_registration_v1(
            &id,
            &[change("r", CatalogExpectationV1::Present { generation: 1 })],
            &c,
            &host(&c, &native()),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
}

#[test]
fn replay_preserves_complete_ordered_request_context_and_profile_but_does_not_refresh_the_result_ceiling()
 {
    let (mut o, mut r) = setup();
    let c = context();
    let n = native();
    let mut f = host(&c, &n);
    f.host_result_allowed = None;
    let batch = [
        change("a", CatalogExpectationV1::Absent),
        change("b", CatalogExpectationV1::Absent),
    ];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_resource_registration_v1(&id, &batch, &c, &f, &mut r)
        .unwrap();
    f.host_result_allowed = Some(true);
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &c, &f, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.submit_resource_registration_v1(
            &id,
            &[
                change("b", CatalogExpectationV1::Absent),
                change("a", CatalogExpectationV1::Absent)
            ],
            &c,
            &f,
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    let mut foreign = c.clone();
    foreign.purpose.push_str("other");
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &foreign, &host(&foreign, &n), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let mut altered = host(&c, &n);
    altered.max_result_bytes += 1;
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &c, &altered, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_resource_registration_v1(&id, &c, &f, &mut r).unwrap();
    assert!(matches!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::Unavailable)
    ));
}

#[test]
fn registration_admission_charges_complete_native_intent_and_receipt_before_binding() {
    let (mut o, mut r) = setup();
    let c = context();
    let n = native();
    let mut f = host(&c, &n);
    let batch = [change("r", CatalogExpectationV1::Absent)];
    let bytes = CatalogRequestCapture::bytes(&batch, profile()).unwrap() + size_of::<Capture>()
        - size_of::<CatalogRequestCapture>()
        + context_payload(&c).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.invocations.limits.max_intent_bytes = bytes;
    f.max_result_bytes = 0;
    assert_eq!(
        o.submit_resource_registration_v1(&id, &batch, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    f.max_result_bytes = 8192;
    o.submit_resource_registration_v1(&id, &batch, &c, &f, &mut r)
        .unwrap();
    o.run_resource_registration_v1(&id, &c, &f, &mut r).unwrap();
    let good = o
        .retained_resource_registration_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    let exact = receipt_bytes(&good.result).unwrap();
    for delta in [-1, 0, 1] {
        let mut g = release(&c);
        g.max_result_bytes = exact.checked_add_signed(delta).unwrap();
        assert_eq!(
            o.retained_resource_registration_result_v1(&id, &g, &mut r)
                .is_ok(),
            delta >= 0
        );
    }
}

#[test]
fn retained_registration_delivery_checks_all_current_facts_without_rewriting_execution() {
    let (mut o, mut r) = setup();
    let c = context();
    let id = submit(
        &mut o,
        &mut r,
        &c,
        &[change("r", CatalogExpectationV1::Absent)],
    );
    o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r)
        .unwrap();
    let generation = o.generation();
    for field in 0..6 {
        for value in [None, Some(false)] {
            let mut g = release(&c);
            match field {
                0 => g.outcome_readable = value,
                1 => g.registration_result_readable = value,
                2 => g.registration_result_context_current = value,
                3 => g.host_result_allowed = value,
                4 => g.reply_ready = value,
                _ => g.resources_ready = value,
            }
            assert!(matches!(
                o.retained_resource_registration_result_v1(&id, &g, &mut r),
                Err(InvocationError::Unavailable)
            ));
            assert_eq!(o.generation(), generation);
        }
    }
    let mut foreign = c.clone();
    foreign.recipient.push_str("other");
    assert!(
        o.retained_resource_registration_result_v1(&id, &release(&foreign), &mut r)
            .is_err()
    );
    assert!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .is_ok()
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert!(matches!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    ));
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.invocation_usage().retained, 0);
    assert!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r)
            .is_err()
    );
    assert_eq!(o.generation(), generation);
}

#[test]
fn protected_stop_does_not_require_registration_rights_and_wrong_context_cannot_service_or_settle_it()
 {
    let (mut o, mut r) = setup();
    let c = context();
    let id = submit(
        &mut o,
        &mut r,
        &c,
        &[change("r", CatalogExpectationV1::Absent)],
    );
    let mut foreign = c.clone();
    foreign.destination.push_str("other");
    let before = o.generation();
    assert_eq!(
        o.run_resource_registration_v1(&id, &foreign, &host(&foreign, &native()), &mut r),
        Err(InvocationError::Unavailable)
    );
    let control = TrustedInvocationControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    };
    o.request_invocation_stop(&id, &control, &mut r).unwrap();
    assert_eq!(
        o.run_resource_registration_v1(&id, &foreign, &host(&foreign, &native()), &mut r),
        Err(InvocationError::Unavailable)
    );
    let n = TrustedResourceCatalogFactsV1::default();
    let f = host(&c, &n);
    o.run_resource_registration_v1(&id, &c, &f, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert_eq!(o.generation(), before);
}

#[test]
fn foreign_pending_capture_and_ticket_substitution_fault_before_catalog_publication() {
    for swap_intent in [false, true] {
        let (mut o, mut r) = setup();
        let c = context();
        let a = submit(
            &mut o,
            &mut r,
            &c,
            &[change("a", CatalogExpectationV1::Absent)],
        );
        let b = submit(
            &mut o,
            &mut r,
            &c,
            &[change("b", CatalogExpectationV1::Absent)],
        );
        let Some(BoundIntent::ResourceCatalog(other)) =
            &o.invocations.slots[b.slot].as_ref().unwrap().intent
        else {
            unreachable!()
        };
        let other = other.clone();
        o.invocations.slots[a.slot].as_mut().unwrap().phase =
            Phase::ResourceCatalogPending(other.clone());
        if swap_intent {
            o.invocations.slots[a.slot].as_mut().unwrap().intent =
                Some(BoundIntent::ResourceCatalog(other));
        }
        let before = o.generation();
        assert_eq!(
            o.run_resource_registration_v1(&a, &c, &host(&c, &native()), &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.generation(), before);
        assert!(o.is_faulted());
    }
}

#[test]
fn unexpected_staging_and_post_publication_faults_never_become_safe_rejections_or_retries() {
    for after in [false, true] {
        let (mut o, mut r) = setup();
        let c = context();
        let id = submit(
            &mut o,
            &mut r,
            &c,
            &[change("r", CatalogExpectationV1::Absent)],
        );
        let before = o.generation();
        if after {
            AFTER.with(|v| v.set(true));
        } else {
            BEFORE.with(|v| v.set(true));
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r)
        }));
        assert_eq!(result.unwrap(), Err(InvocationError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.generation(), before + u64::from(after));
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        ));
        assert_eq!(
            o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.generation(), before + u64::from(after));
        assert!(o.retire_invocation(&id, &mut r).is_err());
        assert_eq!(o.invocation_usage().retained, 1);
    }
}

#[test]
fn retained_publication_origin_and_immutable_result_seal_reject_private_producer_drift_without_changing_root()
 {
    let (mut o, mut r) = setup();
    let c = context();
    let id = submit(
        &mut o,
        &mut r,
        &c,
        &[change("r", CatalogExpectationV1::Absent)],
    );
    o.run_resource_registration_v1(&id, &c, &host(&c, &native()), &mut r)
        .unwrap();
    let generation = o.generation();
    let Phase::Terminal(Some(InvocationOutcome::ResourceCatalogCommitted(retained))) =
        &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
    else {
        unreachable!()
    };
    retained.value = Arc::new(ResourceCatalogOutcomeV1 {
        status: ResourceCatalogStatusV1::Unchanged,
        registrations: Vec::new(),
    });
    assert!(matches!(
        o.retained_resource_registration_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::Faulted)
    ));
    assert!(!o.is_faulted());
    assert_eq!(o.generation(), generation);
}
