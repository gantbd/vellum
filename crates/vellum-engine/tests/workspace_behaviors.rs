use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Boundary;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::Limits;
fn owner() -> (SourceCoordinator, Recorder) {
    let mut owner = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 1024,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 16,
            max_edit_payload_bytes: 4096,
        },
    });
    let mut r = Recorder::disabled();
    owner
        .configure_workspaces(
            WorkspaceLimits {
                max_issued: 8,
                max_live: 4,
                max_memberships: 8,
                max_work: 4096,
            },
            &mut r,
        )
        .unwrap();
    (owner, r)
}
#[test]
fn membership_timing_preserves_add_remove_noop_stale_refusal_and_document_identity() {
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1, Observation};
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let d = o.create(b"private target", &mut setup).unwrap();
        let other = o.create(b"private peer", &mut setup).unwrap();
        let workspace = o.create_workspace(&mut setup).unwrap();
        let peer = o.create_workspace(&mut setup).unwrap();
        let peer = o
            .set_workspace_membership(&peer, &other.document, true, &mut setup)
            .unwrap()
            .context;
        let generation = o.generation();
        let held = o.retention_usage();
        let lifecycle = o.lifecycle_state(&d.document, &mut setup).unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
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
        let added = o
            .set_workspace_membership(&workspace, &d.document, true, &mut r)
            .unwrap();
        assert!(added.changed);
        assert!(
            !o.set_workspace_membership(&added.context, &d.document, true, &mut r)
                .unwrap()
                .changed
        );
        assert_eq!(
            o.set_workspace_membership(&workspace, &d.document, true, &mut r),
            Err(CoordinatorError::Stale)
        );
        assert!(!o.is_faulted());
        let removed = o
            .set_workspace_membership(&added.context, &d.document, false, &mut r)
            .unwrap();
        assert!(removed.changed);
        assert!(
            !o.set_workspace_membership(&removed.context, &d.document, false, &mut r)
                .unwrap()
                .changed
        );
        assert_eq!(o.generation(), generation + 2);
        assert!(!o.is_faulted());
        assert_eq!(
            o.workspace(&peer.workspace, &mut setup).unwrap().context,
            peer
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(
            o.lifecycle_state(&d.document, &mut setup).unwrap(),
            lifecycle
        );
        assert_eq!(
            o.read(&d.document, &mut setup).unwrap().source.text(),
            "private target"
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|e| e.boundary == Boundary::WorkspaceMembership)
            .collect();
        assert_eq!(events.len(), if mode == 1 || mode == 2 { 5 } else { 0 });
        if mode == 1 || mode == 2 {
            assert_eq!(
                events.iter().map(|e| e.disposition).collect::<Vec<_>>(),
                [
                    Disposition::Completed,
                    Disposition::Completed,
                    Disposition::Refused,
                    Disposition::Completed,
                    Disposition::Completed
                ]
            );
            for e in events {
                assert_eq!(e.service_time.is_some(), mode == 2);
                assert!(!format!("{e:?}").contains("private"));
            }
        }
    }
}
#[test]
fn shared_membership_does_not_transfer_document_ownership_and_retirement_does_not_dispose_it() {
    let (mut o, mut r) = owner();
    let a = o.create(b"document", &mut r).unwrap();
    let x = o.create_workspace(&mut r).unwrap();
    let y = o.create_workspace(&mut r).unwrap();
    assert_eq!(x.generation, 1);
    assert!(
        o.workspace(&x.workspace, &mut r)
            .unwrap()
            .members
            .is_empty()
    );
    let x = o
        .set_workspace_membership(&x, &a.document, true, &mut r)
        .unwrap()
        .context;
    let y = o
        .set_workspace_membership(&y, &a.document, true, &mut r)
        .unwrap()
        .context;
    let g = o.generation();
    assert!(
        !o.set_workspace_membership(&x, &a.document, true, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), g);
    o.retire_workspace(&x, &mut r).unwrap();
    assert!(o.workspace(&x.workspace, &mut r).is_err());
    assert_eq!(
        o.workspace(&y.workspace, &mut r).unwrap().members,
        vec![a.document.clone()]
    );
    assert_eq!(o.read(&a.document, &mut r).unwrap().version, a);
}

