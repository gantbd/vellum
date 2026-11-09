#![cfg(target_os = "macos")]
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_annotation_json::*;
use vellum_editor::{Action, CommentPointProfile, Editor, EditorError, reference_profile};
use vellum_engine::{
    coordinator::AnnotationSnapshot,
    instrumentation::{Boundary, Recorder},
    resource::{Application, SavePurpose, Synchronization},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let p = std::env::temp_dir().join(format!(
                "vellum-editor-persist-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::write(p.join("source"), "\u{feff}ABC\r\n").unwrap();
                    fs::write(p.join("notes"), b"old destination").unwrap();
                    return Self(p);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    fn source(&self) -> PathBuf {
        self.0.join("source")
    }
    fn notes(&self) -> PathBuf {
        self.0.join("notes")
    }
    fn app(&self, capacity: usize) -> Editor {
        let mut config = reference_profile();
        config.capture_capacity = capacity;
        Editor::open(&[self.source()], config).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.source()).unwrap();
        fs::remove_file(self.notes()).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn limits() -> CodecLimits {
    CodecLimits {
        max_input_bytes: 65536,
        max_output_bytes: 65536,
        max_nodes: 8192,
        max_depth: 32,
        max_string_bytes: 65536,
        max_records: 128,
        max_integer_digits: 20,
        max_body_bytes: 4096,
        max_total_body_bytes: 65536,
        max_associations: 256,
        max_source_bytes: 65536,
    }
}
fn aliases(snapshot: &AnnotationSnapshot) -> CaptureAssociations {
    // Deliberate host premises for this fixture, not a product identity policy.
    CaptureAssociations {
        document: DocumentAssociation {
            native: snapshot.source().document.clone(),
            identity: DocumentIdentity {
                namespace: "test-host".into(),
                value: "document".into(),
            },
        },
        records: snapshot
            .records()
            .iter()
            .enumerate()
            .map(|(i, r)| RecordAssociation {
                native: r.annotation.clone(),
                identity: RecordIdentity {
                    namespace: "test-host".into(),
                    value: format!("note-{i}"),
                    incarnation: 1,
                },
            })
            .collect(),
        sources: vec![SourceAssociation {
            native: snapshot.source().clone(),
            evidence: SourceEvidence {
                domain: "test-exact-source".into(),
                token: "current".into(),
                byte_length: snapshot.source_text().len() as u64,
            },
            text: snapshot.source_text().into(),
        }],
    }
}
fn create(app: &mut Editor, body: &str) {
    for profile in [CommentPointProfile::Position, CommentPointProfile::Content] {
        let draft = app.begin_comment(profile).unwrap();
        app.create_comments(&draft, body).unwrap();
    }
}
fn save(app: &mut Editor) {
    let snapshot = app.capture_comments().unwrap();
    let aliases = aliases(&snapshot);
    app.save_comments(&snapshot, &aliases, limits()).unwrap();
}

#[test]
fn explicit_comment_save_uses_native_receipts_preserves_profiles_and_cleans_up_without_discard() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let mut app = f.app(capacity);
        let original = fs::read(f.source()).unwrap();
        let raw = "\u{feff}Grüße\r\n\n\r$(inert)\x1b";
        create(&mut app, raw);
        app.bind_comment_file(&f.notes()).unwrap();
        assert!(app.comment_file_bound());
        assert!(app.comments_dirty().unwrap());
        assert_eq!(fs::read(f.notes()).unwrap(), b"old destination");
        let snapshot = app.capture_comments().unwrap();
        let aliases = aliases(&snapshot);
        app.save_comments(&snapshot, &aliases, limits()).unwrap();
        assert!(!app.is_dirty().unwrap());
        assert!(app.last_save_outcome().unwrap().is_none());
        let outcome = app.last_comment_save_outcome().unwrap().unwrap();
        assert_eq!(outcome.purpose, SavePurpose::OrdinaryAnnotations);
        assert_eq!(outcome.application, Application::Written);
        assert_eq!(outcome.synchronization, Synchronization::Succeeded);
        assert!(outcome.quiescent && outcome.no_future_effects && !outcome.contested);
        let bytes = fs::read(f.notes()).unwrap();
        let context = DecodeContext {
            document: aliases.document.identity,
            source: aliases.sources[0].evidence.clone(),
            text: snapshot.source_text().into(),
        };
        let decoded = decode(&bytes, &context, limits(), &mut Recorder::disabled()).unwrap();
        assert_eq!(decoded.records.len(), 2);
        assert!(matches!(
            decoded.records[0].authored_target,
            Target::Point {
                profile: PointMeaning::Position,
                ..
            }
        ));
        assert!(matches!(
            decoded.records[1].authored_target,
            Target::Point {
                profile: PointMeaning::Content,
                ..
            }
        ));
        assert!(decoded.records.iter().all(|r| r.body == raw));
        assert_eq!(fs::read(f.source()).unwrap(), original);
        drop(snapshot);
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(o) = app.pop_diagnostic() {
                boundaries.push(o.boundary);
            }
            assert!(boundaries.contains(&Boundary::SubstrateReplacement));
            assert!(boundaries.contains(&Boundary::SubstrateSynchronization));
        }
        app.close(false).unwrap();
        assert!(app.is_closed());
        assert_eq!(app.native_view_usage().live, 0);
        assert_eq!(fs::read(f.notes()).unwrap(), bytes);
    }
}

