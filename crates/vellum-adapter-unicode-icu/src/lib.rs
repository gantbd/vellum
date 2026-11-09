//! Bounded, in-process Unicode 18 provider, outside engine publication.
//!
//! Exact profile: ICU4C 79.1 root character iteration, normative UAX #29 rev.49
//! default word rules without dictionary/locale tailoring, and ICU4X 2.4 data.
//! Native libraries and loader are trusted host prerequisites, not hot-swappable
//! providers. No source retention, Java, subprocess, UI, credentials or publisher.
use icu_properties::{
    CodePointMapData, CodePointSetData,
    props::{Alphabetic, GeneralCategory},
};
mod word;
use rust_icu_sys::UBreakIteratorType;
use rust_icu_ubrk::UBreakIterator;
use std::{mem::size_of, time::Instant};
use vellum_engine::{
    coordinates::BytePosition,
    coordinator::{
        BoundaryCaptureId, BoundaryError, BoundaryEvidenceId, BoundaryProfileV1, BoundaryTableV1,
        ByteRange, SourceCoordinator,
    },
    instrumentation::{Boundary, Disposition, Observation, Recorder},
};
use vellum_sdk::unicode::{
    UnicodeProvider, UnicodeProviderFactory, UnicodeProviderFailure, UnicodeProviderUsage,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_source_bytes: usize,
    /// Same population as engine BoundaryLimits: both grids plus two endpoints
    /// for each significant word. No partial table is returned on exhaustion.
    pub max_boundaries: usize,
    pub max_words: usize,
    /// Adapter-owned reserved Rust vectors, including UTF-16 mapping. Excludes
    /// allocator overhead and binding/ICU internals; not an RSS bound.
    pub max_logical_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timing {
    Off,
    ProcessService,
}
#[derive(Clone, Copy, Debug)]
pub struct IcuFactory {
    pub limits: Limits,
    pub timing: Timing,
}
impl IcuFactory {
    pub fn reference() -> Self {
        Self {
            limits: Limits {
                max_source_bytes: 65536,
                max_boundaries: 262144,
                max_words: 65536,
                max_logical_bytes: 4 * 1024 * 1024,
            },
            timing: Timing::ProcessService,
        }
    }
}
pub use vellum_sdk::unicode::UnicodeProviderFailure as Error;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureError {
    Engine(BoundaryError),
    Provider(Error),
}
pub use vellum_sdk::unicode::UnicodeProviderUsage as Usage;
#[derive(Debug)]
pub struct Provider {
    limits: Limits,
    timing: Timing,
    usage: Usage,
}

fn observe<T>(
    r: &mut Recorder,
    boundary: Boundary,
    start: Option<Instant>,
    result: &Result<T, Error>,
) {
    r.record(Observation {
        boundary,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: start.and_then(|t| Instant::now().checked_duration_since(t)),
    });
}
fn reserved<T>(count: usize) -> Result<Vec<T>, Error> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::Capacity)?;
    Ok(values)
}
struct Budget {
    map: usize,
    points: usize,
    words: usize,
}

