#![cfg(target_os = "macos")]
#[path = "support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::reference_profile;
use vellum_editor::{Action, Editor, Movement};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "native-unicode")]
mod native_unicode {
    use super::*;
    use vellum_adapter_unicode_icu::IcuFactory;
    use vellum_editor::{EditorError, NativeUnicodeProfile};

    fn open(f: &Fixture, capacity: usize) -> Editor {
        let mut config = reference_profile();
        config.capture_capacity = capacity;
        let mut editor = Editor::open(&[f.path()], config).unwrap();
        let mut profile = NativeUnicodeProfile::reference();
        profile.capture_capacity = capacity;
        editor
            .enable_native_unicode(IcuFactory::reference(), profile)
            .unwrap();
        editor
    }
    #[test]
    fn native_18_commands_cache_current_evidence_and_preserve_history_fidelity_and_cleanup() {
        for capacity in [0, 1, 256] {
            let raw = "\u{feff}\u{94d}\u{915}\r\ne\u{301}";
            let f = Fixture::new(raw.as_bytes());
            let mut editor = open(&f, capacity);
            editor.apply(Action::Right).unwrap();
            assert_eq!(
                editor.cursor(),
                6,
                "Unicode 18 WB/GB profile must not use the old split"
            );
            for _ in 0..8 {
                editor.frame(80, 20).unwrap();
                editor.apply(Action::Left).unwrap();
                editor.apply(Action::Right).unwrap();
            }
            assert_eq!(editor.unicode_provider_usage().unwrap().completed, 1);
            assert_eq!(editor.native_boundary_usage().ready, 1);
            editor.apply(Action::Down).unwrap();
            assert_eq!(editor.cursor(), 11);
            editor.apply(Action::Up).unwrap();
            assert_eq!(editor.cursor(), 6);
            editor.apply(Action::Right).unwrap();
            assert_eq!(editor.cursor(), 8);
            editor.apply(Action::Backspace).unwrap();
            assert_eq!(editor.text().unwrap(), "\u{94d}\u{915}e\u{301}");
            editor.apply(Action::Undo).unwrap();
            assert_eq!(editor.text().unwrap(), &raw[3..]);
            editor.apply(Action::ToggleEditingLock).unwrap();
            let before = editor.version().unwrap();
            assert!(editor.apply(Action::Delete).is_err());
            assert_eq!(editor.version().unwrap(), before);
            editor.apply(Action::Left).unwrap();
            editor.apply(Action::ToggleEditingLock).unwrap();
            editor.close(false).unwrap();
            assert_eq!(editor.native_boundary_usage().retained_bytes, 0);
            assert_eq!(fs::read(f.path()).unwrap(), raw.as_bytes());
        }
    }

    #[test]
    fn native_multiselection_deletes_each_complete_cluster_in_one_reversible_edit() {
        let original = "\u{94d}\u{915}!\r\n\u{94d}\u{915}!";
        let f = Fixture::new(original.as_bytes());
        let mut editor = open(&f, 256);
        editor.find("!").unwrap();
        editor.select_matches().unwrap();
        assert_eq!(editor.selection_count(), 2);
        editor.apply(Action::Left).unwrap();
        editor.apply(Action::Backspace).unwrap();
        assert_eq!(editor.text().unwrap(), "!\r\n!");
        editor.apply(Action::Undo).unwrap();
        assert_eq!(editor.text().unwrap(), original);
        editor.close(false).unwrap();
        assert_eq!(
            editor.native_boundary_usage().pending + editor.native_boundary_usage().ready,
            0
        );
    }

    #[test]
    fn provider_refusal_reuses_original_capture_without_legacy_fallback_and_close_releases_it() {
        let f = Fixture::new(b"a");
        let mut editor = Editor::open(&[f.path()], reference_profile()).unwrap();
        let profile = NativeUnicodeProfile::reference();
        let mut factory = IcuFactory::reference();
        factory.limits.max_source_bytes = 0;
        editor.enable_native_unicode(factory, profile).unwrap();
        for _ in 0..2 {
            assert!(matches!(
                editor.apply(Action::Right),
                Err(EditorError::Unicode(_))
            ));
            assert_eq!(editor.cursor(), 0);
            assert_eq!(editor.native_boundary_usage().pending, 1);
            assert_eq!(editor.native_boundary_usage().issued_lifetime, 1);
        }
        assert_eq!(editor.unicode_provider_usage().unwrap().admitted, 0);
        editor.close(false).unwrap();
        assert_eq!(editor.native_boundary_usage().retained_bytes, 0);
    }

