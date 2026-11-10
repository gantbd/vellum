use super::*;
use crate::source_context::search::{qfacts, request, search_profile};
fn mutation<'a>(
    c: &'a SourceSearchConsumerV1,
    s: &'a SourceReadScopeV1,
) -> TrustedSourceSearchConsumerFactsV1<'a> {
    TrustedSourceSearchConsumerFactsV1 {
        scope: s,
        command: c,
        mutation_allowed: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn source_only_and_version_set_results_coexist_and_bootstrap_retirement_does_not_cross_holds() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    o.configure_source_read_contexts(
        "engine",
        "local",
        crate::source_context::profile(),
        &pool_control(),
        &mut r,
    )
    .unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let single = o
        .capture_source_read_context(&a, &crate::source_context::facts(&a), &mut r)
        .unwrap();
    let q = request("first");
    let source_result = o
        .query_source_search(
            &single,
            &a,
            &q,
            &crate::source_context::facts(&a),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &acq.participants[0].snapshot,
        scope: &a,
        parent: &parent,
    };
    let result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    assert!(!o.retire_managed_search_result(
        &source_result,
        SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
        &mut r
    ));
    assert!(o.retire_source_search_result(&source_result, &controls(&a), &mut r));
    assert_eq!(o.version_set_context_usage().holds, 1);
    o.stop_source_read_context(&single, &controls(&a), &mut r);
    assert!(o.retire_source_read_context(&single, &controls(&a), &mut r));
    assert_eq!(
        o.retire_source_read_contexts(&pool_control(), &mut r),
        Ok(true)
    );
    assert!(
        o.release_managed_search(
            &result,
            &a,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r
        )
        .is_some()
    );
}
#[test]
fn annotation_only_search_and_wrong_parent_frames_refuse_before_query_loss() {
    let (mut o, mut r, a, b) = base();
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
    o.configure_version_set_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let participants = [
        VersionSetParticipantV1 {
            scope: &a,
            data: AnnotationProjectionClassesV1::Source,
        },
        VersionSetParticipantV1 {
            scope: &b,
            data: AnnotationProjectionClassesV1::Annotations,
        },
    ];
    let members = [current_member(&a), current_member(&b)];
    let cut = o
        .prepare_version_set(
            &participants,
            &[capture_facts(&a, &[]), capture_facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let cut = o
        .prepare_version_set(
            &participants,
            &[capture_facts(&a, &[]), capture_facts(&b, &[])],
            &mut r,
        )
        .unwrap();
    let other = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let q = request("first");
    let parent = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    let anno = TrustedVersionSetSearchFactsV1 {
        member: &acq.participants[1].snapshot,
        scope: &b,
        parent: &parent,
    };
    assert!(
        o.query_managed_search(
            SearchReadContextV1::VersionSet(anno.member),
            &b,
            &q,
            SearchReadFactsV1::VersionSet(&anno),
            &qfacts(&q),
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_context_usage().holds, 0);
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &acq.participants[0].snapshot,
        scope: &a,
        parent: &parent,
    };
    let result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let wrong = TrustedVersionSetContextFactsV1 {
        context: &other.context,
        members: &members,
    };
    let wrong = TrustedVersionSetSearchFactsV1 {
        member: sf.member,
        scope: &a,
        parent: &wrong,
    };
    let mut lost = qfacts(&q);
    lost.grant = Some(false);
    assert!(
        o.release_managed_search(
            &result,
            &a,
            SearchReadFactsV1::VersionSet(&wrong),
            &lost,
            &mut r
        )
        .is_none()
    );
    assert!(
        o.release_managed_search(
            &result,
            &a,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r
        )
        .is_some()
    );
    assert_eq!(o.version_set_context_usage().stopped, 0);
}
#[test]
fn version_set_search_signals_are_optional_bounded_content_free_and_not_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, mut r, a, b, cut) = fixture(owner_limits());
        let members = [current_member(&a), current_member(&b)];
        let acq = o
            .install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &members,
                },
                &mut r,
            )
            .unwrap();
        o.configure_source_search(search_profile(), &pool_control(), &mut r)
            .unwrap();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let parent = TrustedVersionSetContextFactsV1 {
            context: &acq.context,
            members: &members,
        };
        let sf = TrustedVersionSetSearchFactsV1 {
            member: &acq.participants[0].snapshot,
            scope: &a,
            parent: &parent,
        };
        let q = request("first");
        let result = o
            .query_managed_search(
                SearchReadContextV1::VersionSet(sf.member),
                &a,
                &q,
                SearchReadFactsV1::VersionSet(&sf),
                &qfacts(&q),
                &mut r,
            )
            .unwrap();
        assert!(
            o.release_managed_search(
                &result,
                &a,
                SearchReadFactsV1::VersionSet(&sf),
                &qfacts(&q),
                &mut r
            )
            .is_some()
        );
        assert!(o.retire_managed_search_result(
            &result,
            SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
            &mut r
        ));
        assert_eq!(o.generation(), generation);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::SourceContextHold,
                Boundary::SourceContextRelease,
                Boundary::SourceSearchRetirement,
            ] {
                assert_eq!(events.iter().filter(|e| e.boundary == boundary).count(), 1);
            }
            assert!(
                events
                    .iter()
                    .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
            );
            assert!(!format!("{events:?}").contains("first"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
#[test]
fn version_set_replacement_and_selection_share_existing_publishers_and_recheck_peer_facts_at_service()
 {
    use vellum_engine::{
        attachment::Affinity,
        selection::{Selection, SelectionBudget},
    };
    for kind in 0..3 {
        for lost in [false, true] {
            let (mut o, mut r, a, b, cut) = fixture(owner_limits());
            let d = o.read(&a.document, &mut r).unwrap().version;
            o.configure_logical_views(
                LogicalViewLimits {
                    max_live_views: 4,
                    max_issued_views: 4,
                    max_total_selections: 8,
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
            let view = o.create_logical_view(&mut r).unwrap();
            let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
            o.attach_logical_view(
                &view,
                stamp,
                &d,
                &[Selection {
                    anchor: BytePosition(5),
                    head: BytePosition(5),
                    affinity: Affinity::After,
                }],
                0,
                &mut r,
            )
            .unwrap();
            let context = o.logical_view(&view, &mut r).unwrap().context().unwrap();
            let members = [current_member(&a), current_member(&b)];
            let acq = o
                .install_version_set(
                    &cut,
                    &TrustedPreparedVersionSetFactsV1 {
                        projection: &cut,
                        members: &members,
                    },
                    &mut r,
                )
                .unwrap();
            o.configure_source_search(search_profile(), &pool_control(), &mut r)
                .unwrap();
            let parent = TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &members,
            };
            let sf = TrustedVersionSetSearchFactsV1 {
                member: &acq.participants[0].snapshot,
                scope: &a,
                parent: &parent,
            };
            let q = request("i");
            let result = o
                .query_managed_search(
                    SearchReadContextV1::VersionSet(sf.member),
                    &a,
                    &q,
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mut r,
                )
                .unwrap();
            let found = o
                .navigate_managed_search(
                    &result,
                    &a,
                    SourceSearchNavigationV1 {
                        direction: SearchDirection::Next,
                        start: 0,
                        current: None,
                        wrap: false,
                    },
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mut r,
                )
                .unwrap()
                .found
                .unwrap();
            assert_eq!(found.range, ByteRange { start: 1, end: 2 });
            assert_eq!(
                o.managed_search_occurrence(
                    &result,
                    0,
                    &a,
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mut r
                ),
                Some(found.occurrence.clone())
            );
            let command = match kind {
                0 => SourceSearchConsumerV1::ReplaceAll {
                    result,
                    replacement: "Z".into(),
                },
                1 => SourceSearchConsumerV1::ReplaceCurrent {
                    occurrence: found.occurrence,
                    replacement: "Z".into(),
                },
                _ => SourceSearchConsumerV1::SelectMatch {
                    occurrence: found.occurrence,
                    view: context,
                    direction: SelectionDirectionV1::Backward,
                    affinity: Affinity::Before,
                    mode: SearchSelectionModeV1::ReplaceAll,
                },
            };
            let id = o.issue_invocation(&mut r).unwrap();
            assert_eq!(
                o.submit_managed_search_consumer(
                    &id,
                    &command,
                    &a,
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mutation(&command, &a),
                    &mut r
                ),
                Ok(InvocationState::Pending)
            );
            assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
            let mut fresh = [current_member(&a), current_member(&b)];
            if lost {
                fresh[1].annotation_readable = Some(false);
            }
            let parent = TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &fresh,
            };
            let sf = TrustedVersionSetSearchFactsV1 {
                member: &acq.participants[0].snapshot,
                scope: &a,
                parent: &parent,
            };
            assert_eq!(
                o.run_managed_search_consumer(
                    &id,
                    &a,
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mutation(&command, &a),
                    &mut r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(
                o.read(&a.document, &mut r).unwrap().source.text(),
                if kind < 2 && !lost { "fZrst" } else { "first" }
            );
            if kind == 2 {
                let state = o.logical_view(&view, &mut r).unwrap();
                assert_eq!(
                    state.selections.selections()[0].anchor,
                    BytePosition(if lost { 5 } else { 2 })
                );
                assert_eq!(
                    state.selections.selections()[0].head,
                    BytePosition(if lost { 5 } else { 1 })
                );
            }
            assert_eq!(o.version_set_context_usage().holds, 1);
        }
    }
}
#[test]
fn malformed_peer_scope_cannot_consume_pending_intent_and_retired_result_settles_without_a_new_lease()
 {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &acq.participants[0].snapshot,
        scope: &a,
        parent: &parent,
    };
    let q = request("first");
    let result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let command = SourceSearchConsumerV1::ReplaceAll {
        result: result.clone(),
        replacement: "Z".into(),
    };
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_managed_search_consumer(
        &id,
        &command,
        &a,
        SearchReadFactsV1::VersionSet(&sf),
        &qfacts(&q),
        &mutation(&command, &a),
        &mut r,
    )
    .unwrap();
    let mut wrong = b.clone();
    wrong.policy.push('x');
    let wrong_members = [current_member(&a), current_member(&wrong)];
    let wrong_parent = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &wrong_members,
    };
    let wrong_sf = TrustedVersionSetSearchFactsV1 {
        member: sf.member,
        scope: &a,
        parent: &wrong_parent,
    };
    assert_eq!(
        o.run_managed_search_consumer(
            &id,
            &a,
            SearchReadFactsV1::VersionSet(&wrong_sf),
            &qfacts(&q),
            &mutation(&command, &a),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.version_set_context_usage().stopped, 0);
    assert!(o.retire_managed_search_result(
        &result,
        SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
        &mut r
    ));
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert_eq!(
        o.run_managed_search_consumer(
            &id,
            &a,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mutation(&command, &a),
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(CoordinatorError::Unavailable)
    ));
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "first");
}
#[test]
fn hold_caps_and_shared_nonrefunding_issuance_are_distinct_from_search_result_issuance() {
    for (cap, issued) in [(0, 64), (1, 64), (8, 4)] {
        let mut p = owner_limits();
        p.max_holds = cap;
        p.max_issued = issued;
        let (mut o, mut r, a, b, cut) = fixture(p);
        let members = [current_member(&a), current_member(&b)];
        let acq = o
            .install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &members,
                },
                &mut r,
            )
            .unwrap();
        o.configure_source_search(search_profile(), &pool_control(), &mut r)
            .unwrap();
        let parent = TrustedVersionSetContextFactsV1 {
            context: &acq.context,
            members: &members,
        };
        let sf = TrustedVersionSetSearchFactsV1 {
            member: &acq.participants[0].snapshot,
            scope: &a,
            parent: &parent,
        };
        let q = request("first");
        let result = o.query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        );
        assert_eq!(result.is_some(), cap > 0);
        if let Some(result) = result {
            assert!(
                o.query_managed_search(
                    SearchReadContextV1::VersionSet(sf.member),
                    &a,
                    &q,
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mut r
                )
                .is_none()
            );
            assert!(o.retire_managed_search_result(
                &result,
                SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
                &mut r
            ));
            assert_eq!(
                o.query_managed_search(
                    SearchReadContextV1::VersionSet(sf.member),
                    &a,
                    &q,
                    SearchReadFactsV1::VersionSet(&sf),
                    &qfacts(&q),
                    &mut r
                )
                .is_some(),
                issued > 4
            );
        } else {
            assert_eq!(o.source_search_usage().issued_lifetime, 0);
        }
        assert!(!o.is_faulted());
    }
}
#[test]
fn complete_protected_control_frames_release_only_their_original_result_hold() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &acq.participants[0].snapshot,
        scope: &a,
        parent: &parent,
    };
    let q = request("first");
    let a_result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let b_result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    for controls in [vec![], vec![controls(&a)], vec![controls(&b), controls(&a)]] {
        assert!(!o.retire_managed_search_result(
            &a_result,
            SearchResultControlV1::VersionSet(&controls),
            &mut r
        ));
    }
    let mut denied = controls(&b);
    denied.delivery_drained = Some(false);
    assert!(!o.retire_managed_search_result(
        &a_result,
        SearchResultControlV1::VersionSet(&[controls(&a), denied]),
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().holds, 2);
    assert!(o.retire_managed_search_result(
        &a_result,
        SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
        &mut r
    ));
    assert!(!o.retire_managed_search_result(
        &a_result,
        SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().holds, 1);
    assert!(
        o.release_managed_search(
            &b_result,
            &a,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r
        )
        .is_some()
    );
}
#[test]
fn version_member_search_holds_the_whole_parent_until_original_result_retirement() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    o.retire_version_set_preparation(&pool_control(), &mut r)
        .unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &acq.participants[0].snapshot,
        scope: &a,
        parent: &parent,
    };
    let q = request("first");
    let result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    assert_eq!(o.version_set_context_usage().holds, 1);
    assert_eq!(
        o.release_managed_search(
            &result,
            &a,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r
        )
        .unwrap()
        .matches,
        vec![ByteRange { start: 0, end: 5 }]
    );
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Member(&acq.participants[1].snapshot),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(!o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(!o.retire_source_search_result(&result, &controls(&a), &mut r));
    assert!(o.retire_managed_search_result(
        &result,
        SearchResultControlV1::VersionSet(&[controls(&a), controls(&b)]),
        &mut r
    ));
    assert_eq!(o.version_set_context_usage().holds, 0);
    assert_eq!(o.version_set_context_usage().issued_lifetime, 4);
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&acq.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
}
