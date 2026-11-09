use super::*;
pub(in crate::coordinator) fn setup() -> (
    SourceCoordinator,
    DocumentVersion,
    CompoundIntent,
    ReadDependencyRequestV3,
    Recorder,
) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 8,
                max_edit_payload_bytes: 2048,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 16384,
        }),
        annotations: Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 4096,
            max_live_snapshots: 8,
            max_snapshot_bytes: 4096,
        }),
        history: None,
        resources: None,
        search: None,
    });
    let mut r = Recorder::disabled();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    o.install_initial_settings(settings::settings_test_fixture(&mut r), &mut r)
        .unwrap();
    let source = o.create(b"A", &mut r).unwrap();
    let edit = CompoundIntent {
        source: SourceIntent {
            writes: vec![DocumentEdits {
                expected: source.clone(),
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
    let target = o
        .settings_target(
            &crate::settings::SettingIdentityV1 {
                authority: "engine".into(),
                id: "target".into(),
                incarnation: 1,
            },
            &mut r,
        )
        .unwrap();
    let req = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::ConfigurationDomain {
            target,
            domain: SettingDomainV1::Analysis,
            expected: 1,
            interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
        }],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    };
    (o, source, edit, req, r)
}
#[test]
fn new_executable_predicates_cannot_be_smuggled_into_an_older_private_read_profile() {
    for old in [ReadProfile::SourceAnnotation, ReadProfile::Workspace] {
        for whole in [false, true] {
            let (mut o, source, intent, mut req, mut r) = setup();
            if whole {
                req.predicates[0] = ReadDependencyV3::ConfigurationState {
                    registry: o.settings_registry(&mut r).unwrap(),
                    expected: 1,
                    interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
                };
            }
            let mut p = o
                .prepare_compound_with_reads_v3(&intent, &req, &mut r)
                .unwrap();
            p.declared_reads.as_mut().unwrap().profile = old;
            let generation = o.generation();
            assert_eq!(o.commit(p, &mut r), Err(CoordinatorError::Faulted));
            assert!(o.is_faulted());
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
}
#[test]
fn legacy_base_predicates_keep_their_single_step_budget_and_duplicate_subject_rules() {
    let (o, source, _, _, mut r) = setup();
    let budget = ReadDependencyBudget {
        max_predicates: 2,
        max_work: 1,
    };
    o.check_read_dependencies(
        &[],
        &ReadDependencyRequest {
            predicates: vec![ReadDependencyV1::Source(source.clone())],
            budget,
        },
        &mut r,
    )
    .unwrap();
    o.check_read_dependencies_v2(
        &[],
        &ReadDependencyRequestV2 {
            predicates: vec![ReadDependencyV2::Base(ReadDependencyV1::Source(
                source.clone(),
            ))],
            budget,
        },
        &mut r,
    )
    .unwrap();
    let req = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(source.clone()),
        ))],
        budget,
    };
    o.check_read_dependencies_v3(&[], &req, &mut r).unwrap();
    let req = ReadDependencyRequestV3 {
        budget: ReadDependencyBudget {
            max_work: 256,
            ..budget
        },
        ..req
    };
    assert_eq!(
        o.check_read_dependencies_v3(&[source], &req, &mut r),
        Err(CoordinatorError::ReadDependency(
            ReadDependencyError::DuplicateSubject
        ))
    );
}
