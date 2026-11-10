//! Adapter-neutral Unicode boundary provider algebra.

use vellum_engine::coordinator::BoundaryTableV1;
use vellum_engine::instrumentation::Recorder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnicodeProviderFailure {
    InvalidLimits,
    InputLimit,
    OutputLimit,
    LogicalLimit,
    Capacity,
    Native,
    InvalidNativeBoundary,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnicodeProviderUsage {
    pub offered: u64,
    pub admitted: u64,
    pub completed: u64,
    pub refused: u64,
    pub limit_refusals: u64,
    pub capacity_refusals: u64,
    pub native_refusals: u64,
    pub overflowed: bool,
}

/// A bounded read-only interpretation of scalar text as native boundary facts.
pub trait UnicodeProvider {
    fn segment(
        &mut self,
        text: &str,
        recorder: &mut Recorder,
    ) -> Result<BoundaryTableV1, UnicodeProviderFailure>;

    fn usage(&self) -> UnicodeProviderUsage;
}

/// Composition-time construction keeps platform limits and timing policy out
/// of the model while preserving qualification observations in its recorder.
pub trait UnicodeProviderFactory {
    fn build(
        self,
        recorder: &mut Recorder,
    ) -> Result<Box<dyn UnicodeProvider>, UnicodeProviderFailure>;
}