#[test]
fn membership_aba_stale_contexts_foreign_handles_and_retired_ids_never_revive() {
    let (mut o, mut r) = owner();
    let a = o.create(b"A", &mut r).unwrap();
    let initial = o.create_workspace(&mut r).unwrap();
    let joined = o
        .set_workspace_membership(&initial, &a.document, true, &mut r)
        .unwrap()
        .context;
    let left = o
        .set_workspace_membership(&joined, &a.document, false, &mut r)
        .unwrap()
        .context;
    assert!(left.generation > joined.generation);
    assert_eq!(
        o.set_workspace_membership(&initial, &a.document, false, &mut r)
            .err(),
        Some(CoordinatorError::Stale)
    );
    let (mut foreign, mut fr) = owner();
    let foreign_workspace = foreign.create_workspace(&mut fr).unwrap();
    let foreign_doc = foreign.create(b"foreign", &mut fr).unwrap();
    assert_eq!(
        o.retire_workspace(&foreign_workspace, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        o.set_workspace_membership(&left, &foreign_doc.document, true, &mut r)
            .err(),
        Some(CoordinatorError::Unavailable)
    );
    o.retire_workspace(&left, &mut r).unwrap();
    assert_eq!(
        o.retire_workspace(&left, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let next = o.create_workspace(&mut r).unwrap();
    assert_ne!(next.workspace, left.workspace);
    assert_eq!(next.generation, 1);
}

#[test]
fn both_document_disposal_paths_remove_all_actual_memberships_not_peer_edges() {
    for retained in [false, true] {
        let (mut o, mut r) = owner();
        let a = o.create(b"A", &mut r).unwrap();
        let b = o.create(b"B", &mut r).unwrap();
        let x = o.create_workspace(&mut r).unwrap();
        let x = o
            .set_workspace_membership(&x, &a.document, true, &mut r)
            .unwrap()
            .context;
        let x = o
            .set_workspace_membership(&x, &b.document, true, &mut r)
            .unwrap()
            .context;
        let y = o.create_workspace(&mut r).unwrap();
        let y = o
            .set_workspace_membership(&y, &a.document, true, &mut r)
            .unwrap()
            .context;
        let z = o.create_workspace(&mut r).unwrap();
        let z = o
            .set_workspace_membership(&z, &b.document, true, &mut r)
            .unwrap()
            .context;
        let g = o.generation();
        if retained {
            let p = o.prepare_retained_disposal(&a, 0, &mut r).unwrap();
            o.commit_retained_disposal(p, &mut r).unwrap();
        } else {
            o.dispose(&a.document, &mut r).unwrap();
        }
        assert_eq!(o.generation(), g + 1);
        let current = o.workspace(&x.workspace, &mut r).unwrap();
        assert_eq!(current.context.generation, x.generation + 1);
        assert_eq!(current.members, vec![b.document.clone()]);
        let current = o.workspace(&y.workspace, &mut r).unwrap();
        assert_eq!(current.context.generation, y.generation + 1);
        assert!(current.members.is_empty());
        assert_eq!(o.workspace(&z.workspace, &mut r).unwrap().context, z);
        assert_eq!(o.workspace_usage().memberships, 2);
        assert!(
            o.set_workspace_membership(&current.context, &a.document, true, &mut r)
                .is_err()
        );
        assert_eq!(o.read(&b.document, &mut r).unwrap().version, b);
    }
}

#[test]
fn live_lifetime_global_membership_and_work_bounds_refuse_without_a_prefix() {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 128,
        source: Limits {
            max_source_bytes: 128,
            max_edits: 8,
            max_edit_payload_bytes: 256,
        },
    });
    let mut r = Recorder::disabled();
    let limits = WorkspaceLimits {
        max_issued: 3,
        max_live: 2,
        max_memberships: 1,
        max_work: 64,
    };
    o.configure_workspaces(limits, &mut r).unwrap();
    assert!(o.configure_workspaces(limits, &mut r).is_err());
    let a = o.create(b"A", &mut r).unwrap();
    o.set_read_only(&a.document, true, &mut r).unwrap();
    let x = o.create_workspace(&mut r).unwrap();
    let y = o.create_workspace(&mut r).unwrap();
    assert_eq!(o.create_workspace(&mut r), Err(CoordinatorError::Limit));
    let x = o
        .set_workspace_membership(&x, &a.document, true, &mut r)
        .unwrap()
        .context;
    let g = o.generation();
    assert_eq!(
        o.set_workspace_membership(&y, &a.document, true, &mut r)
            .err(),
        Some(CoordinatorError::Limit)
    );
    assert_eq!(o.generation(), g);
    o.retire_workspace(&x, &mut r).unwrap();
    let z = o.create_workspace(&mut r).unwrap();
    o.retire_workspace(&z, &mut r).unwrap();
    assert_eq!(o.create_workspace(&mut r), Err(CoordinatorError::Limit));
    assert_eq!(o.read(&a.document, &mut r).unwrap().version, a);
}

#[test]
fn finite_membership_traces_agree_with_an_independent_two_container_set_model() {
    for encoded in 0..1728 {
        let (mut o, mut r) = owner();
        let docs = [
            o.create(b"A", &mut r).unwrap(),
            o.create(b"B", &mut r).unwrap(),
            o.create(b"C", &mut r).unwrap(),
        ];
        let mut contexts = [
            o.create_workspace(&mut r).unwrap(),
            o.create_workspace(&mut r).unwrap(),
        ];
        let mut masks = [0u8; 2];
        let mut code = encoded;
        for _ in 0..3 {
            let action = code % 12;
            code /= 12;
            let workspace = action / 6;
            let document = (action % 6) / 2;
            let present = action % 2 == 1;
            let old = masks[workspace];
            if present {
                masks[workspace] |= 1 << document;
            } else {
                masks[workspace] &= !(1 << document);
            }
            let g = o.generation();
            let previous = contexts[workspace].generation;
            let update = o
                .set_workspace_membership(
                    &contexts[workspace],
                    &docs[document].document,
                    present,
                    &mut r,
                )
                .unwrap();
            assert_eq!(update.changed, old != masks[workspace]);
            assert_eq!(
                update.context.generation,
                previous + u64::from(update.changed)
            );
            assert_eq!(o.generation(), g + u64::from(update.changed));
            contexts[workspace] = update.context;
            for (i, context) in contexts.iter().enumerate() {
                let expected: Vec<_> = docs
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| masks[i] & (1 << j) != 0)
                    .map(|(_, doc)| doc.document.clone())
                    .collect();
                let snapshot = o.workspace(&context.workspace, &mut r).unwrap();
                assert_eq!(snapshot.context, *context);
                assert_eq!(snapshot.members, expected);
            }
        }
        for doc in docs {
            assert_eq!(o.read(&doc.document, &mut r).unwrap().version, doc);
        }
    }
}

#[test]
fn workspace_observations_are_optional_and_never_supply_effect_evidence() {
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, _) = owner();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let a = o.create(b"secret", &mut r).unwrap();
        let x = o.create_workspace(&mut r).unwrap();
        let x = o
            .set_workspace_membership(&x, &a.document, true, &mut r)
            .unwrap()
            .context;
        o.dispose(&a.document, &mut r).unwrap();
        let snapshot = o.workspace(&x.workspace, &mut r).unwrap();
        assert!(snapshot.members.is_empty());
        assert_eq!(snapshot.context.generation, x.generation + 1);
        assert!(!format!("{snapshot:?}").contains("secret"));
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::WorkspaceDisposalStaging)
            );
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