impl Provider {
    pub fn new(limits: Limits, timing: Timing, r: &mut Recorder) -> Result<Self, Error> {
        let start = (timing == Timing::ProcessService).then(Instant::now);
        let result = if limits.max_source_bytes > i32::MAX as usize
            || limits.max_boundaries < 2
            || limits.max_logical_bytes == 0
        {
            Err(Error::InvalidLimits)
        } else {
            Ok(Self {
                limits,
                timing,
                usage: Usage::default(),
            })
        };
        observe(r, Boundary::UnicodeProviderQualification, start, &result);
        result
    }
    pub fn usage(&self) -> Usage {
        self.usage
    }
    fn start(&self) -> Option<Instant> {
        (self.timing == Timing::ProcessService).then(Instant::now)
    }
    fn budget(&self, text: &str) -> Result<Budget, Error> {
        if text.len() > self.limits.max_source_bytes {
            return Err(Error::InputLimit);
        }
        let chars = text.chars().count();
        let map = text
            .encode_utf16()
            .count()
            .checked_add(1)
            .ok_or(Error::LogicalLimit)?;
        let points = chars
            .checked_add(1)
            .ok_or(Error::LogicalLimit)?
            .min(self.limits.max_boundaries);
        let words = chars.min(self.limits.max_words);
        let bytes = map
            .checked_mul(size_of::<usize>())
            .and_then(|n| {
                points
                    .checked_mul(2 * size_of::<BytePosition>())
                    .and_then(|p| n.checked_add(p))
            })
            .and_then(|n| {
                words
                    .checked_mul(size_of::<ByteRange>())
                    .and_then(|w| n.checked_add(w))
            })
            .ok_or(Error::LogicalLimit)?;
        if bytes > self.limits.max_logical_bytes {
            return Err(Error::LogicalLimit);
        }
        Ok(Budget { map, points, words })
    }
    /// Caller supplies an already authorized, exact source snapshot. This does
    /// not infer a file BOM: an initial U+FEFF remains literal segmentation data.
    pub fn segment(&mut self, text: &str, r: &mut Recorder) -> Result<BoundaryTableV1, Error> {
        increment(&mut self.usage.offered, &mut self.usage.overflowed);
        let start = self.start();
        let budget = self.budget(text);
        observe(r, Boundary::UnicodeProviderInput, start, &budget);
        let result = match budget {
            Err(e) => Err(e),
            Ok(budget) => {
                increment(&mut self.usage.admitted, &mut self.usage.overflowed);
                let start = self.start();
                let result = self.execute(text, budget);
                observe(r, Boundary::UnicodeProviderExecution, start, &result);
                result
            }
        };
        match result.as_ref().err() {
            None => increment(&mut self.usage.completed, &mut self.usage.overflowed),
            Some(error) => {
                increment(&mut self.usage.refused, &mut self.usage.overflowed);
                let count = match error {
                    Error::Capacity => &mut self.usage.capacity_refusals,
                    Error::Native | Error::InvalidNativeBoundary => &mut self.usage.native_refusals,
                    _ => &mut self.usage.limit_refusals,
                };
                increment(count, &mut self.usage.overflowed);
            }
        }
        result
    }
    fn execute(&self, text: &str, budget: Budget) -> Result<BoundaryTableV1, Error> {
        // All adapter-owned buffers are reserved before invoking the native
        // segmenter. Neither partial output nor partially accepted evidence leaks.
        let mut mapping = reserved::<usize>(budget.map)?;
        let mut graphemes = reserved::<BytePosition>(budget.points)?;
        let mut words = reserved::<BytePosition>(budget.points)?;
        let mut significant_words = reserved::<ByteRange>(budget.words)?;
        mapping.resize(budget.map, usize::MAX);
        let mut units = 0;
        for (byte, ch) in text.char_indices() {
            mapping[units] = byte;
            units += ch.len_utf16();
        }
        mapping[units] = text.len();
        let iterator = UBreakIterator::try_new(UBreakIteratorType::UBRK_CHARACTER, "root", text)
            .map_err(|_| Error::Native)?;
        for native in std::iter::once(iterator.first()).chain(iterator) {
            let at = usize::try_from(native).map_err(|_| Error::InvalidNativeBoundary)?;
            let byte = *mapping.get(at).ok_or(Error::InvalidNativeBoundary)?;
            if byte == usize::MAX {
                return Err(Error::InvalidNativeBoundary);
            }
            push_point(&mut graphemes, byte, text, budget.points)?;
        }
        word::append(text, &mut words, budget.points)?;
        if graphemes.last() != Some(&BytePosition(text.len()))
            || words.last() != Some(&BytePosition(text.len()))
        {
            return Err(Error::InvalidNativeBoundary);
        }
        let alphabetic = CodePointSetData::new::<Alphabetic>();
        let categories = CodePointMapData::<GeneralCategory>::new();
        let mut count = graphemes
            .len()
            .checked_add(words.len())
            .ok_or(Error::OutputLimit)?;
        if count > self.limits.max_boundaries {
            return Err(Error::OutputLimit);
        }
        for pair in words.windows(2) {
            let (start, end) = (pair[0].0, pair[1].0);
            let significant = text[start..end].chars().any(|ch| {
                alphabetic.contains(ch)
                    || matches!(
                        categories.get(ch),
                        GeneralCategory::DecimalNumber
                            | GeneralCategory::LetterNumber
                            | GeneralCategory::OtherNumber
                    )
            });
            if significant {
                count = count.checked_add(2).ok_or(Error::OutputLimit)?;
                if significant_words.len() >= budget.words || count > self.limits.max_boundaries {
                    return Err(Error::OutputLimit);
                }
                significant_words.push(ByteRange { start, end });
            }
        }
        Ok(BoundaryTableV1 {
            profile: BoundaryProfileV1::Unicode18Uax29Revision49,
            graphemes,
            words,
            significant_words,
        })
    }
    /// Uses the original engine capture, then revalidates it at acceptance.
    /// Failure leaves that capture owned by the caller for retry or retirement;
    /// success returns the engine's evidence handle, not a second authority.
    pub fn attest_capture(
        &mut self,
        owner: &mut SourceCoordinator,
        capture: &BoundaryCaptureId,
        r: &mut Recorder,
    ) -> Result<BoundaryEvidenceId, CaptureError> {
        let source = owner
            .boundary_source(capture, r)
            .map_err(CaptureError::Engine)?;
        let table = self
            .segment(source.text, r)
            .map_err(CaptureError::Provider)?;
        owner
            .accept_boundary_evidence(capture, &table, r)
            .map_err(CaptureError::Engine)
    }
}

impl UnicodeProvider for Provider {
    fn segment(
        &mut self,
        text: &str,
        recorder: &mut Recorder,
    ) -> Result<BoundaryTableV1, UnicodeProviderFailure> {
        Provider::segment(self, text, recorder)
    }

    fn usage(&self) -> UnicodeProviderUsage {
        Provider::usage(self)
    }
}

impl UnicodeProviderFactory for IcuFactory {
    fn build(
        self,
        recorder: &mut Recorder,
    ) -> Result<Box<dyn UnicodeProvider>, UnicodeProviderFailure> {
        Ok(Box::new(Provider::new(self.limits, self.timing, recorder)?))
    }
}
fn increment(value: &mut u64, overflowed: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next;
    } else {
        *overflowed = true;
    }
}
fn push_point(
    grid: &mut Vec<BytePosition>,
    byte: usize,
    text: &str,
    limit: usize,
) -> Result<(), Error> {
    if !text.is_char_boundary(byte) || grid.last().map_or(byte != 0, |last| byte <= last.0) {
        return Err(Error::InvalidNativeBoundary);
    }
    if grid.len() >= limit {
        return Err(Error::OutputLimit);
    }
    grid.push(BytePosition(byte));
    Ok(())
}

#[cfg(test)]
mod tests;
