//! Adapter-neutral, bounded append-journal capability.
//!
//! Records are opaque to this boundary. Services own their canonical codecs
//! and recovery algebra; adapters own the durability mechanism.

pub const MAX_APPEND_JOURNAL_RECORD_BYTES_V1: usize = 8 * 1024 * 1024;
pub const MAX_APPEND_JOURNAL_BYTES_V1: usize = 128 * 1024 * 1024;
pub const MAX_APPEND_JOURNAL_RECORDS_V1: usize = 10_240;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendJournalLimitsV1 {
    pub max_records: usize,
    pub max_record_bytes: usize,
    pub max_total_bytes: usize,
}

impl AppendJournalLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_records: MAX_APPEND_JOURNAL_RECORDS_V1,
            max_record_bytes: MAX_APPEND_JOURNAL_RECORD_BYTES_V1,
            max_total_bytes: MAX_APPEND_JOURNAL_BYTES_V1,
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_records > 0
            && self.max_records <= MAX_APPEND_JOURNAL_RECORDS_V1
            && self.max_record_bytes > 0
            && self.max_record_bytes <= MAX_APPEND_JOURNAL_RECORD_BYTES_V1
            && self.max_total_bytes >= self.max_record_bytes
            && self.max_total_bytes <= MAX_APPEND_JOURNAL_BYTES_V1
    }
}

/// Truth at an append synchronization boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppendJournalCommitV1 {
    /// The complete record reached the adapter's advertised durability domain.
    Committed,
    /// The adapter proves that no record was created.
    Refused,
    /// The adapter can prove neither commit nor non-commit.
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppendJournalReadRefusalV1 {
    Unavailable,
    Limit,
}

/// Non-destructive, oldest-first opaque record storage.
pub trait AppendJournalPortV1 {
    fn read_bounded(
        &mut self,
        limits: AppendJournalLimitsV1,
    ) -> Result<Vec<Vec<u8>>, AppendJournalReadRefusalV1>;

    fn append_synchronized(&mut self, record: &[u8]) -> AppendJournalCommitV1;
}
