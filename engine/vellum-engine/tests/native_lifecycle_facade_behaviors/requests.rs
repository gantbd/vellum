use super::*;
#[test]
fn data_lifecycle_facade_preserves_scratch_and_all_live_relation_owners() {
    let (mut o, mut r) = scratch::owner();
    let c = scratch::context();
    let f = scratch::facts(&c);
    let a = scratch::admission(&f);
    let q = scratch::request(b"original");
    let id = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    o.submit_native_document_lifecycle_request_v1(
        &id,
        NativeDocumentLifecycleRequestV1::Scratch(&q),
        NativeDocumentLifecycleAdmissionV2::Existing(NativeDocumentLifecycleAdmissionV1::Scratch(
            &a,
        )),
        &mut r,
    )
    .unwrap();
    assert_eq!(o.generation(), generation);
    o.run_native_document_lifecycle_v2(
        &id,
        NativeDocumentLifecycleServiceV2::Existing(NativeDocumentLifecycleServiceV1::Scratch(&f)),
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.scratch_result_v1(&id, &scratch::release(&c), &mut r)
            .unwrap()
            .invocation,
        id
    );
    assert_eq!(o.generation(), generation + 1);
    for kind in 0..3 {
        let (mut o, mut r) = scratch::owner();
        let c = scratch::context();
        let f = relation_facts(&c);
        let a = relation_admission(&f);
        let q = relation_request(&mut o, &mut r, kind);
        let id = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        o.submit_native_document_lifecycle_request_v1(
            &id,
            NativeDocumentLifecycleRequestV1::LiveRelation(&q),
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::LiveRelation(&a),
            ),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation);
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::Existing(
                NativeDocumentLifecycleServiceV1::LiveRelation(&f),
            ),
            &mut r,
        )
        .unwrap();
        let n = TrustedLiveRelationResultV1 {
            context: &c,
            publication_epoch: 7,
            publication_allowed: Some(true),
            outcome_readable: Some(true),
            result_readable: Some(true),
            result_context_current: Some(true),
            historical_document_readable: Some(true),
            reply_ready: Some(true),
            resources_ready: Some(true),
            max_result_bytes: 512,
        };
        let result = o.live_relation_result_v1(&id, &n, &mut r).unwrap();
        assert_eq!(result.document, q.document);
        assert!(result.changed);
        assert_eq!(o.generation(), generation + 1);
        o.retire_live_relation_v1(&id, &scratch::control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn data_lifecycle_facade_rejects_every_mismatched_admission_without_engine_preparation() {
    let (mut o, mut r) = scratch::owner();
    let c = scratch::context();
    let q = relation_request(&mut o, &mut r, 0);
    let source = o.read(&q.document, &mut r).unwrap().version;
    let explicit = ExplicitDisposalRequestV1 {
        source,
        annotation_generation: 0,
        lifecycle: q.expected_lifecycle,
        serialization_generation: 1,
        binding_generation: None,
        source_disposition: DisposalDispositionV1::Retain,
        annotation_disposition: DisposalDispositionV1::Retain,
    };
    let sq = scratch::request(b"new");
    let sf = scratch::facts(&c);
    let sa = scratch::admission(&sf);
    let lf = relation_facts(&c);
    let la = relation_admission(&lf);
    let df = disposal_facts(&c);
    let da = disposal_admission(&df);
    let g = TrustedDisposalCoverageV1 {
        not_stopped: None,
        companions_ready: None,
        versions_available: None,
        reply_ready: None,
        lifecycle_allowed: None,
        coverage_complete: None,
        owner_control_allowed: None,
        source_retention_ready: None,
        source_discard_allowed: None,
        annotation_retention_ready: None,
        annotation_discard_allowed: None,
        retention_complete: None,
        effects_retained: None,
        history_retained: None,
        snapshots_retained: None,
        providers_retained: None,
        views_detachable: None,
        memberships_detachable: None,
        observer_scopes_fenced: None,
        binding_coverage_complete: None,
    };
    let requests = [
        NativeDocumentLifecycleRequestV1::Scratch(&sq),
        NativeDocumentLifecycleRequestV1::LiveRelation(&q),
        NativeDocumentLifecycleRequestV1::ExplicitDisposal(&explicit),
    ];
    let admissions = [
        NativeDocumentLifecycleAdmissionV2::Existing(NativeDocumentLifecycleAdmissionV1::Scratch(
            &sa,
        )),
        NativeDocumentLifecycleAdmissionV2::Existing(
            NativeDocumentLifecycleAdmissionV1::LiveRelation(&la),
        ),
        NativeDocumentLifecycleAdmissionV2::Existing(
            NativeDocumentLifecycleAdmissionV1::PreparedDisposal(&da),
        ),
        NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
            native: &da,
            coverage: &g,
        },
    ];
    for (i, request) in requests.into_iter().enumerate() {
        for (j, admission) in admissions.into_iter().enumerate() {
            if matches!((i, j), (0, 0) | (1, 1) | (2, 3)) {
                continue;
            }
            let id = o.issue_invocation(&mut r).unwrap();
            let held = o.retention_usage();
            let generation = o.generation();
            assert_eq!(
                o.submit_native_document_lifecycle_request_v1(&id, request, admission, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
            assert_eq!(o.retention_usage(), held);
            assert_eq!(o.generation(), generation);
            assert!(!o.is_faulted());
        }
    }
}
