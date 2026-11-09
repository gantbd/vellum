//! Consumer-level completion of the already installed lifecycle routes.
use super::*;
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::selection::{Selection, SelectionBudget};
type V<'a, 'h> = NativeDocumentLifecycleEnvelopeValueV1<'a, 'h>;
fn normalize<'h>(
    id: &'h InvocationId,
    body: V<'_, 'h>,
    r: &mut Recorder,
) -> NormalizedDocumentLifecycleArgumentsV1<'h> {
    let outer = [
        ("contract", V::Data(D::Text("lifecycle-invocation/1"))),
        ("operation", V::Data(D::Text("change-document-lifecycle"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1)),
        ),
        ("request", body),
    ];
    normalize_native_document_lifecycle_arguments_v1(
        V::Object(&outer),
        NativeDocumentLifecycleArgumentLimitsV1 {
            max_owned_bytes: 8192,
            max_source_bytes: 4096,
            max_selections: 8,
        },
        r,
    )
    .unwrap()
}
pub(super) fn configure(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 4,
            max_issued_views: 4,
            max_total_selections: 16,
            normalization: SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 4096,
            },
            max_mapping_steps: 4096,
        },
        r,
    )
    .unwrap();
    o.configure_workspaces(
        WorkspaceLimits {
            max_live: 4,
            max_issued: 4,
            max_memberships: 8,
            max_work: 4096,
        },
        r,
    )
    .unwrap();
}
fn bind(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) {
    observation::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    }
    .bind(o, id, c, r);
}
fn open(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> InvocationObservationContextV1 {
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV6::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    o.open_invocation_observation_with_receipts_v6(
        id,
        c,
        &observation::policy(c, 7),
        l,
        InvocationReceiptAdmissionV1 {
            observation: &observation::facts(c),
            capabilities: InvocationReceiptCapabilitiesV1 {
                lifecycle: true,
                ..Default::default()
            },
            operation_receipt_admission_allowed: Some(true),
        },
        r,
    )
    .unwrap()
}
fn receipt<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    q: InvocationReceiptRequestV6<'_>,
    allowed: bool,
    r: &mut Recorder,
) -> Result<InvocationReceiptV6<'a>, InvocationError> {
    let current = TrustedInvocationObservationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    };
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &current,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    o.query_invocation_receipt_v6(
        v,
        q,
        &observation::policy(c, 7),
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: Some(allowed),
        },
        r,
    )
}
fn live_facts(c: &SettingsExecutionContextV1) -> TrustedLiveRelationFactsV1<'_> {
    TrustedLiveRelationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        view_writable: Some(true),
        source_readable: Some(true),
        membership_writable: Some(true),
        workspace_readable: Some(true),
    }
}
fn live_admission<'a>(f: &'a TrustedLiveRelationFactsV1<'a>) -> LiveRelationAdmissionV1<'a> {
    LiveRelationAdmissionV1 {
        facts: f,
        limits: LiveRelationLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_identifiers_allowed: Some(true),
    }
}
fn live_release(c: &SettingsExecutionContextV1) -> TrustedLiveRelationResultV1<'_> {
    TrustedLiveRelationResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_readable: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
    }
}
#[test]
fn registered_attach_detach_and_membership_complete_through_f06_and_once_only_retirement() {
    for kind in 0..3 {
        let (mut o, mut r) = owner();
        configure(&mut o, &mut r);
        let c = scratch::context();
        let f = live_facts(&c);
        let d = o.create(b"English\nDeutsch\n", &mut r).unwrap();
        let view = o.create_logical_view(&mut r).unwrap();
        let mut stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        let selections = [Selection {
            anchor: BytePosition(1),
            head: BytePosition(1),
            affinity: Affinity::After,
        }];
        if kind == 1 {
            stamp = o
                .attach_logical_view(&view, stamp, &d, &selections, 0, &mut r)
                .unwrap()
                .stamp;
        }
        let workspace = o.create_workspace(&mut r).unwrap();
        let life = o.lifecycle_state(&d.document, &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        bind(&mut o, &id, &c, &mut r);
        let mut body = vec![
            ("contract", V::Data(D::Text("document-lifecycle/1"))),
            (
                "operation",
                V::Data(D::Text(["attach-view", "detach-view", "membership"][kind])),
            ),
            ("document", V::Document(&d.document)),
            ("expectedLifecycle", V::Lifecycle(life)),
        ];
        if kind == 2 {
            body.extend([
                ("workspace", V::Workspace(&workspace.workspace)),
                ("expectedWorkspace", V::WorkspaceContext(&workspace)),
                ("present", V::Data(D::Boolean(true))),
            ]);
        } else {
            body.extend([
                ("view", V::View(&view)),
                ("expectedAttachment", V::ViewStamp(stamp)),
            ]);
            if kind == 0 {
                body.extend([
                    ("sourceRevision", V::SourceVersion(&d)),
                    ("selections", V::Selections(&selections)),
                    ("primary", V::Index(0)),
                ]);
            }
        }
        let n = normalize(&id, V::Object(&body), &mut r);
        let (dc, e, v) = selectors(&mut o, &mut r);
        let p = prepare(&mut o, &mut r);
        let before = o.generation();
        o.submit_discovered_native_document_lifecycle_v1(
            NativeDiscoveredLifecycleRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[if kind == 2 { 2 } else { 1 }].reference,
                facts: &facts(),
            },
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::LiveRelation(&live_admission(&f)),
            ),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), before);
        let mut observer = open(&mut o, &id, &c, &mut r);
        let release = live_release(&c);
        let request = InvocationReceiptRequestV6::Existing(
            InvocationReceiptRequestV5::LiveRelation(&release),
        );
        assert_eq!(
            receipt(&o, &mut observer, &c, request, true, &mut r).err(),
            Some(InvocationError::Unavailable)
        );
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::Existing(
                NativeDocumentLifecycleServiceV1::LiveRelation(&f),
            ),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), before + 1);
        let expected = o.live_relation_result_v1(&id, &release, &mut r).unwrap();
        assert!(expected.changed);
        assert_eq!(expected.document, d.document);
        let applied = o.generation();
        let p = prepare(&mut o, &mut r);
        o.submit_discovered_native_document_lifecycle_v1(
            NativeDiscoveredLifecycleRouteV1 {
                context: &dc,
                scope: &scope4(),
                cut: &p,
                entry: &e,
                variant: &v[if kind == 2 { 2 } else { 1 }].reference,
                facts: &facts(),
            },
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::LiveRelation(&live_admission(&f)),
            ),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), applied);
        for allowed in [false, true, true] {
            let value = receipt(&o, &mut observer, &c, request, allowed, &mut r);
            if allowed {
                let InvocationReceiptV6::Existing(InvocationReceiptV5::LiveRelation(result)) =
                    value.unwrap()
                else {
                    panic!()
                };
                assert_eq!(result, expected);
            } else {
                assert_eq!(value.err(), Some(InvocationError::Unavailable));
            }
        }
        let before = o.retention_usage().reserved;
        o.retire_live_relation_v1(&id, &scratch::control(&c), &mut r)
            .unwrap();
        let after = o.retention_usage().reserved;
        assert!(after < before);
        assert_eq!(
            o.retire_live_relation_v1(&id, &scratch::control(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(o.retention_usage().reserved, after);
        assert_eq!(
            receipt(&o, &mut observer, &c, request, true, &mut r).err(),
            Some(InvocationError::OutcomeUnavailable)
        );
        assert_eq!(o.generation(), applied);
    }
}
fn disposal_facts(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalFactsV1<'_> {
    TrustedRetainedDisposalFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        disposal_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
fn disposal_coverage() -> TrustedDisposalCoverageV1 {
    TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: Some(true),
        source_discard_allowed: Some(true),
        annotation_retention_ready: Some(true),
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
    }
}
#[test]
fn registered_explicit_disposal_detaches_dependents_and_releases_only_its_historical_f06_result() {
    let (mut o, mut r) = owner();
    configure(&mut o, &mut r);
    let c = scratch::context();
    let f = disposal_facts(&c);
    let gates = disposal_coverage();
    let d = o.create(b"prose and code", &mut r).unwrap();
    let view = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
    let selections = [Selection {
        anchor: BytePosition(1),
        head: BytePosition(1),
        affinity: Affinity::After,
    }];
    o.attach_logical_view(&view, stamp, &d, &selections, 0, &mut r)
        .unwrap();
    let workspace = o.create_workspace(&mut r).unwrap();
    o.set_workspace_membership(&workspace, &d.document, true, &mut r)
        .unwrap();
    let life = o.lifecycle_state(&d.document, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    bind(&mut o, &id, &c, &mut r);
    let body = [
        ("contract", V::Data(D::Text("document-lifecycle/1"))),
        ("operation", V::Data(D::Text("dispose"))),
        ("document", V::Document(&d.document)),
        ("expectedLifecycle", V::Lifecycle(life)),
        ("expectedSource", V::SourceVersion(&d)),
        ("expectedAnnotations", V::Generation(0)),
        ("expectedSerialization", V::Generation(1)),
        ("expectedBinding", V::BindingGeneration(None)),
        ("sourceDisposition", V::Data(D::Text("retain"))),
        ("annotationDisposition", V::Data(D::Text("discard"))),
    ];
    let n = normalize(&id, V::Object(&body), &mut r);
    let (dc, e, v) = selectors(&mut o, &mut r);
    let p = prepare(&mut o, &mut r);
    let a = RetainedDisposalAdmissionV1 {
        facts: &f,
        limits: RetainedDisposalLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    };
    o.submit_discovered_native_document_lifecycle_v1(
        NativeDiscoveredLifecycleRouteV1 {
            context: &dc,
            scope: &scope4(),
            cut: &p,
            entry: &e,
            variant: &v[3].reference,
            facts: &facts(),
        },
        &n,
        NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
            native: &a,
            coverage: &gates,
        },
        &mut r,
    )
    .unwrap();
    let mut observer = open(&mut o, &id, &c, &mut r);
    let release = TrustedRetainedDisposalResultV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_allowed: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    let request = InvocationReceiptRequestV6::Existing(InvocationReceiptRequestV5::Existing(
        InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::Existing(
            InvocationReceiptRequestV2::RetainedDisposal(&release),
        )),
    ));
    assert_eq!(
        receipt(&o, &mut observer, &c, request, true, &mut r).err(),
        Some(InvocationError::Unavailable)
    );
    o.run_native_document_lifecycle_v2(
        &id,
        NativeDocumentLifecycleServiceV2::ExplicitDisposal {
            native: &f,
            coverage: &gates,
        },
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.lifecycle_state(&d.document, &mut r).unwrap().phase,
        LifecyclePhase::DisposedRetained
    );
    assert_eq!(o.logical_view(&view, &mut r).unwrap().source, None);
    assert!(
        o.workspace(&workspace.workspace, &mut r)
            .unwrap()
            .members
            .is_empty()
    );
    let expected = o
        .retained_disposal_result_v1(&id, &release, &mut r)
        .unwrap();
    let applied = o.generation();
    let p = prepare(&mut o, &mut r);
    o.submit_discovered_native_document_lifecycle_v1(
        NativeDiscoveredLifecycleRouteV1 {
            context: &dc,
            scope: &scope4(),
            cut: &p,
            entry: &e,
            variant: &v[3].reference,
            facts: &facts(),
        },
        &n,
        NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
            native: &a,
            coverage: &gates,
        },
        &mut r,
    )
    .unwrap();
    assert_eq!(o.generation(), applied);
    for allowed in [false, true, true] {
        let value = receipt(&o, &mut observer, &c, request, allowed, &mut r);
        if allowed {
            let InvocationReceiptV6::Existing(InvocationReceiptV5::Existing(
                InvocationReceiptV4::Existing(InvocationReceiptV3::Existing(
                    InvocationReceiptV2::RetainedDisposal(result),
                )),
            )) = value.unwrap()
            else {
                panic!()
            };
            assert_eq!(result, expected);
        } else {
            assert_eq!(value.err(), Some(InvocationError::Unavailable));
        }
    }
    let before = o.retention_usage().reserved;
    o.retire_retained_disposal_v1(&id, &scratch::control(&c), &mut r)
        .unwrap();
    let after = o.retention_usage().reserved;
    assert!(after < before);
    assert_eq!(
        o.retire_retained_disposal_v1(&id, &scratch::control(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.retention_usage().reserved, after);
    assert_eq!(
        receipt(&o, &mut observer, &c, request, true, &mut r).err(),
        Some(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(o.generation(), applied);
}
