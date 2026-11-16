use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::{
    DocumentVersion, HistoryDirection, HistoryError, HistoryGroupId, LifecycleState,
    TrustedDocumentReclamationV1,
};
use vellum_engine::instrumentation::Observation;
use vellum_engine::resource::{
    Acknowledgment, Application, ContentComparison, Protection, ResourceError, SaveId, SaveOutcome,
    SaveRequirements, Synchronization,
};
use vellum_engine::source::Replacement;
use vellum_host::annotation::AnnotationCodec;
use vellum_host::resource::{
    ErasedPathResourceAdapter, ErasedPathResourceAdapterFactory, ErasedResourceHandle,
};
use vellum_session::host::{
    LocalDocument as SessionDocument, LocalHost as SessionHost, SessionConfig, SessionError,
};
type LocalDocument = SessionDocument<ErasedResourceHandle>;
type LocalHost = SessionHost<ErasedPathResourceAdapter>;
/// Qualified reference composition used by the current editor applications.
///
/// Applications should treat this as a composition profile, not as a claim
/// that local filesystem effects belong to the editor application layer.
pub struct ReferenceSessionConfig {
    session: SessionConfig<ErasedPathResourceAdapterFactory>,
    annotation_codec: Box<dyn AnnotationCodec>,
}
impl std::ops::Deref for ReferenceSessionConfig {
    type Target = SessionConfig<ErasedPathResourceAdapterFactory>;
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}
impl std::ops::DerefMut for ReferenceSessionConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}
mod clipboard;
mod comment_storage;
pub use comment_storage::CommentFileLimits;
pub use comment_storage::CommentFilePlan;
mod comments;
mod destination;
mod multi_selection;
mod native_selection;
mod search;
mod unicode;
pub use clipboard::{ClipboardPlan, ClipboardReport};
pub use comments::{CommentDraft, CommentHandle, CommentRow};
pub use destination::DestinationPlan;
#[cfg(feature = "native-unicode")]
pub use unicode::NativeUnicodeProfile;
#[cfg(feature = "native-unicode")]
pub use vellum_host::unicode::UnicodeProviderUsage as NativeUnicodeUsage;

/// Presentation intents, not engine executable vocabulary or key bindings.
#[derive(Debug)]
pub enum Action {
    Insert(String),
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    ExtendSelection(Movement),
    SelectAll,
    Indent,
    Outdent,
    Undo,
    Redo,
    Save,
    NextDocument,
    ToggleEditingLock,
}
/// Presentation movement, not a keymap or semantic engine command.
#[derive(Clone, Copy, Debug)]
pub enum Movement {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}
#[derive(Debug)]
pub enum EditorError {
    Session(SessionError),
    Empty,
    Closed,
    Capacity,
    Unsaved,
    SaveIncomplete,
    Position,
    ClipboardPending,
    ClipboardUnavailable,
    CommentsUnavailable,
    CommentsIncomplete,
    IdentityUnavailable,
    IdentityConflict,
    #[cfg(feature = "native-unicode")]
    Unicode(vellum_host::unicode::UnicodeProviderFailure),
}
impl From<SessionError> for EditorError {
    fn from(e: SessionError) -> Self {
        Self::Session(e)
    }
}
impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "editor: {self:?}")
    }
}
impl std::error::Error for EditorError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OpenMany {
    pub added: usize,
    pub joined: usize,
}

#[derive(Debug)]
pub struct OpenManyFailure {
    pub added: usize,
    pub joined: usize,
    pub error: EditorError,
}

impl std::fmt::Display for OpenManyFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "editor import stopped after {} added and {} joined: {}",
            self.added, self.joined, self.error
        )
    }
}

impl std::error::Error for OpenManyFailure {}

#[derive(Clone, Copy)]
enum ClosePhase {
    Live,
    Disposed(LifecycleState),
    SourceUnbound(LifecycleState),
    Unbound(LifecycleState),
    Released(LifecycleState),
    Reclaimed,
}
struct Tab {
    document: LocalDocument,
    cursor: usize,
    anchor: Option<usize>,
    view: Option<vellum_engine::coordinator::LogicalViewId>,
    selection_count: usize,
    phase: ClosePhase,
    editing_locked: bool,
    search: Option<search::State>,
    comment_anchor: Option<vellum_engine::coordinator::AnnotationId>,
    comments_locked: bool,
    comment_file: Option<vellum_session::host::AnnotationResource<ErasedResourceHandle>>,
    comment_names: Option<comment_storage::Names>,
    comment_file_requires_load: bool,
}
struct Attempt {
    task: SaveId,
    tab: usize,
    annotations: bool,
}

