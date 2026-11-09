use super::*;
#[path = "../support/native_operation_decoder.rs"]
mod decoder_support;
#[path = "../support/native_operation_discovery_fixture.rs"]
mod installed;
#[test]
fn prepaid_source_decoder_routes_all_four_forms_without_new_native_attempts_or_reservations() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let task_ref = NativeOperationTaskV1::SourceLoad(&task);
    decoder_support::envelope(
        "typed",
        "status",
        task_ref,
        &id,
        &c.profile,
        "source-load",
        None,
        1.0,
        |e| {
            assert_eq!(
                f.owner.query_decoded_native_operation_v1(
                    &id,
                    e,
                    &policy(&c),
                    &facts(&c),
                    None,
                    &mut f.recorder
                ),
                Err(Error::Unavailable)
            )
        },
    );
    f.owner
        .enable_native_operation_decoder_v1(
            &id,
            decoder_support::limits(),
            &control(&c),
            &mut f.recorder,
        )
        .unwrap();
    let binding = installed::Route::install(&mut f.owner, &c, &mut f.recorder);
    let funding = f.owner.retention_usage();
    let producer = f.owner.source_reader_usage_v1().unwrap();
    let pool = f
        .owner
        .native_operation_observation_pool_usage_v1()
        .unwrap();
    let mut ordinary = None;
    for route in ["typed", "dynamic"] {
        for variant in ["status", "retained-outcome", "minimal-control-status"] {
            let result = decoder_support::envelope(
                route,
                variant,
                task_ref,
                &id,
                &c.profile,
                "source-load",
                None,
                1.0,
                |e| {
                    binding.query(
                        &mut f.owner,
                        &id,
                        e,
                        &policy(&c),
                        &facts(&c),
                        None,
                        &mut f.recorder,
                    )
                },
            )
            .unwrap();
            let NativeOperationDecodedReplyV1::Query(q) = result else {
                panic!("query")
            };
            if variant == "status" {
                ordinary = Some(q.observation().clone());
            }
        }
        let result = decoder_support::envelope(
            route,
            "bounded-observation-wait",
            task_ref,
            &id,
            &c.profile,
            "source-load",
            ordinary.as_ref(),
            1.0,
            |e| {
                binding.query(
                    &mut f.owner,
                    &id,
                    e,
                    &policy(&c),
                    &facts(&c),
                    Some(NativeOperationWaitClockV1 { epoch: 1, tick: 1 }),
                    &mut f.recorder,
                )
            },
        )
        .unwrap();
        let NativeOperationDecodedReplyV1::Waiting(wait) = result else {
            panic!("wait")
        };
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &wait,
                NativeOperationWaitEventV1::Stop,
                &policy(&c),
                &facts(&c),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .retire_native_operation_observation_wait_v1(&id, &wait, &control(&c), &mut f.recorder)
            .unwrap();
    }
    assert_eq!(f.owner.retention_usage(), funding);
    assert_eq!(f.owner.source_reader_usage_v1().unwrap(), producer);
    assert_eq!(
        f.owner
            .native_operation_observation_pool_usage_v1()
            .unwrap(),
        pool
    );
    f.owner
        .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
        .unwrap();
    assert_eq!(
        funding.reserved - f.owner.retention_usage().reserved,
        limits().max_held_bytes + decoder_support::limits().reserved_bytes().unwrap()
    );
    binding.retire(&mut f.owner, &mut f.recorder);
}
#[test]
fn bad_decoder_target_profile_facet_or_scope_cannot_apply_loss_to_another_context() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let other = open(&mut f, &task, &c);
    f.owner
        .enable_native_operation_decoder_v1(
            &id,
            decoder_support::limits(),
            &control(&c),
            &mut f.recorder,
        )
        .unwrap();
    for field in 0..4 {
        let mut lost = facts(&c);
        lost.scope = Some(false);
        let mut changed = c.clone();
        changed.purpose = "wrong".into();
        let result = decoder_support::envelope(
            "typed",
            if field == 2 {
                "retained-outcome"
            } else {
                "status"
            },
            NativeOperationTaskV1::SourceLoad(&task),
            if field == 0 { &other } else { &id },
            if field == 1 { "wrong" } else { &c.profile },
            "annotation-load",
            None,
            1.0,
            |e| {
                let g = if field == 3 {
                    let mut g = facts(&changed);
                    g.scope = Some(false);
                    g
                } else {
                    lost
                };
                f.owner.query_decoded_native_operation_v1(
                    &id,
                    e,
                    &policy(&c),
                    &g,
                    None,
                    &mut f.recorder,
                )
            },
        );
        assert_eq!(result, Err(Error::Unavailable));
        assert!(
            f.owner
                .native_operation_observation_usage_v1(&id)
                .unwrap()
                .live
        );
    }
    assert!(
        !f.owner
            .native_operation_decoder_usage_v1(&id)
            .unwrap()
            .faulted
    );
}
#[test]
fn malformed_routes_versions_roles_and_numeric_wait_budgets_refuse_without_decoder_fault() {
    use NativeOperationEnvelopeValueV1 as V;
    use vellum_engine::capture::InertValueV1 as D;
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    f.owner
        .enable_native_operation_decoder_v1(
            &id,
            decoder_support::limits(),
            &control(&c),
            &mut f.recorder,
        )
        .unwrap();
    let first = query(&mut f, &id, &c);
    let funded = f.owner.retention_usage();
    for value in [
        0.0,
        -1.0,
        1.5,
        f64::NAN,
        f64::INFINITY,
        9_007_199_254_740_992.0,
    ] {
        assert!(
            decoder_support::envelope(
                "typed",
                "bounded-observation-wait",
                NativeOperationTaskV1::SourceLoad(&task),
                &id,
                &c.profile,
                "source-load",
                Some(first.observation()),
                value,
                |e| f.owner.query_decoded_native_operation_v1(
                    &id,
                    e,
                    &policy(&c),
                    &facts(&c),
                    Some(NativeOperationWaitClockV1 { epoch: 1, tick: 1 }),
                    &mut f.recorder
                )
            )
            .is_err()
        );
    }
    for field in 0..6 {
        let mut args = [
            (
                "contract",
                V::Data(D::Text("native-operation-observation/1")),
            ),
            ("operation", V::Data(D::Text("status"))),
            ("task", V::Task(NativeOperationTaskV1::SourceLoad(&task))),
            ("context", V::Context(&id)),
            ("profile", V::Data(D::Text(&c.profile))),
        ];
        if field == 0 {
            args[2].1 = V::Data(D::Object(&[]));
        }
        if field == 1 {
            args[3].1 = V::Data(D::Null);
        }
        if field == 2 {
            args[4].0 = "task";
        }
        if field == 3 {
            args[0].1 = V::Data(D::Text("execution-observation/1"));
        }
        let header = [
            (
                "family",
                V::Data(D::Number(if field == 4 { 99.0 } else { 5.0 })),
            ),
            ("variant", V::Data(D::Text("status"))),
            (
                "version",
                V::Data(D::Number(if field == 5 { 2.0 } else { 1.0 })),
            ),
            ("arguments", V::Object(&args)),
        ];
        let outer = [
            ("route", V::Data(D::Text("typed"))),
            ("request", V::Object(&header)),
        ];
        assert_eq!(
            f.owner.query_decoded_native_operation_v1(
                &id,
                &V::Object(&outer),
                &policy(&c),
                &facts(&c),
                None,
                &mut f.recorder
            ),
            Err(Error::InvalidRequest)
        );
    }
    assert_eq!(f.owner.retention_usage(), funded);
    assert!(
        !f.owner
            .native_operation_decoder_usage_v1(&id)
            .unwrap()
            .faulted
    );
}
#[test]
fn typed_and_dynamic_native_envelopes_normalize_to_the_same_original_opaque_roles() {
    use NativeOperationEnvelopeValueV1 as V;
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let args = [
        (
            "contract",
            V::Data(D::Text("native-operation-observation/1")),
        ),
        ("operation", V::Data(D::Text("status"))),
        ("task", V::Task(NativeOperationTaskV1::SourceLoad(&task))),
        ("context", V::Context(&id)),
        ("profile", V::Data(D::Text(&c.profile))),
    ];
    let typed = [
        ("family", V::Data(D::Number(5.0))),
        ("variant", V::Data(D::Text("status"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let dynamic = [
        ("name", V::Data(D::Text("inspect-execution"))),
        ("variant", V::Data(D::Text("status"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let a = [
        ("route", V::Data(D::Text("typed"))),
        ("request", V::Object(&typed)),
    ];
    let b = [
        ("route", V::Data(D::Text("dynamic"))),
        ("request", V::Object(&dynamic)),
    ];
    let cap = CaptureLimitsV1 {
        max_bytes: 16384,
        max_nodes: 128,
        max_depth: 8,
    };
    let first = normalize_native_operation_envelope_v1(
        &V::Object(&a),
        &c,
        cap,
        cap.max_bytes,
        &mut f.recorder,
    )
    .unwrap();
    let second = normalize_native_operation_envelope_v1(
        &V::Object(&b),
        &c,
        cap,
        cap.max_bytes,
        &mut f.recorder,
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.task(), NativeOperationTaskV1::SourceLoad(&task));
    assert_eq!(first.context(), &id);
}
type Error = NativeOperationObservationErrorV1;
type Query = NativeOperationObservationQueryV1;
fn limits() -> NativeOperationObservationLimitsV1 {
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
fn policy(c: &SettingsExecutionContextV1) -> NativeOperationObservationPolicyV1 {
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
fn facts(c: &SettingsExecutionContextV1) -> NativeOperationObservationFactsV1<'_> {
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
        annotation_readable: None,
        disposition_readable: None,
        read_allowed: Some(true),
        reply_ready: Some(true),
        wait_capacity: Some(true),
        wait_clock_valid: Some(true),
        owner_control_allowed: Some(true),
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
fn setup() -> (Fixture, SourceReadIdV1, SettingsExecutionContextV1) {
    let (mut f, c) = operations::configured();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = operations::request(&target, b"load");
    let task = f
        .owner
        .request_owned_source_read_v1(&req, &operations::facts(&c), &mut f.recorder)
        .unwrap()
        .task;
    f.owner
        .configure_native_operation_observation_v1(
            NativeOperationObservationPoolLimitsV1 {
                max_contexts: 4,
                max_issued: 4,
                max_pool_bytes: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    (f, task, c)
}
fn open(
    f: &mut Fixture,
    task: &SourceReadIdV1,
    c: &SettingsExecutionContextV1,
) -> NativeOperationObservationIdV1 {
    f.owner
        .open_native_source_read_observation_v1(
            task,
            &policy(c),
            limits(),
            &facts(c),
            &mut f.recorder,
        )
        .unwrap()
}
fn query(
    f: &mut Fixture,
    id: &NativeOperationObservationIdV1,
    c: &SettingsExecutionContextV1,
) -> NativeOperationObservationReplyV1 {
    f.owner
        .query_native_operation_observation_v1(
            id,
            Query::RetainedOutcome,
            &policy(c),
            &facts(c),
            &mut f.recorder,
        )
        .unwrap()
}
fn receipt(reply: &NativeOperationObservationReplyV1) -> &NativeSourceReadReceiptV1 {
    let NativeOperationObservationReplyV1::Receipt {
        receipt: NativeOperationReceiptV1::SourceLoad(value),
        ..
    } = reply
    else {
        panic!("source read receipt")
    };
    value
}
#[test]
fn actual_source_load_status_and_receipt_share_progress_without_executing_or_disclosing_a_candidate()
 {
    let (mut f, task, c) = setup();
    let generation = f.owner.generation();
    let work = f.owner.resource_usage();
    let id = open(&mut f, &task, &c);
    let first = query(&mut f, &id, &c);
    assert_eq!(receipt(&first).phase, ReadPhase::Ready);
    assert_eq!(receipt(&first).witnesses, [None, None]);
    let status = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            Query::Status,
            &policy(&c),
            &facts(&c),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(status.observation(), first.observation());
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.resource_usage(), work);
}
#[test]
fn typed_end_versions_and_late_chunk_conflicts_preserve_safe_metadata_not_source_or_activation_results()
 {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let first = query(&mut f, &id, &c);
    let dispatch = f
        .owner
        .submit_source_read_v1(&task, &mut f.recorder)
        .unwrap();
    let version = SourceReadVersionV1::EpochToken(ResourceVersionV1 {
        epoch: b"EPOCH",
        token: b"TOKEN",
    });
    for e in [
        f.port.source_read_start_v1(&dispatch, version).unwrap(),
        f.port
            .source_read_chunk_v1(&dispatch, 0, 0, b"PRIVATE-BODY")
            .unwrap(),
        f.port
            .source_read_end_v1(&dispatch, version, 1, 12)
            .unwrap(),
    ] {
        f.owner.record_source_read_v1(e, &mut f.recorder).unwrap();
    }
    let end = query(&mut f, &id, &c);
    let r = receipt(&end);
    assert_ne!(first.observation(), end.observation());
    let Some(NativeSourceReadTerminalWitnessV1::End {
        version,
        coherence,
        chunks: 1,
        bytes: 12,
    }) = &r.witnesses[0]
    else {
        panic!("end")
    };
    assert_eq!(version.epoch(), Some(b"EPOCH".as_slice()));
    assert_eq!(version.token(), b"TOKEN");
    assert_eq!(*coherence, ReadStrength::CapturedRead);
    assert_eq!(r.received_bytes, 12);
    assert!(!r.quiescent);
    f.owner
        .activate_source_read_v1(&task, &mut f.recorder)
        .unwrap();
    assert_eq!(query(&mut f, &id, &c), end); // Activation is not receipt progress.
    f.owner
        .record_source_read_v1(
            f.port
                .source_read_chunk_v1(&dispatch, 0, 0, b"OTHER-SECRET")
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let conflict = query(&mut f, &id, &c);
    assert!(receipt(&conflict).contested);
    assert_eq!(receipt(&conflict).witnesses, r.witnesses);
    assert!(!format!("{conflict:?}").contains("PRIVATE-BODY"));
    assert!(!format!("{conflict:?}").contains("OTHER-SECRET"));
}
#[test]
fn current_source_read_grants_are_independent_of_annotation_and_write_disposition_and_loss_absorbs()
{
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let first = query(&mut f, &id, &c);
    for field in 0..3 {
        for denied in [None, Some(false)] {
            let mut g = facts(&c);
            match field {
                0 => g.read_allowed = denied,
                1 => g.source_readable = denied,
                _ => g.operation_receipt_allowed = denied,
            }
            assert_eq!(
                f.owner.query_native_operation_observation_v1(
                    &id,
                    Query::RetainedOutcome,
                    &policy(&c),
                    &g,
                    &mut f.recorder
                ),
                Err(Error::Unavailable)
            );
            assert!(matches!(
                f.owner
                    .query_native_operation_observation_v1(
                        &id,
                        Query::MinimalControlStatus,
                        &policy(&c),
                        &g,
                        &mut f.recorder
                    )
                    .unwrap(),
                NativeOperationObservationReplyV1::Status { receipt: None, .. }
            ));
        }
    }
    assert_eq!(query(&mut f, &id, &c), first);
    let mut g = facts(&c);
    g.continuous = Some(false);
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::Status,
            &policy(&c),
            &g,
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::Status,
            &policy(&c),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    assert!(
        !f.owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .stopped
    );
}
#[test]
fn all_source_observation_admission_facts_original_scope_and_parent_kind_are_required() {
    let (mut f, task, c) = setup();
    let before = f.owner.retention_usage();
    for field in 0..16 {
        let mut g = facts(&c);
        let slot = match field {
            0 => &mut g.authorized,
            1 => &mut g.scope,
            2 => &mut g.continuous,
            3 => &mut g.live,
            4 => &mut g.retained,
            5 => &mut g.transfer_ready,
            6 => &mut g.resources_ready,
            7 => &mut g.preparation_reserved,
            8 => &mut g.registry_ready,
            9 => &mut g.retention_ready,
            10 => &mut g.control_allowed,
            11 => &mut g.operation_receipt_admission_allowed,
            12 => &mut g.operation_receipt_allowed,
            13 => &mut g.source_readable,
            14 => &mut g.read_allowed,
            _ => &mut g.reply_ready,
        };
        *slot = None;
        assert_eq!(
            f.owner.open_native_source_read_observation_v1(
                &task,
                &policy(&c),
                limits(),
                &g,
                &mut f.recorder
            ),
            Err(Error::Unavailable)
        );
        assert_eq!(f.owner.retention_usage(), before);
    }
    let mut wrong = c.clone();
    wrong.purpose = "other".into();
    assert_eq!(
        f.owner.open_native_source_read_observation_v1(
            &task,
            &policy(&wrong),
            limits(),
            &facts(&wrong),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    let raw = admit(&mut f, b"other");
    assert_eq!(
        f.owner.open_native_source_read_observation_v1(
            &raw,
            &policy(&c),
            limits(),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    let (_, foreign, _) = setup();
    assert_eq!(
        f.owner.open_native_source_read_observation_v1(
            &foreign,
            &policy(&c),
            limits(),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
}
#[test]
fn zero_ordinary_credit_and_observer_retirement_do_not_control_or_pin_a_source_producer() {
    let (mut f, task, c) = setup();
    let mut l = limits();
    l.max_issued = 0;
    let id = f
        .owner
        .open_native_source_read_observation_v1(&task, &policy(&c), l, &facts(&c), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::RetainedOutcome,
            &policy(&c),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Limit)
    );
    let mut g = facts(&c);
    g.ordinary_ready = None;
    g.reply_ready = None;
    g.read_allowed = None;
    f.owner
        .query_native_operation_observation_v1(
            &id,
            Query::MinimalControlStatus,
            &policy(&c),
            &g,
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .stop_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
        .unwrap();
    assert!(
        !f.owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .stopped
    );
    f.owner
        .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
        .unwrap();
    let other = open(&mut f, &task, &c);
    f.owner.stop_source_read_v1(&task, &mut f.recorder).unwrap();
    f.owner
        .retire_source_read_v1(&task, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &other,
            Query::Status,
            &policy(&c),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
}
#[test]
fn source_load_waits_observe_native_progress_without_renewing_budget_or_stopping_work() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let first = query(&mut f, &id, &c);
    let wait = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: first.observation(),
                budget: 1,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 1 },
            },
            &policy(&c),
            &facts(&c),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &wait,
                NativeOperationWaitEventV1::Poll,
                &policy(&c),
                &facts(&c),
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationWaitReplyV1::Waiting(wait.clone())
    );
    assert_eq!(
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &wait,
                NativeOperationWaitEventV1::Tick(NativeOperationWaitClockV1 { epoch: 1, tick: 2 }),
                &policy(&c),
                &facts(&c),
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationWaitReplyV1::Ended
    );
    assert_eq!(
        f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .ended_waiters,
        1
    );
    assert!(
        !f.owner
            .source_read_status_v1(&task, &mut f.recorder)
            .unwrap()
            .stopped
    );
    f.owner
        .retire_native_operation_observation_wait_v1(&id, &wait, &control(&c), &mut f.recorder)
        .unwrap();
    let wait = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: first.observation(),
                budget: 2,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 3 },
            },
            &policy(&c),
            &facts(&c),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .submit_source_read_v1(&task, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &wait,
                NativeOperationWaitEventV1::Poll,
                &policy(&c),
                &facts(&c),
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationWaitReplyV1::Observed(_)
    ));
}
#[test]
fn source_receipt_failure_and_optional_diagnostics_never_claim_quiescence_or_disclose_private_data()
{
    for capacity in [None, Some(0), Some(128)] {
        let (mut f, task, c) = setup();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let id = open(&mut f, &task, &c);
        let dispatch = f
            .owner
            .submit_source_read_v1(&task, &mut f.recorder)
            .unwrap();
        f.owner
            .record_source_read_v1(
                f.port
                    .source_read_failure_v1(&dispatch, SourceReadFailureV1::Denied)
                    .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let failed = query(&mut f, &id, &c);
        let r = receipt(&failed);
        assert_eq!(
            r.witnesses,
            [
                Some(NativeSourceReadTerminalWitnessV1::Failure(
                    SourceReadFailureV1::Denied
                )),
                None
            ]
        );
        assert!(!r.quiescent);
        let mut events = String::new();
        while let Some(event) = f.recorder.pop_front() {
            events.push_str(&format!("{event:?}"));
        }
        assert!(!events.contains("recipient") && !events.contains("destination"));
        if capacity == Some(128) {
            assert!(
                events.contains("NativeOperationObservationAdmission")
                    && events.contains("NativeOperationObservationSnapshot")
            );
        }
    }
}
