use super::*;
#[test]
fn full_configuration_read_descriptor_and_invalid_version_are_checked_before_binding() {
    let (mut o, _, intent, mut req, mut r) = read_dependency::configuration_read_test_fixture();
    let id = o.issue_invocation(&mut r).unwrap();
    let bytes = o.source_intent_bytes(&intent.source).unwrap()
        + std::mem::size_of::<CompoundIntent>()
        - std::mem::size_of::<SourceIntent>()
        + read_dependency::ReadInput::V3(&req)
            .retained_bytes(&[])
            .unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    assert_eq!(
        o.submit_compound_with_reads_v3(&id, &intent, &req, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.invocations.limits.max_intent_bytes = bytes;
    let ReadDependencyV3::ConfigurationDomain { expected, .. } = &mut req.predicates[0] else {
        unreachable!()
    };
    *expected = 0;
    assert_eq!(
        o.submit_compound_with_reads_v3(&id, &intent, &req, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    let ReadDependencyV3::ConfigurationDomain { expected, .. } = &mut req.predicates[0] else {
        unreachable!()
    };
    *expected = 1;
    o.submit_compound_with_reads_v3(&id, &intent, &req, &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r),
        Ok(InvocationOutcome::Committed(_))
    ));
}
#[test]
fn original_configuration_read_capture_drift_fences_without_a_source_prefix() {
    for drift in 0..3 {
        let (mut o, source, intent, req, mut r) =
            read_dependency::configuration_read_test_fixture();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_compound_with_reads_v3(&id, &intent, &req, &mut r)
            .unwrap();
        let Phase::Pending {
            prepared: Ok(p), ..
        } = &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
        else {
            unreachable!()
        };
        let reads = p.declared_reads.as_mut().unwrap();
        match drift {
            0 => reads.budget.max_work += 1,
            1 => {
                let ReadDependencyV3::ConfigurationDomain { interpretation, .. } =
                    &mut reads.predicates[0]
                else {
                    unreachable!()
                };
                interpretation.schema += 1;
            }
            _ => reads.predicates.clear(),
        };
        let generation = o.generation();
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.documents[source.document.slot]
                .as_ref()
                .unwrap()
                .source
                .text(),
            "A"
        );
    }
}
fn setup() -> (
    SourceCoordinator,
    DocumentVersion,
    CompoundIntent,
    ReadDependencyRequest,
    Recorder,
) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 128,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 8,
                max_edit_payload_bytes: 256,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 8192,
        }),
        annotations: Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 1024,
            max_live_snapshots: 8,
            max_snapshot_bytes: 4096,
        }),
        history: None,
        search: None,
        resources: None,
    });
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap();
    let b = o.create(b"B", &mut r).unwrap();
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![DocumentEdits {
                expected: a.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "a".into(),
                }],
            }],
            reads: vec![],
        },
        annotations: vec![],
    };
    let request = ReadDependencyRequest {
        predicates: vec![ReadDependencyV1::Source(b)],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 128,
        },
    };
    (o, a, intent, request, r)
}
#[test]
fn complete_read_descriptor_is_charged_before_binding_and_malformed_reads_leave_issued() {
    let (mut o, _, intent, request, mut r) = setup();
    let id = o.issue_invocation(&mut r).unwrap();
    let bytes = o.source_intent_bytes(&intent.source).unwrap()
        + std::mem::size_of::<CompoundIntent>()
        - std::mem::size_of::<SourceIntent>()
        + read_dependency::ReadCapture::retained_bytes(&intent.source.reads, &request).unwrap();
    o.invocations.limits.max_intent_bytes = bytes - 1;
    assert_eq!(
        o.submit_compound_with_reads(&id, &intent, &request, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.invocations.limits.max_intent_bytes = bytes;
    o.submit_compound_with_reads(&id, &intent, &request, &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = 8192;
    let duplicate = ReadDependencyRequest {
        predicates: vec![request.predicates[0].clone(), request.predicates[0].clone()],
        budget: request.budget,
    };
    assert_eq!(
        o.submit_compound_with_reads(&id, &intent, &duplicate, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn read_capture_loss_substitution_or_profile_drift_is_unknown_not_known_nonpublication() {
    for change in 0..4 {
        let (mut o, a, intent, request, mut r) = setup();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_compound_with_reads(&id, &intent, &request, &mut r)
            .unwrap();
        let Phase::Pending {
            prepared: Ok(p), ..
        } = &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
        else {
            panic!("not prepared")
        };
        match change {
            0 => p.declared_reads = None,
            1 => p.reads_required = false,
            2 => {
                p.declared_reads.as_mut().unwrap().predicates[0] = ReadDependencyV3::Base(
                    ReadDependencyV2::Base(ReadDependencyV1::Source(a.clone())),
                )
            }
            _ => p.declared_reads.as_mut().unwrap().budget.max_work += 1,
        }
        let g = o.generation();
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.generation(), g);
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(
            o.documents[a.document.slot].as_ref().unwrap().source.text(),
            "A"
        );
    }
}
#[test]
fn direct_read_capture_loss_or_final_limits_cannot_reclassify_a_restricted_plan_as_unrestricted() {
    for loss in [false, true] {
        let (mut o, a, intent, request, mut r) = setup();
        let mut p = o
            .prepare_compound_with_reads(&intent, &request, &mut r)
            .unwrap();
        if loss {
            p.declared_reads = None;
        } else {
            p.declared_reads.as_mut().unwrap().budget.max_predicates = 0;
        }
        let g = o.generation();
        assert_eq!(
            o.commit(p, &mut r),
            Err(if loss {
                CoordinatorError::Faulted
            } else {
                CoordinatorError::ReadDependency(ReadDependencyError::Limit)
            })
        );
        assert_eq!(o.generation(), g);
        assert_eq!(
            o.documents[a.document.slot].as_ref().unwrap().source.text(),
            "A"
        );
    }
}
