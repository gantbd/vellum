//! Optional example-host Unicode integration. Native owners remain in LocalHost;
//! the sole cached projection is exact-version-bound and is not write authority.
use super::*;
use std::sync::Arc;
use vellum_engine::coordinates::BytePosition;
type Grid = Arc<Vec<BytePosition>>;

#[cfg(feature = "native-unicode")]
mod native {
    use super::*;
    use vellum_engine::{coordinator::*, instrumentation::Recorder};
    use vellum_unicode_icu::{Limits, Provider, Timing, Usage};

    pub struct NativeUnicodeProfile {
        pub provider: Limits,
        pub boundaries: BoundaryLimits,
        pub commands: UnicodeCommandBudget,
        pub capture_capacity: usize,
        pub timing: Timing,
    }
    impl NativeUnicodeProfile {
        /// Explicit 64 KiB example-host budgets, not engine defaults or SLOs.
        pub fn reference() -> Self {
            Self {
                provider: Limits {
                    max_source_bytes: 65536,
                    max_boundaries: 262144,
                    max_words: 65536,
                    max_logical_bytes: 4 * 1024 * 1024,
                },
                boundaries: BoundaryLimits {
                    max_issued: 1_000_000,
                    max_retained: 1,
                    max_source_bytes: 65536,
                    max_boundaries: 262144,
                    max_words: 65536,
                    max_evidence_bytes: 4 * 1024 * 1024,
                    max_retained_bytes: 4 * 1024 * 1024,
                },
                commands: UnicodeCommandBudget {
                    max_steps: 16,
                    max_derived_bytes: 262144,
                },
                capture_capacity: 256,
                timing: Timing::ProcessService,
            }
        }
    }
    enum Held {
        Pending {
            version: DocumentVersion,
            id: BoundaryCaptureId,
        },
        Ready {
            version: DocumentVersion,
            id: BoundaryEvidenceId,
            grid: Grid,
        },
    }
    impl Held {
        fn version(&self) -> &DocumentVersion {
            match self {
                Self::Pending { version, .. } | Self::Ready { version, .. } => version,
            }
        }
    }
    pub(in crate::app) struct State {
        provider: Provider,
        recorder: Recorder,
        held: Option<Held>,
    }
    fn release(state: &mut State, host: &mut LocalHost) -> Result<(), EditorError> {
        match &state.held {
            Some(Held::Pending { id, .. }) => host.retire_boundary_capture(id)?,
            Some(Held::Ready { id, .. }) => host.retire_boundary_evidence(id)?,
            None => {}
        }
        state.held = None;
        Ok(())
    }
    impl Editor {
        pub fn enable_native_unicode(
            &mut self,
            profile: NativeUnicodeProfile,
        ) -> Result<(), EditorError> {
            self.live()?;
            if self.native_unicode.is_some() || !self.native_views {
                return Err(SessionError::InvalidConfig.into());
            }
            let mut recorder = Recorder::try_with_capacity(profile.capture_capacity)
                .map_err(|_| EditorError::Capacity)?;
            let provider = Provider::new(profile.provider, profile.timing, &mut recorder)
                .map_err(EditorError::Unicode)?;
            self.host.configure_boundaries(profile.boundaries)?;
            self.host.configure_unicode_commands(profile.commands)?;
            self.native_unicode = Some(State {
                provider,
                recorder,
                held: None,
            });
            self.status =
                "Ready — native Unicode 18 source commands; rendering remains separate".into();
            Ok(())
        }
        pub fn unicode_provider_usage(&self) -> Option<Usage> {
            self.native_unicode.as_ref().map(|s| s.provider.usage())
        }
        pub fn native_unicode_enabled(&self) -> bool {
            self.native_unicode.is_some()
        }
        fn check_unicode_point(
            &mut self,
            expected: &DocumentVersion,
            point: usize,
        ) -> Result<(), EditorError> {
            self.live()?;
            let read = self.host.read(&self.tabs[self.active].document)?;
            if &read.version != expected {
                return Err(SessionError::Selection(
                    vellum_engine::selection::SelectionError::Stale,
                )
                .into());
            }
            if !read.source.text().is_char_boundary(point) {
                return Err(EditorError::Position);
            }
            Ok(())
        }
        /// Read-only display-proposal inspection for one collapsed caret. The
        /// eventual command independently derives and validates its own target.
        /// A point inside a cluster refuses; this is not executable-target repair.
        pub fn native_grapheme_deletion_target(
            &mut self,
            expected: &DocumentVersion,
            point: usize,
            backward: bool,
        ) -> Result<std::ops::Range<usize>, EditorError> {
            self.check_unicode_point(expected, point)?;
            let grid = self.unicode_grid()?.ok_or(SessionError::InvalidConfig)?;
            let index = grid
                .binary_search_by_key(&point, |p| p.0)
                .map_err(|_| SessionError::Unicode(UnicodeCommandError::InvalidBoundary))?;
            Ok(if backward {
                grid[index.saturating_sub(1)].0..point
            } else {
                point..grid[(index + 1).min(grid.len() - 1)].0
            })
        }
        /// Explicit source-gesture projection, before creating a logical target.
        /// Uses the existing UI 'after' convention; not for replay, SDK exact
        /// positions, literal match ranges or already admitted commands.
        pub fn project_source_caret_after(
            &mut self,
            expected: &DocumentVersion,
            point: usize,
        ) -> Result<usize, EditorError> {
            self.check_unicode_point(expected, point)?;
            let grid = self.unicode_grid()?.ok_or(SessionError::InvalidConfig)?;
            Ok(grid[grid.partition_point(|p| p.0 < point)].0)
        }
        /// Validate a literal display candidate before publication. This performs
        /// provider work but creates no capture/evidence or authority. Its offers
        /// are separate work from later exact-version cache construction.
        pub fn project_candidate_caret_after(
            &mut self,
            text: &str,
            point: usize,
        ) -> Result<usize, EditorError> {
            self.live()?;
            if !text.is_char_boundary(point) {
                return Err(EditorError::Position);
            }
            let state = self
                .native_unicode
                .as_mut()
                .ok_or(SessionError::InvalidConfig)?;
            let table = state
                .provider
                .segment(text, &mut state.recorder)
                .map_err(EditorError::Unicode)?;
            Ok(table.graphemes[table.graphemes.partition_point(|p| p.0 < point)].0)
        }
        pub fn native_boundary_usage(&self) -> BoundaryUsage {
            self.host.boundary_usage()
        }
        pub(in crate::app) fn unicode_grid(&mut self) -> Result<Option<Grid>, EditorError> {
            let Some(mut state) = self.native_unicode.take() else {
                return Ok(None);
            };
            let result = (|| {
                self.live()?;
                let version = self.version()?;
                if state.held.as_ref().is_some_and(|h| h.version() != &version) {
                    release(&mut state, &mut self.host)?;
                }
                if state.held.is_none() {
                    let id = self.host.begin_boundary_capture(&version)?;
                    state.held = Some(Held::Pending {
                        version: version.clone(),
                        id,
                    });
                }
                if let Some(Held::Pending { id, .. }) = &state.held {
                    let source = self.host.boundary_source(id)?;
                    let table = state
                        .provider
                        .segment(source.text, &mut state.recorder)
                        .map_err(EditorError::Unicode)?;
                    let evidence = self.host.accept_boundary_evidence(id, &table)?;
                    state.held = Some(Held::Ready {
                        version,
                        id: evidence,
                        grid: Arc::new(table.graphemes),
                    });
                }
                let Some(Held::Ready { grid, .. }) = &state.held else {
                    unreachable!()
                };
                Ok(Some(grid.clone()))
            })();
            // Keep original ownership through every fallible step, even refusal.
            self.native_unicode = Some(state);
            result
        }
        pub(in crate::app) fn release_unicode(&mut self) -> Result<(), EditorError> {
            if let Some(state) = self.native_unicode.as_mut() {
                release(state, &mut self.host)?;
            }
            Ok(())
        }
        pub(in crate::app) fn unicode_dropped(&self) -> u64 {
            self.native_unicode
                .as_ref()
                .map_or(0, |s| s.recorder.dropped())
        }
        pub(in crate::app) fn pop_unicode_diagnostic(&mut self) -> Option<Observation> {
            self.native_unicode.as_mut()?.recorder.pop_front()
        }
        fn unicode_evidence(&self) -> Result<BoundaryEvidenceId, EditorError> {
            match self.native_unicode.as_ref().and_then(|s| s.held.as_ref()) {
                Some(Held::Ready { id, .. }) => Ok(id.clone()),
                _ => Err(SessionError::InvalidConfig.into()),
            }
        }
        pub(in crate::app) fn unicode_move(
            &mut self,
            backward: bool,
            extend: bool,
        ) -> Result<(), EditorError> {
            self.unicode_grid()?;
            let context = self
                .native_selection()?
                .and_then(|s| s.context())
                .ok_or(SessionError::InvalidConfig)?;
            let evidence = self.unicode_evidence()?;
            self.host
                .move_unicode(&context, &evidence, request(backward, extend))?;
            self.sync_native_selection()
        }
        pub(in crate::app) fn unicode_delete(
            &mut self,
            expected: Option<&DocumentVersion>,
            backward: bool,
        ) -> Result<(), EditorError> {
            if let Some(expected) = expected
                && &self.version()? != expected
            {
                return Err(SessionError::Selection(
                    vellum_engine::selection::SelectionError::Stale,
                )
                .into());
            }
            self.unicode_grid()?;
            let context = self
                .native_selection()?
                .and_then(|s| s.context())
                .ok_or(SessionError::InvalidConfig)?;
            let evidence = self.unicode_evidence()?;
            self.groups
                .try_reserve(1)
                .map_err(|_| EditorError::Capacity)?;
            let outcome =
                self.host
                    .delete_unicode(&context, &evidence, request(backward, false))?;
            if let Some(group) = outcome.history_group {
                self.groups.push(group);
            }
            self.clamp_cursor()?;
            self.status = "Native Unicode deletion published".into();
            self.collect_diagnostics();
            Ok(())
        }
    }
    fn request(backward: bool, extend: bool) -> UnicodeCommandV1 {
        UnicodeCommandV1 {
            unit: UnicodeUnitV1::Grapheme,
            direction: if backward {
                UnicodeDirectionV1::Backward
            } else {
                UnicodeDirectionV1::Forward
            },
            count: 1,
            extend,
        }
    }
}
#[cfg(feature = "native-unicode")]
pub use native::NativeUnicodeProfile;
#[cfg(feature = "native-unicode")]
pub(super) use native::State;

