use super::*;
use vellum_engine::source::Replacement;
fn req(predicates: Vec<ReadDependencyV3>) -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates,
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 4096,
        },
    }
}
fn base(predicate: ReadDependencyV1) -> ReadDependencyV3 {
    ReadDependencyV3::Base(ReadDependencyV2::Base(predicate))
}
pub(super) fn source_edit(o: &mut SourceCoordinator, v: &DocumentVersion, r: &mut Recorder) {
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: v.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "S".into(),
                    inserted: "s".into(),
                }],
            }],
            &[],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap();
}
fn create(o: &mut SourceCoordinator, v: &DocumentVersion, r: &mut Recorder) -> AnnotationId {
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: v.clone(),
                    basis: TargetBasis::Before,
                    target: vellum_engine::attachment::AttachmentTarget::ContentRange {
                        start: 0,
                        end: 1,
                    },
                },
                body: "note".into(),
            }],
            r,
        )
        .unwrap();
    let record = p.created_annotations().next().unwrap().clone();
    o.commit(p, r).unwrap();
    record
}
#[test]
fn mandatory_settings_contributors_and_extra_source_dependencies_are_both_required() {
    for changed in ["left", "host", "right", "source"] {
        let (mut o, mut r) = setup();
        let source = o.create(b"S", &mut r).unwrap();
        let reads = req(vec![base(ReadDependencyV1::Source(source.clone()))]);
        let changes = [change(
            "left",
            1,
            SettingOperationV1::Set(SettingValueV1::Number(2.0)),
        )];
        let p = o
            .prepare_settings_change_with_reads(&changes, &reads, &mut r)
            .unwrap();
        if changed == "source" {
            source_edit(&mut o, &source, &mut r);
        } else {
            set(&mut o, changed, 3.0, &mut r);
        }
        let generation = o.generation();
        let result = o.commit_settings_change_with_reads(p, &mut r);
        match changed {
            "right" => {
                assert!(result.unwrap().changed);
                assert_eq!(read(&o, "A", &mut r), 2.0);
                assert_eq!(read(&o, "B", &mut r), 3.0);
            }
            "source" => {
                assert_eq!(
                    result,
                    Err(SettingsChangeErrorV2::Reads(CoordinatorError::Stale))
                );
                assert_eq!(o.generation(), generation);
            }
            _ => {
                assert_eq!(
                    result,
                    Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
                );
                assert_eq!(o.generation(), generation);
            }
        }
        assert!(!o.is_faulted());
    }
}
#[test]
fn finite_settings_publications_conjoin_every_supported_read_kind_for_changes_and_noops() {
    let affected = [1u8, 2, 2 | 4, 2 | 8, 16, 32 | 64, 64];
    for mask in 0u8..128 {
        for (producer, affected_mask) in affected.iter().copied().enumerate() {
            for noop in [false, true] {
                let (mut o, mut r) = setup_with(profile(), true);
                let source = o.create(b"S", &mut r).unwrap();
                let annotation_source = o.create(b"TUV", &mut r).unwrap();
                let record = create(&mut o, &annotation_source, &mut r);
                o.configure_workspaces(
                    WorkspaceLimits {
                        max_issued: 4,
                        max_live: 4,
                        max_memberships: 8,
                        max_work: 4096,
                    },
                    &mut r,
                )
                .unwrap();
                let workspace = o.create_workspace(&mut r).unwrap();
                let body = o.annotation(&record, &mut r).unwrap().content_version;
                let target_generation = o.annotation(&record, &mut r).unwrap().target_generation;
                let set_generation = o
                    .annotation_set_version(&annotation_source.document, &mut r)
                    .unwrap();
                let target = o.settings_target(&id("B"), &mut r).unwrap();
                let basis = o.settings_basis(&mut r).unwrap();
                let all = [
                    base(ReadDependencyV1::Source(source.clone())),
                    base(ReadDependencyV1::AnnotationSet {
                        document: annotation_source.document.clone(),
                        expected: set_generation,
                    }),
                    base(ReadDependencyV1::AnnotationBody {
                        annotation: record.clone(),
                        expected: body,
                    }),
                    base(ReadDependencyV1::AnnotationTarget {
                        annotation: record.clone(),
                        expected: target_generation,
                    }),
                    ReadDependencyV3::Base(ReadDependencyV2::WorkspaceMembership {
                        workspace: workspace.workspace.clone(),
                        expected: workspace.generation,
                    }),
                    ReadDependencyV3::ConfigurationDomain {
                        target,
                        domain: SettingDomainV1::Source,
                        expected: 1,
                        interpretation: basis.interpretation(),
                    },
                    ReadDependencyV3::ConfigurationState {
                        registry: o.settings_registry(&mut r).unwrap(),
                        expected: basis.configuration_generation,
                        interpretation: basis.interpretation(),
                    },
                ];
                let reads = req(all
                    .into_iter()
                    .enumerate()
                    .filter_map(|(bit, p)| (mask & (1 << bit) != 0).then_some(p))
                    .collect());
                let changes = if noop {
                    vec![]
                } else {
                    vec![change(
                        "left",
                        1,
                        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
                    )]
                };
                let p = o
                    .prepare_settings_change_with_reads(&changes, &reads, &mut r)
                    .unwrap();
                match producer {
                    0 => source_edit(&mut o, &source, &mut r),
                    1 => {
                        create(&mut o, &annotation_source, &mut r);
                    }
                    2 => {
                        let p = o
                            .prepare_compound(
                                &[],
                                &[],
                                &[AnnotationMutation::UpdateBody {
                                    annotation: record.clone(),
                                    expected_content: body,
                                    body: "later".into(),
                                }],
                                &mut r,
                            )
                            .unwrap();
                        o.commit(p, &mut r).unwrap();
                    }
                    3 => {
                        let p=o.prepare_compound(&[],&[],&[AnnotationMutation::Reattach {annotation:record.clone(),expected_target:target_generation,target:SourceTarget {source:annotation_source.clone(),basis:TargetBasis::Before,target:vellum_engine::attachment::AttachmentTarget::ContentRange {start:1,end:2}}}],&mut r).unwrap();
                        o.commit(p, &mut r).unwrap();
                    }
                    4 => {
                        o.set_workspace_membership(&workspace, &source.document, true, &mut r)
                            .unwrap();
                    }
                    5 => {
                        set(&mut o, "right", 2.0, &mut r);
                    }
                    _ => {
                        let p = o
                            .prepare_settings_change(
                                &[SettingChangeV1 {
                                    layer: id("right"),
                                    expected_generation: 1,
                                    key: "analysis".into(),
                                    operation: SettingOperationV1::Set(SettingValueV1::Number(2.0)),
                                }],
                                &mut r,
                            )
                            .unwrap();
                        o.commit_settings_change(p, &mut r).unwrap();
                    }
                }
                let generation = o.generation();
                let before = o.settings_basis(&mut r).unwrap();
                let result = o.commit_settings_change_with_reads(p, &mut r);
                let stale = mask & affected_mask != 0;
                if stale {
                    assert_eq!(
                        result,
                        Err(SettingsChangeErrorV2::Reads(CoordinatorError::Stale)),
                        "mask={mask} producer={producer} noop={noop}"
                    );
                    assert_eq!(o.generation(), generation);
                    assert_eq!(o.settings_basis(&mut r).unwrap(), before);
                    assert_eq!(read(&o, "A", &mut r), 1.0);
                } else {
                    assert_eq!(result.unwrap().changed, !noop);
                    assert_eq!(read(&o, "A", &mut r), if noop { 1.0 } else { 2.0 });
                }
                assert_eq!(
                    o.read(&annotation_source.document, &mut r).unwrap().version,
                    annotation_source
                );
                assert_eq!(
                    o.annotation_usage().issued_lifetime,
                    if producer == 1 { 2 } else { 1 }
                );
                assert!(!o.is_faulted());
            }
        }
    }
}
#[test]
fn dependent_settings_replay_never_refreshes_reads_and_old_vocabulary_cannot_accept_them() {
    let (mut o, mut r) = setup();
    let source = o.create(b"S", &mut r).unwrap();
    let mut reads = req(vec![
        base(ReadDependencyV1::Source(source.clone())),
        ReadDependencyV3::ConfigurationState {
            registry: o.settings_registry(&mut r).unwrap(),
            expected: 1,
            interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
        },
    ]);
    let changes = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change_with_reads(&invocation, &changes, &reads, &mut r)
        .unwrap();
    source_edit(&mut o, &source, &mut r);
    reads.predicates.reverse();
    assert_eq!(
        o.submit_settings_change_with_reads(&invocation, &changes, &reads, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.submit_settings_change(&invocation, &changes, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Ok(&InvocationOutcome::SettingsRejectedV2(
            SettingsChangeErrorV2::Reads(CoordinatorError::Stale)
        ))
    );
    o.forget_invocation_outcome(&invocation, &mut r).unwrap();
    o.submit_settings_change_with_reads(&invocation, &changes, &reads, &mut r)
        .unwrap();
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    o.retire_invocation(&invocation, &mut r).unwrap();
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change(&invocation, &changes, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_settings_change_with_reads(&invocation, &changes, &req(vec![]), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.stop_invocation(&invocation, &mut r).unwrap();
    o.retire_invocation(&invocation, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
#[test]
fn readonly_source_dependencies_do_not_gain_writes_and_empty_reads_need_no_ancillary_module() {
    let (mut o, mut r) = setup();
    let source = o.create(b"S", &mut r).unwrap();
    o.set_read_only(&source.document, true, &mut r).unwrap();
    let reads = req(vec![base(ReadDependencyV1::Source(source.clone()))]);
    let changes = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_change_with_reads(&invocation, &changes, &reads, &mut r)
        .unwrap();
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(read(&o, "A", &mut r), 2.0);
    assert_eq!(o.read(&source.document, &mut r).unwrap().version, source);
    assert_eq!(
        o.prepare(
            &[DocumentEdits {
                expected: source,
                edits: vec![]
            }],
            &[],
            &mut r
        )
        .err(),
        Some(CoordinatorError::Unavailable)
    );
    let empty = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 0,
            max_work: 0,
        },
    };
    let p = o
        .prepare_settings_change_with_reads(&[], &empty, &mut r)
        .unwrap();
    let generation = o.generation();
    assert!(
        !o.commit_settings_change_with_reads(p, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), generation);
}
#[test]
fn foreign_or_duplicate_reads_refuse_without_settings_prefix_or_spurious_holds() {
    let (mut o, mut r) = setup();
    let (mut foreign, mut fr) = setup();
    let source = foreign.create(b"S", &mut fr).unwrap();
    let changes = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    let reads = req(vec![base(ReadDependencyV1::Source(source.clone()))]);
    assert_eq!(
        o.prepare_settings_change_with_reads(&changes, &reads, &mut r)
            .err(),
        Some(SettingsChangeErrorV2::Reads(CoordinatorError::Unavailable))
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    let duplicate = req(vec![
        base(ReadDependencyV1::Source(source.clone())),
        base(ReadDependencyV1::Source(source)),
    ]);
    let invocation = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_settings_change_with_reads(&invocation, &changes, &duplicate, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(
        o.invocation_state(&invocation, &mut r),
        Ok(InvocationState::Issued)
    );
    let prepared = foreign
        .prepare_settings_change_with_reads(&changes, &req(vec![]), &mut fr)
        .unwrap();
    assert_eq!(
        o.commit_settings_change_with_reads(prepared, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable))
    );
    assert!(!o.is_faulted());
    assert_eq!(foreign.settings_retention_usage().open_preparations, 0);
    assert_eq!(read(&o, "A", &mut r), 1.0);
}
#[test]
fn dependent_settings_preparation_validation_and_publication_capture_is_optional_and_content_free()
{
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = setup();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = o.create(b"S", &mut r).unwrap();
        let reads = req(vec![base(ReadDependencyV1::Source(source))]);
        let changes = [change(
            "left",
            1,
            SettingOperationV1::Set(SettingValueV1::Number(2.0)),
        )];
        let p = o
            .prepare_settings_change_with_reads(&changes, &reads, &mut r)
            .unwrap();
        o.commit_settings_change_with_reads(p, &mut r).unwrap();
        assert_eq!(read(&o, "A", &mut r), 2.0);
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::SettingsDependencyPreparation,
                Boundary::SettingsDependencyValidation,
                Boundary::SettingsDependencyPublication,
                Boundary::ReadDependencyPreparation,
                Boundary::ReadDependencyValidation,
            ] {
                assert!(records.iter().any(|record| record.boundary == boundary));
            }
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(records.is_empty());
        }
    }
}
