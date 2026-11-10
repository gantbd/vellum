use super::arguments::{header, limits, reads};
use super::*;
use AnnotationAuthoringEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "host-selected-annotation-profile".into(),
        continuity: "c".into(),
    }
}
fn profile() -> AnnotationInvocationLimitsV1 {
    AnnotationInvocationLimitsV1 {
        max_context_bytes: 1024,
        max_capture_bytes: 8192,
        max_plan_bytes: 32768,
        max_construction_bytes: 65536,
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedAnnotationInvocationV1<'_> {
    TrustedAnnotationInvocationV1 {
        context: c,
        limits: profile(),
        required_reads: &[],
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        companions_ready: Some(true),
        history_ready: Some(true),
        annotation_writable: Some(true),
        resources_ready: Some(true),
    }
}
pub(super) fn funded(maximum: usize) -> (SourceCoordinator, Recorder) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(maximum, &mut r).unwrap();
    (o, r)
}
#[test]
fn scoped_annotation_jobs_preserve_all_four_operations_and_cannot_use_generic_service() {
    for operation in ["create", "update-body", "reattach", "delete"] {
        let (mut o, mut r) = funded(8388608);
        let source = o.create(b"abc", &mut r).unwrap();
        let (annotation, _) = create(&mut o, target(&source, 0, 1), "PRIVATE_OLD", &mut r);
        o.set_read_only(&source.document, true, &mut r).unwrap();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let dependencies = reads();
        let c = context();
        let f = facts(&c);
        let target = target(&source, 1, 2);
        let mut raw = header(&ticket, &source.document, &dependencies, operation);
        match operation {
            "create" => raw.extend([
                ("body", V::Data(D::Text("PRIVATE_NEW"))),
                ("target", V::Target(&target)),
            ]),
            "update-body" => raw.extend([
                ("annotation", V::Annotation(&annotation)),
                ("expectedContent", V::Version(1)),
                ("body", V::Data(D::Text("PRIVATE_NEW"))),
            ]),
            "reattach" => raw.extend([
                ("annotation", V::Annotation(&annotation)),
                ("expectedTarget", V::Version(1)),
                ("target", V::Target(&target)),
            ]),
            _ => raw.extend([
                ("annotation", V::Annotation(&annotation)),
                ("expectedContent", V::Version(1)),
                ("expectedTarget", V::Version(1)),
            ]),
        }
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        assert_eq!(
            o.submit_scoped_annotation_v1(&q, &c, &f, &mut r),
            Ok(InvocationState::Pending)
        );
        let held = o.retention_usage().reserved;
        assert!(held > 0);
        let issued = o.annotation_usage().issued_lifetime;
        assert_eq!(
            o.submit_scoped_annotation_v1(&q, &c, &f, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.annotation_usage().issued_lifetime, issued);
        assert_eq!(o.retention_usage().reserved, held);
        assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Unavailable));
        assert_eq!(
            o.run_scoped_annotation_v1(&ticket, &c, &f, &mut r),
            Ok(InvocationState::Finished)
        );
        let InvocationOutcome::Committed(result) = o.invocation_outcome(&ticket, &mut r).unwrap()
        else {
            panic!("expected actual native commit")
        };
        assert!(result.operation.is_some());
        let result_id = result.annotation_changes[0].annotation.clone();
        if operation == "delete" {
            assert!(o.annotation(&result_id, &mut r).is_err());
        } else {
            let value = o.annotation(&result_id, &mut r).unwrap();
            assert_eq!(
                value.body,
                if operation == "reattach" {
                    "PRIVATE_OLD"
                } else {
                    "PRIVATE_NEW"
                }
            );
        }
        assert_eq!(o.read(&source.document, &mut r).unwrap().version, source);
        let generation = o.generation();
        assert_eq!(
            o.submit_scoped_annotation_v1(&q, &c, &f, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            o.run_scoped_annotation_v1(&ticket, &c, &f, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage().reserved, held);
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
fn exact_parent_funding_precedes_native_allocation_and_retirement_never_refunds_issuance() {
    let required = profile().required_hold_bytes(32768, 1048576).unwrap();
    for maximum in [required - 1, required] {
        let (mut o, mut r) = funded(maximum);
        let source = o.create(b"abc", &mut r).unwrap();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let f = facts(&c);
        let dependencies = reads();
        let target = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("new"))),
            ("target", V::Target(&target)),
        ]);
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        let before = o.annotation_usage();
        let result = o.submit_scoped_annotation_v1(&q, &c, &f, &mut r);
        if maximum < required {
            assert_eq!(result, Err(InvocationError::Limit));
            assert_eq!(o.retention_usage().reserved, 0);
            assert_eq!(o.annotation_usage(), before);
            assert_eq!(
                o.invocation_state(&ticket, &mut r),
                Ok(InvocationState::Issued)
            );
            continue;
        }
        assert_eq!(result, Ok(InvocationState::Pending));
        assert_eq!(o.retention_usage().reserved, required);
        assert_eq!(o.annotation_usage().issued_lifetime, 1);
        assert_eq!(
            o.retire_scoped_annotation_v1(&ticket, &c, &retirement(), &mut r),
            Err(InvocationError::Pending)
        );
        o.stop_invocation(&ticket, &mut r).unwrap();
        o.forget_invocation_outcome(&ticket, &mut r).unwrap();
        assert_eq!(
            o.retire_invocation(&ticket, &mut r),
            Err(InvocationError::Unavailable)
        );
        for field in 0..3 {
            for value in [None, Some(false)] {
                let mut denied = retirement();
                match field {
                    0 => denied.owner_control_allowed = value,
                    1 => denied.retention_released = value,
                    _ => denied.delivery_drained = value,
                }
                assert_eq!(
                    o.retire_scoped_annotation_v1(&ticket, &c, &denied, &mut r),
                    Err(InvocationError::Unavailable)
                );
                assert_eq!(o.retention_usage().reserved, required);
            }
        }
        o.retire_scoped_annotation_v1(&ticket, &c, &retirement(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(o.annotation_usage().issued_lifetime, 1);
        assert_eq!(
            o.submit_scoped_annotation_v1(&q, &c, &f, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.retire_scoped_annotation_v1(&ticket, &c, &retirement(), &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_invocation(&ticket, &mut r).unwrap();
        assert_eq!(o.invocation_usage().issued_lifetime, 1);
    }
    let mut invalid = profile();
    invalid.max_capture_bytes = usize::MAX;
    assert_eq!(
        invalid.required_hold_bytes(32768, 1048576),
        Err(InvocationError::Limit)
    );
    assert!(
        profile().max_construction_bytes > 32768,
        "fund a valid small plan even when N exceeds M"
    );
}

#[test]
fn every_admission_premise_is_strict_and_failed_capture_refunds_only_provisional_funding() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let target = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text("new"))),
        ("target", V::Target(&target)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    for index in 0..10 {
        for value in [None, Some(false)] {
            let mut f = facts(&c);
            match index {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.not_stopped = value,
                5 => f.interpretation_current = value,
                6 => f.companions_ready = value,
                7 => f.history_ready = value,
                8 => f.annotation_writable = value,
                _ => f.resources_ready = value,
            }
            assert_eq!(
                o.submit_scoped_annotation_v1(&q, &c, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.retention_usage().reserved, 0);
            assert_eq!(o.annotation_usage().issued_lifetime, 0);
        }
    }
    let mut f = facts(&c);
    f.limits.max_context_bytes = 1;
    assert_eq!(
        o.submit_scoped_annotation_v1(&q, &c, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.annotation_usage().issued_lifetime, 0);
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Issued)
    );
}

#[test]
fn fresh_context_and_permissions_are_required_without_replacing_original_preparation() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let target = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text("new"))),
        ("target", V::Target(&target)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    o.submit_scoped_annotation_v1(&q, &c, &facts(&c), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
    for field in 0..7 {
        let mut wrong = c.clone();
        match field {
            0 => wrong.authority.push('x'),
            1 => wrong.engine.push('x'),
            2 => wrong.recipient.push('x'),
            3 => wrong.purpose.push('x'),
            4 => wrong.destination.push('x'),
            5 => wrong.profile.push('x'),
            _ => wrong.continuity.push('x'),
        }
        assert_eq!(
            o.run_scoped_annotation_v1(&ticket, &wrong, &facts(&wrong), &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Pending)
        );
    }
    let mut changed = limits();
    changed.max_body_bytes -= 1;
    let other =
        normalize_annotation_authoring_arguments_v1(V::Object(&raw), changed, &mut r).unwrap();
    assert_eq!(
        o.submit_scoped_annotation_v1(&other, &c, &facts(&c), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let mut denied = facts(&c);
    denied.annotation_writable = None;
    assert_eq!(
        o.run_scoped_annotation_v1(&ticket, &c, &denied, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Unavailable))
    );
    assert_eq!(o.annotation_usage().retained_records, 0);
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.annotation_usage().retained_records, 0);
}

#[test]
fn current_required_reads_join_caller_reads_and_cannot_be_refreshed_after_preparation() {
    for host_required in [false, true] {
        let (mut o, mut r) = funded(8388608);
        let source = o.create(b"abc", &mut r).unwrap();
        let other = o.create(b"x", &mut r).unwrap();
        let ticket = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let predicate = ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(
            other.clone(),
        )));
        let required = [predicate.clone()];
        let mut dependencies = reads();
        if !host_required {
            dependencies.predicates.push(predicate);
        }
        let mut f = facts(&c);
        if host_required {
            f.required_reads = &required;
        }
        let target = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("new"))),
            ("target", V::Target(&target)),
        ]);
        let q =
            normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
        o.submit_scoped_annotation_v1(&q, &c, &f, &mut r).unwrap();
        let p = o
            .prepare(&[write(other, 0, 1, "x", "y")], &[], &mut r)
            .unwrap();
        o.commit(p, &mut r).unwrap();
        let generation = o.generation();
        assert_eq!(
            o.run_scoped_annotation_v1(&ticket, &c, &f, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            o.invocation_outcome(&ticket, &mut r),
            Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.annotation_usage().retained_records, 0);
    }
}

