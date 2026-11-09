//! Native annotations presented as comments; no new annotation/history owner.
use super::*;
use vellum_engine::{
    attachment::{AttachmentTarget, Placement, PointProfile},
    coordinator::{
        AnnotationId, AnnotationMutation, LogicalViewContext, SourceTarget, TargetBasis,
    },
};

#[derive(Clone)]
pub struct CommentDraft {
    context: LogicalViewContext,
    targets: Vec<AttachmentTarget>,
}
impl CommentDraft {
    pub fn target_count(&self) -> usize {
        self.targets.len()
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct CommentHandle {
    annotation: AnnotationId,
    document: vellum_engine::coordinator::DocumentId,
    content: u64,
    target: u64,
}
pub struct CommentRow {
    pub handle: CommentHandle,
    pub body: String,
    pub placement: Placement,
    pub authored_target: AttachmentTarget,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(concat!(stringify!($t), "(<redacted>)")) }
})+ }; }
redacted!(CommentDraft, CommentHandle, CommentRow);
pub(super) struct Cache {
    source: DocumentVersion,
    set: u64,
    rows: Vec<CommentRow>,
}
impl Editor {
    pub fn comments_enabled(&self) -> bool {
        self.annotation_limits.is_some()
    }
    pub fn comment_body_limit(&self) -> Option<usize> {
        self.annotation_limits.map(|limits| limits.max_body_bytes)
    }
    pub fn comments_locked(&self) -> bool {
        self.tabs[self.active].comments_locked
    }
    /// Session comments start without a saved baseline. Explicitly bound/written
    /// files can establish one; deleted or conservatively changed source/set
    /// captures remain dirty. No source/body copy or native snapshot per frame.
    pub fn comments_dirty(&mut self) -> Result<bool, EditorError> {
        self.live()?;
        self.comments_dirty_at(self.active)
    }
    pub fn comments(&mut self) -> Result<&[CommentRow], EditorError> {
        self.live()?;
        if !self.comments_enabled() {
            return Err(EditorError::CommentsUnavailable);
        }
        let source = self.version()?;
        let summary = self
            .host
            .annotation_document_summary(&self.tabs[self.active].document)?;
        if !self
            .comment_cache
            .as_ref()
            .is_some_and(|c| c.source == source && c.set == summary.set_version)
        {
            let mut old = self.comment_cache.take();
            let facts = self
                .host
                .annotation_document_facts(&source, summary.set_version)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(facts.len())
                .map_err(|_| EditorError::Capacity)?;
            for fact in facts {
                let read = self.host.annotation(&fact.annotation)?;
                let body = if let Some(row) = old.as_mut().and_then(|c| {
                    c.rows.iter_mut().find(|r| {
                        r.handle.annotation == fact.annotation
                            && r.handle.content == fact.content_version
                    })
                }) {
                    std::mem::take(&mut row.body)
                } else {
                    let mut body = String::new();
                    body.try_reserve_exact(read.body.len())
                        .map_err(|_| EditorError::Capacity)?;
                    body.push_str(read.body);
                    body
                };
                rows.push(CommentRow {
                    handle: CommentHandle {
                        annotation: fact.annotation,
                        document: source.document.clone(),
                        content: fact.content_version,
                        target: fact.target_generation,
                    },
                    body,
                    placement: read.placement,
                    authored_target: read.authored.target,
                });
            }
            self.comment_cache = Some(Cache {
                source,
                set: summary.set_version,
                rows,
            });
        }
        self.collect_diagnostics();
        Ok(&self
            .comment_cache
            .as_ref()
            .expect("installed complete comment cache")
            .rows)
    }
    pub fn cached_comments(&self, source: &DocumentVersion) -> Option<&[CommentRow]> {
        self.comment_cache
            .as_ref()
            .filter(|c| &c.source == source)
            .map(|c| c.rows.as_slice())
    }
    /// One explicit annotation per normalized selection; empty targets use the
    /// explicitly chosen point profile. A later prompt cannot reacquire focus.
    pub fn begin_comment(&mut self, profile: PointProfile) -> Result<CommentDraft, EditorError> {
        self.live()?;
        if !self.comments_enabled() {
            return Err(EditorError::CommentsUnavailable);
        }
        let view = self.native_selection()?.ok_or(EditorError::Position)?;
        let context = view.context().ok_or(EditorError::Position)?;
        let mut targets = Vec::new();
        targets
            .try_reserve_exact(view.selections.selections().len())
            .map_err(|_| EditorError::Capacity)?;
        for s in view.selections.selections() {
            let start = s.anchor.0.min(s.head.0);
            let end = s.anchor.0.max(s.head.0);
            targets.push(if start == end {
                AttachmentTarget::Point {
                    offset: start,
                    affinity: s.affinity,
                    profile,
                }
            } else {
                AttachmentTarget::ContentRange { start, end }
            });
        }
        Ok(CommentDraft { context, targets })
    }
    pub fn create_comments(
        &mut self,
        draft: &CommentDraft,
        body: &str,
    ) -> Result<Vec<AnnotationId>, EditorError> {
        self.live()?;
        if self.native_selection()?.and_then(|s| s.context()).as_ref() != Some(&draft.context) {
            return Err(EditorError::Position);
        }
        let limits = self
            .annotation_limits
            .ok_or(EditorError::CommentsUnavailable)?;
        if body.len() > limits.max_body_bytes {
            return Err(EditorError::Capacity);
        }
        let mut mutations = Vec::new();
        mutations
            .try_reserve_exact(draft.targets.len())
            .map_err(|_| EditorError::Capacity)?;
        for target in &draft.targets {
            let mut text = String::new();
            text.try_reserve_exact(body.len())
                .map_err(|_| EditorError::Capacity)?;
            text.push_str(body);
            mutations.push(AnnotationMutation::Create {
                target: SourceTarget {
                    source: draft.context.source.clone(),
                    target: *target,
                    basis: TargetBasis::Before,
                },
                body: text,
            });
        }
        self.groups
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let names = self.prepare_comment_names(&draft.context.source, mutations.len())?;
        let result =
            self.host
                .commit_compound(&[], std::slice::from_ref(&draft.context.source), &mutations);
        self.collect_diagnostics();
        let commit = result?;
        self.commit_comment_names(names, &commit.created);
        if let Some(group) = commit.outcome.history_group {
            self.groups.push(group);
        }
        self.tabs[self.active].comment_anchor = commit.created.last().cloned();
        self.comment_cache = None;
        self.status = "Session comments created; source save does not persist comments".into();
        Ok(commit.created)
    }
    fn comment_owner(&mut self, handle: &CommentHandle) -> Result<(), EditorError> {
        self.live()?;
        if self.version()?.document != handle.document {
            return Err(EditorError::Position);
        }
        Ok(())
    }
    pub fn update_comment(
        &mut self,
        handle: &CommentHandle,
        body: &str,
    ) -> Result<(), EditorError> {
        self.comment_owner(handle)?;
        let limit = self
            .annotation_limits
            .ok_or(EditorError::CommentsUnavailable)?
            .max_body_bytes;
        if body.len() > limit {
            return Err(EditorError::Capacity);
        }
        let mut text = String::new();
        text.try_reserve_exact(body.len())
            .map_err(|_| EditorError::Capacity)?;
        text.push_str(body);
        self.comment_mutation(
            handle,
            AnnotationMutation::UpdateBody {
                annotation: handle.annotation.clone(),
                expected_content: handle.content,
                body: text,
            },
        )
    }
    pub fn delete_comment(&mut self, handle: &CommentHandle) -> Result<(), EditorError> {
        self.comment_owner(handle)?;
        self.comment_mutation(
            handle,
            AnnotationMutation::Delete {
                annotation: handle.annotation.clone(),
                expected_content: handle.content,
                expected_target: handle.target,
            },
        )
    }
    pub fn reattach_comment(
        &mut self,
        handle: &CommentHandle,
        draft: &CommentDraft,
    ) -> Result<(), EditorError> {
        self.comment_owner(handle)?;
        if draft.targets.len() != 1
            || self.native_selection()?.and_then(|s| s.context()).as_ref() != Some(&draft.context)
        {
            return Err(EditorError::Position);
        }
        let names = self.prepare_comment_names(&draft.context.source, 0)?;
        self.comment_mutation(
            handle,
            AnnotationMutation::Reattach {
                annotation: handle.annotation.clone(),
                expected_target: handle.target,
                target: SourceTarget {
                    source: draft.context.source.clone(),
                    target: draft.targets[0],
                    basis: TargetBasis::Before,
                },
            },
        )?;
        self.commit_comment_names(names, &[]);
        Ok(())
    }
    fn comment_mutation(
        &mut self,
        handle: &CommentHandle,
        mutation: AnnotationMutation,
    ) -> Result<(), EditorError> {
        self.groups
            .try_reserve(1)
            .map_err(|_| EditorError::Capacity)?;
        let result = self.host.commit_compound(&[], &[], &[mutation]);
        self.collect_diagnostics();
        let commit = result?;
        if let Some(group) = commit.outcome.history_group {
            self.groups.push(group);
        }
        self.tabs[self.active].comment_anchor = Some(handle.annotation.clone());
        self.comment_cache = None;
        self.status = "Session comment changed; source save does not persist comments".into();
        Ok(())
    }
    pub fn reverse_comment(&mut self, direction: HistoryDirection) -> Result<bool, EditorError> {
        self.live()?;
        let before = self.version()?;
        let id = self.tabs[self.active]
            .comment_anchor
            .clone()
            .ok_or(EditorError::Position)?;
        let result = self.host.reverse_annotation(&id, direction);
        self.collect_diagnostics();
        let changed = result?;
        self.comment_cache = None;
        if self.version()? != before {
            self.clamp_cursor()?;
        } else {
            self.sync_native_selection()?;
        }
        self.status =
            "Comment history checked; linked source changes follow the native group".into();
        Ok(changed)
    }
    pub fn set_comments_locked(&mut self, locked: bool) -> Result<(), EditorError> {
        self.live()?;
        if !self.comments_enabled() {
            return Err(EditorError::CommentsUnavailable);
        }
        self.host
            .set_annotations_locked(&self.tabs[self.active].document, locked)?;
        self.tabs[self.active].comments_locked = locked;
        self.collect_diagnostics();
        Ok(())
    }
}
