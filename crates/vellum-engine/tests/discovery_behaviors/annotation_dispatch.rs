use super::observation_support as observation;
use super::*;
use AnnotationAuthoringEnvelopeValueV1 as V;
use vellum_engine::attachment::AttachmentTarget;
fn authority(c: &SettingsExecutionContextV1) -> TrustedAnnotationInvocationV1<'_> {
    TrustedAnnotationInvocationV1 {
        context: c,
        limits: AnnotationInvocationLimitsV1 {
            max_context_bytes: 1024,
            max_capture_bytes: 8192,
            max_plan_bytes: 32768,
            max_construction_bytes: 65536,
        },
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
#[test]
fn installed_annotation_forms_keep_authority_replay_and_f06_results_in_the_original_owner() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 512] {
        for (index, operation) in ["create", "update-body", "reattach", "delete"]
            .into_iter()
            .enumerate()
        {
            let mut f = support::Fixture::with_invocations();
            f.owner
                .configure_semantic_root_budget(1048576, &mut f.recorder)
                .unwrap();
            f.owner
                .configure_retention_budget(67108864, &mut f.recorder)
                .unwrap();
            let mut l = context::profile_limits();
            l.max_page_rows = 5;
            f.owner
                .configure_discovery_v1(&lifecycle::scope4(), l, &facts(), &mut f.recorder)
                .unwrap();
            let target = SourceTarget {
                source: f.doc.clone(),
                target: AttachmentTarget::ContentRange { start: 1, end: 2 },
                basis: TargetBasis::Before,
            };
            let reattach = SourceTarget {
                source: f.doc.clone(),
                target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                basis: TargetBasis::Before,
            };
            let existing = if operation != "create" {
                let p = f
                    .owner
                    .prepare_compound(
                        &[],
                        &[],
                        &[AnnotationMutation::Create {
                            target: target.clone(),
                            body: "old".into(),
                        }],
                        &mut f.recorder,
                    )
                    .unwrap();
                let a = p.created_annotations().next().unwrap().clone();
                f.owner.commit(p, &mut f.recorder).unwrap();
                Some(a)
            } else {
                None
            };
            f.owner
                .set_read_only(&f.doc.document, true, &mut f.recorder)
                .unwrap();
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let mut c = lifecycle::scope4().context;
            c.profile = "annotation-test".into();
            let profile = observation::Profile {
                epoch: 7,
                maximum: AnnotationResultV1::required_bytes(),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    annotation: true,
                    ..Default::default()
                },
            };
            profile.bind(&mut f.owner, &id, &c, &mut f.recorder);
            let reads = ReadDependencyRequestV3 {
                predicates: vec![],
                budget: ReadDependencyBudget {
                    max_predicates: 16,
                    max_work: 256,
                },
            };
            let mut raw = vec![
                ("contract", V::Data(D::Text("annotation-authoring/1"))),
                ("operation", V::Data(D::Text(operation))),
                ("invocation", V::Invocation(&id)),
                ("document", V::Document(&f.doc.document)),
                ("profile", V::Data(D::Text(&c.profile))),
                ("dependencies", V::Dependencies(&reads)),
            ];
            match operation {
                "create" => raw.extend([
                    (
                        "body",
                        V::Data(D::Text("English\nDeutsch\n```mermaid\n```")),
                    ),
                    ("target", V::Target(&target)),
                ]),
                "update-body" => raw.extend([
                    ("annotation", V::Annotation(existing.as_ref().unwrap())),
                    ("expectedContent", V::Version(1)),
                    ("body", V::Data(D::Text("new"))),
                ]),
                "reattach" => raw.extend([
                    ("annotation", V::Annotation(existing.as_ref().unwrap())),
                    ("expectedTarget", V::Version(1)),
                    ("target", V::Target(&reattach)),
                ]),
                _ => raw.extend([
                    ("annotation", V::Annotation(existing.as_ref().unwrap())),
                    ("expectedContent", V::Version(1)),
                    ("expectedTarget", V::Version(1)),
                ]),
            }
            let q = normalize_annotation_authoring_arguments_v1(
                V::Object(&raw),
                AnnotationAuthoringArgumentLimitsV1 {
                    max_body_bytes: 4096,
                    max_profile_bytes: 1024,
                    max_dependencies: 16,
                    max_owned_bytes: 8192,
                },
                &mut f.recorder,
            )
            .unwrap();
            let grant = TrustedAnnotationResultGrantV1 {
                invocation: &id,
                authority: &c.authority,
                target: existing.as_ref().map_or(
                    AnnotationResultGrantTargetV1::CreatedByInvocation(&f.doc.document),
                    AnnotationResultGrantTargetV1::Annotation,
                ),
                capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
            };
            let result = TrustedAnnotationResultAdmissionV1 {
                context: &c,
                publication_epoch: 7,
                publication_allowed: Some(true),
                context_current: Some(true),
                result_admission_allowed: Some(true),
                max_result_bytes: AnnotationResultV1::required_bytes(),
                grant: Some(grant),
            };
            let (dc, e, v) = lifecycle::selectors_for(&mut f.owner, &mut f.recorder, 0);
            let p = lifecycle::prepare(&mut f.owner, &mut f.recorder);
            f.recorder = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let mut lost = facts();
            lost.authorized = Some(false);
            assert_eq!(
                f.owner.submit_discovered_annotation_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &lifecycle::scope4(),
                        cut: &p,
                        entry: &e,
                        variant: &v[(index + 1) % 4].reference,
                        facts: &lost
                    },
                    &q,
                    NativeDiscoveredAnnotationAdmissionV1 {
                        context: &c,
                        facts: &authority(&c),
                        result: Some(&result)
                    },
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Discovery(
                    DiscoveryErrorV1::Unavailable
                ))
            );
            assert!(!f.owner.discovery_context_usage_v1(&dc).unwrap().stopped);
            let mut denied = authority(&c);
            denied.annotation_writable = Some(false);
            assert_eq!(
                f.owner.submit_discovered_annotation_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &lifecycle::scope4(),
                        cut: &p,
                        entry: &e,
                        variant: &v[index].reference,
                        facts: &facts()
                    },
                    &q,
                    NativeDiscoveredAnnotationAdmissionV1 {
                        context: &c,
                        facts: &denied,
                        result: Some(&result)
                    },
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Invocation(
                    InvocationError::Unavailable
                ))
            );
            assert_eq!(
                f.owner.invocation_state(&id, &mut f.recorder),
                Ok(InvocationState::Issued)
            );
            let p = lifecycle::prepare(&mut f.owner, &mut f.recorder);
            f.owner
                .submit_discovered_annotation_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &lifecycle::scope4(),
                        cut: &p,
                        entry: &e,
                        variant: &v[index].reference,
                        facts: &facts(),
                    },
                    &q,
                    NativeDiscoveredAnnotationAdmissionV1 {
                        context: &c,
                        facts: &authority(&c),
                        result: Some(&result),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            let mut observer = profile.open(&mut f.owner, &id, &c, &mut f.recorder);
            f.owner
                .run_scoped_annotation_v1(&id, &c, &authority(&c), &mut f.recorder)
                .unwrap();
            let native = TrustedAnnotationResultReleaseV1 {
                context: &c,
                publication_epoch: 7,
                publication_allowed: Some(true),
                outcome_readable: Some(true),
                result_context_current: Some(true),
                reply_ready: Some(true),
                resources_ready: Some(true),
                max_result_bytes: AnnotationResultV1::required_bytes(),
                grant: Some(grant),
            };
            let InvocationReceiptV1::Annotation(expected) = profile
                .query(
                    &f.owner,
                    &mut observer,
                    InvocationReceiptRequestV1::Annotation(&native),
                    &c,
                    &mut f.recorder,
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(expected.invocation, id);
            assert!(matches!(
                expected.decision,
                AnnotationResultDecisionV1::Applied { .. }
            ));
            let generation = f.owner.generation();
            let issued = f.owner.annotation_usage().issued_lifetime;
            let p = lifecycle::prepare(&mut f.owner, &mut f.recorder);
            f.owner
                .submit_discovered_annotation_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &lifecycle::scope4(),
                        cut: &p,
                        entry: &e,
                        variant: &v[index].reference,
                        facts: &facts(),
                    },
                    &q,
                    NativeDiscoveredAnnotationAdmissionV1 {
                        context: &c,
                        facts: &authority(&c),
                        result: Some(&result),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            let denied = TrustedAnnotationResultReleaseV1 {
                grant: None,
                ..native
            };
            assert_eq!(
                profile
                    .query(
                        &f.owner,
                        &mut observer,
                        InvocationReceiptRequestV1::Annotation(&denied),
                        &c,
                        &mut f.recorder
                    )
                    .err(),
                Some(InvocationError::Unavailable)
            );
            for _ in 0..2 {
                let InvocationReceiptV1::Annotation(actual) = profile
                    .query(
                        &f.owner,
                        &mut observer,
                        InvocationReceiptRequestV1::Annotation(&native),
                        &c,
                        &mut f.recorder,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                assert_eq!(actual, expected);
            }
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.annotation_usage().issued_lifetime, issued);
            assert_eq!(
                f.owner
                    .read(&f.doc.document, &mut f.recorder)
                    .unwrap()
                    .source
                    .text(),
                "text"
            );
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            if capacity == 512 {
                assert!(records.iter().any(|v|v.boundary==Boundary::DiscoveryQuery&&v.service_time.is_some()));
                assert!(
                    records
                        .iter()
                        .any(|v| v.boundary == Boundary::AnnotationInvocationAdmission
                            && v.service_time.is_some())
                );
                assert!(
                    records
                        .iter()
                        .any(|v| v.boundary == Boundary::InvocationReceiptRelease
                            && v.service_time.is_some())
                );
            } else {
                assert!(records.len() <= capacity);
                if capacity == 1 {
                    assert!(f.recorder.dropped() > 0);
                }
            }
            if operation == "create" && capacity == 0 {
                let without = f.owner.issue_invocation(&mut f.recorder).unwrap();
                let mut no_result = raw.clone();
                no_result[2].1 = V::Invocation(&without);
                let no_result = normalize_annotation_authoring_arguments_v1(
                    V::Object(&no_result),
                    q.limits(),
                    &mut f.recorder,
                )
                .unwrap();
                let p = lifecycle::prepare(&mut f.owner, &mut f.recorder);
                f.owner
                    .submit_discovered_annotation_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &dc,
                            scope: &lifecycle::scope4(),
                            cut: &p,
                            entry: &e,
                            variant: &v[0].reference,
                            facts: &facts(),
                        },
                        &no_result,
                        NativeDiscoveredAnnotationAdmissionV1 {
                            context: &c,
                            facts: &authority(&c),
                            result: None,
                        },
                        &mut f.recorder,
                    )
                    .unwrap();
                f.owner
                    .run_scoped_annotation_v1(&without, &c, &authority(&c), &mut f.recorder)
                    .unwrap();
                let allocation = f.owner.annotation_usage().issued_lifetime;
                let generation = f.owner.generation();
                let upgrade = TrustedAnnotationResultAdmissionV1 {
                    grant: Some(TrustedAnnotationResultGrantV1 {
                        invocation: &without,
                        ..grant
                    }),
                    ..result
                };
                let p = lifecycle::prepare(&mut f.owner, &mut f.recorder);
                assert_eq!(
                    f.owner.submit_discovered_annotation_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &dc,
                            scope: &lifecycle::scope4(),
                            cut: &p,
                            entry: &e,
                            variant: &v[0].reference,
                            facts: &facts()
                        },
                        &no_result,
                        NativeDiscoveredAnnotationAdmissionV1 {
                            context: &c,
                            facts: &authority(&c),
                            result: Some(&upgrade)
                        },
                        &mut f.recorder
                    ),
                    Err(DiscoveredNativeCommandErrorV1::Invocation(
                        InvocationError::IntentMismatch
                    ))
                );
                assert_eq!(f.owner.annotation_usage().issued_lifetime, allocation);
                assert_eq!(f.owner.generation(), generation);
            }
        }
    }
}
