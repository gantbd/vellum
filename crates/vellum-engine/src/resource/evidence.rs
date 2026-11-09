//! Closed native F27 intake values. These contain original provider-issued
//! evidence, not caller-selectable operation IDs, status flags or authority.
use super::*;

/// Move an existing authenticated native evidence value into its original owner.
/// The provider port establishes registration/dispatch correspondence; adapters
/// remain responsible for authenticating external observations and OS truth.
/// Progress, acknowledgment, failure and unknown are family-specific facts;
/// duplicates and contradictions are intake results, not extra effect commands.
pub enum ProviderEvidenceV1 {
    SourceRead(SourceReadEvidenceV1),
    AnnotationRead(AnnotationReadEvidence),
    Save(SaveEvidence),
}
impl std::fmt::Debug for ProviderEvidenceV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderEvidenceV1(<redacted>)")
    }
}