#[test]
fn deletion_empty_checkpoint_staleness_and_source_save_remain_separate() {
    let f = Fixture::new();
    let mut app = f.app(256);
    app.apply(Action::Insert("ß".into())).unwrap();
    create(&mut app, "note");
    app.bind_comment_file(&f.notes()).unwrap();
    save(&mut app);
    assert!(!app.comments_dirty().unwrap());
    assert!(app.is_source_dirty().unwrap());
    let notes = fs::read(f.notes()).unwrap();
    assert_eq!(fs::read(f.source()).unwrap(), "\u{feff}ABC\r\n".as_bytes());
    app.apply(Action::Save).unwrap();
    assert!(!app.is_dirty().unwrap());
    assert_eq!(fs::read(f.notes()).unwrap(), notes);
    let source_outcome = app.last_save_outcome().unwrap().unwrap();
    assert_eq!(source_outcome.purpose, SavePurpose::OrdinarySource);
    let stale = app.capture_comments().unwrap();
    let handle = app.comments().unwrap()[0].handle.clone();
    app.update_comment(&handle, "new body").unwrap();
    assert!(app.comments_dirty().unwrap());
    assert!(matches!(app.close(false), Err(EditorError::Unsaved)));
    assert!(
        app.save_comments(&stale, &aliases(&stale), limits())
            .is_err()
    );
    assert_eq!(fs::read(f.notes()).unwrap(), notes);
    drop(stale);
    save(&mut app);
    assert!(!app.comments_dirty().unwrap());
    // More checkpoints than the native live-snapshot budget: superseded task
    // AND artifact owners must discharge, not just disappear from the UI list.
    for i in 0..10 {
        let handle = app.comments().unwrap()[0].handle.clone();
        app.update_comment(&handle, &format!("checkpoint-{i}"))
            .unwrap();
        save(&mut app);
    }
    let handles: Vec<_> = app
        .comments()
        .unwrap()
        .iter()
        .map(|r| r.handle.clone())
        .collect();
    for handle in handles {
        app.delete_comment(&handle).unwrap();
    }
    assert!(app.comments_dirty().unwrap()); // Empty is NOT an unchanged baseline.
    assert!(matches!(app.close(false), Err(EditorError::Unsaved)));
    save(&mut app);
    assert!(!app.comments_dirty().unwrap());
    assert_eq!(
        app.last_save_outcome().unwrap().unwrap().artifact,
        source_outcome.artifact
    );
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 2);
    app.close(false).unwrap();
}

