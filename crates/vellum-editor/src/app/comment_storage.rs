//! Explicit trusted-host persistence. No path-derived identities, auto sidecars,
//! source-save coupling or fabricated historical witnesses.
use super::*;
use vellum_engine::coordinator::AnnotationSnapshot;
use vellum_host::annotation::{CaptureAssociations, CodecLimits};
mod managed;
mod read;
pub use managed::CommentFileLimits;
pub use managed::CommentFilePlan;
pub(super) use managed::Names;
pub(super) use read::ReadAttempt;

impl Editor {
    /// Select an existing, distinct resource. Binding alone neither loads its
    /// records nor adopts a saved baseline. First write requires explicit Save.
    /// The reference integration currently keeps one association per document.
    pub fn bind_comment_file(&mut self, path: &Path) -> Result<(), EditorError> {
        let result = (|| {
            self.live()?;
            if !self.comments_enabled() {
                return Err(EditorError::CommentsUnavailable);
            }
            if self.tabs[self.active].comment_file.is_some() {
                return Err(EditorError::Position);
            }
            let resource = self
                .host
                .bind_annotations(&self.tabs[self.active].document, path)?;
            self.tabs[self.active].comment_file = Some(resource);
            self.status = "Comment file associated; not loaded or saved".into();
            Ok(())
        })();
        self.collect_diagnostics();
        result
    }
    pub fn comment_file_bound(&self) -> bool {
        self.tabs[self.active].comment_file.is_some()
    }

    /// Exact privileged capture for the host's explicit identity/source mapping.
    /// Retaining it retains a native loan; drop it before closing the editor.
    pub fn capture_comments(&mut self) -> Result<AnnotationSnapshot, EditorError> {
        let result = (|| {
            self.live()?;
            let source = self.version()?;
            let set = self
                .host
                .annotation_set_version(&self.tabs[self.active].document)?;
            Ok(self.host.capture_annotations(&source, set)?)
        })();
        self.collect_diagnostics();
        result
    }

    /// Explicit caller-authorized save of the original capture. Associations and
    /// codec budgets remain supplied host premises, not authority from JSON.
    /// No automatic refresh, source write, compensating overwrite or retry.
    pub fn save_comments(
        &mut self,
        snapshot: &AnnotationSnapshot,
        associations: &CaptureAssociations,
        limits: CodecLimits,
    ) -> Result<(), EditorError> {
        let result = (|| {
            self.live()?;
            if self.version()? != *snapshot.source()
                || self
                    .host
                    .annotation_set_version(&self.tabs[self.active].document)?
                    != snapshot.set_version()
            {
                return Err(EditorError::Position);
            }
            let destination = self.tabs[self.active]
                .comment_file
                .clone()
                .ok_or(EditorError::CommentsUnavailable)?;
            self.attempts
                .try_reserve(1)
                .map_err(|_| EditorError::Capacity)?;
            if self.comment_encoder.is_none() {
                self.comment_encoder = Some(self.host.register_annotation_encoder()?);
            }
            let encoded = self.host.encode_annotations(
                self.annotation_codec.as_ref(),
                snapshot,
                associations,
                limits,
                self.comment_encoder.as_ref().expect("registered encoder"),
            )?;
            let task = self.host.queue_annotation_save(
                &destination,
                encoded,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
            )?;
            // Retain the original native attempt BEFORE any external effect.
            self.attempts.push(Attempt {
                task: task.clone(),
                tab: self.active,
                annotations: true,
            });
            let report = self.host.service_annotation_save(&destination, &task)?;
            self.status = format!(
                "Comment save: {:?}; sync: {:?}; cleanup failed: {}",
                report.outcome.application, report.outcome.synchronization, report.cleanup_failed
            );
            if report.failure.is_some()
                || report.cleanup_failed
                || report.outcome.contested
                || report.outcome.application != Application::Written
                || report.outcome.synchronization != Synchronization::Succeeded
                || !report.outcome.quiescent
                || !report.outcome.no_future_effects
            {
                return Err(EditorError::SaveIncomplete);
            }
            self.host
                .publish_annotation_save_baseline(&destination, &task)?;
            // Superseded originals may discharge; retained/failed tasks remain
            // addressable and continue to fence close through their native facts.
            let mut index = 0;
            while index + 1 < self.attempts.len() {
                if self.attempts[index].tab != self.active || !self.attempts[index].annotations {
                    index += 1;
                    continue;
                }
                self.orphaned_comment_artifacts
                    .try_reserve(1)
                    .map_err(|_| EditorError::Capacity)?;
                let artifact = self.host.save_outcome(&self.attempts[index].task)?.artifact;
                match self.host.retire_save(&self.attempts[index].task) {
                    Ok(()) => {
                        self.orphaned_comment_artifacts.push(artifact);
                        self.attempts.remove(index);
                    }
                    Err(SessionError::Resource(ResourceError::Retained)) => index += 1,
                    Err(error) => return Err(error.into()),
                }
            }
            self.retire_orphaned_comment_artifacts()?;
            self.status = "Comments saved separately — Written; synchronization Succeeded".into();
            Ok(())
        })();
        if let Err(error) = &result {
            self.status = format!(
                "Comment save not completed: {error}; retained outcomes remain authoritative"
            );
        }
        self.collect_diagnostics();
        result
    }
    pub fn last_comment_save_outcome(&mut self) -> Result<Option<SaveOutcome>, EditorError> {
        self.attempts
            .iter()
            .rev()
            .find(|a| a.annotations && a.tab == self.active)
            .map(|a| self.host.save_outcome(&a.task))
            .transpose()
            .map_err(Into::into)
    }
    pub(super) fn retire_orphaned_comment_artifacts(&mut self) -> Result<(), EditorError> {
        while let Some(artifact) = self.orphaned_comment_artifacts.last() {
            self.host.retire_resource_artifact(artifact)?;
            self.orphaned_comment_artifacts.pop();
        }
        Ok(())
    }
    pub(super) fn comments_dirty_at(&mut self, index: usize) -> Result<bool, EditorError> {
        if !self.comments_enabled() {
            return Ok(false);
        }
        let document = &self.tabs[index].document;
        let summary = self.host.annotation_document_summary(document)?;
        if self.tabs[index].comment_file.is_none() {
            return Ok(summary.present_records != 0);
        }
        for attempt in self
            .attempts
            .iter()
            .filter(|a| a.annotations && a.tab == index)
        {
            let outcome = self.host.save_outcome(&attempt.task)?;
            if outcome.contested || !outcome.quiescent || !outcome.no_future_effects {
                return Ok(true);
            }
        }
        let Some(baseline) = self.host.annotation_baseline_evidence(document)? else {
            return Ok(summary.present_records != 0);
        };
        let (source, set) = match baseline {
            vellum_engine::resource::AnnotationBaselineEvidence::Written(fact) => {
                (fact.source, fact.set_version)
            }
            vellum_engine::resource::AnnotationBaselineEvidence::Loaded(fact) => {
                (fact.source, fact.set_version)
            }
        };
        // Exact unchanged native source + set is a cheap sufficient equality
        // witness. Undo or a new source revision conservatively requires another
        // save; this is not output-byte comparison or external freshness.
        Ok(set != summary.set_version || source != self.host.read(document)?.version)
    }
}
