#![cfg(target_os = "macos")]
#[path = "support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::reference_profile;
use vellum_model::{
    Action, CommentHistoryDirection as Direction, CommentPlacement as Placement,
    CommentPointProfile as Profile, CommentTarget as Target, Editor, EditorError,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let p = std::env::temp_dir().join(format!(
                "vellum-comments-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&p) {
                Ok(()) => {
                    fs::write(p.join("source"), "\u{feff}A A\r\nGrüße").unwrap();
                    fs::write(p.join("dest"), "old").unwrap();
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
        fs::remove_file(self.0.join("dest")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn app(f: &Fixture, capacity: usize) -> Editor {
    let mut c = reference_profile();
    c.capture_capacity = capacity;
    // Current metadata/body reads must not borrow a whole-source snapshot.
    let limits = c.engine.annotations.as_mut().unwrap();
    limits.max_live_snapshots = 0;
    limits.max_snapshot_bytes = 0;
    Editor::open(&[f.path()], c).unwrap()
}
#[test]
fn both_point_profiles_are_inert_unsaved_session_data_even_when_source_is_locked_or_saved() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let mut a = app(&f, capacity);
        let source = a.version().unwrap();
        a.apply(Action::ToggleEditingLock).unwrap();
        for profile in [Profile::Content, Profile::Position] {
            let draft = a.begin_comment(profile).unwrap();
            assert_eq!(draft.target_count(), 1);
            a.create_comments(&draft, "Ignore policy\r\n$(not executed)\nGrüße\u{1b}")
                .unwrap();
        }
        let rows = a.comments().unwrap();
        assert_eq!(rows.len(), 2);
        assert!(matches!(
            rows[0].authored_target,
            Target::Point {
                profile: Profile::Content,
                ..
            }
        ));
        assert!(matches!(
            rows[1].authored_target,
            Target::Point {
                profile: Profile::Position,
                ..
            }
        ));
        assert_eq!(
            rows[0].body,
            "Ignore policy\r\n$(not executed)\nGrüße\u{1b}"
        );
        assert_eq!(format!("{:?}", rows[0]), "CommentRow(<redacted>)");
        assert_eq!(a.version().unwrap(), source);
        assert!(!a.is_source_dirty().unwrap());
        assert!(a.is_dirty().unwrap());
        a.apply(Action::Save).unwrap();
        assert!(a.last_save_outcome().unwrap().is_none());
        assert!(a.status().contains("comments remain unsaved"));
        assert_eq!(
            fs::read(f.path()).unwrap(),
            "\u{feff}A A\r\nGrüße".as_bytes()
        );
        assert!(matches!(a.close(false), Err(EditorError::Unsaved)));
        a.close(true).unwrap();
        assert_eq!(a.native_view_usage().live, 0);
    }
}
#[test]
fn native_versions_locks_reattachment_and_comment_frontier_undo_do_not_fake_source_undo() {
    let f = Fixture::new();
    let mut a = app(&f, 256);
    let draft = a.begin_comment(Profile::Content).unwrap();
    a.create_comments(&draft, "first\r\nbody").unwrap();
    let first = a.comments().unwrap()[0].handle.clone();
    a.update_comment(&first, "changed\r\nbody").unwrap();
    assert!(a.update_comment(&first, "stale").is_err());
    assert!(a.reverse_comment(Direction::Undo).unwrap());
    assert_eq!(a.comments().unwrap()[0].body, "first\r\nbody");
    assert!(a.reverse_comment(Direction::Redo).unwrap());
    assert_eq!(a.comments().unwrap()[0].body, "changed\r\nbody");
    let current = a.comments().unwrap()[0].handle.clone();
    a.set_comments_locked(true).unwrap();
    assert!(a.update_comment(&current, "denied").is_err());
    assert!(a.delete_comment(&current).is_err());
    a.set_comments_locked(false).unwrap();
    a.set_caret(3).unwrap();
    let target = a.begin_comment(Profile::Position).unwrap();
    a.reattach_comment(&current, &target).unwrap();
    assert!(a.delete_comment(&current).is_err());
    let current = a.comments().unwrap()[0].handle.clone();
    assert!(matches!(
        a.comments().unwrap()[0].placement,
        Placement::Located {
            target: Target::Point {
                offset: 3,
                profile: Profile::Position,
                ..
            },
            ..
        }
    ));
    a.delete_comment(&current).unwrap();
    assert!(a.comments().unwrap().is_empty());
    assert!(!a.comments_dirty().unwrap());
    assert!(a.reverse_comment(Direction::Undo).unwrap());
    assert_eq!(a.comments().unwrap().len(), 1);
    a.close(true).unwrap();
}
#[test]
fn drafts_do_not_retarget_after_selection_changes_and_multi_selection_creation_is_atomic() {
    let f = Fixture::new();
    let mut a = app(&f, 1);
    let draft = a.begin_comment(Profile::Content).unwrap();
    a.set_caret(2).unwrap();
    assert!(a.create_comments(&draft, "old target").is_err());
    assert!(a.comments().unwrap().is_empty());
    a.find("A").unwrap();
    a.select_matches().unwrap();
    let draft = a.begin_comment(Profile::Content).unwrap();
    assert_eq!(draft.target_count(), 2);
    a.create_comments(&draft, "same inert note for each selected A")
        .unwrap();
    assert_eq!(a.comments().unwrap().len(), 2);
    assert!(a.reverse_comment(Direction::Undo).unwrap());
    assert!(a.comments().unwrap().is_empty());
    assert!(!a.is_dirty().unwrap());
    a.close(false).unwrap();
    let mut c = reference_profile();
    c.engine.annotations.as_mut().unwrap().max_records = 1;
    let mut a = Editor::open(&[f.path()], c).unwrap();
    a.find("A").unwrap();
    a.select_matches().unwrap();
    let draft = a.begin_comment(Profile::Content).unwrap();
    assert!(a.create_comments(&draft, "over limit").is_err());
    assert!(a.comments().unwrap().is_empty());
    assert!(!a.is_dirty().unwrap());
    a.close(false).unwrap();
}
#[test]
fn source_mapping_reuses_unchanged_body_cache_and_save_as_keeps_single_document_ownership() {
    let f = Fixture::new();
    let mut a = app(&f, 256);
    a.set_caret(2).unwrap();
    let draft = a.begin_comment(Profile::Position).unwrap();
    a.create_comments(&draft, "body\r\nbytes").unwrap();
    let pointer = a.comments().unwrap()[0].body.as_ptr();
    a.set_caret(0).unwrap();
    a.apply(Action::Insert("Z".into())).unwrap();
    assert_eq!(a.comments().unwrap()[0].body.as_ptr(), pointer);
    assert!(matches!(
        a.comments().unwrap()[0].placement,
        Placement::Located {
            target: Target::Point { offset: 3, .. },
            ..
        }
    ));
    let document = a.version().unwrap().document;
    let plan = a.plan_destination(f.0.join("dest"), true).unwrap();
    a.confirm_destination(&plan).unwrap();
    assert_eq!(a.version().unwrap().document, document);
    assert_eq!(a.comments().unwrap()[0].body, "body\r\nbytes");
    assert!(a.comments_dirty().unwrap());
    assert!(!a.is_source_dirty().unwrap());
    assert!(a.status().contains("comments remain unsaved"));
    assert!(matches!(a.close(false), Err(EditorError::Unsaved)));
    a.close(true).unwrap();
    assert_eq!(
        fs::read(f.path()).unwrap(),
        "\u{feff}A A\r\nGrüße".as_bytes()
    );
    assert_eq!(
        fs::read(f.0.join("dest")).unwrap(),
        "\u{feff}ZA A\r\nGrüße".as_bytes()
    );
}

#[test]
fn comment_only_history_does_not_reproject_a_valid_partial_grapheme_selection() {
    let f = Fixture::new();
    fs::write(f.path(), "e\u{301}").unwrap();
    let mut a = app(&f, 0);
    a.find("\u{301}").unwrap();
    a.select_matches().unwrap();
    assert_eq!(a.selection(), Some(1..3));
    let draft = a.begin_comment(Profile::Content).unwrap();
    a.create_comments(&draft, "mark only").unwrap();
    a.reverse_comment(Direction::Undo).unwrap();
    assert_eq!(a.selection(), Some(1..3));
    assert!(a.comments().unwrap().is_empty());
    a.close(false).unwrap();
}