#[test]
fn aliases_missing_resources_invalid_mappings_and_external_changes_do_not_fabricate_saves() {
    let f = Fixture::new();
    let mut app = f.app(256);
    create(&mut app, "note");
    assert!(app.bind_comment_file(&f.source()).is_err());
    assert!(!app.comment_file_bound());
    assert!(app.bind_comment_file(&f.0.join("missing")).is_err());
    app.bind_comment_file(&f.notes()).unwrap();
    assert!(app.bind_comment_file(&f.notes()).is_err());
    let snapshot = app.capture_comments().unwrap();
    let mut mapping = aliases(&snapshot);
    mapping.sources[0].text.push('!');
    assert!(app.save_comments(&snapshot, &mapping, limits()).is_err());
    assert!(app.last_comment_save_outcome().unwrap().is_none());
    assert_eq!(fs::read(f.notes()).unwrap(), b"old destination");
    fs::write(f.notes(), b"external update").unwrap();
    assert!(
        app.save_comments(&snapshot, &aliases(&snapshot), limits())
            .is_err()
    );
    assert_eq!(fs::read(f.notes()).unwrap(), b"external update");
    assert!(app.comments_dirty().unwrap());
    assert_eq!(fs::read(f.source()).unwrap(), "\u{feff}ABC\r\n".as_bytes());
    drop(snapshot);
    app.close(true).unwrap();
}

fn persisted(
    f: &Fixture,
) -> (
    DecodeContext,
    Vec<RecordIdentity>,
    vellum_editor::CommentHandle,
) {
    let mut app = f.app(0);
    create(&mut app, "\u{feff}inert\r\nGrüße\x1b");
    let handle = app.comments().unwrap()[0].handle.clone();
    app.bind_comment_file(&f.notes()).unwrap();
    let snapshot = app.capture_comments().unwrap();
    let mapping = aliases(&snapshot);
    app.save_comments(&snapshot, &mapping, limits()).unwrap();
    let context = DecodeContext {
        document: mapping.document.identity.clone(),
        source: mapping.sources[0].evidence.clone(),
        text: snapshot.source_text().into(),
    };
    let records = mapping.records.into_iter().map(|r| r.identity).collect();
    drop(snapshot);
    app.close(false).unwrap();
    (context, records, handle)
}
fn associations_for(
    app: &mut Editor,
    context: &DecodeContext,
    records: &[RecordIdentity],
) -> RestorationAssociations {
    let witness = app.capture_comments().unwrap();
    RestorationAssociations {
        document: DocumentAssociation {
            native: witness.source().document.clone(),
            identity: context.document.clone(),
        },
        records: records.to_vec(),
        sources: vec![RestorationSourceAssociation {
            witness,
            evidence: context.source.clone(),
        }],
    }
}

#[test]
fn fresh_editor_reopens_exact_comments_with_new_native_ids_loaded_baseline_and_truthful_cleanup() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let (context, identities, old_handle) = persisted(&f);
        let source = fs::read(f.source()).unwrap();
        let json = fs::read(f.notes()).unwrap();
        let mut app = f.app(capacity);
        app.bind_comment_file(&f.notes()).unwrap();
        let approved = associations_for(&mut app, &context, &identities);
        let result = app
            .load_comments(&context, &approved, limits(), 65536)
            .unwrap();
        assert!(result.changed && result.loaded_baseline_adopted && !result.evidence_contested);
        assert_eq!(result.annotations.len(), 2);
        let status = app.last_comment_read_status().unwrap().unwrap();
        assert!(status.complete && status.quiescent && status.restored && !status.contested);
        assert!(app.last_comment_save_outcome().unwrap().is_none());
        assert!(!app.is_dirty().unwrap());
        let rows = app.comments().unwrap();
        assert!(rows.iter().all(|r| r.body == "\u{feff}inert\r\nGrüße\x1b"));
        assert!(matches!(
            rows[0].authored_target,
            vellum_editor::CommentTarget::Point {
                profile: CommentPointProfile::Position,
                ..
            }
        ));
        assert!(matches!(
            rows[1].authored_target,
            vellum_editor::CommentTarget::Point {
                profile: CommentPointProfile::Content,
                ..
            }
        ));
        assert!(
            app.update_comment(&old_handle, "cannot revive old-process handle")
                .is_err()
        );
        assert!(!app.is_dirty().unwrap());
        assert!(app.retire_failed_comment_read().is_err());
        assert_eq!(fs::read(f.source()).unwrap(), source);
        assert_eq!(fs::read(f.notes()).unwrap(), json);
        drop(approved);
        if capacity == 256 {
            let mut boundaries = Vec::new();
            while let Some(o) = app.pop_diagnostic() {
                boundaries.push(o.boundary);
            }
            for b in [
                Boundary::AnnotationReadCapture,
                Boundary::AnnotationRestorationPublication,
                Boundary::LoadedBaselineAdoption,
            ] {
                assert!(boundaries.contains(&b), "{b:?}");
            }
        }
        app.close(false).unwrap();
        assert!(app.is_closed());
        assert_eq!(app.native_view_usage().live, 0);
        assert_eq!(fs::read(f.notes()).unwrap(), json);
    }
}

