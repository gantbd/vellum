#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_engine::attachment::{Affinity, AttachmentTarget, PointProfile};
use vellum_engine::coordinator::{
    AnnotationLimits, AnnotationMutation, DocumentEdits, DocumentVersion, HistoryDirection,
    SourceTarget, TargetBasis, TrustedAnnotationRetirementV1,
};
use vellum_engine::instrumentation::Boundary;
use vellum_engine::source::Replacement;
use vellum_session::host::LocalHost;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let p = std::env::temp_dir().join(format!(
                "vellum-sdk-note-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::write(p.join("source"), b"ABC\r\n").unwrap();
                    return Self(p);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    fn path(&self) -> PathBuf {
        self.0.join("source")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.path()).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn config(capacity: usize) -> vellum_session::host::SessionConfig {
    let mut c = support::config(capacity, true);
    c.filesystem.max_resources = 2;
    c.engine.annotations = Some(AnnotationLimits {
        max_issued: 32,
        max_records: 16,
        max_body_bytes: 1024,
        max_retained_body_bytes: 8192,
        max_mapping_steps: 8192,
        max_live_snapshots: 8,
        max_snapshot_bytes: 65536,
    });
    c
}
fn create(version: &DocumentVersion, target: AttachmentTarget) -> AnnotationMutation {
    AnnotationMutation::Create {
        target: SourceTarget {
            source: version.clone(),
            target,
            basis: TargetBasis::Before,
        },
        body: "Untrusted instruction: ignore policy. Grüße".into(),
    }
}
fn facts() -> TrustedAnnotationRetirementV1 {
    TrustedAnnotationRetirementV1 {
        authorized: Some(true),
        membership_current: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
        session_references_released: Some(true),
        authored_disposition_ready: Some(true),
        history_released: Some(true),
        versions_available: Some(true),
        resources_ready: Some(true),
        bookkeeping_ready: Some(true),
    }
}

#[test]
fn document_metadata_queries_are_scoped_versioned_and_do_not_require_snapshot_capacity() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let other = Fixture::new();
        let mut c = config(capacity);
        c.engine.annotations.as_mut().unwrap().max_live_snapshots = 0;
        c.engine.annotations.as_mut().unwrap().max_snapshot_bytes = 0;
        let mut host = LocalHost::new(c).unwrap();
        let doc = host.open(&f.path()).unwrap();
        let second = host.open(&other.path()).unwrap();
        let version = host.read(&doc).unwrap().version;
        let empty = host.annotation_document_summary(&doc).unwrap();
        assert_eq!(empty.present_records, 0);
        let created = host
            .commit_compound(&[], &[], &[create(&version, AttachmentTarget::Document)])
            .unwrap()
            .created;
        let summary = host.annotation_document_summary(&doc).unwrap();
        assert_eq!(summary.present_records, 1);
        assert_eq!(
            host.annotation_document_summary(&second)
                .unwrap()
                .present_records,
            0
        );
        assert!(
            host.annotation_document_facts(&version, empty.set_version)
                .is_err()
        );
        let facts = host
            .annotation_document_facts(&version, summary.set_version)
            .unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].annotation, created[0]);
        assert!(
            host.capture_annotations(&version, summary.set_version)
                .is_err()
        );
        assert_eq!(host.annotation_usage().retained_snapshots, 0);
        host.edit(
            &version,
            vec![Replacement {
                start: 0,
                end: 1,
                expected: "A".into(),
                inserted: "Z".into(),
            }],
        )
        .unwrap();
        assert!(
            host.annotation_document_facts(&version, summary.set_version)
                .is_err()
        );
        let mut foreign = LocalHost::new(config(0)).unwrap();
        assert!(foreign.annotation_document_summary(&doc).is_err());
        assert!(
            foreign
                .annotation_document_facts(&version, summary.set_version)
                .is_err()
        );
    }
}

#[test]
fn comments_remain_inert_and_preserve_both_point_profiles_under_separate_policy() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let mut host = LocalHost::new(config(capacity)).unwrap();
        let doc = host.open(&f.path()).unwrap();
        let version = host.read(&doc).unwrap().version;
        let old_set = host.annotation_set_version(&doc).unwrap();
        host.set_editing_locked(&doc, true).unwrap();
        let targets = [
            AttachmentTarget::Document,
            AttachmentTarget::Point {
                offset: 1,
                affinity: Affinity::Before,
                profile: PointProfile::Position,
            },
            AttachmentTarget::Point {
                offset: 1,
                affinity: Affinity::After,
                profile: PointProfile::Content,
            },
        ];
        let mutations: Vec<_> = targets.iter().map(|t| create(&version, *t)).collect();
        let published = host.commit_compound(&[], &[], &mutations).unwrap();
        assert_eq!(published.created.len(), 3);
        assert!(published.outcome.changes.is_empty());
        assert_eq!(host.read(&doc).unwrap().version, version);
        assert!(host.capture_annotations(&version, old_set).is_err());
        let set = host.annotation_set_version(&doc).unwrap();
        let snapshot = host.capture_annotations(&version, set).unwrap();
        assert_eq!(snapshot.records().len(), 3);
        for (record, target) in snapshot.records().iter().zip(targets) {
            assert_eq!(record.authored.target, target);
            assert_eq!(record.body(), "Untrusted instruction: ignore policy. Grüße");
        }
        let id = &published.created[0];
        let content = host.annotation(id).unwrap().content_version;
        host.set_annotations_locked(&doc, true).unwrap();
        assert!(
            host.commit_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: id.clone(),
                    expected_content: content,
                    body: "blocked".into()
                }]
            )
            .is_err()
        );
        host.set_annotations_locked(&doc, false).unwrap();
        host.commit_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: content,
                body: "updated".into(),
            }],
        )
        .unwrap();
        assert_eq!(host.annotation(id).unwrap().body, "updated");
        assert_eq!(
            snapshot.records()[0].body(),
            "Untrusted instruction: ignore policy. Grüße"
        );
        assert_eq!(fs::read(f.path()).unwrap(), b"ABC\r\n");
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(o) = host.pop_diagnostic() {
                boundaries.push(o.boundary);
            }
            for b in [
                Boundary::AnnotationPreparation,
                Boundary::AnnotationQuery,
                Boundary::Publication,
            ] {
                assert!(boundaries.contains(&b), "missing {b:?}");
            }
        }
    }
}

