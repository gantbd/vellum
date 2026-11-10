use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;

#[test]
fn malformed_shutdown_requests_do_not_decode_opaque_ids_from_data() {
    use ShutdownDispositionEnvelopeValueV1 as V;
    let mut r = Recorder::disabled();
    assert!(matches!(
        normalize_shutdown_disposition_v1(&V::Data(D::Text("document")), &mut r),
        Err(ShutdownDispositionIngressErrorV1::InvalidRequest)
    ));
}

#[test]
fn closed_shutdown_headers_preserve_native_roles_and_never_grant_disposition() {
    use ShutdownDispositionEnvelopeValueV1 as V;
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 2,
        max_participants: 2,
        max_staged_bytes: 128,
        source: vellum_engine::source::Limits {
            max_source_bytes: 64,
            max_edits: 4,
            max_edit_payload_bytes: 64,
        },
    });
    let mut r = Recorder::disabled();
    let d = o.create(b"PRIVATE_PROSE", &mut r).unwrap();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "native".into(),
        recipient: "host".into(),
        purpose: "shutdown".into(),
        destination: "host".into(),
        profile: "native".into(),
        continuity: "local".into(),
    };
    let control = TrustedShutdownFenceFactsV1 {
        context: &c,
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        owner_control_allowed: Some(true),
        admission_fence_allowed: Some(true),
        resources_ready: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 65536,
                max_capture_bytes: 131072,
            },
            &control,
            &mut r,
        )
        .unwrap();
    let fence = o
        .install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let coverage = TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: None,
        source_discard_allowed: Some(true),
        annotation_retention_ready: None,
        annotation_discard_allowed: Some(true),
        retention_complete: Some(true),
        effects_retained: Some(true),
        history_retained: Some(true),
        snapshots_retained: Some(true),
        providers_retained: Some(true),
        views_detachable: Some(true),
        memberships_detachable: Some(true),
        observer_scopes_fenced: Some(true),
        binding_coverage_complete: Some(true),
    };
    let before = (o.generation(), o.retention_usage());
    for typed in [false, true] {
        for mutation in 0..12 {
            let mut args = vec![
                (
                    "contract",
                    V::Data(D::Text(SHUTDOWN_DISPOSITION_PROFILE_V1)),
                ),
                ("operation", V::Data(D::Text("shutdown-disposition"))),
                ("fence", V::Fence(&fence)),
                ("document", V::Document(&d.document)),
            ];
            match mutation {
                1 => args[0].1 = V::Data(D::Text("shutdown-disposition/2")),
                2 => args[1].1 = V::Data(D::Text("discard")),
                3 => args[2].1 = V::Data(D::Text("forged-fence")),
                4 => args[3].1 = V::Data(D::Number(0.0)),
                5 => args[2].1 = V::Document(&d.document),
                6 => args[3].0 = "fence",
                7 => {
                    args.pop();
                }
                8 => args.push(("authority", V::Data(D::Boolean(true)))),
                _ => {}
            }
            let mut header = vec![
                if typed {
                    ("family", V::Data(D::Number(17.0)))
                } else {
                    ("name", V::Data(D::Text("control-execution")))
                },
                ("variant", V::Data(D::Text("shutdown-disposition"))),
                ("version", V::Data(D::Number(1.0))),
                ("arguments", V::Object(&args)),
            ];
            match mutation {
                9 => header[2].1 = V::Data(D::Number(2.0)),
                10 => header[1].1 = V::Data(D::Text("request-cancellation")),
                11 => header[0].1 = V::Data(D::Text("other-family")),
                _ => {}
            }
            let outer = [
                (
                    "route",
                    V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
                ),
                ("request", V::Object(&header)),
            ];
            let envelope = V::Object(&outer);
            if mutation == 0 {
                let n = normalize_shutdown_disposition_v1(&envelope, &mut r).unwrap();
                assert_eq!(n.fence(), &fence);
                assert_eq!(n.document(), &d.document);
                assert_eq!(
                    o.request_decoded_shutdown_disposition_v1(&envelope, &auth, &coverage, &mut r),
                    Err(ShutdownDispositionIngressErrorV1::Execution(
                        CoordinatorError::Unavailable
                    )),
                    "a fence and complete authority do not invent an accepted consent plan"
                );
            } else {
                assert_eq!(
                    normalize_shutdown_disposition_v1(&envelope, &mut r),
                    Err(ShutdownDispositionIngressErrorV1::InvalidRequest)
                );
            }
            assert_eq!((o.generation(), o.retention_usage()), before);
            assert_eq!(
                o.lifecycle_state(&d.document, &mut r).unwrap().phase,
                LifecyclePhase::Live
            );
            assert!(!format!("{envelope:?}").contains("PRIVATE_PROSE"));
        }
    }
}