#[test]
fn baseline_failure_does_not_hide_restoration_and_explicit_checkpoint_discharges_original_read() {
    let f = Fixture::new();
    let (context, identities, _) = persisted(&f);
    let json = fs::read(f.notes()).unwrap();
    let mut app = f.app(256);
    app.bind_comment_file(&f.notes()).unwrap();
    let approved = associations_for(&mut app, &context, &identities);
    assert!(app.load_comments(&context, &approved, limits(), 0).is_err());
    assert!(app.last_comment_read_status().unwrap().unwrap().restored);
    let original = app.last_comment_restoration_outcome().unwrap().unwrap();
    assert!(original.changed && !original.loaded_baseline_adopted);
    assert_eq!(app.comments().unwrap().len(), 2);
    assert!(app.comments_dirty().unwrap());
    assert!(matches!(app.close(false), Err(EditorError::Unsaved)));
    assert!(
        app.load_comments(&context, &approved, limits(), 65536)
            .is_err()
    );
    assert_eq!(
        app.last_comment_restoration_outcome().unwrap().unwrap(),
        original
    );
    assert_eq!(fs::read(f.notes()).unwrap(), json);
    assert!(app.adopt_comment_read_baseline(65536).unwrap());
    assert!(!app.comments_dirty().unwrap());
    assert!(
        app.last_comment_restoration_outcome()
            .unwrap()
            .unwrap()
            .loaded_baseline_adopted
    );
    let handle = app.comments().unwrap()[0].handle.clone();
    app.update_comment(&handle, "edited after reload").unwrap();
    assert!(app.comments_dirty().unwrap());
    save(&mut app);
    assert!(!app.comments_dirty().unwrap());
    assert!(app.retire_checkpointed_comment_read().unwrap());
    assert!(app.last_comment_read_status().unwrap().is_none());
    assert!(!app.retire_checkpointed_comment_read().unwrap());
    drop(approved);
    app.close(false).unwrap();
}

#[test]
fn malformed_file_wrong_approval_and_authored_comments_never_become_a_successful_load() {
    let f = Fixture::new();
    let (context, identities, _) = persisted(&f);
    let original = fs::read(f.notes()).unwrap();
    let mut app = f.app(256);
    app.bind_comment_file(&f.notes()).unwrap();
    let mut approved = associations_for(&mut app, &context, &identities);
    approved.records[0].incarnation += 1;
    assert!(
        app.load_comments(&context, &approved, limits(), 65536)
            .is_err()
    );
    assert!(app.comments().unwrap().is_empty());
    assert!(!app.last_comment_read_status().unwrap().unwrap().restored);
    assert!(app.last_comment_restoration_outcome().is_err());
    assert!(app.retire_failed_comment_read().unwrap());
    assert!(app.last_comment_read_status().unwrap().is_none());
    create(&mut app, "my unsaved comments");
    assert!(
        app.load_comments(&context, &approved, limits(), 65536)
            .is_err()
    );
    assert!(app.last_comment_read_status().unwrap().is_none());
    assert!(
        app.comments()
            .unwrap()
            .iter()
            .all(|r| r.body == "my unsaved comments")
    );
    assert_eq!(fs::read(f.notes()).unwrap(), original);
    drop(approved);
    app.close(true).unwrap();
    fs::write(f.notes(), b"{broken JSON").unwrap();
    let mut app = f.app(256);
    app.bind_comment_file(&f.notes()).unwrap();
    let approved = associations_for(&mut app, &context, &identities);
    assert!(
        app.load_comments(&context, &approved, limits(), 65536)
            .is_err()
    );
    let status = app.last_comment_read_status().unwrap().unwrap();
    assert!(status.complete && status.quiescent && !status.restored);
    assert!(app.comments().unwrap().is_empty());
    assert_eq!(fs::read(f.notes()).unwrap(), b"{broken JSON");
    drop(approved);
    app.close(false).unwrap();
}

