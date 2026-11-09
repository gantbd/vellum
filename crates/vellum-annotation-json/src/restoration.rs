//! Explicit approved associations, never file-selected native edit versions.
use super::*;
use vellum_engine::coordinator::{SourceTarget, TargetBasis};
use vellum_engine::resource::{
    AnnotationRestoration, AnnotationRestorerPort, ResourceError, RestoredAnnotation,
};
pub struct RestorationSourceAssociation {
    pub witness: AnnotationSnapshot,
    pub evidence: SourceEvidence,
}
/// Supplying approvals is a trusted host assertion of persistent continuity and
/// selector applicability, not authority inferred from parsed names. The boundary
/// checks injection, exact managed source associations and scalar consistency;
/// arbitrary historical truth is not established by matching labels or lengths.
pub struct RestorationAssociations {
    pub document: DocumentAssociation,
    pub records: Vec<RecordIdentity>,
    pub sources: Vec<RestorationSourceAssociation>,
}
impl std::fmt::Debug for RestorationAssociations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RestorationAssociations(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestorationBoundaryError {
    Codec(CodecError),
    Engine(ResourceError),
}
impl From<CodecError> for RestorationBoundaryError {
    fn from(e: CodecError) -> Self {
        Self::Codec(e)
    }
}
impl From<ResourceError> for RestorationBoundaryError {
    fn from(e: ResourceError) -> Self {
        Self::Engine(e)
    }
}
fn native_target(
    target: &Target,
    sources: &[RestorationSourceAssociation],
) -> Result<SourceTarget, CodecError> {
    let source = sources
        .iter()
        .find(|s| s.evidence == *target.source())
        .ok_or(CodecError::HistoricalEvidenceMissing)?;
    let text = source.witness.source_text();
    let offset = |n: u64| -> Result<usize, CodecError> {
        let n = usize::try_from(n).map_err(|_| CodecError::InvalidTarget)?;
        if !text.is_char_boundary(n) {
            return Err(CodecError::InvalidTarget);
        }
        Ok(n)
    };
    let target = match target {
        Target::Document { .. } => AttachmentTarget::Document,
        Target::Point {
            offset: n,
            affinity,
            profile,
            ..
        } => AttachmentTarget::Point {
            offset: offset(*n)?,
            affinity: match affinity {
                PointAffinity::Before => NativeAffinity::Before,
                PointAffinity::After => NativeAffinity::After,
            },
            profile: match profile {
                PointMeaning::Position => NativeProfile::Position,
                PointMeaning::Content => NativeProfile::Content,
            },
        },
        Target::Range { start, end, .. } => {
            if start >= end {
                return Err(CodecError::InvalidTarget);
            }
            AttachmentTarget::ContentRange {
                start: offset(*start)?,
                end: offset(*end)?,
            }
        }
    };
    Ok(SourceTarget {
        source: source.witness.source().clone(),
        target,
        basis: TargetBasis::Before,
    })
}
impl DecodedAnnotationRead {
    pub fn into_restoration(
        self,
        associations: &RestorationAssociations,
        port: &AnnotationRestorerPort,
        limits: CodecLimits,
        recorder: &mut Recorder,
    ) -> Result<AnnotationRestoration, RestorationBoundaryError> {
        let result = (|| {
            profile(limits)?;
            if associations
                .records
                .len()
                .checked_add(associations.sources.len())
                .and_then(|n| n.checked_add(1))
                .is_none_or(|n| n > limits.max_associations)
            {
                return Err(CodecError::Limit.into());
            }
            if self.snapshot.document != associations.document.identity
                || associations.document.native != self.capture.source().document
                || associations.records.len() != self.snapshot.records.len()
            {
                return Err(CodecError::Association.into());
            }
            for (i, record) in associations.records.iter().enumerate() {
                if !self.snapshot.records.iter().any(|r| r.identity == *record)
                    || associations.records[..i]
                        .iter()
                        .any(|r| r.namespace == record.namespace && r.value == record.value)
                {
                    return Err(CodecError::Association.into());
                }
            }
            let mut bytes = 0usize;
            let mut strings = 0usize;
            for (i, source) in associations.sources.iter().enumerate() {
                if source.witness.source().document != self.capture.source().document
                    || source.evidence.byte_length != source.witness.source_text().len() as u64
                    || source.evidence.domain.is_empty()
                    || source.evidence.token.is_empty()
                {
                    return Err(CodecError::SourceMismatch.into());
                }
                if associations.sources[..i].iter().any(|s| {
                    s.evidence.same_name(&source.evidence)
                        || s.witness.source() == source.witness.source()
                }) {
                    return Err(CodecError::AmbiguousIdentity.into());
                }
                bytes = bytes
                    .checked_add(source.witness.source_text().len())
                    .ok_or(CodecError::Limit)?;
                strings = strings
                    .checked_add(source.evidence.domain.len())
                    .and_then(|n| n.checked_add(source.evidence.token.len()))
                    .ok_or(CodecError::Limit)?;
            }
            for record in &associations.records {
                strings = strings
                    .checked_add(record.namespace.len())
                    .and_then(|n| n.checked_add(record.value.len()))
                    .ok_or(CodecError::Limit)?;
            }
            if bytes > limits.max_source_bytes || strings > limits.max_string_bytes {
                return Err(CodecError::Limit.into());
            }
            let current = associations
                .sources
                .iter()
                .find(|s| s.witness.source() == self.capture.source())
                .ok_or(CodecError::HistoricalEvidenceMissing)?;
            if current.evidence != self.snapshot.source {
                return Err(CodecError::SourceMismatch.into());
            }
            // Recheck these exact owned bytes under this invocation's finite
            // profile and the actual managed current witness. An earlier decode
            // under different limits/context is not a transferable acceptance.
            let checked = decode(
                self.capture.bytes(),
                &DecodeContext {
                    document: associations.document.identity.clone(),
                    source: current.evidence.clone(),
                    text: current.witness.source_text().into(),
                },
                limits,
                recorder,
            )?;
            if checked != self.snapshot {
                return Err(CodecError::Association.into());
            }
            let mut records = Vec::new();
            records
                .try_reserve_exact(self.snapshot.records.len())
                .map_err(|_| CodecError::Capacity)?;
            for row in self.snapshot.records {
                let authored = native_target(&row.authored_target, &associations.sources)?;
                let affected = matches!(&row.placement, StoredPlacement::Affected { .. });
                let placement = match row.placement {
                    StoredPlacement::Detached => Placement::Detached,
                    StoredPlacement::Attached { target } | StoredPlacement::Affected { target } => {
                        if target.source() != &self.snapshot.source {
                            return Err(CodecError::SourceMismatch.into());
                        }
                        Placement::Located {
                            target: native_target(&target, &associations.sources)?.target,
                            evidence: if affected {
                                LocationEvidence::Affected
                            } else {
                                LocationEvidence::Located
                            },
                        }
                    }
                };
                // Canonical, unambiguous host key of namespace/value. Incarnation
                // remains an independent exact value, never a native handle.
                let key = serde_json::to_vec(&[&row.identity.namespace, &row.identity.value])
                    .map_err(|_| CodecError::Capacity)?;
                records.push(RestoredAnnotation {
                    identity: port.identity(&key, row.identity.incarnation)?,
                    body: row.body,
                    content_version: row.content_version,
                    target_generation: row.target_generation,
                    authored,
                    placement,
                });
            }
            let mut witnesses = Vec::new();
            witnesses
                .try_reserve_exact(associations.sources.len())
                .map_err(|_| CodecError::Capacity)?;
            witnesses.extend(associations.sources.iter().map(|s| s.witness.clone()));
            port.seal(self.capture, witnesses, records)
                .map_err(Into::into)
        })();
        recorder.record(Observation {
            boundary: Boundary::AnnotationRestorationPreparation,
            disposition: if result.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: None,
        });
        result
    }
}
