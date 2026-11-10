use super::scoped::{context, funded};
use super::*;
use AnnotationResultCapabilitiesV1 as C;

pub(super) fn grant<'a>(
    id: &'a InvocationId,
    c: &'a SettingsExecutionContextV1,
    d: &'a DocumentId,
    index: usize,
    caps: C,
) -> TrustedCompoundAnnotationResultGrantV1<'a> {
    TrustedCompoundAnnotationResultGrantV1 {
        invocation: id,
        authority: &c.authority,
        target: CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
            document: d,
            component: index,
        },
        capabilities: caps,
    }
}
pub(super) fn admission<'a>(
    c: &'a SettingsExecutionContextV1,
    grants: &'a [Option<TrustedCompoundAnnotationResultGrantV1<'a>>],
) -> TrustedCompoundAnnotationResultAdmissionV1<'a> {
    TrustedCompoundAnnotationResultAdmissionV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: 512,
        grants,
    }
}
pub(super) fn release<'a>(
    c: &'a SettingsExecutionContextV1,
    grant: Option<TrustedCompoundAnnotationResultGrantV1<'a>>,
) -> TrustedCompoundAnnotationResultReleaseV1<'a> {
    TrustedCompoundAnnotationResultReleaseV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
        grant,
    }
}
pub(super) fn record(out: &AnnotationResultV1) -> &AnnotationResultRecordV1 {
    match &out.decision {
        AnnotationResultDecisionV1::Applied { record, .. }
        | AnnotationResultDecisionV1::Unchanged { record } => record,
        _ => panic!("expected historical record"),
    }
}
#[test]
fn original_component_create_authority_cannot_release_a_sibling_or_widen_its_ceiling() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![],
            reads: vec![],
        },
        annotations: vec![
            AnnotationMutation::Create {
                target: target(&source, 0, 1),
                body: "PRIVATE_A".into(),
            },
            AnnotationMutation::Create {
                target: target(&source, 1, 2),
                body: "PRIVATE_B".into(),
            },
        ],
    };
    let grants = [
        Some(grant(
            &ticket,
            &c,
            &source.document,
            0,
            C::IdentityContentTarget,
        )),
        Some(grant(&ticket, &c, &source.document, 1, C::Identity)),
    ];
    o.submit_compound_with_annotation_results_v1(&ticket, &intent, &admission(&c, &grants), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
    o.run(&ticket, &mut r).unwrap();
    let a = o
        .retained_compound_annotation_result_v1(&ticket, 0, &release(&c, grants[0]), &mut r)
        .unwrap();
    let b = o
        .retained_compound_annotation_result_v1(
            &ticket,
            1,
            &release(
                &c,
                Some(grant(
                    &ticket,
                    &c,
                    &source.document,
                    1,
                    C::IdentityContentTarget,
                )),
            ),
            &mut r,
        )
        .unwrap();
    assert_ne!(record(&a).annotation, record(&b).annotation);
    assert_eq!(record(&a).content_version, Some(1));
    assert_eq!(record(&b).content_version, None);
    assert_eq!(
        o.retained_compound_annotation_result_v1(
            &ticket,
            0,
            &release(&c, Some(exact(&ticket, &c, &record(&a).annotation))),
            &mut r
        )
        .unwrap(),
        a
    );
    assert_eq!(
        o.retained_compound_annotation_result_v1(
            &ticket,
            0,
            &release(&c, Some(exact(&ticket, &c, &record(&b).annotation))),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.retained_compound_annotation_result_v1(&ticket, 1, &release(&c, grants[0]), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.retained_compound_annotation_result_v1(&ticket, 0, &release(&c, None), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.retained_compound_annotation_result_v1(&ticket, 2, &release(&c, grants[0]), &mut r),
        Err(InvocationError::Unavailable)
    );
    let generation = o.generation();
    o.submit_compound_with_annotation_results_v1(&ticket, &intent, &admission(&c, &grants), &mut r)
        .unwrap();
    o.run(&ticket, &mut r).unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(o.annotation_usage().issued_lifetime, 2);
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: record(&a).annotation.clone(),
                expected_content: 1,
                body: "later".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(
        o.retained_compound_annotation_result_v1(&ticket, 0, &release(&c, grants[0]), &mut r)
            .unwrap(),
        a
    );
}

pub(super) fn single(source: &DocumentVersion) -> CompoundIntent {
    CompoundIntent {
        source: SourceIntent {
            writes: vec![],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: target(source, 0, 1),
            body: "PRIVATE_CREATE".into(),
        }],
    }
}
fn exact<'a>(
    id: &'a InvocationId,
    c: &'a SettingsExecutionContextV1,
    a: &'a AnnotationId,
) -> TrustedCompoundAnnotationResultGrantV1<'a> {
    TrustedCompoundAnnotationResultGrantV1 {
        invocation: id,
        authority: &c.authority,
        target: CompoundAnnotationResultGrantTargetV1::Annotation(a),
        capabilities: C::IdentityContentTarget,
    }
}
pub(super) fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn compound_receipts_follow_original_order_and_the_final_shared_source_annotation_candidate() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let source = o.create(b"abcd", &mut r).unwrap();
    let (a, _) = create(&mut o, target(&source, 0, 1), "old", &mut r);
    let (b, _) = create(&mut o, target(&source, 1, 2), "delete", &mut r);
    let id = o.issue_invocation(&mut r).unwrap();
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![write(source.clone(), 0, 0, "", "X")],
            reads: vec![],
        },
        annotations: vec![
            AnnotationMutation::Reattach {
                annotation: a.clone(),
                expected_target: 1,
                target: target(&source, 2, 3),
            },
            AnnotationMutation::Create {
                target: target(&source, 0, 1),
                body: "created".into(),
            },
            AnnotationMutation::UpdateBody {
                annotation: a.clone(),
                expected_content: 1,
                body: "updated".into(),
            },
            AnnotationMutation::Delete {
                annotation: b.clone(),
                expected_content: 1,
                expected_target: 1,
            },
        ],
    };
    let gs = [
        Some(exact(&id, &c, &a)),
        Some(grant(
            &id,
            &c,
            &source.document,
            1,
            C::IdentityContentTarget,
        )),
        Some(exact(&id, &c, &a)),
        Some(exact(&id, &c, &b)),
    ];
    o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &gs), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(coarse) = o.invocation_outcome(&id, &mut r).unwrap() else {
        panic!("whole compound refused")
    };
    assert!(coarse.history_group.is_some());
    assert_eq!(coarse.changes.len(), 1);
    let mut out = Vec::new();
    for (i, g) in gs.iter().enumerate() {
        out.push(
            o.retained_compound_annotation_result_v1(&id, i, &release(&c, *g), &mut r)
                .unwrap(),
        );
    }
    assert_eq!(
        out.iter().map(|v| v.operation).collect::<Vec<_>>(),
        vec![
            AnnotationResultOperationV1::Reattach,
            AnnotationResultOperationV1::Create,
            AnnotationResultOperationV1::UpdateBody,
            AnnotationResultOperationV1::Delete
        ]
    );
    assert_eq!(record(&out[0]).annotation, a);
    assert_eq!(record(&out[0]), record(&out[2]));
    assert_eq!(
        (
            record(&out[0]).content_version,
            record(&out[0]).target_generation
        ),
        (Some(2), Some(2))
    );
    assert_eq!(record(&out[3]).annotation, b);
    assert!(!record(&out[3]).present);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "Xabcd"
    );
}