#[test]
fn historical_targets_require_actual_source_witnesses_not_just_json_labels_or_matching_lengths() {
    let f = Fixture::new();
    let mut app = f.app(256);
    create(&mut app, "historical comment");
    let historical = app.capture_comments().unwrap();
    app.apply(Action::Insert("X".into())).unwrap();
    app.apply(Action::Save).unwrap();
    app.bind_comment_file(&f.notes()).unwrap();
    let current = app.capture_comments().unwrap();
    let mut mapping = aliases(&current);
    mapping.sources.push(SourceAssociation {
        native: historical.source().clone(),
        evidence: SourceEvidence {
            domain: "test-exact-source".into(),
            token: "historical".into(),
            byte_length: historical.source_text().len() as u64,
        },
        text: historical.source_text().into(),
    });
    app.save_comments(&current, &mapping, limits()).unwrap();
    let context = DecodeContext {
        document: mapping.document.identity,
        source: mapping.sources[0].evidence.clone(),
        text: current.source_text().into(),
    };
    let identities: Vec<_> = mapping.records.into_iter().map(|r| r.identity).collect();
    drop(current);
    drop(historical);
    app.close(false).unwrap();
    let json = fs::read(f.notes()).unwrap();
    let decoded = decode(&json, &context, limits(), &mut Recorder::disabled()).unwrap();
    assert!(decoded.needs_historical_evidence());
    let mut app = f.app(256);
    app.bind_comment_file(&f.notes()).unwrap();
    let approved = associations_for(&mut app, &context, &identities);
    assert!(
        app.load_comments(&context, &approved, limits(), 65536)
            .is_err()
    );
    assert!(app.comments().unwrap().is_empty());
    assert!(!app.last_comment_read_status().unwrap().unwrap().restored);
    assert_eq!(fs::read(f.notes()).unwrap(), json);
    assert_eq!(fs::read(f.source()).unwrap(), "\u{feff}XABC\r\n".as_bytes());
    drop(approved);
    app.close(false).unwrap();
}

