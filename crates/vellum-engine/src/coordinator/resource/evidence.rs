//! One native F27 ingress over existing protected evidence owners. No additional
//! task store, decoder, status lattice, provider lookup, allocation or host I/O.
use super::*;
impl SourceCoordinator {
    /// Record original provider-attributed historical evidence. No current
    /// document permission or operational/admission guard is added: stopping or
    /// withdrawing a provider must not erase evidence of its admitted work.
    /// Each existing owner validates original engine/task/registration/attempt
    /// before duplicate/conflict handling. Intake neither activates a read nor
    /// publishes a save baseline, releases an observer result or implies quiescence.
    pub fn ingest_provider_evidence_v1(
        &mut self,
        evidence: ProviderEvidenceV1,
        r: &mut Recorder,
    ) -> Result<EvidenceDisposition, ResourceError> {
        let timer = r.begin_service_timing();
        let result = match evidence {
            ProviderEvidenceV1::SourceRead(e) => self.record_source_read_v1(e, r),
            ProviderEvidenceV1::AnnotationRead(e) => self.record_annotation_read(e, r),
            ProviderEvidenceV1::Save(e) => self.record_save_evidence(e, r),
        };
        r.record(Observation {
            boundary: Boundary::ProviderEvidenceIntake,
            disposition: match result {
                Ok(EvidenceDisposition::Contradiction) | Err(ResourceError::Faulted) => {
                    Disposition::Unknown
                }
                Ok(_) => Disposition::Completed,
                Err(_) => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        // One primary ingress population per call. Secondary fixed tags count
        // duplicate/conflict outcomes only; do not add these to the denominator.
        let category = match result {
            Ok(EvidenceDisposition::Duplicate) => {
                Some((Boundary::ProviderEvidenceDuplicate, Disposition::Completed))
            }
            Ok(EvidenceDisposition::Contradiction) => Some((
                Boundary::ProviderEvidenceContradiction,
                Disposition::Unknown,
            )),
            _ => None,
        };
        if let Some((boundary, disposition)) = category {
            r.record(Observation {
                boundary,
                disposition,
                service_time: None,
            });
        }
        result
    }
}
