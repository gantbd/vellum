use super::*;
#[test]
fn nested_owner_integrity_faults_are_not_downgraded_to_known_view_refusals() {
    for fault in [
        CoordinatorError::Faulted,
        CoordinatorError::History(HistoryError::Faulted),
        CoordinatorError::Resource(crate::resource::ResourceError::Faulted),
    ] {
        assert_eq!(error(fault), SelectionError::Faulted);
    }
}
#[test]
fn interrupted_actual_view_attachment_must_fence_the_owner() {
    let (mut o, mut r) = owner();
    let d = o.create(b"private", &mut r).unwrap();
    let view = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
    let generation = o.generation();
    o.publication_fault_after = Some(1);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.attach_logical_view(
            &view,
            stamp,
            &d,
            &[caret(0, Affinity::After)],
            0,
            &mut r
        )))
        .is_err()
    );
    assert_eq!(
        o.views.rows[view.slot].as_ref().unwrap().source.as_ref(),
        Some(&d)
    );
    assert_eq!(o.generation(), generation);
    assert!(o.is_faulted());
}

#[test]
fn attach_and_detach_interruption_cuts_preserve_unknown_peer_framing_and_parent_responsibility() {
    for detach in [false, true] {
        for cut in 0..3 {
            for mode in 0..4 {
                let (mut o, mut setup) = owner();
                let d = o.create(b"private target", &mut setup).unwrap();
                let other = o.create(b"private peer", &mut setup).unwrap();
                let peer = attach(&mut o, &other, caret(1, Affinity::After), &mut setup);
                let peer_stamp = o.views.rows[peer.slot].as_ref().unwrap().stamp;
                let view = o.create_logical_view(&mut setup).unwrap();
                if detach {
                    let expected = o.logical_view(&view, &mut setup).unwrap().stamp;
                    o.attach_logical_view(
                        &view,
                        expected,
                        &d,
                        &[caret(0, Affinity::After)],
                        0,
                        &mut setup,
                    )
                    .unwrap();
                }
                let expected = o.logical_view(&view, &mut setup).unwrap().stamp;
                let generation = o.generation();
                o.configure_retention_budget(8192, &mut setup).unwrap();
                let held = o.retention.reserve(71).unwrap();
                let usage = o.retention_usage();
                o.publication_fault_after = Some(cut);
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(
                        crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                    );
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Control,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| if detach {
                        o.detach_logical_view(&view, expected, &mut r)
                    } else {
                        o.attach_logical_view(
                            &view,
                            expected,
                            &d,
                            &[caret(0, Affinity::After)],
                            0,
                            &mut r,
                        )
                    }))
                    .is_err()
                );
                assert!(o.faulted);
                assert_eq!(o.generation(), generation + u64::from(cut == 2));
                let actual = o.views.rows[view.slot].as_ref().unwrap();
                assert_eq!(
                    actual.source.is_some(),
                    if cut == 0 { detach } else { !detach }
                );
                assert_eq!(
                    actual.stamp.attachment_generation,
                    expected.attachment_generation + u64::from(cut >= 1)
                );
                assert_eq!(
                    o.views.rows[peer.slot].as_ref().unwrap().source.as_ref(),
                    Some(&other)
                );
                assert_eq!(o.views.rows[peer.slot].as_ref().unwrap().stamp, peer_stamp);
                assert_eq!(o.retention_usage(), usage);
                assert!(o.retention.validate_exact(&held, 71).is_ok());
                assert_eq!(
                    o.documents[d.document.slot].as_ref().unwrap().revision,
                    d.revision
                );
                assert_eq!(
                    o.create(b"blocked", &mut setup),
                    Err(CoordinatorError::Faulted)
                );
                assert_eq!(
                    o.detach_logical_view(&view, expected, &mut setup),
                    Err(SelectionError::Faulted)
                );
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
                    .filter(|e| e.boundary == Boundary::LogicalViewLifecycle)
                    .collect();
                assert_eq!(events.len(), usize::from(mode == 1 || mode == 2));
                for e in events {
                    assert_eq!(e.disposition, Disposition::Unknown);
                    assert_eq!(e.service_time.is_some(), mode == 2);
                    assert!(!format!("{e:?}").contains("private"));
                }
            }
        }
    }
}
fn limits() -> LogicalViewLimits {
    LogicalViewLimits {
        max_live_views: 8,
        max_issued_views: 16,
        max_total_selections: 64,
        normalization: SelectionBudget {
            max_input_selections: 16,
            max_output_selections: 16,
            max_work: 65536,
        },
        max_mapping_steps: 65536,
    }
}
fn owner() -> (SourceCoordinator, Recorder) {
    let mut owner = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 16,
        },
    );
    let mut r = Recorder::disabled();
    owner.configure_logical_views(limits(), &mut r).unwrap();
    (owner, r)
}
fn caret(at: usize, affinity: Affinity) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity,
    }
}
fn attach(
    owner: &mut SourceCoordinator,
    source: &DocumentVersion,
    point: Selection,
    r: &mut Recorder,
) -> LogicalViewId {
    let id = owner.create_logical_view(r).unwrap();
    let stamp = owner.logical_view(&id, r).unwrap().stamp;
    owner
        .attach_logical_view(&id, stamp, source, &[point], 0, r)
        .unwrap();
    id
}
fn write(source: &DocumentVersion, at: usize, text: &str) -> DocumentEdits {
    DocumentEdits {
        expected: source.clone(),
        edits: vec![Replacement {
            start: at,
            end: at,
            expected: String::new(),
            inserted: text.into(),
        }],
    }
}