#[test]
fn independent_later_target_publication_is_preserved_by_owned_body_preparation() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let (annotation, _) = create(&mut o, target(&source, 0, 1), "old", &mut r);
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let mut raw = header(&ticket, &source.document, &dependencies, "update-body");
    raw.extend([
        ("annotation", V::Annotation(&annotation)),
        ("expectedContent", V::Version(1)),
        ("body", V::Data(D::Text("new"))),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    o.submit_scoped_annotation_v1(&q, &c, &facts(&c), &mut r)
        .unwrap();
    let moved = target(&source, 1, 2);
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Reattach {
                annotation: annotation.clone(),
                expected_target: 1,
                target: moved.clone(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let value = o.annotation(&annotation, &mut r).unwrap();
    assert_eq!(value.body, "new");
    assert_eq!(value.content_version, 2);
    assert_eq!(value.target_generation, 2);
    assert_eq!(value.authored, &moved);
}

#[test]
fn protected_stop_requires_actual_service_and_retains_its_funding_until_drain() {
    let (mut o, mut r) = funded(8388608);
    let source = o.create(b"abc", &mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let dependencies = reads();
    let target = target(&source, 0, 1);
    let mut raw = header(&ticket, &source.document, &dependencies, "create");
    raw.extend([
        ("body", V::Data(D::Text("new"))),
        ("target", V::Target(&target)),
    ]);
    let q = normalize_annotation_authoring_arguments_v1(V::Object(&raw), limits(), &mut r).unwrap();
    o.submit_scoped_annotation_v1(&q, &c, &facts(&c), &mut r)
        .unwrap();
    let held = o.retention_usage().reserved;
    let generation = o.generation();
    o.request_invocation_stop(
        &ticket,
        &TrustedInvocationControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            control_context_current: Some(true),
            control_scope_valid: Some(true),
            protected_control_ready: Some(true),
        },
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(o.retention_usage().reserved, held);
    assert_eq!(o.run(&ticket, &mut r), Ok(InvocationState::Finished));
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Ok(&InvocationOutcome::StoppedBeforePublication)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.annotation_usage().retained_records, 0);
    assert_eq!(o.retention_usage().reserved, held);
    o.retire_scoped_annotation_v1(&ticket, &c, &retirement(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.annotation_usage().issued_lifetime, 1);
}

#[test]
fn scoped_annotation_service_and_retirement_are_invariant_under_bounded_optional_timing() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = funded(8388608);
        let source = o.create(b"abc", &mut setup).unwrap();
        let ticket = o.issue_invocation(&mut setup).unwrap();
        let c = context();
        let dependencies = reads();
        let target = target(&source, 0, 1);
        let mut raw = header(&ticket, &source.document, &dependencies, "create");
        raw.extend([
            ("body", V::Data(D::Text("PRIVATE_BODY"))),
            ("target", V::Target(&target)),
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
        o.submit_scoped_annotation_v1(&q, &c, &facts(&c), &mut r)
            .unwrap();
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        o.retire_scoped_annotation_v1(&ticket, &c, &retirement(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(o.annotation_usage().retained_records, 1);
        let mut seen = [0; 3];
        while let Some(event) = r.pop_front() {
            let index = match event.boundary {
                Boundary::AnnotationInvocationAdmission => Some(0),
                Boundary::AnnotationInvocationService => Some(1),
                Boundary::AnnotationInvocationRetirement => Some(2),
                _ => None,
            };
            if let Some(index) = index {
                seen[index] += 1;
                assert_eq!(event.disposition, Disposition::Completed);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{event:?}").contains("PRIVATE_BODY"));
        }
        assert_eq!(seen, [usize::from(mode == 1 || mode == 2); 3]);
        if mode == 3 {
            assert!(r.dropped() >= 3);
        }
    }
}
