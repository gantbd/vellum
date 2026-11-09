//! Read, decode, activation and baseline adoption are separate native facts.
use super::*;
use vellum_engine::resource::{AnnotationReadStatus, AnnotationRestorationOutcome};
use vellum_sdk::annotation::{
    DecodeContext, DecodedAnnotationRead, RecordIdentity, RestorationAssociations,
};
use vellum_session::host::AnnotationReadAttempt;

pub(in crate::app) struct ReadAttempt {
    attempt: AnnotationReadAttempt,
    tab: usize,
}
pub(super) enum Approval<'a> {
    Borrowed(&'a RestorationAssociations),
    Owned(RestorationAssociations),
}
impl Approval<'_> {
    fn get(&self) -> &RestorationAssociations {
        match self {
            Self::Borrowed(a) => a,
            Self::Owned(a) => a,
        }
    }
}
pub(super) struct PreparedLoad<'a> {
    pub(super) decoded: DecodedAnnotationRead,
    pub(super) approved: Approval<'a>,
    pub(super) names: Option<(Names, Vec<RecordIdentity>)>,
}
impl Editor {
    /// Explicit first load into an untouched annotation set. Never merges over
    /// authored comments, reacquires identities from a path, or retries a failed
    /// original. A failure can occur AFTER restoration; inspect the retained
    /// native status/outcome rather than assuming no publication occurred.
    pub fn load_comments(
        &mut self,
        context: &DecodeContext,
        approved: &RestorationAssociations,
        limits: CodecLimits,
        max_baseline_bytes: usize,
    ) -> Result<AnnotationRestorationOutcome, EditorError> {
        self.load_comments_prepared(limits, max_baseline_bytes, |host, codec, attempt, _| {
            Ok(PreparedLoad {
                decoded: host.decode_annotations(codec, attempt, context, limits)?,
                approved: Approval::Borrowed(approved),
                names: None,
            })
        })
    }
    pub(super) fn load_comments_prepared<'a>(
        &mut self,
        limits: CodecLimits,
        max_baseline_bytes: usize,
        prepare: impl FnOnce(
            &mut LocalHost,
            &dyn vellum_sdk::annotation::AnnotationCodec,
            &AnnotationReadAttempt,
            &LocalDocument,
        ) -> Result<PreparedLoad<'a>, EditorError>,
    ) -> Result<AnnotationRestorationOutcome, EditorError> {
        let result = (|| {
            self.live()?;
            let summary = self
                .host
                .annotation_document_summary(&self.tabs[self.active].document)?;
            if summary.set_version != 0
                || summary.present_records != 0
                || self.comment_reads.iter().any(|r| r.tab == self.active)
            {
                return Err(EditorError::Position);
            }
            let destination = self.tabs[self.active]
                .comment_file
                .as_ref()
                .ok_or(EditorError::CommentsUnavailable)?;
            self.comment_reads
                .try_reserve(1)
                .map_err(|_| EditorError::Capacity)?;
            if self.comment_encoder.is_none() {
                self.comment_encoder = Some(self.host.register_annotation_encoder()?);
            }
            if self.comment_restorer.is_none() {
                self.comment_restorer = Some(self.host.register_annotation_restorer(
                    self.comment_encoder.as_ref().expect("registered encoder"),
                )?);
            }
            let source = self.host.read(&self.tabs[self.active].document)?.version;
            let attempt =
                self.host
                    .queue_annotation_read(destination, &source, summary.set_version)?;
            self.comment_reads.push(ReadAttempt {
                attempt,
                tab: self.active,
            });
            let original = &self
                .comment_reads
                .last()
                .expect("retained original read")
                .attempt;
            let report = self.host.service_annotation_read(original)?;
            if report.failure.is_some()
                || report.status.failure.is_some()
                || report.status.contested
                || report.status.refused
                || !report.status.complete
                || !report.status.quiescent
            {
                return Err(EditorError::CommentsIncomplete);
            }
            let prepared = prepare(
                &mut self.host,
                self.annotation_codec.as_ref(),
                original,
                &self.tabs[self.active].document,
            )?;
            let outcome = self.host.restore_annotations(
                self.annotation_codec.as_ref(),
                prepared.decoded,
                prepared.approved.get(),
                self.comment_restorer.as_ref().expect("registered restorer"),
                limits,
            )?;
            // Native publication already occurred. Baseline adoption below is
            // independent and must never roll it back or hide its new identities.
            self.comment_cache = None;
            if let Some((mut names, labels)) = prepared.names {
                assert_eq!(
                    outcome.annotations.len(),
                    labels.len(),
                    "original restored identities"
                );
                names
                    .records
                    .extend(outcome.annotations.iter().zip(labels).map(|(a, identity)| {
                        vellum_sdk::annotation::RecordAssociation {
                            native: a.annotation.clone(),
                            identity,
                        }
                    }));
                self.tabs[self.active].comment_names = Some(names);
                self.tabs[self.active].comment_file_requires_load = false;
            }
            self.tabs[self.active].comment_anchor =
                outcome.annotations.first().map(|a| a.annotation.clone());
            if !self.comment_read_baselines {
                self.host.enable_annotation_read_baselines()?;
                self.comment_read_baselines = true;
            }
            self.host
                .adopt_loaded_annotation_baseline(original, max_baseline_bytes)?;
            self.status =
                "Comments restored; separate Loaded baseline adopted — no source write".into();
            Ok(self.host.annotation_restoration_outcome(original)?)
        })();
        if let Err(error) = &result {
            self.status = format!(
                "Comment load not completed: {error}; inspect retained read/restoration facts"
            );
        }
        self.collect_diagnostics();
        result
    }
    pub fn last_comment_read_status(
        &mut self,
    ) -> Result<Option<AnnotationReadStatus>, EditorError> {
        let result = self
            .comment_reads
            .iter()
            .rev()
            .find(|r| r.tab == self.active)
            .map(|r| self.host.annotation_read_status(&r.attempt))
            .transpose()
            .map_err(Into::into);
        self.collect_diagnostics();
        result
    }
    /// None means no retained read, not an assertion that an existing read
    /// restored nothing. A retained, non-restored read returns the native refusal.
    pub fn last_comment_restoration_outcome(
        &mut self,
    ) -> Result<Option<AnnotationRestorationOutcome>, EditorError> {
        let result = self
            .comment_reads
            .iter()
            .rev()
            .find(|r| r.tab == self.active)
            .map(|r| self.host.annotation_restoration_outcome(&r.attempt))
            .transpose()
            .map_err(Into::into);
        self.collect_diagnostics();
        result
    }
    /// Retry only baseline adoption after a known restoration, never read/restore
    /// again. Native binding/currentness/budget/loan checks remain authoritative.
    pub fn adopt_comment_read_baseline(&mut self, max_bytes: usize) -> Result<bool, EditorError> {
        self.live()?;
        let attempt = &self
            .comment_reads
            .iter()
            .rev()
            .find(|r| r.tab == self.active)
            .ok_or(EditorError::CommentsUnavailable)?
            .attempt;
        if !self.comment_read_baselines {
            self.host.enable_annotation_read_baselines()?;
            self.comment_read_baselines = true;
        }
        let result = self
            .host
            .adopt_loaded_annotation_baseline(attempt, max_bytes)
            .map_err(Into::into);
        self.status = if result.is_ok() {
            "Loaded comment baseline adopted; no read, restoration or source write repeated".into()
        } else {
            "Loaded comment baseline not adopted; original restoration facts remain authoritative"
                .into()
        };
        self.collect_diagnostics();
        result
    }
    /// Explicit loss of a failed, non-restored read's retained result. This is not
    /// successful load or permission to discard a restored document's provenance.
    pub fn retire_failed_comment_read(&mut self) -> Result<bool, EditorError> {
        self.live()?;
        let Some(index) = self
            .comment_reads
            .iter()
            .rposition(|r| r.tab == self.active)
        else {
            return Ok(false);
        };
        let attempt = &self.comment_reads[index].attempt;
        if self.host.annotation_read_status(attempt)?.restored {
            return Err(EditorError::Position);
        }
        self.host.stop_annotation_read(attempt)?;
        self.host.retire_annotation_read(attempt)?;
        self.comment_reads.remove(index);
        self.collect_diagnostics();
        Ok(true)
    }
    /// Explicit historical-result discharge after an independently acknowledged
    /// current Written checkpoint; native provenance and loans still fence it.
    pub fn retire_checkpointed_comment_read(&mut self) -> Result<bool, EditorError> {
        self.live()?;
        let Some(index) = self
            .comment_reads
            .iter()
            .rposition(|r| r.tab == self.active)
        else {
            return Ok(false);
        };
        let attempt = &self.comment_reads[index].attempt;
        self.host.stop_annotation_read(attempt)?;
        self.host.retire_checkpointed_annotation_read(attempt)?;
        self.comment_reads.remove(index);
        self.collect_diagnostics();
        Ok(true)
    }
    pub(in crate::app) fn stop_comment_reads(&mut self) -> Result<(), EditorError> {
        for read in &self.comment_reads {
            self.host.stop_annotation_read(&read.attempt)?;
            if !self.host.annotation_read_status(&read.attempt)?.quiescent {
                return Err(EditorError::CommentsIncomplete);
            }
        }
        Ok(())
    }
    pub(in crate::app) fn retire_disposed_comment_reads(&mut self) -> Result<(), EditorError> {
        while let Some(read) = self.comment_reads.last() {
            if self.host.annotation_read_status(&read.attempt)?.restored {
                self.host.retire_disposed_annotation_read(&read.attempt)?;
            } else {
                self.host.retire_annotation_read(&read.attempt)?;
            }
            self.comment_reads.pop();
        }
        if self.comment_read_baselines {
            self.host.disable_annotation_read_baselines()?;
            self.comment_read_baselines = false;
        }
        Ok(())
    }
}