#[test]
fn validated_identity_needs_no_unused_view_or_publication_counter() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"abc", &mut r).unwrap();
    let id = attach(&mut owner, &source, caret(1, Affinity::After), &mut r);
    owner.views.rows[id.slot]
        .as_mut()
        .unwrap()
        .stamp
        .selection_generation = u64::MAX;
    owner.generation = u64::MAX;
    let context = owner.logical_view(&id, &mut r).unwrap().context().unwrap();
    let before = owner.logical_view_usage();
    assert!(
        !owner
            .set_logical_selections(&context, &[caret(1, Affinity::After)], 0, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(
        owner.set_logical_selections(&context, &[caret(2, Affinity::After)], 0, &mut r),
        Err(SelectionError::VersionExhausted)
    );
    assert_eq!(owner.generation, u64::MAX);
    assert_eq!(owner.logical_view_usage(), before);
}

#[test]
fn a_last_view_counter_refusal_is_atomic_but_unchanged_positions_need_no_counter() {
    for changed in [true, false] {
        let (mut owner, mut r) = owner();
        let a = owner.create(b"a", &mut r).unwrap();
        let b = owner.create(b"b", &mut r).unwrap();
        let va = attach(&mut owner, &a, caret(0, Affinity::After), &mut r);
        let vb = attach(
            &mut owner,
            &b,
            caret(
                0,
                if changed {
                    Affinity::After
                } else {
                    Affinity::Before
                },
            ),
            &mut r,
        );
        owner.views.rows[vb.slot]
            .as_mut()
            .unwrap()
            .stamp
            .selection_generation = u64::MAX;
        let prepared = owner
            .prepare(&[write(&a, 0, "X"), write(&b, 0, "X")], &[], &mut r)
            .unwrap();
        let generation = owner.generation;
        if changed {
            assert_eq!(
                owner.commit(prepared, &mut r),
                Err(CoordinatorError::VersionExhausted)
            );
            assert_eq!(owner.generation, generation);
            assert_eq!(owner.read(&a.document, &mut r).unwrap().source.text(), "a");
            assert_eq!(
                owner
                    .logical_view(&va, &mut r)
                    .unwrap()
                    .selections
                    .selections(),
                &[caret(0, Affinity::After)]
            );
        } else {
            let outcome = owner.commit(prepared, &mut r).unwrap();
            let current = outcome.changes[1].after.clone();
            assert_eq!(
                owner
                    .logical_view(&vb, &mut r)
                    .unwrap()
                    .stamp
                    .selection_generation,
                u64::MAX
            );
            assert_eq!(
                owner.logical_view(&vb, &mut r).unwrap().source,
                Some(current.clone())
            );
            let plan = owner
                .prepare_retained_disposal(&current, 0, &mut r)
                .unwrap();
            assert_eq!(
                owner.commit_retained_disposal(plan, &mut r),
                Err(CoordinatorError::VersionExhausted)
            );
            assert_eq!(
                owner
                    .lifecycle_state(&current.document, &mut r)
                    .unwrap()
                    .phase,
                LifecyclePhase::Live
            );
        }
    }
}

#[test]
fn owned_selection_drift_is_a_fault_not_a_known_stale_caller_or_an_implicit_repair() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"abc", &mut r).unwrap();
    let view = attach(&mut owner, &source, caret(1, Affinity::After), &mut r);
    let prepared = owner
        .prepare(&[write(&source, 0, "X")], &[], &mut r)
        .unwrap();
    let generation = owner.generation;
    let changed = normalize_selections(
        &owner.document(&source.document).unwrap().source,
        &[caret(2, Affinity::After)],
        0,
        limits().normalization,
    )
    .unwrap();
    owner.views.rows[view.slot].as_mut().unwrap().selections = changed;
    assert_eq!(
        owner.commit(prepared, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert!(owner.faulted);
    assert_eq!(owner.generation, generation);
    assert_eq!(
        owner.documents[source.document.slot]
            .as_ref()
            .unwrap()
            .source
            .text(),
        "abc"
    );
    assert_eq!(
        owner.views.rows[view.slot]
            .as_ref()
            .unwrap()
            .selections
            .selections(),
        &[caret(2, Affinity::After)]
    );
}

#[test]
fn bounded_mapping_refusal_and_interrupted_publication_have_distinct_known_and_unknown_results() {
    for interrupted in [false, true] {
        let (mut owner, mut r) = owner();
        let source = owner.create(b"abc", &mut r).unwrap();
        let id = attach(&mut owner, &source, caret(1, Affinity::After), &mut r);
        if !interrupted {
            owner.views.limits.as_mut().unwrap().max_mapping_steps = 0;
        }
        let prepared = owner
            .prepare(&[write(&source, 0, "X")], &[], &mut r)
            .unwrap();
        if interrupted {
            owner.publication_fault_after = Some(1);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || owner.commit(prepared, &mut r)
                ))
                .is_err()
            );
            assert!(owner.faulted);
            assert_eq!(
                owner.logical_view(&id, &mut r).err(),
                Some(SelectionError::Faulted)
            );
        } else {
            assert_eq!(owner.commit(prepared, &mut r), Err(CoordinatorError::Limit));
            assert!(!owner.faulted);
            assert_eq!(
                owner.read(&source.document, &mut r).unwrap().source.text(),
                "abc"
            );
        }
        assert_eq!(
            owner.views.rows[id.slot].as_ref().unwrap().source,
            Some(source)
        );
    }
}

