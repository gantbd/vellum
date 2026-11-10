use super::*;
use crate::capture::InertValueV1 as D;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1.into(),
        continuity: "c".into(),
    }
}
fn request<'h>(
    owner: &SourceCoordinator,
    id: &'h InvocationId,
    source: &DocumentVersion,
    r: &mut Recorder,
) -> NormalizedExactBatchArgumentsV1<'h> {
    use ExactBatchEnvelopeValueV1 as V;
    let writes = [DocumentEdits {
        expected: source.clone(),
        edits: vec![Replacement {
            start: 0,
            end: 1,
            expected: "a".into(),
            inserted: "X".into(),
        }],
    }];
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 1024,
        },
    };
    let scopes = [SourceOperationScope {
        expected: source.clone(),
        replacement_ranges: vec![ByteRange { start: 0, end: 3 }],
        insertion_sites: vec![InsertionSiteRange { first: 0, last: 3 }],
    }];
    let arguments = [
        ("contract", V::Data(D::Text("exact-source-batch/1"))),
        ("operation", V::Data(D::Text("exact-batch"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1)),
        ),
        ("sources", V::Sources(&writes)),
        ("reads", V::Reads(&reads)),
        (
            "scopes",
            V::Scopes(
                &scopes,
                SourceScopeBudget {
                    max_scopes: 8,
                    max_records: 16,
                    max_work: 1024,
                },
            ),
        ),
    ];
    owner
        .normalize_exact_batch_arguments_v1(
            V::Object(&arguments),
            &context(),
            CaptureLimitsV1 {
                max_bytes: 32768,
                max_nodes: 1024,
                max_depth: 16,
            },
            r,
        )
        .unwrap()
}
fn captured_set(owner: &SourceCoordinator, id: &InvocationId) -> u64 {
    let Phase::Pending {
        prepared: Ok(p), ..
    } = &owner.invocation_slot(id).unwrap().phase
    else {
        panic!()
    };
    p.candidates[0].annotation_generation
}

#[test]
fn actual_annotation_membership_change_invalidates_the_original_private_stamp_after_retry() {
    let mut r = Recorder::disabled();
    let (mut owner, source, _) = annotation::tests::setup(&mut r);
    let id = owner.issue_invocation(&mut r).unwrap();
    let q = request(&owner, &id, &source, &mut r);
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    let original_set = captured_set(&owner, &id);
    let p = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: source.clone(),
                    target: crate::attachment::AttachmentTarget::ContentRange { start: 1, end: 2 },
                    basis: TargetBasis::Before,
                },
                body: "new annotation".into(),
            }],
            &mut r,
        )
        .unwrap();
    owner.commit(p, &mut r).unwrap();
    let history = owner.history_usage();
    assert_ne!(
        owner
            .document(&source.document)
            .unwrap()
            .annotation_generation,
        original_set
    );
    let same = request(&owner, &id, &source, &mut r);
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&same, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(captured_set(&owner, &id), original_set);
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
    );
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "abc"
    );
    assert_eq!(owner.history_usage(), history);
}

#[test]
fn missing_declared_read_capture_is_an_unknown_fault_not_an_unconstrained_batch() {
    let mut r = Recorder::disabled();
    let (mut owner, source, _) = annotation::tests::setup(&mut r);
    let id = owner.issue_invocation(&mut r).unwrap();
    let q = request(&owner, &id, &source, &mut r);
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    let Phase::Pending {
        prepared: Ok(p), ..
    } = &mut owner.invocations.slots[id.slot].as_mut().unwrap().phase
    else {
        panic!()
    };
    p.declared_reads = None;
    let generation = owner.generation;
    let history = owner.history_usage();
    assert_eq!(owner.run(&id, &mut r), Err(InvocationError::Faulted));
    assert!(owner.is_faulted());
    assert_eq!(owner.generation, generation);
    assert_eq!(owner.history_usage(), history);
    assert_eq!(
        owner.documents[source.document.slot]
            .as_ref()
            .unwrap()
            .source
            .text(),
        "abc"
    );
}

#[test]
fn fresh_service_views_must_fit_the_complete_native_envelope_before_publication() {
    let mut r = Recorder::disabled();
    let (mut owner, source, _) = annotation::tests::setup(&mut r);
    owner
        .configure_logical_views(
            LogicalViewLimits {
                max_live_views: 4,
                max_issued_views: 4,
                max_total_selections: 8,
                normalization: crate::selection::SelectionBudget {
                    max_input_selections: 8,
                    max_output_selections: 8,
                    max_work: 1024,
                },
                max_mapping_steps: 4096,
            },
            &mut r,
        )
        .unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    let q = request(&owner, &id, &source, &mut r);
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    let slot = owner.invocation_slot(&id).unwrap();
    let (
        Some(BoundIntent::ExactBatch(c)),
        Phase::Pending {
            prepared: Ok(p), ..
        },
    ) = (&slot.intent, &slot.phase)
    else {
        panic!()
    };
    let original_bytes = envelope_bytes(c, p).unwrap();
    // Private fixture ceiling selects a precise retained-envelope boundary;
    // this is not a public live quota-reconfiguration facility.
    owner.invocations.limits.max_intent_bytes = original_bytes;
    let view = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&view, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(
            &view,
            stamp,
            &source,
            &[crate::selection::Selection {
                anchor: crate::coordinates::BytePosition(2),
                head: crate::coordinates::BytePosition(2),
                affinity: crate::attachment::Affinity::After,
            }],
            0,
            &mut r,
        )
        .unwrap();
    let before = owner.logical_view(&view, &mut r).unwrap();
    let generation = owner.generation;
    let history = owner.history_usage();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Limit))
    );
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "abc"
    );
    assert_eq!(
        owner.logical_view(&view, &mut r).unwrap().selections,
        before.selections
    );
    assert_eq!(owner.generation, generation);
    assert_eq!(owner.history_usage(), history);
}

#[test]
fn complete_retained_envelope_is_checked_before_admission_and_refusal_does_not_spend_the_ticket() {
    let mut r = Recorder::disabled();
    let (mut owner, source, _) = annotation::tests::setup(&mut r);
    let first = owner.issue_invocation(&mut r).unwrap();
    let q = request(&owner, &first, &source, &mut r);
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    let slot = owner.invocation_slot(&first).unwrap();
    let (
        Some(BoundIntent::ExactBatch(c)),
        Phase::Pending {
            prepared: Ok(p), ..
        },
    ) = (&slot.intent, &slot.phase)
    else {
        panic!()
    };
    let required = envelope_bytes(c, p).unwrap();
    let second = owner.issue_invocation(&mut r).unwrap();
    let q = request(&owner, &second, &source, &mut r);
    owner.invocations.limits.max_intent_bytes = required - 1;
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        owner.invocation_state(&second, &mut r),
        Ok(InvocationState::Issued)
    );
    owner.invocations.limits.max_intent_bytes = required;
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Ok(InvocationState::Pending)
    );
    owner.run(&second, &mut r).unwrap();
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "Xbc"
    );
}
