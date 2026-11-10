#[path = "support/scratch_invocation_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use vellum_engine::attachment::Affinity;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::selection::{Selection, SelectionBudget};
type V<'a, 'h> = NativeDocumentLifecycleEnvelopeValueV1<'a, 'h>;
fn limits() -> NativeDocumentLifecycleArgumentLimitsV1 {
    NativeDocumentLifecycleArgumentLimitsV1 {
        max_owned_bytes: 8192,
        max_source_bytes: 4096,
        max_selections: 8,
    }
}
fn scratch_envelope<'h, T>(
    id: &'h InvocationId,
    source: &[u8],
    f: impl FnOnce(V<'_, 'h>) -> T,
) -> T {
    let bytes: Vec<_> = source.iter().map(|b| D::Number(f64::from(*b))).collect();
    let serialization = [
        ("profile", D::Text("strict-utf8-one-bom/1")),
        ("bom", D::Boolean(false)),
    ];
    let body = [
        ("contract", V::Data(D::Text("document-lifecycle/1"))),
        ("operation", V::Data(D::Text("create-scratch"))),
        ("source", V::Data(D::Array(&bytes))),
        ("serialization", V::Data(D::Object(&serialization))),
        ("insertionLineBreak", V::Data(D::Text("LF"))),
    ];
    envelope(id, V::Object(&body), f)
}
fn envelope<'a, 'h, T>(id: &'h InvocationId, body: V<'a, 'h>, f: impl FnOnce(V<'_, 'h>) -> T) -> T {
    let rows = [
        ("contract", V::Data(D::Text("lifecycle-invocation/1"))),
        ("operation", V::Data(D::Text("change-document-lifecycle"))),
        ("invocation", V::Invocation(id)),
        ("profile", V::Data(D::Text("native-document-lifecycle/1"))),
        ("request", body),
    ];
    f(V::Object(&rows))
}
#[test]
fn native_lifecycle_scratch_arguments_own_exact_bytes_and_delegate_to_the_original_owner() {
    let (mut o, mut r) = fixture::owner();
    let c = fixture::context();
    let facts = fixture::facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    let mut bytes = "\u{feff}English\r\nDeutsch\n```mermaid\ngraph LR\n A-->B\n```"
        .as_bytes()
        .to_vec();
    let original = bytes.clone();
    let n = scratch_envelope(&id, &bytes, |v| {
        normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
    })
    .unwrap();
    bytes.fill(b'!');
    assert_eq!(n.source_bytes(), Some(original.as_slice()));
    assert_eq!(n.invocation(), &id);
    assert_eq!(n.operation(), "create-scratch");
    o.submit_native_document_lifecycle_arguments_v1(
        &n,
        NativeDocumentLifecycleAdmissionV2::Existing(NativeDocumentLifecycleAdmissionV1::Scratch(
            &fixture::admission(&facts),
        )),
        &mut r,
    )
    .unwrap();
    o.run_native_document_lifecycle_v2(
        &id,
        NativeDocumentLifecycleServiceV2::Existing(NativeDocumentLifecycleServiceV1::Scratch(
            &facts,
        )),
        &mut r,
    )
    .unwrap();
    let result = o
        .scratch_result_v1(&id, &fixture::release(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.read(&result.document, &mut r)
            .unwrap()
            .source
            .text()
            .as_bytes(),
        original
    );
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
#[test]
fn native_lifecycle_live_argument_forms_preserve_native_roles_owned_selections_and_one_service() {
    for kind in 0..3 {
        let (mut o, mut r) = fixture::owner();
        let c = fixture::context();
        let f = live_facts(&c);
        let a = live_admission(&f);
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
            &mut r,
        )
        .unwrap();
        o.configure_workspaces(
            WorkspaceLimits {
                max_live: 4,
                max_issued: 4,
                max_memberships: 8,
                max_work: 4096,
            },
            &mut r,
        )
        .unwrap();
        let d = o.create(b"abc", &mut r).unwrap();
        let life = o.lifecycle_state(&d.document, &mut r).unwrap();
        let view = o.create_logical_view(&mut r).unwrap();
        let mut stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        let mut selections = vec![Selection {
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
        let w = o.create_workspace(&mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
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
                ("workspace", V::Workspace(&w.workspace)),
                ("expectedWorkspace", V::WorkspaceContext(&w)),
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
        let n = envelope(&id, V::Object(&body), |v| {
            normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
        })
        .unwrap();
        if kind == 0 {
            let exact = n.owned_bytes();
            let mut l = limits();
            l.max_owned_bytes = exact - 1;
            assert_eq!(
                envelope(&id, V::Object(&body), |v| {
                    normalize_native_document_lifecycle_arguments_v1(v, l, &mut r)
                })
                .err(),
                Some(InvocationError::Limit)
            );
            l = limits();
            l.max_selections = 0;
            assert_eq!(
                envelope(&id, V::Object(&body), |v| {
                    normalize_native_document_lifecycle_arguments_v1(v, l, &mut r)
                })
                .err(),
                Some(InvocationError::Limit)
            );
        }
        drop(body);
        selections.clear();
        if let LiveRelationOperationV1::Attach { selections, .. } =
            &n.live_relation().unwrap().operation
        {
            assert_eq!(selections.len(), 1);
        }
        let generation = o.generation();
        o.submit_native_document_lifecycle_arguments_v1(
            &n,
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
        assert_eq!(o.generation(), generation + 1);
        let native = TrustedLiveRelationResultV1 {
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
        assert_eq!(
            o.live_relation_result_v1(&id, &native, &mut r)
                .unwrap()
                .document,
            d.document
        );
    }
}
fn coverage() -> TrustedDisposalCoverageV1 {
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
fn disposal_body<'h, T>(
    id: &'h InvocationId,
    d: &'h DocumentVersion,
    life: LifecycleState,
    binding: Option<u64>,
    annotations: u64,
    f: impl FnOnce(V<'_, 'h>) -> T,
) -> T {
    let body = [
        ("contract", V::Data(D::Text("document-lifecycle/1"))),
        ("operation", V::Data(D::Text("dispose"))),
        ("document", V::Document(&d.document)),
        ("expectedLifecycle", V::Lifecycle(life)),
        ("expectedSource", V::SourceVersion(d)),
        ("expectedAnnotations", V::Generation(annotations)),
        ("expectedSerialization", V::Generation(1)),
        ("expectedBinding", V::BindingGeneration(binding)),
        ("sourceDisposition", V::Data(D::Text("retain"))),
        ("annotationDisposition", V::Data(D::Text("discard"))),
    ];
    envelope(id, V::Object(&body), f)
}
#[test]
fn native_lifecycle_disposal_arguments_preserve_binding_presence_full_counters_and_original_replay()
{
    let (mut o, mut r) = fixture::owner();
    let c = fixture::context();
    let d = o.create(b"original", &mut r).unwrap();
    let life = o.lifecycle_state(&d.document, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    for binding in [None, Some(0), Some(u64::MAX)] {
        let n = disposal_body(&id, &d, life, binding, u64::MAX, |v| {
            normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
        })
        .unwrap();
        let q = n.explicit_disposal().unwrap();
        assert_eq!(q.binding_generation, binding);
        assert_eq!(q.annotation_generation, u64::MAX);
    }
    let n = disposal_body(&id, &d, life, None, 0, |v| {
        normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
    })
    .unwrap();
    let f = TrustedRetainedDisposalFactsV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        disposal_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    };
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
    let g = coverage();
    o.submit_native_document_lifecycle_arguments_v1(
        &n,
        NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
            native: &a,
            coverage: &g,
        },
        &mut r,
    )
    .unwrap();
    o.run_native_document_lifecycle_v2(
        &id,
        NativeDocumentLifecycleServiceV2::ExplicitDisposal {
            native: &f,
            coverage: &g,
        },
        &mut r,
    )
    .unwrap();
    let generation = o.generation();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.submit_native_document_lifecycle_arguments_v1(
            &n,
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &g
            },
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
}
#[test]
fn native_lifecycle_exact_headers_and_roles_refuse_unknown_private_or_coerced_arguments() {
    let (mut o, mut r) = fixture::owner();
    let id = o.issue_invocation(&mut r).unwrap();
    scratch_envelope(&id, b"valid", |value| {
        let V::Object(original) = value else {
            unreachable!()
        };
        for bad in 0..7 {
            let mut rows = original.to_vec();
            match bad {
                0 => rows.push(("unknown", V::Data(D::Null))),
                1 => rows.push(original[0]),
                2 => {
                    rows.remove(3);
                }
                3 => rows[2].1 = V::Data(D::Text("not-a-native-ticket")),
                4 => rows[0].1 = V::Data(D::Text("lifecycle-invocation/2")),
                5 => rows[3].1 = V::Data(D::Text("other")),
                _ => rows[1].1 = V::Data(D::Text("release-provenance")),
            }
            assert_eq!(
                normalize_native_document_lifecycle_arguments_v1(
                    V::Object(&rows),
                    limits(),
                    &mut r
                )
                .err(),
                Some(InvocationError::MalformedIntent)
            );
        }
        let V::Object(body) = original[4].1 else {
            unreachable!()
        };
        for bad in 0..3 {
            let mut rows = body.to_vec();
            match bad {
                0 => rows.push(("extra", V::Data(D::Null))),
                1 => rows[1].1 = V::Data(D::Text("release-binding")),
                _ => rows[4].1 = V::Data(D::Text("automatic")),
            };
            assert_eq!(
                envelope(&id, V::Object(&rows), |v| {
                    normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
                })
                .err(),
                Some(InvocationError::MalformedIntent)
            );
        }
    });
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.lifecycle_usage().live, 0);
}
#[test]
fn native_lifecycle_source_bounds_are_exact_and_shape_is_not_semantic_validity_or_authority() {
    let (mut o, mut r) = fixture::owner();
    let c = fixture::context();
    let id = o.issue_invocation(&mut r).unwrap();
    let bytes = b"abc";
    let n = scratch_envelope(&id, bytes, |v| {
        normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
    })
    .unwrap();
    let exact = n.owned_bytes();
    for short in [false, true] {
        let mut l = limits();
        l.max_owned_bytes = exact - usize::from(short);
        l.max_source_bytes = bytes.len();
        let result = scratch_envelope(&id, bytes, |v| {
            normalize_native_document_lifecycle_arguments_v1(v, l, &mut r)
        });
        assert_eq!(result.is_ok(), !short);
    }
    let mut l = limits();
    l.max_source_bytes = 2;
    assert_eq!(
        scratch_envelope(&id, bytes, |v| {
            normalize_native_document_lifecycle_arguments_v1(v, l, &mut r)
        })
        .err(),
        Some(InvocationError::Limit)
    );
    let mut denied = fixture::facts(&c);
    denied.authorized = None;
    let held = o.retention_usage();
    assert_eq!(
        o.submit_native_document_lifecycle_arguments_v1(
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&fixture::admission(&denied))
            ),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.retention_usage(), held);
    let malformed = scratch_envelope(&id, &[0xff], |v| {
        normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
    })
    .unwrap();
    let f = fixture::facts(&c);
    assert_eq!(
        o.submit_native_document_lifecycle_arguments_v1(
            &malformed,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&fixture::admission(&f))
            ),
            &mut r
        ),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(o.retention_usage(), held);
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert!(!o.is_faulted());
}

#[test]
fn native_lifecycle_redundant_roles_must_agree_and_scalars_are_not_coerced() {
    let (mut o, mut r) = fixture::owner();
    let d = o.create(b"one", &mut r).unwrap();
    let other = o.create(b"two", &mut r).unwrap();
    let life = o.lifecycle_state(&d.document, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    disposal_body(&id, &d, life, None, 0, |v| {
        let V::Object(header) = v else { unreachable!() };
        let V::Object(body) = header[4].1 else {
            unreachable!()
        };
        for bad in 0..5 {
            let mut rows = body.to_vec();
            match bad {
                0 => rows[4].1 = V::SourceVersion(&other),
                1 => rows[7].1 = V::Data(D::Number(0.0)),
                2 => rows[5].1 = V::Data(D::Number(0.0)),
                3 => rows[8].1 = V::Data(D::Text("delete")),
                _ => rows[3].1 = V::Data(D::Number(0.0)),
            };
            assert_eq!(
                envelope(&id, V::Object(&rows), |v| {
                    normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
                })
                .err(),
                Some(InvocationError::MalformedIntent)
            );
        }
    });
    o.configure_workspaces(
        WorkspaceLimits {
            max_live: 4,
            max_issued: 4,
            max_memberships: 8,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let w = o.create_workspace(&mut r).unwrap();
    let wrong = o.create_workspace(&mut r).unwrap();
    let body = [
        ("contract", V::Data(D::Text("document-lifecycle/1"))),
        ("operation", V::Data(D::Text("membership"))),
        ("document", V::Document(&d.document)),
        ("expectedLifecycle", V::Lifecycle(life)),
        ("workspace", V::Workspace(&w.workspace)),
        ("expectedWorkspace", V::WorkspaceContext(&wrong)),
        ("present", V::Data(D::Boolean(true))),
    ];
    assert_eq!(
        envelope(&id, V::Object(&body), |v| {
            normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
        })
        .err(),
        Some(InvocationError::MalformedIntent)
    );
    scratch_envelope(&id, b"x", |v| {
        let V::Object(header) = v else { unreachable!() };
        let V::Object(body) = header[4].1 else {
            unreachable!()
        };
        for byte in [
            D::Number(f64::NAN),
            D::Number(f64::INFINITY),
            D::Number(-1.0),
            D::Number(256.0),
            D::Number(0.5),
            D::Null,
            D::Text("0"),
        ] {
            let bytes = [byte];
            let mut rows = body.to_vec();
            rows[2].1 = V::Data(D::Array(&bytes));
            assert_eq!(
                envelope(&id, V::Object(&rows), |v| {
                    normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
                })
                .err(),
                Some(InvocationError::MalformedIntent)
            );
        }
    });
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn native_lifecycle_argument_capture_is_optional_bounded_and_not_a_shared_parent_claim() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = fixture::owner();
        let c = fixture::context();
        let f = fixture::facts(&c);
        let id = o.issue_invocation(&mut setup).unwrap();
        let held = o.retention_usage();
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
        let n = scratch_envelope(&id, b"PRIVATE", |v| {
            normalize_native_document_lifecycle_arguments_v1(v, limits(), &mut r)
        })
        .unwrap();
        assert_eq!(o.retention_usage(), held);
        assert!(!format!("{n:?}").contains("PRIVATE"));
        o.submit_native_document_lifecycle_arguments_v1(
            &n,
            NativeDocumentLifecycleAdmissionV2::Existing(
                NativeDocumentLifecycleAdmissionV1::Scratch(&fixture::admission(&f)),
            ),
            &mut r,
        )
        .unwrap();
        assert!(o.retention_usage().reserved > held.reserved);
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::Existing(NativeDocumentLifecycleServiceV1::Scratch(
                &f,
            )),
            &mut r,
        )
        .unwrap();
        let mut counts = [0; 3];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::DocumentLifecycleArgumentNormalization => Some(0),
                Boundary::DocumentLifecycleFacadeAdmission => Some(1),
                Boundary::ScratchInvocationAdmission => Some(2),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{e:?}").contains("PRIVATE"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [1, 1, 1]
            } else {
                [0, 0, 0]
            }
        );
        o.retire_scratch_v1(&id, &fixture::control(&c), &mut setup)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