fn managed(f: &Fixture) -> Editor {
    let mut app = f.app(256);
    app.enable_comment_files(vellum_editor::CommentFileLimits::reference())
        .unwrap();
    app
}
fn stored(f: &Fixture, app: &mut Editor) -> Snapshot {
    let bytes = fs::read(f.notes()).unwrap();
    let profile = vellum_editor::CommentFileLimits::reference().codec;
    let declared = inspect_context(&bytes, profile).unwrap();
    let text = app.snapshot().unwrap().text;
    decode(
        &bytes,
        &DecodeContext {
            document: declared.document,
            source: declared.source,
            text,
        },
        profile,
        &mut Recorder::disabled(),
    )
    .unwrap()
}
#[test]
fn managed_uuid_names_survive_body_history_save_as_and_fresh_owner_round_trip() {
    let f = Fixture::new();
    let other = Fixture::new();
    let mut app = managed(&f);
    create(&mut app, "\u{feff}raw\r\n$(inert)\x1b");
    let plan = app.plan_comment_file(f.notes(), false).unwrap();
    app.confirm_comment_file(&plan).unwrap();
    let original = stored(&f, &mut app);
    assert_eq!(original.document.value.len(), 36);
    assert_eq!(&original.document.value[14..15], "4");
    assert_ne!(original.records[0].identity, original.records[1].identity);
    assert!(app.confirm_comment_file(&plan).is_err());
    let handle = app.comments().unwrap()[0].handle.clone();
    app.update_comment(&handle, "changed\n\rß").unwrap();
    app.save_comment_file().unwrap();
    let edited = stored(&f, &mut app);
    assert_eq!(original.document, edited.document);
    assert_eq!(original.records[0].identity, edited.records[0].identity);
    let export = app.plan_destination(other.source(), true).unwrap();
    app.confirm_destination(&export).unwrap();
    assert_eq!(stored(&f, &mut app).document, original.document);
    app.close(false).unwrap();
    let json = fs::read(f.notes()).unwrap();
    let mut reopened = managed(&other);
    let plan = reopened.plan_comment_file(f.notes(), true).unwrap();
    reopened.confirm_comment_file(&plan).unwrap();
    assert!(!reopened.is_dirty().unwrap());
    assert_eq!(reopened.comments().unwrap()[0].body, "changed\n\rß");
    assert!(
        reopened
            .last_comment_restoration_outcome()
            .unwrap()
            .unwrap()
            .loaded_baseline_adopted
    );
    reopened.save_comment_file().unwrap(); // Unchanged Loaded is not Written.
    assert!(reopened.last_comment_save_outcome().unwrap().is_none());
    assert_eq!(fs::read(f.notes()).unwrap(), json);
    let handle = reopened.comments().unwrap()[0].handle.clone();
    reopened.update_comment(&handle, "after reopen").unwrap();
    reopened.save_comment_file().unwrap();
    assert_eq!(
        stored(&f, &mut reopened).records[0].identity,
        original.records[0].identity
    );
    reopened.close(false).unwrap();
}
#[test]
fn managed_stale_plans_and_host_retention_refusals_do_not_publish_or_write() {
    let f = Fixture::new();
    let mut app = managed(&f);
    let plan = app.plan_comment_file(f.notes(), false).unwrap();
    create(&mut app, "note");
    assert!(app.confirm_comment_file(&plan).is_err());
    assert!(!app.comment_file_bound());
    assert_eq!(fs::read(f.notes()).unwrap(), b"old destination");
    let plan = app.plan_comment_file(f.notes(), false).unwrap();
    app.cancel_comment_file_plan();
    assert!(app.confirm_comment_file(&plan).is_err());
    app.close(true).unwrap();
    let mut app = f.app(256);
    let mut profile = vellum_editor::CommentFileLimits::reference();
    profile.max_source_bytes = 1;
    app.enable_comment_files(profile).unwrap();
    let before = app.version().unwrap();
    let draft = app.begin_comment(CommentPointProfile::Content).unwrap();
    assert!(
        app.create_comments(&draft, "refused before native publication")
            .is_err()
    );
    assert!(app.comments().unwrap().is_empty());
    assert_eq!(app.version().unwrap(), before);
    app.close(false).unwrap();
}
#[test]
fn managed_historical_json_is_preserved_and_failed_load_cannot_be_saved_as_empty() {
    let f = Fixture::new();
    let mut app = managed(&f);
    create(&mut app, "history");
    app.apply(Action::Insert("X".into())).unwrap();
    app.apply(Action::Save).unwrap();
    app.bind_comment_file(&f.notes()).unwrap();
    app.save_comment_file().unwrap();
    assert!(stored(&f, &mut app).needs_historical_evidence());
    app.close(false).unwrap();
    let json = fs::read(f.notes()).unwrap();
    let mut app = managed(&f);
    let plan = app.plan_comment_file(f.notes(), true).unwrap();
    assert!(app.confirm_comment_file(&plan).is_err());
    assert!(app.comments().unwrap().is_empty());
    assert!(app.save_comment_file().is_err());
    app.retire_failed_comment_read().unwrap();
    assert!(app.save_comment_file().is_err());
    assert_eq!(fs::read(f.notes()).unwrap(), json);
    app.close(false).unwrap();
}