    #[test]
    fn gesture_projection_is_explicit_version_bound_and_never_repairs_exact_targets() {
        let f = Fixture::new("\u{94d}कX".as_bytes());
        let mut editor = open(&f, 256);
        let version = editor.version().unwrap();
        assert_eq!(editor.project_source_caret_after(&version, 3).unwrap(), 6);
        assert!(
            editor
                .native_grapheme_deletion_target(&version, 3, true)
                .is_err()
        );
        assert_eq!(
            editor
                .native_grapheme_deletion_target(&version, 6, true)
                .unwrap(),
            0..6
        );
        let offered = editor.unicode_provider_usage().unwrap().offered;
        assert!(matches!(
            editor.project_source_caret_after(&version, 1),
            Err(EditorError::Position)
        ));
        assert_eq!(editor.unicode_provider_usage().unwrap().offered, offered);
        editor.apply(Action::Insert("Q".into())).unwrap();
        let offered = editor.unicode_provider_usage().unwrap().offered;
        assert!(editor.project_source_caret_after(&version, 3).is_err());
        assert_eq!(editor.unicode_provider_usage().unwrap().offered, offered);
        editor.close(true).unwrap();
    }

    #[test]
    fn projection_failure_after_publication_does_not_erase_authored_source_or_discard_consent() {
        let f = Fixture::new(b"");
        let mut editor = Editor::open(&[f.path()], reference_profile()).unwrap();
        let profile = NativeUnicodeProfile::reference();
        let mut factory = IcuFactory::reference();
        factory.limits.max_source_bytes = 0;
        editor.enable_native_unicode(factory, profile).unwrap();
        assert!(matches!(
            editor.apply(Action::Insert("a".into())),
            Err(EditorError::Unicode(_))
        ));
        assert_eq!(editor.text().unwrap(), "a");
        assert!(editor.status().contains("Native state changed"));
        assert!(editor.is_dirty().unwrap());
        assert!(matches!(editor.close(false), Err(EditorError::Unsaved)));
        assert_eq!(editor.native_boundary_usage().pending, 1);
        editor.close(true).unwrap();
        assert_eq!(editor.native_boundary_usage().retained_bytes, 0);
        assert_eq!(fs::read(f.path()).unwrap(), b"");
    }
}
struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-ui-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("source"), bytes).unwrap();
                    return Self(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture directory: {e}"),
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

#[test]
fn selected_text_is_one_native_edit_with_policy_history_and_lossless_save() {
    let original = "\u{feff}A👩‍💻\r\nDeutsch\n```mermaid\rgraph LR";
    let source = original.strip_prefix('\u{feff}').unwrap();
    for capacity in [0, 1, 256] {
        let f = Fixture::new(original.as_bytes());
        let mut config = reference_profile();
        config.capture_capacity = capacity;
        let mut app = Editor::open(&[f.path()], config).unwrap();
        app.set_caret(0).unwrap();
        app.apply(Action::ExtendSelection(Movement::Right)).unwrap();
        app.apply(Action::ExtendSelection(Movement::Right)).unwrap();
        let selected = Some(0.."A👩‍💻".len());
        assert_eq!(app.selection(), selected);
        assert!(!app.is_dirty().unwrap());
        app.apply(Action::ToggleEditingLock).unwrap();
        assert!(app.apply(Action::Insert("blocked".into())).is_err());
        assert_eq!(app.selection(), selected);
        assert_eq!(app.text().unwrap(), source);
        app.apply(Action::ToggleEditingLock).unwrap();
        app.apply(Action::Insert("Grüße\n".into())).unwrap();
        assert!(app.selection().is_none());
        assert_eq!(
            app.text().unwrap(),
            "Grüße\n\r\nDeutsch\n```mermaid\rgraph LR"
        );
        app.apply(Action::Undo).unwrap();
        assert_eq!(app.text().unwrap(), source);
        assert!(!app.is_dirty().unwrap());
        app.apply(Action::Redo).unwrap();
        app.apply(Action::Save).unwrap();
        let expected = app.text().unwrap().to_owned();
        assert_eq!(
            fs::read(f.path()).unwrap(),
            format!("\u{feff}{expected}").as_bytes()
        );
        app.close(false).unwrap();
        let mut reopened = Editor::open(&[f.path()], reference_profile()).unwrap();
        assert_eq!(reopened.text().unwrap(), expected);
        reopened.close(false).unwrap();
    }
}

#[test]
fn selection_reverses_across_anchor_and_deletes_whole_graphemes_and_crlf() {
    for deletion in [Action::Backspace, Action::Delete] {
        let f = Fixture::new("e\u{301}👩‍💻\r\nB".as_bytes());
        let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
        app.apply(Action::Right).unwrap();
        let anchor = app.cursor();
        app.apply(Action::ExtendSelection(Movement::Left)).unwrap();
        assert_eq!(app.selection(), Some(0..anchor));
        app.apply(Action::ExtendSelection(Movement::Right)).unwrap();
        assert!(app.selection().is_none());
        app.apply(Action::ExtendSelection(Movement::Right)).unwrap();
        app.apply(Action::ExtendSelection(Movement::Right)).unwrap();
        assert_eq!(app.selection(), Some(anchor.."e\u{301}👩‍💻\r\n".len()));
        app.apply(deletion).unwrap();
        assert_eq!(app.text().unwrap(), "e\u{301}B");
        app.apply(Action::Undo).unwrap();
        assert_eq!(app.text().unwrap(), "e\u{301}👩‍💻\r\nB");
        app.close(false).unwrap();
    }
}

#[test]
fn selection_cells_clip_tabs_wide_clusters_empty_lines_and_selected_delimiters() {
    let f = Fixture::new("\t界e\u{301}\r\n\nZ".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.apply(Action::SelectAll).unwrap();
    let view = app.frame(4, 3).unwrap();
    assert_eq!(
        view.selection_cells,
        vec![vec![0..4], vec![0..1], vec![0..1]]
    );
    app.set_caret(0).unwrap();
    app.apply(Action::ExtendSelection(Movement::End)).unwrap();
    let view = app.frame(4, 3).unwrap();
    assert_eq!(view.selection_cells, vec![vec![0..3], vec![], vec![]]);
    let view = app.frame(0, 0).unwrap();
    assert!(view.cursor.is_none());
    assert!(view.selection_cells.is_empty());
    app.close(false).unwrap();
}

#[test]
fn extended_vertical_and_line_navigation_is_non_authoring() {
    let f = Fixture::new(b"abcd\nxy\n1234");
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    let before = app.snapshot().unwrap().version;
    app.set_caret(2).unwrap();
    for (movement, caret) in [
        (Movement::Down, 7),
        (Movement::Down, 10),
        (Movement::Up, 7),
        (Movement::Home, 5),
        (Movement::Up, 0),
        (Movement::End, 4),
    ] {
        app.apply(Action::ExtendSelection(movement)).unwrap();
        assert_eq!(app.cursor(), caret);
        assert_eq!(app.selection(), Some(caret.min(2)..caret.max(2)));
        assert_eq!(app.version().unwrap(), before);
        assert!(!app.is_dirty().unwrap());
    }
    app.close(false).unwrap();
}

#[test]
fn reference_selection_is_the_original_native_owner_and_close_releases_each_view() {
    let a = Fixture::new(b"ABC");
    let b = Fixture::new(b"DEF");
    let mut app = Editor::open(&[a.path(), b.path()], reference_profile()).unwrap();
    assert_eq!(app.native_view_usage().attached, 2);
    let first = app.native_selection().unwrap().unwrap().view;
    app.apply(Action::SelectAll).unwrap();
    let selected = app.native_selection().unwrap().unwrap();
    assert_eq!(selected.source.as_ref(), Some(&app.version().unwrap()));
    let selection = selected.selections.selections()[0];
    assert_eq!((selection.anchor.0, selection.head.0), (0, 3));
    app.apply(Action::Insert("Z".into())).unwrap();
    let mapped = app.native_selection().unwrap().unwrap();
    assert_eq!(
        (
            mapped.selections.selections()[0].anchor.0,
            mapped.selections.selections()[0].head.0
        ),
        (1, 1)
    );
    app.apply(Action::NextDocument).unwrap();
    assert_ne!(app.native_selection().unwrap().unwrap().view, first);
    app.open_additional(&a.path()).unwrap();
    assert_eq!(app.native_selection().unwrap().unwrap().view, first);
    assert_eq!(app.native_view_usage().issued_lifetime, 2);
    assert!(app.close(false).is_err());
    assert_eq!(app.native_view_usage().attached, 2);
    app.close(true).unwrap();
    assert_eq!(app.native_view_usage().live, 0);
}

#[test]
fn stale_or_invalid_native_selection_never_retargets_the_presentation_cache() {
    let f = Fixture::new("é\r\nX".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    let before = app.version().unwrap();
    assert!(app.set_selection(&before, 1, 2).is_err());
    assert_eq!(app.cursor(), 0);
    assert!(app.set_selection(&before, 2, 6).is_err());
    assert_eq!(app.cursor(), 0);
    app.apply(Action::Insert("Y".into())).unwrap();
    let current = app.native_selection().unwrap().unwrap();
    assert!(app.set_selection(&before, 0, 0).is_err());
    assert_eq!(
        app.native_selection().unwrap().unwrap().context(),
        current.context()
    );
    assert_eq!(app.cursor(), 1);
    app.close(true).unwrap();
}

#[test]
fn joining_a_cluster_projects_ui_graphemes_without_claiming_an_engine_unicode_provider() {
    let f = Fixture::new("\u{301}X".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.apply(Action::Insert("e".into())).unwrap();
    assert_eq!(app.text().unwrap(), "e\u{301}X");
    assert_eq!(app.cursor(), 3);
    assert_eq!(
        app.native_selection()
            .unwrap()
            .unwrap()
            .selections
            .selections()[0]
            .head
            .0,
        3
    );
    app.apply(Action::Backspace).unwrap();
    assert_eq!(app.text().unwrap(), "X");
    app.close(true).unwrap();
}

#[test]
fn native_column_outdent_and_indent_share_history_and_preserve_other_bytes() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new(" \tprose\r\n\t\tcode\n".as_bytes());
        let mut profile = reference_profile();
        profile.capture_capacity = capacity;
        let mut app = Editor::open(&[f.path()], profile).unwrap();
        app.apply(Action::SelectAll).unwrap();
        app.apply(Action::Outdent).unwrap();
        assert_eq!(app.text().unwrap(), "prose\r\n    code\n");
        app.apply(Action::Undo).unwrap();
        assert_eq!(app.text().unwrap(), " \tprose\r\n\t\tcode\n");
        app.apply(Action::Indent).unwrap();
        assert_eq!(app.text().unwrap(), "     \tprose\r\n    \t\tcode\n");
        app.apply(Action::Save).unwrap();
        assert_eq!(
            fs::read(f.path()).unwrap(),
            "     \tprose\r\n    \t\tcode\n".as_bytes()
        );
        if capacity == 256 {
            let mut seen = false;
            while let Some(record) = app.pop_diagnostic() {
                seen |= record.boundary
                    == vellum_engine::instrumentation::Boundary::LineEditPreparation;
            }
            assert!(seen);
        }
        app.close(false).unwrap();
        assert_eq!(app.native_view_usage().live, 0);
    }
}

#[test]
fn unavailable_or_exhausted_line_profile_never_falls_back_to_ui_edits() {
    for unavailable in [false, true] {
        let f = Fixture::new(b"A\r\nB\n");
        let mut profile = reference_profile();
        if unavailable {
            profile.line_edits = None;
        } else {
            profile.line_edits.as_mut().unwrap().max_lines = 1;
        }
        let mut app = Editor::open(&[f.path()], profile).unwrap();
        app.apply(Action::SelectAll).unwrap();
        let before = app.native_selection().unwrap().unwrap().context();
        let version = app.version().unwrap();
        assert!(app.apply(Action::Indent).is_err());
        assert_eq!(app.native_selection().unwrap().unwrap().context(), before);
        assert_eq!(app.version().unwrap(), version);
        assert_eq!(app.text().unwrap(), "A\r\nB\n");
        assert!(!app.is_dirty().unwrap());
        app.apply(Action::Insert("ordinary edits still work".into()))
            .unwrap();
        assert_eq!(fs::read(f.path()).unwrap(), b"A\r\nB\n");
        app.close(true).unwrap();
    }
}

#[test]
fn complete_matches_become_native_multi_selections_for_typing_movement_history_and_save() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new("name\r\nname 界".as_bytes());
        let mut profile = reference_profile();
        profile.capture_capacity = capacity;
        let mut app = Editor::open(&[f.path()], profile).unwrap();
        app.find("name").unwrap();
        app.select_matches().unwrap();
        assert_eq!(app.selection_count(), 2);
        let selected = app.native_selection().unwrap().unwrap().context();
        app.navigate_match(vellum_editor::SearchDirection::Previous, false)
            .unwrap();
        assert_eq!(app.native_selection().unwrap().unwrap().context(), selected);
        assert_eq!(app.selection_count(), 2);
        let frame = app.frame(40, 3).unwrap();
        assert_eq!(frame.selection_count, 2);
        assert_eq!(frame.selection_cells, vec![vec![0..4], vec![0..4]]);
        app.apply(Action::Insert("ü".into())).unwrap();
        assert_eq!(app.text().unwrap(), "ü\r\nü 界");
        assert_eq!(app.selection_count(), 2);
        let native = app.native_selection().unwrap().unwrap();
        assert_eq!(
            native
                .selections
                .selections()
                .iter()
                .map(|s| s.head.0)
                .collect::<Vec<_>>(),
            vec![2, 6]
        );
        app.apply(Action::Left).unwrap();
        assert_eq!(
            app.native_selection()
                .unwrap()
                .unwrap()
                .selections
                .selections()
                .iter()
                .map(|s| s.head.0)
                .collect::<Vec<_>>(),
            vec![0, 4]
        );
        app.apply(Action::ExtendSelection(Movement::Right)).unwrap();
        app.apply(Action::Delete).unwrap();
        assert_eq!(app.text().unwrap(), "\r\n 界");
        app.apply(Action::Undo).unwrap();
        assert_eq!(app.text().unwrap(), "ü\r\nü 界");
        app.apply(Action::Save).unwrap();
        assert_eq!(fs::read(f.path()).unwrap(), "ü\r\nü 界".as_bytes());
        app.close(false).unwrap();
        assert_eq!(app.native_view_usage().live, 0);
    }
}