/// One engine host across tabs. Only cursor/viewport/status and original owner
/// handles live here; source and history remain authoritative in the engine.
pub struct Editor {
    host: LocalHost,
    annotation_codec: Box<dyn AnnotationCodec>,
    tabs: Vec<Tab>,
    active: usize,
    groups: Vec<HistoryGroupId>,
    attempts: Vec<Attempt>,
    top: usize,
    left: usize,
    status: String,
    closed: bool,
    diagnostics: VecDeque<Observation>,
    diagnostic_limit: usize,
    diagnostic_overwrites: Option<u64>,
    search_limits: Option<vellum_engine::coordinator::SearchLimits>,
    exports: Vec<vellum_session::host::SourceExportAttempt<ErasedResourceHandle>>,
    export_preparation: Option<vellum_session::host::SourceExportPreparation>,
    native_views: bool,
    selection_budget: Option<vellum_engine::selection::SelectionBudget>,
    clipboard_attempts: Vec<clipboard::Attempt>,
    clipboard_limit: Option<usize>,
    annotation_limits: Option<vellum_engine::coordinator::AnnotationLimits>,
    comment_cache: Option<comments::Cache>,
    comment_encoder: Option<vellum_engine::resource::AnnotationEncoderPort>,
    orphaned_comment_artifacts: Vec<vellum_engine::resource::ResourceArtifactId>,
    comment_reads: Vec<comment_storage::ReadAttempt>,
    comment_restorer: Option<vellum_engine::resource::AnnotationRestorerPort>,
    comment_read_baselines: bool,
    comment_file_limits: Option<CommentFileLimits>,
    comment_file_plan: Option<std::sync::Arc<()>>,
    #[cfg(feature = "native-unicode")]
    native_unicode: Option<unicode::State>,
}
pub struct EditorFrame {
    pub lines: Vec<String>,
    /// Viewport-relative terminal cells, exclusive end. Never source offsets.
    pub selection_cells: Vec<Vec<std::ops::Range<u16>>>,
    pub selection_count: usize,
    pub cursor: Option<(u16, u16)>,
    pub status: String,
    pub active: usize,
    pub documents: usize,
    pub dirty: bool,
    pub editing_locked: bool,
}
/// Immutable presentation capture, not write authority or a mutable source root.
pub struct EditorSnapshot {
    pub version: DocumentVersion,
    pub text: String,
}
impl std::fmt::Debug for EditorSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EditorSnapshot(<redacted>)")
    }
}
impl Editor {
    pub fn open(paths: &[PathBuf], config: ReferenceSessionConfig) -> Result<Self, EditorError> {
        let ReferenceSessionConfig {
            session: config,
            annotation_codec,
        } = config;
        if paths.is_empty() {
            return Err(EditorError::Empty);
        }
        if paths.len() > config.engine.limits.max_document_incarnations {
            return Err(EditorError::Capacity);
        }
        let diagnostic_limit = config.capture_capacity;
        let native_views = config.logical_views.is_some();
        let selection_budget = config.logical_views.map(|limits| limits.normalization);
        let search_limits = config.engine.search;
        let annotation_limits = config.engine.annotations;
        let mut diagnostics = VecDeque::new();
        diagnostics
            .try_reserve_exact(diagnostic_limit)
            .map_err(|_| EditorError::Capacity)?;
        let mut host = LocalHost::new(config)?;
        let mut tabs: Vec<Tab> = Vec::new();
        tabs.try_reserve_exact(paths.len())
            .map_err(|_| EditorError::Capacity)?;
        for path in paths {
            let document = host.open(path)?;
            let id = host.read(&document)?.version.document;
            let mut duplicate = false;
            for tab in &tabs {
                if host.read(&tab.document)?.version.document == id {
                    duplicate = true;
                    break;
                }
            }
            if !duplicate {
                tabs.push(Tab {
                    document,
                    cursor: 0,
                    anchor: None,
                    view: None,
                    selection_count: 1,
                    phase: ClosePhase::Live,
                    editing_locked: false,
                    search: None,
                    comment_anchor: None,
                    comments_locked: false,
                    comment_file: None,
                    comment_names: None,
                    comment_file_requires_load: false,
                });
            }
        }
        let mut editor = Self {
            host,
            annotation_codec,
            tabs,
            active: 0,
            groups: Vec::new(),
            attempts: Vec::new(),
            top: 0,
            left: 0,
            status: "Ready — UI graphemes: Unicode 17; generated newline: LF".into(),
            closed: false,
            diagnostics,
            diagnostic_limit,
            diagnostic_overwrites: Some(0),
            search_limits,
            exports: Vec::new(),
            export_preparation: None,
            native_views,
            selection_budget,
            clipboard_attempts: Vec::new(),
            clipboard_limit: None,
            annotation_limits,
            comment_cache: None,
            comment_encoder: None,
            orphaned_comment_artifacts: Vec::new(),
            comment_reads: Vec::new(),
            comment_restorer: None,
            comment_read_baselines: false,
            comment_file_limits: None,
            comment_file_plan: None,
            #[cfg(feature = "native-unicode")]
            native_unicode: None,
        };
        for index in 0..editor.tabs.len() {
            editor.attach_tab_view(index)?;
        }
        Ok(editor)
    }
    /// Open through the existing host. Reopening an owned resource joins its
    /// original dirty document; it never reloads over authored changes.
    /// Returns true for a newly attached document, false for an existing owner.
    pub fn open_additional(&mut self, path: &Path) -> Result<bool, EditorError> {
        self.live()?;
        if self
            .tabs
            .iter()
            .any(|tab| !matches!(tab.phase, ClosePhase::Live))
        {
            return Err(EditorError::Closed);
        }
        self.tabs
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let document = self.host.open(path)?;
        let id = self.host.read(&document)?.version.document;
        let mut existing = None;
        for (index, tab) in self.tabs.iter().enumerate() {
            if self.host.read(&tab.document)?.version.document == id {
                existing = Some(index);
                break;
            }
        }
        let added = existing.is_none();
        let next = if let Some(index) = existing {
            index
        } else {
            self.tabs.push(Tab {
                document,
                cursor: 0,
                anchor: None,
                view: None,
                selection_count: 1,
                phase: ClosePhase::Live,
                editing_locked: false,
                search: None,
                comment_anchor: None,
                comments_locked: false,
                comment_file: None,
                comment_names: None,
                comment_file_requires_load: false,
            });
            self.tabs.len() - 1
        };
        self.attach_tab_view(next)?;
        self.active = next;
        self.sync_native_selection()?;
        self.top = 0;
        self.left = 0;
        self.status = if added {
            "Opened additional document".into()
        } else {
            "Joined existing document without reload".into()
        };
        self.collect_diagnostics();
        Ok(added)
    }
    /// Opens a finite sequence in order and reports joins separately from new
    /// attachments. Filesystem effects are not transactional: on refusal the
    /// returned counts make all successfully completed prefix work explicit.
    pub fn open_additional_many(&mut self, paths: &[PathBuf]) -> Result<OpenMany, OpenManyFailure> {
        let mut outcome = OpenMany::default();
        for path in paths {
            match self.open_additional(path) {
                Ok(true) => outcome.added += 1,
                Ok(false) => outcome.joined += 1,
                Err(error) => {
                    return Err(OpenManyFailure {
                        added: outcome.added,
                        joined: outcome.joined,
                        error,
                    });
                }
            }
        }
        Ok(outcome)
    }
    pub fn text(&mut self) -> Result<&str, EditorError> {
        self.live()?;
        Ok(self
            .host
            .read(&self.tabs[self.active].document)?
            .source
            .text())
    }
    pub fn cursor(&self) -> usize {
        self.tabs[self.active].cursor
    }
    pub fn editing_locked(&self) -> bool {
        self.tabs[self.active].editing_locked
    }
    /// Current native identity/revision without copying source. The original
    /// engine invariant binds one version to one immutable text value; this is
    /// not a source disclosure grant or a cached presentation revision.
    pub fn version(&mut self) -> Result<DocumentVersion, EditorError> {
        self.live()?;
        Ok(self.host.read(&self.tabs[self.active].document)?.version)
    }
    pub fn snapshot(&mut self) -> Result<EditorSnapshot, EditorError> {
        self.live()?;
        let read = self.host.read(&self.tabs[self.active].document)?;
        let mut text = String::new();
        text.try_reserve_exact(read.source.text().len())
            .map_err(|_| EditorError::Capacity)?;
        text.push_str(read.source.text());
        Ok(EditorSnapshot {
            version: read.version,
            text,
        })
    }
    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn active_document(&self) -> usize {
        self.active
    }
    pub fn document_count(&self) -> usize {
        self.tabs.len()
    }
    pub fn set_caret(&mut self, byte: usize) -> Result<(), EditorError> {
        let expected = self.version()?;
        self.set_selection(&expected, byte, byte)
    }

