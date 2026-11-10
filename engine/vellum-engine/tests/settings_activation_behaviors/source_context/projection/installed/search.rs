use super::super::super::search::{qfacts, request, search_profile};
use super::*;

fn install(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    s: &SourceReadScopeV1,
    classes: AnnotationProjectionClassesV1,
) -> AnnotationReadContextId {
    let cut = o
        .prepare_annotation_projection(classes, s, &capture_facts(s, &[]), r)
        .unwrap();
    o.install_annotation_context(&cut, s, &current(&cut, s), r)
        .unwrap()
        .context
}
fn configure(o: &mut SourceCoordinator, r: &mut Recorder, limits: AnnotationContextLimitsV1) {
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), r)
        .unwrap();
    o.configure_annotation_contexts(limits, &pool_control(), r)
        .unwrap();
    o.configure_source_search(search_profile(), &pool_control(), r)
        .unwrap();
}
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
fn combined_result_keeps_its_real_hold_after_source_bootstrap_retirement_and_discharges_once() {
    let (mut o, mut r, d, s) = ready(b"aba");
    configure(&mut o, &mut r, owner_limits());
    let combined = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let a = o
        .query_managed_search(
            SearchReadContextV1::Combined(&combined),
            &s,
            &q,
            SearchReadFactsV1::Combined(&eligible(&combined, &s)),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let b = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    assert_eq!(o.annotation_context_usage().holds, 1);
    assert_eq!(o.source_read_context_usage().holds, 1);
    assert!(o.retire_source_search_result(&b, &controls(&s), &mut r));
    assert_eq!(o.annotation_context_usage().holds, 1);
    assert!(o.stop_source_read_context(&source, &controls(&s), &mut r));
    assert!(o.retire_source_read_context(&source, &controls(&s), &mut r));
    assert_eq!(
        o.retire_source_read_contexts(&pool_control(), &mut r),
        Ok(true)
    );
    assert_eq!(
        o.retire_annotation_preparation(&pool_control(), &mut r),
        Ok(true)
    );
    let reply = o
        .release_managed_search(
            &a,
            &s,
            SearchReadFactsV1::Combined(&eligible(&combined, &s)),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    assert_eq!(reply.version, d);
    assert_eq!(
        reply.matches,
        vec![
            ByteRange { start: 0, end: 1 },
            ByteRange { start: 2, end: 3 }
        ]
    );
    assert!(o.stop_annotation_context(&combined, &controls(&s), &mut r));
    assert!(!o.retire_annotation_context(&combined, &controls(&s), &mut r));
    assert!(
        o.release_managed_search(
            &a,
            &s,
            SearchReadFactsV1::Combined(&eligible(&combined, &s)),
            &qfacts(&q),
            &mut r
        )
        .is_none()
    );
    assert!(o.retire_source_search_result(&a, &controls(&s), &mut r));
    assert!(!o.retire_source_search_result(&a, &controls(&s), &mut r));
    assert_eq!(o.annotation_context_usage().holds, 0);
    assert_eq!(o.annotation_context_usage().issued_lifetime, 2);
    assert!(o.retire_annotation_context(&combined, &controls(&s), &mut r));
    assert_eq!(
        o.retire_annotation_contexts(&pool_control(), &mut r),
        Ok(true)
    );
    assert_eq!(o.retire_source_search(&pool_control(), &mut r), Ok(true));
}

#[test]
fn annotation_only_and_wrong_issuer_facts_refuse_without_poisoning_or_taking_a_hold() {
    let (mut o, mut r, _, s) = ready(b"aba");
    configure(&mut o, &mut r, owner_limits());
    let annotation = install(
        &mut o,
        &mut r,
        &s,
        AnnotationProjectionClassesV1::Annotations,
    );
    let combined = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
    let q = request("a");
    let mut loss = eligible(&annotation, &s);
    loss.authorized = Some(false);
    assert!(
        o.query_managed_search(
            SearchReadContextV1::Combined(&annotation),
            &s,
            &q,
            SearchReadFactsV1::Combined(&loss),
            &qfacts(&q),
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_context_usage().holds, 0);
    let a = o
        .query_managed_search(
            SearchReadContextV1::Combined(&combined),
            &s,
            &q,
            SearchReadFactsV1::Combined(&eligible(&combined, &s)),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    assert!(
        o.release_source_search(&a, &s, &facts(&s), &qfacts(&q), &mut r)
            .is_none()
    );
    assert!(
        o.release_managed_search(
            &a,
            &s,
            SearchReadFactsV1::Combined(&loss),
            &qfacts(&q),
            &mut r
        )
        .is_none()
    );
    assert!(
        o.release_managed_search(
            &a,
            &s,
            SearchReadFactsV1::Combined(&eligible(&combined, &s)),
            &qfacts(&q),
            &mut r
        )
        .is_some()
    );
    assert!(o.stop_annotation_context(&annotation, &controls(&s), &mut r));
    assert!(o.retire_annotation_context(&annotation, &controls(&s), &mut r));
    assert_eq!(o.annotation_context_usage().holds, 1);
}

#[test]
fn combined_hold_occupancy_and_nonrefunding_shared_issuance_are_both_enforced() {
    for (max_holds, max_issued) in [(0, 8), (1, 8), (4, 2)] {
        let (mut o, mut r, _, s) = ready(b"aba");
        let mut p = owner_limits();
        p.max_holds = max_holds;
        p.max_issued = max_issued;
        configure(&mut o, &mut r, p);
        let c = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
        let q = request("a");
        let a = o.query_managed_search(
            SearchReadContextV1::Combined(&c),
            &s,
            &q,
            SearchReadFactsV1::Combined(&eligible(&c, &s)),
            &qfacts(&q),
            &mut r,
        );
        assert_eq!(a.is_some(), max_holds > 0);
        if let Some(a) = a {
            assert!(
                o.query_managed_search(
                    SearchReadContextV1::Combined(&c),
                    &s,
                    &q,
                    SearchReadFactsV1::Combined(&eligible(&c, &s)),
                    &qfacts(&q),
                    &mut r
                )
                .is_none()
            );
            assert!(o.retire_source_search_result(&a, &controls(&s), &mut r));
            let b = o.query_managed_search(
                SearchReadContextV1::Combined(&c),
                &s,
                &q,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r,
            );
            assert_eq!(b.is_some(), max_issued > 2);
        }
        assert!(!o.is_faulted());
    }
}

#[test]
fn combined_replacement_revalidates_whole_projection_at_actual_native_service() {
    for lost in 0..4 {
        let (mut o, mut r, d, s) = ready(b"aba");
        configure(&mut o, &mut r, owner_limits());
        let c = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
        let q = request("a");
        let a = o
            .query_managed_search(
                SearchReadContextV1::Combined(&c),
                &s,
                &q,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r,
            )
            .unwrap();
        let command = SourceSearchConsumerV1::ReplaceAll {
            result: a,
            replacement: "Z".into(),
        };
        let id = o.issue_invocation(&mut r).unwrap();
        assert_eq!(
            o.submit_managed_search_consumer(
                &id,
                &command,
                &s,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mutation(&command, &s),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
        assert_eq!(
            o.run_source_search_consumer(
                &id,
                &s,
                &facts(&s),
                &qfacts(&q),
                &mutation(&command, &s),
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        let mut f = eligible(&c, &s);
        match lost {
            1 => f.annotation_readable = Some(false),
            2 => f.source_readable = Some(false),
            3 => f.projection_current = Some(false),
            _ => {}
        }
        assert_eq!(
            o.run_managed_search_consumer(
                &id,
                &s,
                SearchReadFactsV1::Combined(&f),
                &qfacts(&q),
                &mutation(&command, &s),
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            o.read(&d.document, &mut r).unwrap().source.text(),
            if lost == 0 { "ZbZ" } else { "aba" }
        );
        assert_eq!(o.annotation_context_usage().holds, 1);
    }
}

#[test]
fn combined_navigation_and_selection_use_the_same_owner_after_bootstrap_pool_retirement() {
    use vellum_engine::{
        attachment::Affinity,
        selection::{Selection, SelectionBudget},
    };
    for lost in [false, true] {
        let (mut o, mut r, d, s) = ready(b"aba");
        configure(&mut o, &mut r, owner_limits());
        let c = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
        assert_eq!(
            o.retire_source_read_contexts(&pool_control(), &mut r),
            Ok(true)
        );
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
                anchor: BytePosition(3),
                head: BytePosition(3),
                affinity: Affinity::After,
            }],
            0,
            &mut r,
        )
        .unwrap();
        let context = o.logical_view(&view, &mut r).unwrap().context().unwrap();
        let q = request("a");
        let a = o
            .query_managed_search(
                SearchReadContextV1::Combined(&c),
                &s,
                &q,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r,
            )
            .unwrap();
        let found = o
            .navigate_managed_search(
                &a,
                &s,
                SourceSearchNavigationV1 {
                    direction: SearchDirection::Next,
                    start: 0,
                    current: None,
                    wrap: false,
                },
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r,
            )
            .unwrap()
            .found
            .unwrap();
        assert_eq!(found.range, ByteRange { start: 0, end: 1 });
        assert_eq!(
            o.managed_search_occurrence(
                &a,
                0,
                &s,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r
            ),
            Some(found.occurrence.clone())
        );
        let command = SourceSearchConsumerV1::SelectMatch {
            occurrence: found.occurrence,
            view: context,
            direction: SelectionDirectionV1::Backward,
            affinity: Affinity::Before,
            mode: SearchSelectionModeV1::ReplaceAll,
        };
        let id = o.issue_invocation(&mut r).unwrap();
        assert_eq!(
            o.submit_managed_search_consumer(
                &id,
                &command,
                &s,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mutation(&command, &s),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        let mut f = eligible(&c, &s);
        if lost {
            f.annotation_readable = Some(false);
        }
        assert_eq!(
            o.run_managed_search_consumer(
                &id,
                &s,
                SearchReadFactsV1::Combined(&f),
                &qfacts(&q),
                &mutation(&command, &s),
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        let selections = o.logical_view(&view, &mut r).unwrap();
        assert_eq!(
            selections.selections.selections()[0].anchor,
            BytePosition(if lost { 3 } else { 1 })
        );
        assert_eq!(
            selections.selections.selections()[0].head,
            BytePosition(if lost { 3 } else { 0 })
        );
        assert_eq!(o.annotation_context_usage().holds, 1);
        assert_eq!(o.source_read_context_usage().holds, 0);
    }
}

#[test]
fn shared_search_can_bootstrap_directly_from_installed_combined_owner() {
    let (mut o, mut r, _, s) = ready(b"aba");
    assert_eq!(
        o.retire_source_read_contexts(&pool_control(), &mut r),
        Ok(true)
    );
    configure(&mut o, &mut r, owner_limits());
    let c = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
    let q = request("a");
    assert!(
        o.query_managed_search(
            SearchReadContextV1::Combined(&c),
            &s,
            &q,
            SearchReadFactsV1::Combined(&eligible(&c, &s)),
            &qfacts(&q),
            &mut r
        )
        .is_some()
    );
    assert_eq!(o.annotation_context_usage().holds, 1);
}

#[test]
fn combined_search_capture_is_optional_bounded_content_free_and_not_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, mut r, _, s) = ready(b"private searchable source");
        configure(&mut o, &mut r, owner_limits());
        let c = install(&mut o, &mut r, &s, AnnotationProjectionClassesV1::Combined);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let q = request("private");
        let a = o
            .query_managed_search(
                SearchReadContextV1::Combined(&c),
                &s,
                &q,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r,
            )
            .unwrap();
        assert_eq!(
            o.release_managed_search(
                &a,
                &s,
                SearchReadFactsV1::Combined(&eligible(&c, &s)),
                &qfacts(&q),
                &mut r
            )
            .unwrap()
            .matches,
            vec![ByteRange { start: 0, end: 7 }]
        );
        assert!(o.retire_source_search_result(&a, &controls(&s), &mut r));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.annotation_context_usage().holds, 0);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::SourceContextHold,
                Boundary::SourceContextRelease,
                Boundary::SourceSearchRetirement,
            ] {
                assert_eq!(events.iter().filter(|e| e.boundary == boundary).count(), 1);
            }
            assert!(!events.iter().any(|e| e.boundary == Boundary::Publication));
            assert!(events.iter().all(|e| e.service_time.is_none()));
            assert!(!format!("{events:?}").contains("private"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
