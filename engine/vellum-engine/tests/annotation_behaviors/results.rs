use super::arguments::{header, limits, reads};
use super::scoped::{context, facts, funded};
use super::*;
use AnnotationAuthoringEnvelopeValueV1 as V;
use AnnotationResultCapabilitiesV1 as C;
use vellum_engine::capture::InertValueV1 as D;
const CAPS: [C; 5] = [
    C::None,
    C::Identity,
    C::IdentityContent,
    C::IdentityTarget,
    C::IdentityContentTarget,
];

#[test]
fn f06_receipt_assembly_preserves_annotation_native_identity_and_field_filtering() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let profile = super::observation_support::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            annotation: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &ticket, &c, &mut r);
    let dependencies = reads();
    let t = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text("PRIVATE_BODY"))),
        ("target", V::Target(&t)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    o.submit_scoped_annotation_with_result_v1(
        &q,
        &c,
        &facts(&c),
        &admission(&ticket, &c, &source.document, C::IdentityContentTarget),
        &mut r,
    )
    .unwrap();
    let mut observer = profile.open(&mut o, &ticket, &c, &mut r);
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let native = release(&ticket, &c, &source.document, C::IdentityContent);
    let InvocationReceiptV1::Annotation(out) = profile
        .query(
            &o,
            &mut observer,
            InvocationReceiptRequestV1::Annotation(&native),
            &c,
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(record(&out).content_version, Some(1));
    assert_eq!(record(&out).target_generation, None);
    let mut denied = native;
    denied.grant = None;
    assert_eq!(
        profile
            .query(
                &o,
                &mut observer,
                InvocationReceiptRequestV1::Annotation(&denied),
                &c,
                &mut r
            )
            .err(),
        Some(InvocationError::Unavailable)
    );
}

#[test]
fn result_admission_release_timing_is_optional_content_free_and_cannot_change_delivery() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = funded(8388608);
        let source = o.create(b"abc", &mut setup).unwrap();
        let ticket = o.issue_invocation(&mut setup).unwrap();
        let c = context();
        let dependencies = reads();
        let t = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("PRIVATE_NEW"))),
            ("target", V::Target(&t)),
        ]);
        let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut setup)
            .unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        o.submit_scoped_annotation_with_result_v1(
            &q,
            &c,
            &facts(&c),
            &admission(&ticket, &c, &source.document, C::IdentityContentTarget),
            &mut r,
        )
        .unwrap();
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        let result = o
            .retained_annotation_result_v1(
                &ticket,
                &release(&ticket, &c, &source.document, C::IdentityContentTarget),
                &mut r,
            )
            .unwrap();
        assert_eq!(record(&result).content_version, Some(1));
        assert_eq!(
            o.retained_annotation_result_v1(
                &ticket,
                &release(&ticket, &c, &source.document, C::None),
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        let mut counts = [0; 2];
        let mut refused = 0;
        while let Some(event) = r.pop_front() {
            let index = match event.boundary {
                Boundary::AnnotationResultAdmission => Some(0),
                Boundary::AnnotationResultRelease => Some(1),
                _ => None,
            };
            if let Some(index) = index {
                counts[index] += 1;
                refused += usize::from(event.disposition == Disposition::Refused);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{event:?}").contains("PRIVATE"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [1, 2]
            } else {
                [0, 0]
            }
        );
        if mode == 1 || mode == 2 {
            assert_eq!(refused, 1);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
fn admitted(
    capabilities: Option<C>,
) -> (
    SourceCoordinator,
    InvocationId,
    DocumentVersion,
    SettingsExecutionContextV1,
    Recorder,
) {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let t = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text("PRIVATE_NEW"))),
        ("target", V::Target(&t)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    let mut a = admission(
        &ticket,
        &c,
        &source.document,
        capabilities.unwrap_or(C::None),
    );
    if capabilities.is_none() {
        a.grant = None;
    }
    o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
        .unwrap();
    (o, ticket, source, c, r)
}
fn record(result: &AnnotationResultV1) -> &AnnotationResultRecordV1 {
    match &result.decision {
        AnnotationResultDecisionV1::Applied { record, .. }
        | AnnotationResultDecisionV1::Unchanged { record } => record,
        _ => panic!("expected original record"),
    }
}
fn retirement() -> AnnotationInvocationRetirementV1 {
    AnnotationInvocationRetirementV1 {
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}

#[test]
fn current_release_checks_refuse_without_poisoning_the_original_receipt_or_allocating() {
    let (mut o, ticket, source, c, mut r) = admitted(Some(C::IdentityContentTarget));
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let other_ticket = o.issue_invocation(&mut r).unwrap();
    let other = o.create(b"xyz", &mut r).unwrap();
    let (other_annotation, _) = create(&mut o, target(&source, 1, 2), "unrelated", &mut r);
    let original = o
        .retained_annotation_result_v1(
            &ticket,
            &release(&ticket, &c, &source.document, C::IdentityContentTarget),
            &mut r,
        )
        .unwrap();
    let held = o.retention_usage();
    let generation = o.generation();
    let issued = o.annotation_usage();
    for case in 0..23 {
        let mut foreign = context();
        if (11..18).contains(&case) {
            match case - 11 {
                0 => foreign.authority.push('!'),
                1 => foreign.engine.push('!'),
                2 => foreign.recipient.push('!'),
                3 => foreign.purpose.push('!'),
                4 => foreign.destination.push('!'),
                5 => foreign.profile.push('!'),
                _ => foreign.continuity.push('!'),
            }
        }
        let mut f = release(&ticket, &c, &source.document, C::IdentityContentTarget);
        if case < 10 {
            let v = if case % 2 == 0 { None } else { Some(false) };
            match case / 2 {
                0 => f.publication_allowed = v,
                1 => f.outcome_readable = v,
                2 => f.result_context_current = v,
                3 => f.reply_ready = v,
                _ => f.resources_ready = v,
            }
        } else {
            match case {
                10 => f.publication_epoch += 1,
                11..=17 => f.context = &foreign,
                18 => f.grant.as_mut().unwrap().invocation = &other_ticket,
                19 => f.grant.as_mut().unwrap().authority = "other",
                20 => {
                    f.grant.as_mut().unwrap().target =
                        AnnotationResultGrantTargetV1::CreatedByInvocation(&other.document)
                }
                21 => {
                    f.grant.as_mut().unwrap().target =
                        AnnotationResultGrantTargetV1::Annotation(&other_annotation)
                }
                _ => f.max_result_bytes = AnnotationResultV1::required_bytes() - 1,
            }
        }
        assert!(
            o.retained_annotation_result_v1(&ticket, &f, &mut r)
                .is_err()
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.annotation_usage(), issued);
        assert_eq!(
            o.retained_annotation_result_v1(
                &ticket,
                &release(&ticket, &c, &source.document, C::IdentityContentTarget),
                &mut r
            )
            .unwrap(),
            original
        );
    }
    let mut exact = release(&ticket, &c, &source.document, C::IdentityContentTarget);
    exact.grant.as_mut().unwrap().target =
        AnnotationResultGrantTargetV1::Annotation(&record(&original).annotation);
    assert_eq!(
        o.retained_annotation_result_v1(&ticket, &exact, &mut r)
            .unwrap(),
        original
    );
}

#[test]
fn all_four_operations_and_noop_release_historical_membership_and_merged_versions() {
    for operation in ["create", "update-body", "reattach", "delete", "unchanged"] {
        let (mut o, mut r) = funded(8388608);
        let source = o.create(b"abc", &mut r).unwrap();
        let (existing, _) = create(&mut o, target(&source, 0, 1), "old", &mut r);
        let ticket = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let dependencies = reads();
        let t = target(&source, 1, 2);
        let mut raw = header(
            &ticket,
            &source.document,
            &dependencies,
            if operation == "unchanged" {
                "update-body"
            } else {
                operation
            },
        );
        match operation {
            "create" => raw.extend([("body", V::Data(D::Text("new"))), ("target", V::Target(&t))]),
            "update-body" | "unchanged" => raw.extend([
                ("annotation", V::Annotation(&existing)),
                ("expectedContent", V::Version(1)),
                (
                    "body",
                    V::Data(D::Text(if operation == "unchanged" {
                        "old"
                    } else {
                        "new"
                    })),
                ),
            ]),
            "reattach" => raw.extend([
                ("annotation", V::Annotation(&existing)),
                ("expectedTarget", V::Version(1)),
                ("target", V::Target(&t)),
            ]),
            _ => raw.extend([
                ("annotation", V::Annotation(&existing)),
                ("expectedContent", V::Version(1)),
                ("expectedTarget", V::Version(1)),
            ]),
        }
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        let mut a = admission(&ticket, &c, &source.document, C::IdentityContentTarget);
        if operation != "create" {
            assert_eq!(
                o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r),
                Err(InvocationError::Unavailable)
            );
            a.grant.as_mut().unwrap().target = AnnotationResultGrantTargetV1::Annotation(&existing);
        }
        o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
            .unwrap();
        if operation == "update-body" || operation == "reattach" {
            let m = if operation == "update-body" {
                AnnotationMutation::Reattach {
                    annotation: existing.clone(),
                    expected_target: 1,
                    target: t.clone(),
                }
            } else {
                AnnotationMutation::UpdateBody {
                    annotation: existing.clone(),
                    expected_content: 1,
                    body: "independent".into(),
                }
            };
            let p = o.prepare_compound(&[], &[], &[m], &mut r).unwrap();
            o.commit(p, &mut r).unwrap();
        }
        let before = o.generation();
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        let mut f = release(&ticket, &c, &source.document, C::IdentityContentTarget);
        if operation != "create" {
            f.grant.as_mut().unwrap().target = AnnotationResultGrantTargetV1::Annotation(&existing);
        }
        let result = o
            .retained_annotation_result_v1(&ticket, &f, &mut r)
            .unwrap();
        let record = record(&result);
        assert_eq!(record.present, operation != "delete");
        let version = if operation == "update-body" || operation == "reattach" {
            2
        } else {
            1
        };
        assert_eq!(record.content_version, Some(version));
        assert_eq!(record.target_generation, Some(version));
        if operation == "unchanged" {
            assert!(matches!(
                result.decision,
                AnnotationResultDecisionV1::Unchanged { .. }
            ));
            assert_eq!(o.generation(), before);
        } else {
            assert!(matches!(
                result.decision,
                AnnotationResultDecisionV1::Applied { .. }
            ));
            assert_eq!(o.generation(), before + 1);
        }
    }
}

#[test]
fn known_refusal_stop_and_original_publication_ceiling_do_not_invent_record_disclosure() {
    for mode in 0..3 {
        let (mut o, ticket, source, c, mut r) = admitted(None);
        if mode == 0 {
            o.stop_invocation(&ticket, &mut r).unwrap();
        } else if mode == 1 {
            let mut f = facts(&c);
            f.authorized = None;
            o.run_scoped_annotation_v1(&ticket, &c, &f, &mut r).unwrap();
        } else {
            o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
                .unwrap();
        }
        let mut f = release(&ticket, &c, &source.document, C::None);
        f.grant = None;
        let result = o.retained_annotation_result_v1(&ticket, &f, &mut r);
        if mode < 2 {
            assert_eq!(
                result.unwrap().decision,
                AnnotationResultDecisionV1::NotPublished
            );
        } else {
            assert_eq!(result, Err(InvocationError::Unavailable));
        }
    }
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let t = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([("body", V::Data(D::Text("new"))), ("target", V::Target(&t))]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    let mut a = admission(&ticket, &c, &source.document, C::IdentityContentTarget);
    a.publication_allowed = Some(false);
    o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
        .unwrap();
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.retained_annotation_result_v1(
            &ticket,
            &release(&ticket, &c, &source.document, C::IdentityContentTarget),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
}

#[test]
fn all_captured_current_capability_pairs_meet_and_the_five_element_lattice_is_closed() {
    for a in CAPS {
        assert_eq!(a.meet(C::None), C::None);
        assert_eq!(a.join(C::IdentityContentTarget), C::IdentityContentTarget);
        assert_eq!(a.meet(a), a);
        assert_eq!(a.join(a), a);
        for b in CAPS {
            assert_eq!(a.meet(b), b.meet(a));
            assert_eq!(a.join(b), b.join(a));
            assert_eq!(a.meet(a.join(b)), a);
            assert_eq!(a.join(a.meet(b)), a);
            for c in CAPS {
                assert_eq!(a.meet(b.meet(c)), a.meet(b).meet(c));
                assert_eq!(a.join(b.join(c)), a.join(b).join(c));
                assert_eq!(a.meet(b.join(c)), a.meet(b).join(a.meet(c)));
                assert_eq!(a.join(b.meet(c)), a.join(b).meet(a.join(c)));
            }
        }
    }
    for ceiling in CAPS {
        let (mut o, ticket, source, c, mut r) = admitted(Some(ceiling));
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        let charge = o.retention_usage().reserved;
        for current in CAPS {
            let result = o.retained_annotation_result_v1(
                &ticket,
                &release(&ticket, &c, &source.document, current),
                &mut r,
            );
            if ceiling == C::None || current == C::None {
                assert_eq!(result, Err(InvocationError::Unavailable));
            } else {
                let result = result.unwrap();
                let record = record(&result);
                let content = matches!(ceiling, C::IdentityContent | C::IdentityContentTarget)
                    && matches!(current, C::IdentityContent | C::IdentityContentTarget);
                let target = matches!(ceiling, C::IdentityTarget | C::IdentityContentTarget)
                    && matches!(current, C::IdentityTarget | C::IdentityContentTarget);
                assert_eq!(record.content_version, content.then_some(1));
                assert_eq!(record.target_generation, target.then_some(1));
            }
            assert_eq!(o.retention_usage().reserved, charge);
            assert_eq!(o.annotation_usage().issued_lifetime, 1);
        }
    }
    let (mut o, ticket, source, c, mut r) = admitted(None);
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.retained_annotation_result_v1(
            &ticket,
            &release(&ticket, &c, &source.document, C::IdentityContentTarget),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
}

#[test]
fn malformed_admission_and_short_frames_refuse_before_allocation_or_funding() {
    for case in 0..14 {
        let (mut o, mut r) = funded(8388608);
        let source = o.create(b"abc", &mut r).unwrap();
        let other = o.create(b"xyz", &mut r).unwrap();
        let other_ticket = o.issue_invocation(&mut r).unwrap();
        let (existing, _) = create(&mut o, target(&source, 0, 1), "existing", &mut r);
        let ticket = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let mut foreign = context();
        foreign.recipient.push('!');
        let dependencies = reads();
        let t = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([("body", V::Data(D::Text("new"))), ("target", V::Target(&t))]);
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        let mut a = admission(&ticket, &c, &source.document, C::IdentityContentTarget);
        match case {
            0 => a.context_current = None,
            1 => a.context_current = Some(false),
            2 => a.result_admission_allowed = None,
            3 => a.result_admission_allowed = Some(false),
            4 => a.publication_allowed = None,
            5 => a.context = &foreign,
            6 => a.grant.as_mut().unwrap().invocation = &other_ticket,
            7 => a.grant.as_mut().unwrap().authority = "other",
            8 => {
                a.grant.as_mut().unwrap().target =
                    AnnotationResultGrantTargetV1::CreatedByInvocation(&other.document)
            }
            9 => {
                a.grant.as_mut().unwrap().target =
                    AnnotationResultGrantTargetV1::Annotation(&existing)
            }
            10 => a.max_result_bytes = AnnotationResultV1::required_bytes() - 1,
            11 => a.max_result_bytes = usize::MAX,
            12 => {
                a.grant.as_mut().unwrap().invocation = &other_ticket;
                a.grant.as_mut().unwrap().capabilities = C::None;
            }
            _ => a.max_result_bytes = 0,
        }
        let before = o.annotation_usage();
        let held = o.retention_usage();
        let generation = o.generation();
        assert!(
            o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
                .is_err()
        );
        assert_eq!(o.annotation_usage(), before);
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Issued)
        );
    }
}

#[test]
fn replay_cannot_retrofit_or_widen_results_and_forget_retirement_never_reexecute() {
    for legacy in [false, true] {
        let (mut o, mut r) = funded(8388608);
        let source = o.create(b"abc", &mut r).unwrap();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let dependencies = reads();
        let t = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("PRIVATE_NEW"))),
            ("target", V::Target(&t)),
        ]);
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        let mut a = admission(&ticket, &c, &source.document, C::Identity);
        if legacy {
            o.submit_scoped_annotation_v1(&q, &c, &facts(&c), &mut r)
                .unwrap();
        } else {
            o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
                .unwrap();
        }
        let issued = o.annotation_usage().issued_lifetime;
        let charge = o.retention_usage().reserved;
        if legacy {
            assert_eq!(
                o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r),
                Err(InvocationError::IntentMismatch)
            );
        } else {
            a.grant.as_mut().unwrap().capabilities = C::IdentityContentTarget;
            assert_eq!(
                o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r),
                Err(InvocationError::IntentMismatch)
            );
            a.grant.as_mut().unwrap().capabilities = C::Identity;
            a.publication_epoch += 1;
            assert_eq!(
                o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r),
                Err(InvocationError::IntentMismatch)
            );
            a.publication_epoch -= 1;
        }
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        let f = release(&ticket, &c, &source.document, C::IdentityContentTarget);
        let result = o.retained_annotation_result_v1(&ticket, &f, &mut r);
        if legacy {
            assert!(result.is_err());
        } else {
            assert_eq!(record(&result.unwrap()).content_version, None);
        }
        assert_eq!(o.retention_usage().reserved, charge);
        o.forget_invocation_outcome(&ticket, &mut r).unwrap();
        assert!(
            o.retained_annotation_result_v1(&ticket, &f, &mut r)
                .is_err()
        );
        assert_eq!(
            o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.annotation_usage().issued_lifetime, issued);
        o.retire_scoped_annotation_v1(&ticket, &c, &retirement(), &mut r)
            .unwrap();
        assert!(
            o.retained_annotation_result_v1(&ticket, &f, &mut r)
                .is_err()
        );
        o.retire_invocation(&ticket, &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(
            o.retained_annotation_result_v1(&ticket, &f, &mut r)
                .is_err()
        );
    }
}