#[test]
fn multi_caret_backspace_preserves_graphemes_and_policy_refusal_is_not_primary_only() {
    let f = Fixture::new("e\u{301}X e\u{301}X".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.find("X").unwrap();
    app.select_matches().unwrap();
    app.apply(Action::Insert("界".into())).unwrap();
    assert_eq!(app.text().unwrap(), "e\u{301}界 e\u{301}界");
    app.apply(Action::ToggleEditingLock).unwrap();
    let before = app.native_selection().unwrap().unwrap().context();
    assert!(app.apply(Action::Backspace).is_err());
    assert_eq!(app.native_selection().unwrap().unwrap().context(), before);
    app.apply(Action::ToggleEditingLock).unwrap();
    app.apply(Action::Backspace).unwrap();
    assert_eq!(app.text().unwrap(), "e\u{301} e\u{301}");
    app.apply(Action::Backspace).unwrap();
    assert_eq!(app.text().unwrap(), " ");
    app.apply(Action::Backspace).unwrap();
    assert_eq!(app.text().unwrap(), "");
    assert_eq!(app.selection_count(), 1);
    app.close(true).unwrap();
}

#[test]
fn combining_mark_matches_widen_only_paint_not_logical_replacement_targets() {
    let f = Fixture::new("e\u{301} e\u{301}".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.find("\u{301}").unwrap();
    app.select_matches().unwrap();
    assert_eq!(
        app.frame(40, 2).unwrap().selection_cells,
        vec![vec![0..1, 2..3]]
    );
    app.apply(Action::Insert("".into())).unwrap();
    assert_eq!(app.text().unwrap(), "e e");
    app.close(true).unwrap();
}

#[test]
fn stale_partial_or_over_budget_matches_cannot_change_the_native_selection_set() {
    for condition in 0..3 {
        let f = Fixture::new(b"name name name");
        let mut profile = reference_profile();
        if condition == 0 {
            profile.engine.search.as_mut().unwrap().max_matches = 1;
        }
        if condition == 1 {
            profile
                .logical_views
                .as_mut()
                .unwrap()
                .normalization
                .max_input_selections = 2;
        }
        let mut app = Editor::open(&[f.path()], profile).unwrap();
        app.find("name").unwrap();
        if condition == 2 {
            app.apply(Action::Insert("X".into())).unwrap();
        }
        let before = app.native_selection().unwrap().unwrap().context();
        assert!(app.select_matches().is_err());
        assert_eq!(app.selection_count(), 1);
        assert_eq!(app.native_selection().unwrap().unwrap().context(), before);
        assert_eq!(fs::read(f.path()).unwrap(), b"name name name");
        app.close(condition == 2).unwrap();
    }
}

#[test]
fn view_capacity_failure_preserves_active_source_and_tracks_cleanup_without_a_fallback_write() {
    let a = Fixture::new(b"ABC");
    let b = Fixture::new(b"DEF");
    let mut profile = reference_profile();
    let limits = profile.logical_views.as_mut().unwrap();
    limits.max_live_views = 1;
    limits.max_issued_views = 1;
    let mut app = Editor::open(&[a.path()], profile).unwrap();
    assert!(app.open_additional(&b.path()).is_err());
    assert_eq!(app.active_document(), 0);
    assert_eq!(app.text().unwrap(), "ABC");
    assert_eq!(app.native_view_usage().live, 1);
    assert!(app.apply(Action::NextDocument).is_err());
    assert_eq!(app.active_document(), 0);
    app.apply(Action::Insert("X".into())).unwrap();
    assert_eq!(app.text().unwrap(), "XABC");
    assert_eq!(fs::read(b.path()).unwrap(), b"DEF");
    app.close(true).unwrap();
    assert_eq!(app.native_view_usage().live, 0);
}

#[test]
fn per_tab_selection_collapses_and_source_publication_never_reuses_old_anchor() {
    let a = Fixture::new(b"ABC");
    let b = Fixture::new(b"DEF");
    let mut app = Editor::open(&[a.path(), b.path()], reference_profile()).unwrap();
    app.apply(Action::SelectAll).unwrap();
    app.apply(Action::NextDocument).unwrap();
    assert!(app.selection().is_none());
    app.apply(Action::NextDocument).unwrap();
    assert_eq!(app.selection(), Some(0..3));
    app.apply(Action::Left).unwrap();
    assert_eq!(app.cursor(), 0);
    assert!(app.selection().is_none());
    app.apply(Action::SelectAll).unwrap();
    app.apply(Action::Right).unwrap();
    assert_eq!(app.cursor(), 3);
    assert!(app.selection().is_none());
    let captured = app.snapshot().unwrap();
    app.apply(Action::SelectAll).unwrap();
    app.edit_snapshot(
        &captured.version,
        vec![vellum_engine::source::Replacement {
            start: 0,
            end: 3,
            expected: "ABC".into(),
            inserted: "Z".into(),
        }],
        1,
    )
    .unwrap();
    assert!(app.selection().is_none());
    app.apply(Action::SelectAll).unwrap();
    assert!(app.edit_snapshot(&captured.version, vec![], 0).is_err());
    assert_eq!(app.selection(), Some(0..1));
    app.close(true).unwrap();
}

#[test]
fn type_history_and_save_preserve_original_bom_and_mixed_endings_in_all_capture_modes() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new("\u{feff}A\r\nDeutsch\n```mermaid\rgraph LR\r```".as_bytes());
        let mut config = reference_profile();
        config.capture_capacity = capacity;
        let mut app = Editor::open(&[f.path()], config).unwrap();
        app.apply(Action::Right).unwrap();
        app.apply(Action::Insert("é".into())).unwrap();
        assert!(app.is_dirty().unwrap());
        app.apply(Action::Undo).unwrap();
        assert!(!app.is_dirty().unwrap());
        app.apply(Action::Redo).unwrap();
        app.apply(Action::Save).unwrap();
        assert!(!app.is_dirty().unwrap());
        // Qualify baseline replacement and superseded-attempt retirement too.
        app.apply(Action::Undo).unwrap();
        assert!(app.is_dirty().unwrap());
        app.apply(Action::Save).unwrap();
        app.apply(Action::Redo).unwrap();
        assert!(app.is_dirty().unwrap());
        app.apply(Action::Save).unwrap();
        assert!(!app.is_dirty().unwrap());
        assert_eq!(
            fs::read(f.path()).unwrap(),
            "\u{feff}Aé\r\nDeutsch\n```mermaid\rgraph LR\r```".as_bytes()
        );
        if capacity == 1 {
            assert!(app.dropped_diagnostics() > 0);
        }
        app.close(false).unwrap();
        assert!(app.is_closed());
    }
}