    /// Primary-selection presentation cache: the native view owns its logical
    /// state when configured. This grants no authority and never serializes text.
    pub fn selection(&self) -> Option<std::ops::Range<usize>> {
        let tab = &self.tabs[self.active];
        tab.anchor
            .filter(|&a| a != tab.cursor)
            .map(|a| a.min(tab.cursor)..a.max(tab.cursor))
    }

    /// A frontend proposes exact original-coordinate edits, not a replacement
    /// authoritative buffer. The engine checks the original version and fragments.
    /// `caret` is a fallback hint; configured native views supply their actual
    /// mapped primary after publication. UI grapheme projection stays separate.
    pub fn edit_snapshot(
        &mut self,
        expected: &DocumentVersion,
        edits: Vec<Replacement>,
        caret: usize,
    ) -> Result<(), EditorError> {
        self.live()?;
        // A delayed projection from a different active tab must not mutate its
        // original document behind the currently attached frontend.
        if self
            .host
            .read(&self.tabs[self.active].document)?
            .version
            .document
            != expected.document
        {
            return Err(SessionError::WrongDocument.into());
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let outcome = self.publish_native_edit(expected, edits)?;
        if let Some(group) = outcome.history_group {
            self.groups.push(group);
        }
        self.tabs[self.active].cursor = caret;
        self.clamp_cursor()?;
        self.status = "Edited".into();
        self.collect_diagnostics();
        Ok(())
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn dropped_diagnostics(&self) -> u64 {
        self.host
            .dropped_diagnostics()
            .saturating_add(self.unicode_dropped())
    }
    /// Captured traces, not authoritative operation counts or effect receipts.
    /// Capture loss and frontend-window overwrites are separate populations.
    pub fn pop_diagnostic(&mut self) -> Option<Observation> {
        self.collect_diagnostics();
        self.diagnostics.pop_front()
    }
    pub fn diagnostic_window_overwrites(&self) -> Option<u64> {
        self.diagnostic_overwrites
    }
    pub fn collect_diagnostics(&mut self) {
        for turn in 0..256 {
            let Some(record) = (if turn % 2 == 0 {
                self.host
                    .pop_diagnostic()
                    .or_else(|| self.pop_unicode_diagnostic())
            } else {
                self.pop_unicode_diagnostic()
                    .or_else(|| self.host.pop_diagnostic())
            }) else {
                break;
            };
            if self.diagnostic_limit == 0 {
                continue;
            }
            if self.diagnostics.len() == self.diagnostic_limit {
                self.diagnostics.pop_front();
                self.diagnostic_overwrites =
                    self.diagnostic_overwrites.and_then(|n| n.checked_add(1));
            }
            self.diagnostics.push_back(record);
        }
    }
    pub fn last_save_outcome(&mut self) -> Result<Option<SaveOutcome>, EditorError> {
        self.attempts
            .iter()
            .rev()
            .find(|a| !a.annotations)
            .map(|a| self.host.save_outcome(&a.task))
            .transpose()
            .map_err(Into::into)
    }
    pub fn is_dirty(&mut self) -> Result<bool, EditorError> {
        Ok(self.is_source_dirty()? || self.comments_dirty()?)
    }
    /// Read-only all-tab close preflight; no focus, retirement or discard effects.
    pub fn has_unsaved_documents(&mut self) -> Result<bool, EditorError> {
        for index in 0..self.tabs.len() {
            let tab = &self.tabs[index];
            if matches!(tab.phase, ClosePhase::Live) {
                let state = self.host.saved_state(&tab.document)?;
                if state.content != ContentComparison::Unchanged
                    || state.contested
                    || state.unresolved != 0
                    || state.queued != 0
                    || self.comments_dirty_at(index)?
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
    pub fn is_source_dirty(&mut self) -> Result<bool, EditorError> {
        self.live()?;
        let state = self.host.saved_state(&self.tabs[self.active].document)?;
        Ok(state.content != ContentComparison::Unchanged
            || state.queued != 0
            || state.unresolved != 0
            || state.contested)
    }
    pub fn apply(&mut self, action: Action) -> Result<(), EditorError> {
        self.live()?;
        #[cfg(feature = "native-unicode")]
        let before_source = if self.native_unicode.is_some() {
            Some(self.version()?)
        } else {
            None
        };
        let before = (self.tabs[self.active].cursor, self.tabs[self.active].anchor);
        let active = self.active;
        let result = self.apply_inner(action);
        if result.is_err() && self.active == active {
            // A refused selection request must not leave a UI-only retarget.
            // Source publication is never rolled back or denied by this cache.
            if self.native_views {
                let _ = self.sync_native_selection();
            } else {
                self.tabs[active].cursor = before.0;
                self.tabs[active].anchor = before.1;
            }
        }
        if let Err(error) = &result
            && !matches!(error, EditorError::SaveIncomplete)
        {
            #[cfg(feature = "native-unicode")]
            let changed = before_source
                .as_ref()
                .is_some_and(|before| self.version().is_ok_and(|now| &now != before));
            #[cfg(not(feature = "native-unicode"))]
            let changed = false;
            self.status = if changed {
                format!("Native state changed; presentation update failed: {error}")
            } else {
                error.to_string()
            };
        }
        // Drain bounded diagnostics outside publication; no external sink/callback.
        // Saturation stays observable in Recorder's independent drop count.
        self.collect_diagnostics();
        result
    }
    fn apply_inner(&mut self, action: Action) -> Result<(), EditorError> {
        let cursor = self.cursor();
        match action {
            Action::SelectAll => {
                let end = self.text()?.len();
                self.tabs[self.active].anchor = Some(0);
                self.tabs[self.active].cursor = end;
                self.publish_ui_selection()?;
            }
            Action::ExtendSelection(movement) => {
                self.move_selections(movement, true)?;
            }
            Action::Insert(text) => {
                if self.selection_count() > 1 {
                    let expected = self.version()?;
                    return self.replace_selections(&expected, &text);
                }
                let range = self.selection().unwrap_or(cursor..cursor);
                self.replace(range.start, range.end, text)?;
            }
            Action::Indent | Action::Outdent => {
                let expected = self.version()?;
                let request = if matches!(action, Action::Indent) {
                    vellum_engine::coordinator::LineEditV1::Indent {
                        unit: "    ".into(),
                        tab_width: 4,
                    }
                } else {
                    vellum_engine::coordinator::LineEditV1::Outdent {
                        unit: "    ".into(),
                        tab_width: 4,
                    }
                };
                self.edit_lines(&expected, &request)?;
            }
            Action::Backspace => {
                #[cfg(feature = "native-unicode")]
                if self.native_unicode.is_some() {
                    return self.unicode_delete(None, true);
                }
                if self.selection_count() > 1 {
                    let expected = self.version()?;
                    return self.delete_selections(&expected, true);
                }
                if let Some(range) = self.selection() {
                    return self.replace(range.start, range.end, String::new());
                }
                let start = self.text()?[..cursor]
                    .grapheme_indices(true)
                    .next_back()
                    .map_or(0, |(i, _)| i);
                if start < cursor {
                    self.replace(start, cursor, String::new())?;
                }
            }
            Action::Delete => {
                #[cfg(feature = "native-unicode")]
                if self.native_unicode.is_some() {
                    return self.unicode_delete(None, false);
                }
                if self.selection_count() > 1 {
                    let expected = self.version()?;
                    return self.delete_selections(&expected, false);
                }
                if let Some(range) = self.selection() {
                    return self.replace(range.start, range.end, String::new());
                }
                let end = self.text()?[cursor..]
                    .graphemes(true)
                    .next()
                    .map_or(cursor, |g| cursor + g.len());
                if cursor < end {
                    self.replace(cursor, end, String::new())?;
                }
            }
            Action::Left => {
                self.move_selections(Movement::Left, false)?;
            }
            Action::Right => {
                self.move_selections(Movement::Right, false)?;
            }
            Action::Home | Action::End | Action::Up | Action::Down => {
                self.move_selections(
                    match action {
                        Action::Home => Movement::Home,
                        Action::End => Movement::End,
                        Action::Up => Movement::Up,
                        _ => Movement::Down,
                    },
                    false,
                )?;
            }
            Action::Undo | Action::Redo => {
                self.host.reverse(
                    &self.tabs[self.active].document,
                    if matches!(action, Action::Undo) {
                        HistoryDirection::Undo
                    } else {
                        HistoryDirection::Redo
                    },
                )?;
                self.clamp_cursor()?;
                self.status = "History changed".into();
            }
            Action::Save => self.save()?,
            Action::NextDocument => {
                let previous = self.active;
                self.active = (self.active + 1) % self.tabs.len();
                if let Err(error) = self.sync_native_selection() {
                    self.active = previous;
                    return Err(error);
                }
                self.top = 0;
                self.left = 0;
                self.status = "Document switched".into();
            }
            Action::ToggleEditingLock => {
                let locked = !self.tabs[self.active].editing_locked;
                self.host
                    .set_editing_locked(&self.tabs[self.active].document, locked)?;
                self.tabs[self.active].editing_locked = locked;
                self.status = if locked {
                    "Editing locked by native source policy".into()
                } else {
                    "Editing unlocked".into()
                };
            }
        }
        Ok(())
    }
    fn replace(&mut self, start: usize, end: usize, inserted: String) -> Result<(), EditorError> {
        self.groups
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let read = self.host.read(&self.tabs[self.active].document)?;
        let expected = read.source.text()[start..end].to_owned();
        let version = read.version;
        let next = start + inserted.len();
        let outcome = self.publish_native_edit(
            &version,
            vec![Replacement {
                start,
                end,
                expected,
                inserted,
            }],
        )?;
        if let Some(group) = outcome.history_group {
            self.groups.push(group);
        }
        self.tabs[self.active].cursor = next;
        self.clamp_cursor()?;
        self.status = "Edited".into();
        Ok(())
    }
    fn clamp_cursor(&mut self) -> Result<(), EditorError> {
        if self.project_native_graphemes()? {
            return Ok(());
        }
        let cursor = self.cursor();
        let text = self.text()?;
        let at = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|&i| i >= cursor)
            .unwrap_or(text.len());
        self.tabs[self.active].cursor = at;
        self.tabs[self.active].anchor = None;
        Ok(())
    }
    fn save(&mut self) -> Result<(), EditorError> {
        if !self.is_source_dirty()? {
            self.status = if self.comments_dirty()? {
                "Source unchanged; session comments remain unsaved"
            } else {
                "Unchanged — no write"
            }
            .into();
            return Ok(());
        }
        self.attempts
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let version = self.host.read(&self.tabs[self.active].document)?.version;
        let task = self.host.queue_save(
            &version,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
        )?;
        self.attempts.push(Attempt {
            task: task.clone(),
            tab: self.active,
            annotations: false,
        });
        let report = self
            .host
            .service_save(&self.tabs[self.active].document, &task)?;
        if report.failure.is_some()
            || report.cleanup_failed
            || report.outcome.application != Application::Written
            || report.outcome.synchronization != Synchronization::Succeeded
        {
            self.status = format!(
                "Save: {:?}; sync: {:?}; cleanup failed: {}",
                report.outcome.application, report.outcome.synchronization, report.cleanup_failed
            );
            return Err(EditorError::SaveIncomplete);
        }
        self.host.publish_save_baseline(&task)?;
        // Older successful tasks may be superseded. Retained/failed originals
        // remain addressable and cannot disappear merely because a save returned.
        let mut index = 0;
        while index + 1 < self.attempts.len() {
            if self.attempts[index].tab != self.active || self.attempts[index].annotations {
                index += 1;
                continue;
            }
            match self.host.retire_save(&self.attempts[index].task) {
                Ok(()) => {
                    self.attempts.remove(index);
                }
                Err(SessionError::Resource(ResourceError::Retained)) => {
                    index += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }
        self.status = if self.comments_dirty()? {
            "Source saved; session comments remain unsaved"
        } else {
            "Saved — application Written; synchronization Succeeded"
        }
        .into();
        Ok(())
    }
    /// Explicit discard consent applies to all tabs. Each completed lifecycle
    /// step is retained so a later refusal cannot fabricate an all-or-none close.
    /// No filesystem delete, physical erasure or whole-engine certificate follows.
    pub fn close(&mut self, discard: bool) -> Result<(), EditorError> {
        if self.closed {
            return Ok(());
        }
        if !self.clipboard_attempts.is_empty()
            || self
                .clipboard_usage()
                .is_some_and(|u| u.held_tasks != 0 || u.providers_retained != 0)
        {
            return Err(EditorError::ClipboardPending);
        }
        if !discard && self.has_unsaved_documents()? {
            return Err(EditorError::Unsaved);
        }
        for attempt in &self.attempts {
            self.host.stop_save(&attempt.task)?;
            let outcome = self.host.save_outcome(&attempt.task)?;
            if !outcome.quiescent || !outcome.no_future_effects {
                return Err(EditorError::SaveIncomplete);
            }
        }
        self.settle_exports()?;
        self.stop_comment_reads()?;
        self.retire_orphaned_comment_artifacts()?;
        self.clear_all_searches()?;
        self.release_unicode()?;
        self.comment_cache = None;
        self.release_native_views()?;
        for tab in &mut self.tabs {
            if matches!(tab.phase, ClosePhase::Live) {
                let version = self.host.read(&tab.document)?.version;
                tab.phase = ClosePhase::Disposed(self.host.dispose(&version)?);
                tab.comment_names = None;
            }
            if let ClosePhase::Disposed(state) = tab.phase {
                tab.phase = ClosePhase::SourceUnbound(
                    self.host.release_source_binding(&tab.document, state)?,
                );
            }
            if let ClosePhase::SourceUnbound(state) = tab.phase {
                tab.phase = ClosePhase::Unbound(
                    self.host.release_annotation_binding(&tab.document, state)?,
                );
            }
        }
        while let Some(group) = self.groups.last() {
            match self.host.retire_history(group) {
                Ok(_) => {}
                // Complete-group retirement can also retire earlier groups in
                // the linked frontier. These are our original handles, not
                // unknown caller input. Final actual occupancy and provenance
                // guards below still independently require no retained history.
                Err(SessionError::History(HistoryError::Unavailable)) => {}
                Err(error) => return Err(error.into()),
            }
            self.groups.pop();
        }
        if self.host.history_usage().retained_groups != 0 {
            return Err(SessionError::History(HistoryError::Retained).into());
        }
        for tab in &mut self.tabs {
            if let ClosePhase::Unbound(state) = tab.phase {
                tab.phase =
                    ClosePhase::Released(self.host.release_provenance(&tab.document, state)?);
            }
        }
        while let Some(attempt) = self.attempts.last() {
            self.host.retire_save(&attempt.task)?;
            self.attempts.pop();
        }
        self.retire_exports()?;
        self.retire_disposed_comment_reads()?;
        for tab in &mut self.tabs {
            if let ClosePhase::Released(state) = tab.phase {
                // This trusted synchronous host exposes no managed reader/view,
                // async worker or result-delivery loans. User consent is checked
                // above; actual native pins still independently fence reclamation.
                self.host.reclaim(
                    &tab.document,
                    state,
                    &TrustedDocumentReclamationV1 {
                        authorized: Some(true),
                        authored_disposition_ready: Some(true),
                        session_references_released: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    },
                )?;
                tab.phase = ClosePhase::Reclaimed;
            }
        }
        self.collect_diagnostics();
        self.closed = true;
        Ok(())
    }
    fn live(&self) -> Result<(), EditorError> {
        if self.closed || !matches!(self.tabs[self.active].phase, ClosePhase::Live) {
            Err(EditorError::Closed)
        } else {
            Ok(())
        }
    }
    pub fn frame(&mut self, width: u16, height: u16) -> Result<EditorFrame, EditorError> {
        let cursor = self.cursor();
        let selections: Vec<_> = if let Some(snapshot) = self.native_selection()? {
            snapshot
                .selections
                .selections()
                .iter()
                .map(|value| value.anchor.0.min(value.head.0)..value.anchor.0.max(value.head.0))
                .collect()
        } else {
            self.selection().into_iter().collect()
        };
        let selection_count = self.selection_count();
        let dirty = self.is_dirty()?;
        let grid = self.unicode_grid()?;
        let grid = grid.as_deref().map(Vec::as_slice);
        let read = self.host.read(&self.tabs[self.active].document)?;
        let text = read.source.text();
        let ranges = line_ranges(text);
        let row = ranges.iter().rposition(|l| l.0 <= cursor).unwrap_or(0);
        let column = visual_width(text, ranges[row].0, cursor.min(ranges[row].1), grid);
        let height_usize = usize::from(height);
        let width_usize = usize::from(width);
        if row < self.top {
            self.top = row;
        }
        if row >= self.top + height_usize {
            self.top = row.saturating_sub(height_usize.saturating_sub(1));
        }
        if column < self.left {
            self.left = column;
        }
        if column >= self.left + width_usize {
            self.left = column.saturating_sub(width_usize.saturating_sub(1));
        }
        let lines = ranges
            .iter()
            .skip(self.top)
            .take(height_usize)
            .map(|&(a, b)| clipped_line(text, a, b, self.left, width_usize, grid))
            .collect();
        let selection_cells = ranges
            .iter()
            .skip(self.top)
            .take(height_usize)
            .map(|&(a, b)| {
                selections
                    .iter()
                    .filter_map(|selected| {
                        if selected.is_empty() {
                            if selection_count == 1 || selected.start < a || selected.start > b {
                                return None;
                            }
                            let first = visual_width(text, a, selected.start, grid);
                            if first < self.left || first >= self.left + width_usize {
                                return None;
                            }
                            let first = (first - self.left) as u16;
                            return Some(first..first + 1);
                        }
                        if selected.start > b || selected.end <= a {
                            return None;
                        }
                        // Paint every intersected display cluster, even when a
                        // literal query selects only its combining mark. This
                        // widens paint only, never the native logical edit range.
                        let (mut column, mut first, mut last) = (0, None, 0);
                        for (offset, grapheme) in unicode::clusters(text, a, b, grid) {
                            let width = cell_width(grapheme, column);
                            if selected.start < offset + grapheme.len() && selected.end > offset {
                                first.get_or_insert(column);
                                last = column + width;
                            }
                            column += width;
                            if selected.end <= offset + grapheme.len() {
                                break;
                            }
                        }
                        if selected.end > b && b < text.len() {
                            first.get_or_insert(column);
                            last = column + 1;
                        }
                        let first = first?;
                        let first = first.saturating_sub(self.left).min(width_usize) as u16;
                        let last = last.saturating_sub(self.left).min(width_usize) as u16;
                        (first < last).then_some(first..last)
                    })
                    .collect()
            })
            .collect();
        Ok(EditorFrame {
            lines,
            selection_cells,
            selection_count,
            cursor: if width == 0 || height == 0 {
                None
            } else {
                Some(((column - self.left) as u16, (row - self.top) as u16))
            },
            status: self.status.clone(),
            active: self.active,
            documents: self.tabs.len(),
            dirty,
            editing_locked: self.editing_locked(),
        })
    }
}

fn line_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'\r' | b'\n' => {
                out.push((start, index));
                index += if text.as_bytes()[index] == b'\r'
                    && text.as_bytes().get(index + 1) == Some(&b'\n')
                {
                    2
                } else {
                    1
                };
                start = index;
            }
            _ => index += 1,
        }
    }
    out.push((start, text.len()));
    out
}
fn display(grapheme: &str, column: usize) -> (String, usize) {
    if grapheme == "\t" {
        let width = 4 - column % 4;
        return (" ".repeat(width), width);
    }
    let mut safe = String::new();
    for ch in grapheme.chars() {
        match ch {
            '\u{0}'..='\u{1f}' => {
                safe.push(char::from_u32(0x2400 + ch as u32).expect("control picture"))
            }
            '\u{7f}' => safe.push('␡'),
            c if c.is_control()
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}') =>
            {
                safe.push_str(&format!("<U+{:04X}>", c as u32))
            }
            c => safe.push(c),
        }
    }
    let width = safe.width();
    (safe, width)
}
fn visual_width(text: &str, start: usize, end: usize, grid: Option<&[BytePosition]>) -> usize {
    unicode::clusters(text, start, end, grid).fold(0, |col, (_, g)| col + cell_width(g, col))
}
fn cell_width(grapheme: &str, column: usize) -> usize {
    if grapheme == "\t" {
        return 4 - column % 4;
    }
    if grapheme
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    {
        display(grapheme, column).1
    } else {
        grapheme.width()
    }
}
fn clipped_line(
    text: &str,
    start: usize,
    end: usize,
    left: usize,
    width: usize,
    grid: Option<&[BytePosition]>,
) -> String {
    let mut out = String::new();
    let mut column = 0;
    for (_, g) in unicode::clusters(text, start, end, grid) {
        let w = cell_width(g, column);
        if column >= left + width {
            break;
        }
        if column >= left && column + w <= left + width {
            out.push_str(&display(g, column).0);
        } else if column + w > left {
            out.push_str(&" ".repeat((column + w).min(left + width) - column.max(left)));
        }
        column += w;
    }
    out
}

/// Reference editor composition. Without explicit native-Unicode opt-in,
/// UI-local graphemes use unicode-segmentation 1.13.3/Unicode 17.
pub fn reference_profile(
    resource_adapter: ErasedPathResourceAdapterFactory,
    annotation_codec: Box<dyn AnnotationCodec>,
) -> ReferenceSessionConfig {
    ReferenceSessionConfig {
        session: vellum_workspace::reference_workspace_profile(resource_adapter).into_session(),
        annotation_codec,
    }
}
