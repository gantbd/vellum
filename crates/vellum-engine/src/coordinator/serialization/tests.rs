use super::*;
pub(in crate::coordinator) fn limits() -> SourceSerializationLimitsV1 {
    SourceSerializationLimitsV1 {
        max_dependencies: 8,
        max_label_bytes: 128,
        max_request_bytes: 4096,
        max_plan_bytes: 4096,
        max_work: 4096,
    }
}
pub(in crate::coordinator) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "engine".into(),
        engine: "local".into(),
        recipient: "author".into(),
        purpose: "edit".into(),
        destination: "client".into(),
        profile: "v1".into(),
        continuity: "session".into(),
    }
}
pub(in crate::coordinator) fn facts<'a>(
    c: &'a SettingsExecutionContextV1,
    d: &'a DocumentId,
) -> TrustedSerializationFactsV1<'a> {
    TrustedSerializationFactsV1 {
        context: c,
        document: d,
        profile: limits(),
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        companions_ready: Some(true),
        settings_writable: Some(true),
        resources_ready: Some(true),
        write_allowed: Some(true),
        required_reads: &[],
    }
}
pub(in crate::coordinator) fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
pub(in crate::coordinator) fn request<'a>(
    d: &'a DocumentId,
    reads: &'a ReadDependencyRequestV3,
) -> SourceSerializationRequestV1<'a> {
    SourceSerializationRequestV1 {
        document: d,
        expected: 1,
        serialization: SourceSerializationV1 {
            profile: SourceSerializationProfileV1::StrictUtf8OneBom,
            bom: true,
        },
        dependencies: reads,
    }
}
pub(in crate::coordinator) fn setup() -> (SourceCoordinator, Recorder, DocumentVersion) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 2048,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 4096,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_source_serialization("engine", "local", limits(), &mut r)
        .unwrap();
    let d = o.create(b"A", &mut r).unwrap();
    (o, r, d)
}
#[test]
fn completed_decode_is_not_publication_evidence_when_a_later_owner_counter_refuses() {
    let (mut o, _, _) = setup();
    let mut r = Recorder::try_with_capacity(16).unwrap();
    o.generation = u64::MAX;
    let issued = o.documents.len();
    assert_eq!(
        o.create(b"private", &mut r),
        Err(CoordinatorError::VersionExhausted)
    );
    assert_eq!(o.documents.len(), issued);
    assert_eq!(o.generation, u64::MAX);
    assert!(!o.is_faulted());
    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
    assert!(
        events.iter().any(
            |e| e.boundary == Boundary::SourceDecode && e.disposition == Disposition::Completed
        )
    );
    assert!(
        events
            .iter()
            .any(|e| e.boundary == Boundary::Lifecycle && e.disposition == Disposition::Refused)
    );
}
#[test]
fn identity_needs_no_unused_metadata_or_publication_counter_but_changes_do() {
    for counter in 0..2 {
        let (mut o, mut r, d) = setup();
        let c = context();
        let reads = reads();
        let f = facts(&c, &d.document);
        if counter == 0 {
            o.documents[d.document.slot]
                .as_mut()
                .unwrap()
                .serialization
                .generation = MAX_SAFE;
        } else {
            o.generation = u64::MAX;
        }
        let state = o.source_serialization(&d.document, &mut r).unwrap();
        let mut req = request(&d.document, &reads);
        req.expected = state.generation;
        req.serialization = state.serialization;
        let g = o.generation;
        let p = o
            .prepare_source_serialization(&req, &c, &f, &mut r)
            .unwrap();
        assert_eq!(
            o.commit_source_serialization(p, &c, &f, &mut r),
            Ok(SourceSerializationUpdateV1 { changed: false })
        );
        assert_eq!(o.generation, g);
        req.serialization.bom = true;
        assert!(matches!(
            o.prepare_source_serialization(&req, &c, &f, &mut r),
            Err(Error::VersionExhausted)
        ));
        assert!(!o.is_faulted());
        assert_eq!(o.source_serialization(&d.document, &mut r).unwrap(), state);
    }
}
#[test]
fn private_plan_seal_read_count_and_same_generation_state_drift_fault_before_install() {
    for drift in 0..4 {
        let (mut o, mut r, d) = setup();
        let c = context();
        let reads = reads();
        let req = request(&d.document, &reads);
        let f = facts(&c, &d.document);
        let mut p = o
            .prepare_source_serialization(&req, &c, &f, &mut r)
            .unwrap();
        match drift {
            0 => p.read_count += 1,
            1 => p.seal = o.capture_source_serialization(&req, &c, &f).unwrap(),
            2 => p.before.serialization.bom = true,
            _ => {
                o.documents[d.document.slot]
                    .as_mut()
                    .unwrap()
                    .serialization
                    .generation = 0
            }
        }
        let g = o.generation;
        assert_eq!(
            o.commit_source_serialization(p, &c, &f, &mut r),
            Err(Error::Faulted)
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation, g);
        assert!(
            !o.documents[d.document.slot]
                .as_ref()
                .unwrap()
                .serialization
                .serialization
                .bom
        );
    }
}
#[test]
fn independent_request_and_complete_plan_bounds_are_exact() {
    let (o, _, d) = setup();
    let c = context();
    let reads = reads();
    let capture = o
        .capture_source_serialization(&request(&d.document, &reads), &c, &facts(&c, &d.document))
        .unwrap();
    let bytes = capture.bytes;
    let mut r = Recorder::disabled();
    let full = o
        .prepare_serialization_capture(capture, &mut r)
        .unwrap()
        .retained_bytes()
        .unwrap();
    assert!(full > bytes);
    for plan in [false, true] {
        for exact in [false, true] {
            let (mut o, mut r, d) = setup();
            let bound = if plan { full } else { bytes } - usize::from(!exact);
            let mut profile = limits();
            if plan {
                profile.max_plan_bytes = bound;
            } else {
                profile.max_request_bytes = bound;
            }
            o.serialization_profile.as_mut().unwrap().limits = profile;
            let mut f = facts(&c, &d.document);
            f.profile = profile;
            let p = o.prepare_source_serialization(&request(&d.document, &reads), &c, &f, &mut r);
            assert_eq!(p.is_ok(), exact);
            if !exact {
                assert!(matches!(p, Err(Error::Limit)));
            }
            assert!(!o.is_faulted());
        }
    }
}