#[test]
fn grapheme_delete_and_crlf_navigation_do_not_split_clusters_or_delimiters() {
    let f = Fixture::new("e\u{301}👩‍💻\r\nB".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.apply(Action::Right).unwrap();
    assert_eq!(app.cursor(), "e\u{301}".len());
    app.apply(Action::Delete).unwrap();
    assert_eq!(app.text().unwrap(), "e\u{301}\r\nB");
    app.apply(Action::Right).unwrap();
    assert_eq!(app.cursor(), "e\u{301}\r\n".len());
    app.apply(Action::Backspace).unwrap();
    assert_eq!(app.text().unwrap(), "e\u{301}B");
    app.apply(Action::Backspace).unwrap();
    assert_eq!(app.text().unwrap(), "B");
    app.close(true).unwrap();
    assert_eq!(fs::read(f.path()).unwrap(), "e\u{301}👩‍💻\r\nB".as_bytes());
}

#[test]
fn external_changes_are_visible_failures_not_successful_saves_or_automatic_retries() {
    let f = Fixture::new(b"A");
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.apply(Action::Insert("B".into())).unwrap();
    fs::write(f.path(), b"external").unwrap();
    assert!(app.apply(Action::Save).is_err());
    assert!(app.is_dirty().unwrap());
    assert_eq!(fs::read(f.path()).unwrap(), b"external");
    app.close(true).unwrap();
}

#[test]
fn quit_requires_explicit_discard_and_switching_files_does_not_redirect_edits() {
    let a = Fixture::new(b"A");
    let b = Fixture::new(b"B");
    let mut app = Editor::open(&[a.path(), b.path()], reference_profile()).unwrap();
    app.apply(Action::Insert("first".into())).unwrap();
    assert!(app.close(false).is_err());
    assert!(!app.is_closed());
    app.apply(Action::NextDocument).unwrap();
    assert_eq!(app.text().unwrap(), "B");
    app.apply(Action::Insert("second".into())).unwrap();
    app.apply(Action::Save).unwrap();
    app.apply(Action::NextDocument).unwrap();
    assert_eq!(app.text().unwrap(), "firstA");
    app.close(true).unwrap();
    assert_eq!(fs::read(a.path()).unwrap(), b"A");
    assert_eq!(fs::read(b.path()).unwrap(), b"secondB");
}

#[test]
fn navigation_and_safe_view_handle_tabs_wide_text_cr_lines_and_terminal_controls() {
    let f = Fixture::new("\t界\x1b[31m\rnext\n".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.apply(Action::End).unwrap();
    assert_eq!(app.cursor(), "\t界\x1b[31m".len());
    let frame = app.frame(40, 3).unwrap();
    assert!(!frame.lines.join("\n").contains('\x1b'));
    assert!(frame.lines[0].contains('␛'));
    app.apply(Action::Down).unwrap();
    assert_eq!(app.cursor(), "\t界\x1b[31m\rnext".len());
    app.apply(Action::Home).unwrap();
    app.apply(Action::Insert("ü".into())).unwrap();
    assert_eq!(app.text().unwrap(), "\t界\x1b[31m\rünext\n");
    app.close(true).unwrap();
}

#[test]
fn exact_frontend_proposals_refuse_stale_captures_and_inactive_documents() {
    let a = Fixture::new(b"A");
    let b = Fixture::new(b"B");
    let mut app = Editor::open(&[a.path(), b.path()], reference_profile()).unwrap();
    let original = app.snapshot().unwrap();
    app.apply(Action::Insert("x".into())).unwrap();
    assert!(
        app.edit_snapshot(
            &original.version,
            vec![vellum_engine::source::Replacement {
                start: 0,
                end: 1,
                expected: "A".into(),
                inserted: "not applied".into(),
            }],
            0
        )
        .is_err()
    );
    let current = app.snapshot().unwrap();
    app.apply(Action::NextDocument).unwrap();
    assert!(app.edit_snapshot(&current.version, Vec::new(), 0).is_err());
    assert_eq!(app.text().unwrap(), "B");
    assert!(app.set_caret(usize::MAX).is_err());
    app.close(true).unwrap();
}

#[test]
fn native_traces_remain_inspectable_with_separate_bounded_window_loss() {
    let f = Fixture::new(b"A");
    let mut config = reference_profile();
    config.capture_capacity = 4;
    let mut app = Editor::open(&[f.path()], config).unwrap();
    for _ in 0..12 {
        app.apply(Action::Insert("x".into())).unwrap();
    }
    assert!(app.dropped_diagnostics() > 0);
    assert!(app.diagnostic_window_overwrites().unwrap() > 0);
    let mut records = 0;
    while app.pop_diagnostic().is_some() {
        records += 1;
    }
    assert!(records > 0 && records <= 4);
    app.close(true).unwrap();
    let mut off = reference_profile();
    off.capture_capacity = 0;
    let mut app = Editor::open(&[f.path()], off).unwrap();
    app.apply(Action::Right).unwrap();
    assert!(app.pop_diagnostic().is_none());
    assert_eq!(app.diagnostic_window_overwrites(), Some(0));
    app.close(false).unwrap();
}

#[test]
fn opening_additional_files_reuses_dirty_owners_and_failure_keeps_the_active_document() {
    let a = Fixture::new(b"A");
    let b = Fixture::new(b"B");
    let mut app = Editor::open(&[a.path()], reference_profile()).unwrap();
    app.apply(Action::Insert("x".into())).unwrap();
    let original = app.snapshot().unwrap();
    assert!(!app.open_additional(&a.path()).unwrap());
    assert_eq!(app.snapshot().unwrap().version, original.version);
    assert_eq!(app.document_count(), 1);
    assert!(app.open_additional(&b.path()).unwrap());
    assert_eq!(app.document_count(), 2);
    assert_eq!(app.text().unwrap(), "B");
    assert!(!app.open_additional(&a.path()).unwrap());
    assert_eq!(app.text().unwrap(), "xA");
    assert!(app.open_additional(&a.0.join("missing")).is_err());
    assert_eq!(app.text().unwrap(), "xA");
    assert!(app.close(false).is_err());
    app.close(true).unwrap();
    assert_eq!(fs::read(a.path()).unwrap(), b"A");
}

#[test]
fn editing_lock_is_native_policy_and_alias_open_does_not_clear_it() {
    let f = Fixture::new(b"A");
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.apply(Action::Insert("x".into())).unwrap();
    let original = app.snapshot().unwrap();
    app.apply(Action::ToggleEditingLock).unwrap();
    assert!(app.editing_locked());
    assert!(app.apply(Action::Insert("!".into())).is_err());
    assert!(app.apply(Action::Undo).is_err());
    assert!(
        app.edit_snapshot(
            &original.version,
            vec![vellum_engine::source::Replacement {
                start: 0,
                end: 1,
                expected: "x".into(),
                inserted: "!".into(),
            }],
            0
        )
        .is_err()
    );
    assert!(!app.open_additional(&f.path()).unwrap());
    assert!(app.editing_locked());
    app.apply(Action::Right).unwrap();
    assert!(app.apply(Action::Save).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"A");
    app.apply(Action::ToggleEditingLock).unwrap();
    app.apply(Action::Save).unwrap();
    assert_eq!(fs::read(f.path()).unwrap(), b"xA");
    app.apply(Action::Undo).unwrap();
    assert_eq!(app.text().unwrap(), "A");
    app.close(true).unwrap();
}

#[test]
fn search_replacement_uses_native_history_and_releases_search_owners_on_close() {
    use vellum_editor::SearchDirection;
    let f = Fixture::new("\u{feff}é name\r\nname".as_bytes());
    let mut app = Editor::open(&[f.path()], reference_profile()).unwrap();
    app.find("name").unwrap();
    assert_eq!(app.cursor(), 3);
    app.navigate_match(SearchDirection::Next, true).unwrap();
    assert_eq!(app.cursor(), 9);
    app.replace_match("", false).unwrap();
    assert_eq!(app.text().unwrap(), "é name\r\n");
    assert!(app.replace_match("wrong", true).is_err());
    app.apply(Action::Undo).unwrap();
    assert_eq!(app.text().unwrap(), "é name\r\nname");
    app.find("name").unwrap();
    app.replace_match("label", true).unwrap();
    app.apply(Action::Save).unwrap();
    app.close(false).unwrap();
    assert!(app.is_closed());
    assert_eq!(
        fs::read(f.path()).unwrap(),
        "\u{feff}é label\r\nlabel".as_bytes()
    );
}

#[test]
fn search_is_tab_owned_and_partial_totals_do_not_authorize_replace_all() {
    let a = Fixture::new(b"name name");
    let b = Fixture::new(b"name");
    let mut cfg = reference_profile();
    cfg.engine.search.as_mut().unwrap().max_matches = 1;
    let mut app = Editor::open(&[a.path(), b.path()], cfg).unwrap();
    app.find("name").unwrap();
    assert!(app.status().contains("Exhausted"));
    assert!(app.replace_match("wrong", true).is_err());
    app.apply(Action::NextDocument).unwrap();
    assert!(app.replace_match("wrong", false).is_err());
    app.find("name").unwrap();
    app.apply(Action::ToggleEditingLock).unwrap();
    assert!(app.replace_match("wrong", false).is_err());
    app.close(false).unwrap();
    assert!(app.is_closed());
    assert_eq!(fs::read(a.path()).unwrap(), b"name name");
    assert_eq!(fs::read(b.path()).unwrap(), b"name");
}

#[test]
fn confirmed_save_as_freezes_source_and_updates_only_the_active_binding_with_explicit_cleanup() {
    let a = Fixture::new(b"A");
    let b = Fixture::new(b"OLD");
    let mut app = Editor::open(&[a.path()], reference_profile()).unwrap();
    let stale = app.plan_destination(b.path(), true).unwrap();
    assert_eq!(fs::read(b.path()).unwrap(), b"OLD");
    app.apply(Action::Insert("x".into())).unwrap();
    assert!(app.confirm_destination(&stale).is_err());
    assert_eq!(fs::read(b.path()).unwrap(), b"OLD");
    let plan = app.plan_destination(b.path(), true).unwrap();
    assert_eq!(format!("{plan:?}"), "DestinationPlan(<redacted>)");
    assert!(plan != app.plan_destination(b.path(), true).unwrap());
    app.confirm_destination(&plan).unwrap();
    assert!(!app.is_dirty().unwrap());
    app.apply(Action::Insert("y".into())).unwrap();
    app.apply(Action::Save).unwrap();
    assert_eq!(fs::read(a.path()).unwrap(), b"A");
    assert_eq!(fs::read(b.path()).unwrap(), b"xyA");
    app.close(false).unwrap();
    assert!(app.is_closed());
}

#[test]
fn copy_export_leaves_original_dirty_and_missing_destination_does_not_strand_preparation() {
    let a = Fixture::new(b"A");
    let b = Fixture::new(b"OLD");
    let mut app = Editor::open(&[a.path()], reference_profile()).unwrap();
    app.apply(Action::Insert("x".into())).unwrap();
    let missing = app.plan_destination(a.0.join("missing"), false).unwrap();
    assert!(app.confirm_destination(&missing).is_err());
    let plan = app.plan_destination(b.path(), false).unwrap();
    app.confirm_destination(&plan).unwrap();
    assert!(app.is_dirty().unwrap());
    assert!(app.close(false).is_err());
    app.close(true).unwrap();
    assert_eq!(fs::read(a.path()).unwrap(), b"A");
    assert_eq!(fs::read(b.path()).unwrap(), b"xA");
}