#[test]
fn absent_original_grants_allow_publication_but_replay_and_current_grants_cannot_add_disclosure() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let intent = single(&d);
    o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &[None]), &mut r)
        .unwrap();
    let held = o.retention_usage();
    o.run(&id, &mut r).unwrap();
    let g = Some(grant(&id, &c, &d.document, 0, C::IdentityContentTarget));
    assert_eq!(
        o.retained_compound_annotation_result_v1(&id, 0, &release(&c, g), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &[g]), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let mut changed = intent.clone();
    if let AnnotationMutation::Create { body, .. } = &mut changed.annotations[0] {
        *body = "changed".into()
    }
    assert_eq!(
        o.submit_compound_with_annotation_results_v1(
            &id,
            &changed,
            &admission(&c, &[None]),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(o.retention_usage(), held);
    assert_eq!(o.annotation_usage().issued_lifetime, 1);
    // A preexisting legacy compound cannot have disclosure retrofitted.
    let legacy = o.issue_invocation(&mut r).unwrap();
    o.submit_compound(&legacy, &intent, &mut r).unwrap();
    assert_eq!(
        o.submit_compound_with_annotation_results_v1(
            &legacy,
            &intent,
            &admission(&c, &[None]),
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
}

#[test]
fn invalid_component_grants_are_refused_before_charge_or_allocation_and_exact_parent_headroom_is_used()
 {
    for bad in 0..7 {
        let (mut o, mut r) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut r).unwrap();
        let other = o.create(b"xyz", &mut r).unwrap();
        let (a, _) = create(&mut o, target(&d, 0, 1), "existing", &mut r);
        let id = o.issue_invocation(&mut r).unwrap();
        let another = o.issue_invocation(&mut r).unwrap();
        let intent = single(&d);
        let mut g = grant(&id, &c, &d.document, 0, C::IdentityContentTarget);
        match bad {
            0 => g.invocation = &another,
            1 => g.authority = "foreign",
            2 => {
                g.target = CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
                    document: &d.document,
                    component: 1,
                }
            }
            3 => {
                g.target = CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
                    document: &other.document,
                    component: 0,
                }
            }
            4 => g.target = CompoundAnnotationResultGrantTargetV1::Annotation(&a),
            _ => {}
        }
        let grants = [Some(g)];
        let mut f = admission(&c, &grants);
        if bad == 5 {
            f.context_current = None
        }
        if bad == 6 {
            f.max_result_bytes = AnnotationResultV1::required_bytes() - 1
        }
        let generation = o.generation();
        let held = o.retention_usage();
        let issued = o.annotation_usage().issued_lifetime;
        assert!(
            o.submit_compound_with_annotation_results_v1(&id, &intent, &f, &mut r)
                .is_err()
        );
        assert_eq!(
            o.invocation_state(&id, &mut r).unwrap(),
            InvocationState::Issued
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.annotation_usage().issued_lifetime, issued);
    }
    let (mut probe, mut r) = funded(8388608);
    let c = context();
    let d = probe.create(b"abc", &mut r).unwrap();
    let id = probe.issue_invocation(&mut r).unwrap();
    probe
        .submit_compound_with_annotation_results_v1(
            &id,
            &single(&d),
            &admission(&c, &[None]),
            &mut r,
        )
        .unwrap();
    let needed = probe.retention_usage().reserved;
    for maximum in [needed - 1, needed] {
        let (mut o, mut r) = funded(maximum);
        let d = o.create(b"abc", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let result = o.submit_compound_with_annotation_results_v1(
            &id,
            &single(&d),
            &admission(&c, &[None]),
            &mut r,
        );
        if maximum == needed {
            assert_eq!(result, Ok(InvocationState::Pending));
            assert_eq!(o.retention_usage().reserved, needed)
        } else {
            assert_eq!(result, Err(InvocationError::Limit));
            assert_eq!(o.retention_usage().reserved, 0);
            assert_eq!(o.annotation_usage().issued_lifetime, 0)
        }
    }
}

#[test]
fn whole_plan_refusal_stop_and_eviction_do_not_mint_success_or_refund_unrelated_owners() {
    for stop in [false, true] {
        let (mut o, mut r) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let intent = single(&d);
        o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &[None]), &mut r)
            .unwrap();
        let second = o.issue_invocation(&mut r).unwrap();
        o.submit_compound_with_annotation_results_v1(
            &second,
            &intent,
            &admission(&c, &[None]),
            &mut r,
        )
        .unwrap();
        let total = o.retention_usage().reserved;
        if stop {
            o.stop_invocation(&id, &mut r).unwrap()
        } else {
            o.set_annotations_read_only(&d.document, true, &mut r)
                .unwrap();
            o.run(&id, &mut r).unwrap();
        }
        assert!(matches!(
            o.retained_compound_annotation_result_v1(&id, 0, &release(&c, None), &mut r)
                .unwrap()
                .decision,
            AnnotationResultDecisionV1::NotPublished
        ));
        assert_eq!(o.annotation_usage().present_records, 0);
        assert_eq!(
            o.forget_invocation_outcome(&id, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.retire_invocation(&id, &mut r),
            Err(InvocationError::Unavailable)
        );
        for bad in 0..4 {
            let mut f = CompoundAnnotationResultEvictionV1 {
                context: &c,
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            match bad {
                0 => f.retention_released = None,
                1 => f.retention_released = Some(false),
                2 => f.delivery_drained = None,
                _ => f.delivery_drained = Some(false),
            }
            assert_eq!(
                o.forget_compound_annotation_results_v1(&id, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
        o.forget_compound_annotation_results_v1(
            &id,
            &CompoundAnnotationResultEvictionV1 {
                context: &c,
                retention_released: Some(true),
                delivery_drained: Some(true),
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(o.retention_usage().reserved, total);
        assert_eq!(
            o.retained_compound_annotation_result_v1(&id, 0, &release(&c, None), &mut r),
            Err(InvocationError::OutcomeUnavailable)
        );
        assert_eq!(
            o.submit_compound_with_annotation_results_v1(
                &id,
                &intent,
                &admission(&c, &[None]),
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        for bad in 0..6 {
            let mut f = control(&c);
            match bad {
                0 => f.owner_control_allowed = None,
                1 => f.owner_control_allowed = Some(false),
                2 => f.retention_released = None,
                3 => f.retention_released = Some(false),
                4 => f.delivery_drained = None,
                _ => f.delivery_drained = Some(false),
            }
            assert_eq!(
                o.retire_compound_annotation_results_v1(&id, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
        o.retire_compound_annotation_results_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, total / 2);
        assert_eq!(
            o.retire_compound_annotation_results_v1(&id, &control(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_invocation(&id, &mut r).unwrap();
        assert_eq!(o.invocation_usage().issued_lifetime, 2);
        o.stop_invocation(&second, &mut r).unwrap();
        o.retire_compound_annotation_results_v1(&second, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}

#[test]
fn every_original_current_capability_meet_is_enforced_for_component_create_results() {
    let caps = [
        C::None,
        C::Identity,
        C::IdentityContent,
        C::IdentityTarget,
        C::IdentityContentTarget,
    ];
    for original in caps {
        for current in caps {
            let (mut o, mut r) = funded(8388608);
            let c = context();
            let d = o.create(b"abc", &mut r).unwrap();
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_compound_with_annotation_results_v1(
                &id,
                &single(&d),
                &admission(&c, &[Some(grant(&id, &c, &d.document, 0, original))]),
                &mut r,
            )
            .unwrap();
            o.run(&id, &mut r).unwrap();
            let out = o.retained_compound_annotation_result_v1(
                &id,
                0,
                &release(&c, Some(grant(&id, &c, &d.document, 0, current))),
                &mut r,
            );
            let expected = original.meet(current);
            if expected == C::None {
                assert_eq!(out, Err(InvocationError::Unavailable))
            } else {
                let out = out.unwrap();
                assert_eq!(
                    record(&out).content_version.is_some(),
                    matches!(expected, C::IdentityContent | C::IdentityContentTarget)
                );
                assert_eq!(
                    record(&out).target_generation.is_some(),
                    matches!(expected, C::IdentityTarget | C::IdentityContentTarget)
                );
            }
        }
    }
}

#[test]
fn unchanged_components_report_whole_compound_disposition_and_known_preparation_refusals_have_no_identity()
 {
    for changed in [false, true] {
        let (mut o, mut r) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut r).unwrap();
        let (a, _) = create(&mut o, target(&d, 0, 1), "same", &mut r);
        let id = o.issue_invocation(&mut r).unwrap();
        let intent = CompoundIntent {
            source: SourceIntent {
                writes: if changed {
                    vec![write(d.clone(), 0, 1, "a", "A")]
                } else {
                    vec![]
                },
                reads: vec![],
            },
            annotations: vec![AnnotationMutation::UpdateBody {
                annotation: a.clone(),
                expected_content: 1,
                body: "same".into(),
            }],
        };
        let gs = [Some(exact(&id, &c, &a))];
        let generation = o.generation();
        o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &gs), &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        let out = o
            .retained_compound_annotation_result_v1(&id, 0, &release(&c, gs[0]), &mut r)
            .unwrap();
        assert_eq!(
            matches!(out.decision, AnnotationResultDecisionV1::Applied { .. }),
            changed
        );
        assert_eq!(record(&out).content_version, Some(1));
        assert_eq!(o.generation(), generation + u64::from(changed));
    }
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let mut intent = single(&d);
    if let AnnotationMutation::Create { target, .. } = &mut intent.annotations[0] {
        target.target = AttachmentTarget::ContentRange { start: 0, end: 99 }
    }
    o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &[None]), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.retained_compound_annotation_result_v1(&id, 0, &release(&c, None), &mut r)
            .unwrap()
            .decision,
        AnnotationResultDecisionV1::NotPublished
    ));
}

#[test]
fn current_result_gates_and_original_publication_ceiling_fail_closed_without_mutation() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let mut other_context = c.clone();
    other_context.recipient = "other".into();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let g = Some(grant(&id, &c, &d.document, 0, C::IdentityContentTarget));
    o.submit_compound_with_annotation_results_v1(&id, &single(&d), &admission(&c, &[g]), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for bad in 0..15 {
        let mut f = release(&c, g);
        match bad {
            0 => f.publication_allowed = None,
            1 => f.publication_allowed = Some(false),
            2 => f.outcome_readable = None,
            3 => f.outcome_readable = Some(false),
            4 => f.result_context_current = None,
            5 => f.result_context_current = Some(false),
            6 => f.reply_ready = None,
            7 => f.reply_ready = Some(false),
            8 => f.resources_ready = None,
            9 => f.resources_ready = Some(false),
            10 => f.context = &other_context,
            11 => f.publication_epoch = 8,
            12 => f.max_result_bytes = AnnotationResultV1::required_bytes() - 1,
            13 => f.grant = None,
            _ => f.grant.as_mut().unwrap().authority = "other",
        }
        assert!(
            o.retained_compound_annotation_result_v1(&id, 0, &f, &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
    let second = o.issue_invocation(&mut r).unwrap();
    let gs = [Some(grant(
        &second,
        &c,
        &d.document,
        0,
        C::IdentityContentTarget,
    ))];
    let mut a = admission(&c, &gs);
    a.publication_allowed = Some(false);
    o.submit_compound_with_annotation_results_v1(&second, &single(&d), &a, &mut r)
        .unwrap();
    o.run(&second, &mut r).unwrap();
    assert_eq!(
        o.retained_compound_annotation_result_v1(&second, 0, &release(&c, gs[0]), &mut r),
        Err(InvocationError::Unavailable)
    );
}

#[test]
fn compound_result_capture_modes_preserve_semantics_with_bounded_redacted_optional_timing() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut setup).unwrap();
        let id = o.issue_invocation(&mut setup).unwrap();
        let gs = [Some(grant(
            &id,
            &c,
            &d.document,
            0,
            C::IdentityContentTarget,
        ))];
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        o.submit_compound_with_annotation_results_v1(&id, &single(&d), &admission(&c, &gs), &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        let out = o
            .retained_compound_annotation_result_v1(&id, 0, &release(&c, gs[0]), &mut r)
            .unwrap();
        assert_eq!(record(&out).content_version, Some(1));
        o.forget_compound_annotation_results_v1(
            &id,
            &CompoundAnnotationResultEvictionV1 {
                context: &c,
                retention_released: Some(true),
                delivery_drained: Some(true),
            },
            &mut r,
        )
        .unwrap();
        o.retire_compound_annotation_results_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(o.annotation_usage().present_records, 1);
        let mut seen = [0; 3];
        while let Some(e) = r.pop_front() {
            assert!(!format!("{e:?}").contains("PRIVATE"));
            let index = match e.boundary {
                Boundary::AnnotationResultAdmission => Some(0),
                Boundary::AnnotationDecisionStaging => Some(1),
                Boundary::AnnotationResultRelease => Some(2),
                _ => None,
            };
            if let Some(i) = index {
                seen[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2)
            }
        }
        if mode == 1 || mode == 2 {
            assert_eq!(seen, [1, 1, 1])
        }
        if mode == 0 || mode == 3 {
            assert_eq!(seen, [0, 0, 0])
        }
    }
}

#[test]
fn protected_compound_stop_checks_original_context_and_waits_for_actual_service() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let mut other = c.clone();
    other.recipient = "other".into();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_compound_with_annotation_results_v1(&id, &single(&d), &admission(&c, &[None]), &mut r)
        .unwrap();
    let held = o.retention_usage();
    let generation = o.generation();
    let facts = |context| TrustedInvocationControlV1 {
        context,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    };
    assert_eq!(
        o.request_invocation_stop(&id, &facts(&other), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(!o.is_faulted());
    o.request_invocation_stop(&id, &facts(&c), &mut r).unwrap();
    assert_eq!(
        o.invocation_state(&id, &mut r).unwrap(),
        InvocationState::Pending
    );
    assert_eq!(
        o.retained_compound_annotation_result_v1(&id, 0, &release(&c, None), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.retained_compound_annotation_result_v1(&id, 0, &release(&c, None), &mut r)
            .unwrap()
            .decision,
        AnnotationResultDecisionV1::NotPublished
    ));
    assert_eq!(o.retention_usage(), held);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.annotation_usage().present_records, 0);
}