#[test]
fn source_annotation_and_view_companions_share_one_publication_and_refusal_boundary() {
    for exhausted in [false, true] {
        let (mut owner, mut r) = owner();
        owner.annotations.limits = Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 128,
            max_retained_body_bytes: 1024,
            max_mapping_steps: 65536,
            max_live_snapshots: 8,
            max_snapshot_bytes: 4096,
        });
        let source = owner.create(b"ab", &mut r).unwrap();
        let view = attach(&mut owner, &source, caret(1, Affinity::After), &mut r);
        let plan = owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: source.clone(),
                        basis: TargetBasis::Before,
                        target: crate::attachment::AttachmentTarget::Point {
                            offset: 1,
                            affinity: Affinity::After,
                            profile: crate::attachment::PointProfile::Position,
                        },
                    },
                    body: "note".into(),
                }],
                &mut r,
            )
            .unwrap();
        let record = owner.commit(plan, &mut r).unwrap().annotation_changes[0]
            .annotation
            .clone();
        if exhausted {
            owner.views.rows[view.slot]
                .as_mut()
                .unwrap()
                .stamp
                .selection_generation = u64::MAX;
        }
        let before = owner.generation;
        let history = owner.history_usage();
        let plan = owner
            .prepare(&[write(&source, 1, "X")], &[], &mut r)
            .unwrap();
        let result = owner.commit(plan, &mut r);
        if exhausted {
            assert_eq!(result, Err(CoordinatorError::VersionExhausted));
            assert_eq!(owner.generation, before);
            assert_eq!(owner.history_usage(), history);
        } else {
            assert_eq!(result.unwrap().generation, before + 1);
        }
        let annotation = owner.annotation(&record, &mut r).unwrap();
        let crate::attachment::Placement::Located {
            target: crate::attachment::AttachmentTarget::Point { offset, .. },
            ..
        } = annotation.placement
        else {
            panic!("positional annotation remains located")
        };
        assert_eq!(offset, if exhausted { 1 } else { 2 });
        assert_eq!(annotation.body, "note");
        assert_eq!(
            owner
                .logical_view(&view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[caret(if exhausted { 1 } else { 2 }, Affinity::After)]
        );
    }
}