fn grant<'a>(
    id: &'a InvocationId,
    c: &'a SettingsExecutionContextV1,
    doc: &'a DocumentId,
    capabilities: AnnotationResultCapabilitiesV1,
) -> TrustedAnnotationResultGrantV1<'a> {
    TrustedAnnotationResultGrantV1 {
        invocation: id,
        authority: &c.authority,
        target: AnnotationResultGrantTargetV1::CreatedByInvocation(doc),
        capabilities,
    }
}
fn admission<'a>(
    id: &'a InvocationId,
    c: &'a SettingsExecutionContextV1,
    doc: &'a DocumentId,
    capabilities: AnnotationResultCapabilitiesV1,
) -> TrustedAnnotationResultAdmissionV1<'a> {
    TrustedAnnotationResultAdmissionV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: AnnotationResultV1::required_bytes(),
        grant: Some(grant(id, c, doc, capabilities)),
    }
}
fn release<'a>(
    id: &'a InvocationId,
    c: &'a SettingsExecutionContextV1,
    doc: &'a DocumentId,
    capabilities: AnnotationResultCapabilitiesV1,
) -> TrustedAnnotationResultReleaseV1<'a> {
    TrustedAnnotationResultReleaseV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: AnnotationResultV1::required_bytes(),
        grant: Some(grant(id, c, doc, capabilities)),
    }
}
#[test]
fn creation_result_authority_binds_only_the_original_engine_allocation_without_another_handshake() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let t = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text("PRIVATE_NEW"))),
        ("target", V::Target(&t)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    let a = admission(
        &ticket,
        &c,
        &source.document,
        AnnotationResultCapabilitiesV1::IdentityContentTarget,
    );
    o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
        .unwrap();
    let issued = o.annotation_usage().issued_lifetime;
    let charge = o.retention_usage().reserved;
    assert!(
        o.retained_annotation_result_v1(
            &ticket,
            &release(&ticket, &c, &source.document, a.grant.unwrap().capabilities),
            &mut r
        )
        .is_err()
    );
    o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
        .unwrap();
    assert_eq!(o.annotation_usage().issued_lifetime, issued);
    assert_eq!(o.retention_usage().reserved, charge);
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let result = o
        .retained_annotation_result_v1(
            &ticket,
            &release(
                &ticket,
                &c,
                &source.document,
                AnnotationResultCapabilitiesV1::IdentityContentTarget,
            ),
            &mut r,
        )
        .unwrap();
    let AnnotationResultDecisionV1::Applied {
        publication,
        record,
    } = &result.decision
    else {
        panic!()
    };
    assert_eq!(publication, &ticket);
    assert!(record.present);
    assert_eq!(record.content_version, Some(1));
    assert_eq!(record.target_generation, Some(1));
    assert_eq!(
        o.annotation(&record.annotation, &mut r).unwrap().body,
        "PRIVATE_NEW"
    );
    assert!(!format!("{result:?}").contains("PRIVATE"));
    assert_eq!(
        o.retained_annotation_result_v1(
            &ticket,
            &release(
                &ticket,
                &c,
                &source.document,
                AnnotationResultCapabilitiesV1::IdentityContentTarget
            ),
            &mut r
        )
        .unwrap(),
        result
    );
    assert_eq!(o.annotation_usage().issued_lifetime, issued);
    assert_eq!(o.retention_usage().reserved, charge);
}