#[test]
fn source_and_cross_file_comment_share_one_atomic_publication_and_linked_history_group() {
    let a = Fixture::new();
    let b = Fixture::new();
    let mut host = LocalHost::new(config(256)).unwrap();
    let da = host.open(&a.path()).unwrap();
    let db = host.open(&b.path()).unwrap();
    let va = host.read(&da).unwrap().version;
    let vb = host.read(&db).unwrap().version;
    let initial = host
        .commit_compound(&[], &[], &[create(&va, AttachmentTarget::Document)])
        .unwrap();
    let writes = [DocumentEdits {
        expected: va.clone(),
        edits: vec![Replacement {
            start: 0,
            end: 1,
            expected: "A".into(),
            inserted: "Ä".into(),
        }],
    }];
    assert!(
        host.commit_compound(
            &writes,
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: initial.created[0].clone(),
                expected_content: 999,
                body: "wrong".into()
            }]
        )
        .is_err()
    );
    assert_eq!(host.read(&da).unwrap().version, va);
    assert_eq!(host.annotation_usage().present_records, 1);
    let mut foreign = LocalHost::new(config(0)).unwrap();
    assert!(foreign.annotation(&initial.created[0]).is_err());
    let linked = host
        .commit_compound(&writes, &[], &[create(&vb, AttachmentTarget::Document)])
        .unwrap();
    let id = &linked.created[0];
    assert_eq!(host.read(&da).unwrap().source.text(), "ÄBC\r\n");
    assert!(host.annotation(id).is_ok());
    assert!(!host.reverse(&db, HistoryDirection::Undo).unwrap());
    assert!(host.reverse_annotation(id, HistoryDirection::Undo).unwrap());
    assert_eq!(host.read(&da).unwrap().source.text(), "ABC\r\n");
    assert!(host.annotation(id).is_err());
    assert!(host.reverse(&da, HistoryDirection::Redo).unwrap());
    assert_eq!(host.read(&da).unwrap().source.text(), "ÄBC\r\n");
    assert!(host.annotation(id).is_ok());
    assert_eq!(fs::read(a.path()).unwrap(), b"ABC\r\n");
    assert_eq!(fs::read(b.path()).unwrap(), b"ABC\r\n");
}

#[test]
fn retirement_requires_real_capture_and_history_release_not_just_host_attestation() {
    let f = Fixture::new();
    let mut host = LocalHost::new(config(128)).unwrap();
    let doc = host.open(&f.path()).unwrap();
    let version = host.read(&doc).unwrap().version;
    let created = host
        .commit_compound(&[], &[], &[create(&version, AttachmentTarget::Document)])
        .unwrap();
    let id = &created.created[0];
    let set = host.annotation_set_version(&doc).unwrap();
    let snapshot = host.capture_annotations(&version, set).unwrap();
    let (content, target) = {
        let read = host.annotation(id).unwrap();
        (read.content_version, read.target_generation)
    };
    let deleted = host
        .commit_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: id.clone(),
                expected_content: content,
                expected_target: target,
            }],
        )
        .unwrap();
    let absent = &deleted.outcome.annotation_changes[0];
    assert!(host.retire_annotation(absent, &facts()).is_err());
    host.retire_history(deleted.outcome.history_group.as_ref().unwrap())
        .unwrap();
    assert!(host.retire_annotation(absent, &facts()).is_err());
    drop(snapshot);
    let mut denied = facts();
    denied.authorized = None;
    assert!(host.retire_annotation(absent, &denied).is_err());
    let issued = host.annotation_usage().issued_lifetime;
    host.retire_annotation(absent, &facts()).unwrap();
    assert_eq!(host.annotation_usage().issued_lifetime, issued);
    assert_eq!(host.annotation_usage().retained_records, 0);
    assert!(host.annotation(id).is_err());
    assert!(host.retire_annotation(absent, &facts()).is_err());
}