#[cfg(not(feature = "native-unicode"))]
impl Editor {
    pub(super) fn unicode_grid(&mut self) -> Result<Option<Grid>, EditorError> {
        Ok(None)
    }
    pub(super) fn release_unicode(&mut self) -> Result<(), EditorError> {
        Ok(())
    }
    pub(super) fn unicode_dropped(&self) -> u64 {
        0
    }
    pub(super) fn pop_unicode_diagnostic(&mut self) -> Option<Observation> {
        None
    }
}

/// Presentation iteration over the qualified full-source grid when enabled.
/// Width/shaping remains a separate renderer concern, not Unicode semantics.
pub(super) struct Clusters<'a> {
    text: &'a str,
    start: usize,
    end: usize,
    legacy: Option<unicode_segmentation::GraphemeIndices<'a>>,
    native: Option<std::slice::Windows<'a, BytePosition>>,
}
pub(super) fn clusters<'a>(
    text: &'a str,
    start: usize,
    end: usize,
    grid: Option<&'a [BytePosition]>,
) -> Clusters<'a> {
    let native = grid
        .map(|grid| grid[grid.partition_point(|p| p.0 <= start).saturating_sub(1)..].windows(2));
    Clusters {
        text,
        start,
        end,
        legacy: grid
            .is_none()
            .then(|| text[start..end].grapheme_indices(true)),
        native,
    }
}
impl<'a> Iterator for Clusters<'a> {
    type Item = (usize, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(legacy) = &mut self.legacy {
            return legacy.next().map(|(i, s)| (self.start + i, s));
        }
        let pair = self.native.as_mut()?.next()?;
        if pair[0].0 >= self.end {
            self.native = None;
            return None;
        }
        let start = pair[0].0.max(self.start);
        Some((start, &self.text[start..pair[1].0.min(self.end)]))
    }
}
