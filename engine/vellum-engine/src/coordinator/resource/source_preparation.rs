//! Codec-owned temporary source preparation; not destination authority or a task.
use super::*;
use sha2::{Digest, Sha256};
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(super) struct SourcePreparation {
    id: PreparedSourceArtifactIdV1,
    pub(super) source: DocumentVersion,
    pub(super) serialization: SourceSerializationStateV1,
    pub(super) bytes: Blob,
    capture_issued: bool,
}

fn copy_source_capture(
    preparation: &SourcePreparation,
) -> Result<CapturedSourceSerializationV1, ResourceError> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(preparation.bytes.0.len())
        .map_err(|_| ResourceError::Capacity)?;
    bytes.extend_from_slice(&preparation.bytes.0);
    let mut digest = Sha256::new();
    digest.update(b"vellum.source-serialization-capture.v1\0");
    digest.update(preparation.source.document.slot.to_le_bytes());
    digest.update(preparation.source.revision.to_le_bytes());
    digest.update(preparation.serialization.generation.to_le_bytes());
    digest.update([match preparation.serialization.serialization.profile {
        SourceSerializationProfileV1::StrictUtf8OneBom => 1,
    }]);
    digest.update([u8::from(preparation.serialization.serialization.bom)]);
    digest.update(preparation.bytes.0.len().to_le_bytes());
    digest.update(&*preparation.bytes.0);
    let digest = digest.finalize();
    let mut witness = [0_u8; 16];
    witness.copy_from_slice(&digest[..16]);
    Ok(CapturedSourceSerializationV1 {
        witness: SourceSerializationCaptureWitnessV1(witness),
        source: preparation.source.clone(),
        serialization: preparation.serialization,
        bytes,
    })
}

impl SourceCoordinator {
    pub(super) fn artifact_charge_intact(&self) -> Result<(), ResourceError> {
        let endpoint = if let Some(a) = &self.resources.read_baseline_attachment {
            a.current(&self.namespace)?;
            std::mem::size_of::<ReadBaselineEndpoint>()
        } else {
            0
        };
        let total = self
            .resources
            .artifacts
            .iter()
            .flatten()
            .try_fold(0usize, |n, row| n.checked_add(row.held_size()?))
            .and_then(|n| {
                n.checked_add(
                    self.resources
                        .source_preparation
                        .as_ref()
                        .map_or(0, |p| p.bytes.0.len()),
                )
            })
            .and_then(|n| n.checked_add(endpoint))
            .ok_or(ResourceError::Faulted)?;
        if total != self.resources.held_artifact_bytes {
            return Err(ResourceError::Faulted);
        }
        Ok(())
    }
    /// Exact trusted-owner capture, not a source export/write grant. One pending
    /// source preparation reserves shared artifact/issuance credit and bytes.
    pub fn prepare_source_artifact(
        &mut self,
        source: &DocumentVersion,
        serialization: SourceSerializationStateV1,
        r: &mut Recorder,
    ) -> Result<PreparedSourceArtifactIdV1, ResourceError> {
        let out = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            self.artifact_charge_intact()?;
            if self.resources.source_preparation.is_some() {
                return Err(ResourceError::Blocked);
            }
            let document = self.validate_version(source).map_err(owner_error)?;
            if !document.serialization.valid() {
                return Err(ResourceError::Faulted);
            }
            if serialization != document.serialization {
                return Err(ResourceError::Stale);
            }
            let length = serialization
                .serialization
                .encoded_len(&document.source)
                .ok_or(ResourceError::Limit)?;
            self.preflight_artifact(length)?;
            let document = self.validate_version(source).map_err(owner_error)?;
            let bytes = serialization
                .serialization
                .encode(&document.source, r)
                .map_err(ResourceError::Source)?;
            if bytes.len() != length {
                return Err(ResourceError::Faulted);
            }
            let id = PreparedSourceArtifactIdV1 {
                owner: self.namespace.clone(),
                marker: Arc::new(()),
            };
            let held = self
                .resources
                .held_artifact_bytes
                .checked_add(length)
                .ok_or(ResourceError::Faulted)?;
            self.resources.source_preparation = Some(SourcePreparation {
                id: id.clone(),
                source: source.clone(),
                serialization,
                bytes: Blob(Arc::from(bytes)),
                capture_issued: false,
            });
            self.resources.held_artifact_bytes = held;
            Ok(id)
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceArtifactPreparation, &out);
        out
    }
    /// Materializes only the original current capture, without encoding again.
    /// Refusal leaves the preparation held for explicit protected discard.
    pub fn retain_prepared_source_artifact(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        r: &mut Recorder,
    ) -> Result<ResourceArtifactId, ResourceError> {
        self.retain_prepared_source_artifact_catalog(prepared, None, r)
    }

    /// Consume the original codec preparation into a one-use immutable carrier.
    /// This is intentionally separate from destination authority: callers must
    /// acquire that authority before requesting content-bearing capture. Later
    /// source edits do not refresh the already prepared bytes.
    pub fn capture_prepared_source_serialization(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        r: &mut Recorder,
    ) -> Result<CapturedSourceSerializationV1, ResourceError> {
        let out = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            let p = self.source_preparation(prepared)?;
            self.artifact_charge_intact()?;
            if p.capture_issued {
                return Err(ResourceError::Unavailable);
            }
            if !p.serialization.valid() {
                return Err(ResourceError::Faulted);
            }
            let held = self
                .resources
                .held_artifact_bytes
                .checked_sub(p.bytes.0.len())
                .ok_or(ResourceError::Faulted)?;
            let captured = copy_source_capture(p)?;
            self.resources
                .source_preparation
                .take()
                .expect("validated original preparation");
            self.resources.held_artifact_bytes = held;
            Ok(captured)
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceArtifactExportCapture, &out);
        out
    }

    /// Copies the exact prepared serialization into a one-use authority
    /// carrier while retaining the original preparation for later native task
    /// admission. At most one preparation exists, and both byte owners remain
    /// bounded by the configured source/artifact limits. Neither carrier is a
    /// destination grant; callers must explicitly discharge the preparation.
    pub fn capture_prepared_source_serialization_retained(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        r: &mut Recorder,
    ) -> Result<CapturedSourceSerializationV1, ResourceError> {
        let out = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            let preparation = self.source_preparation(prepared)?;
            self.artifact_charge_intact()?;
            if !preparation.serialization.valid() {
                return Err(ResourceError::Faulted);
            }
            if preparation.capture_issued {
                return Err(ResourceError::Unavailable);
            }
            let captured = copy_source_capture(preparation)?;
            let preparation = self
                .resources
                .source_preparation
                .as_mut()
                .expect("validated original preparation");
            preparation.capture_issued = true;
            Ok(captured)
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceArtifactExportCapture, &out);
        out
    }
    pub(in crate::coordinator) fn retain_prepared_source_artifact_catalog(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        catalog: Option<Arc<WriteCatalogCaptureV1>>,
        r: &mut Recorder,
    ) -> Result<ResourceArtifactId, ResourceError> {
        let out = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            let p = self.source_preparation(prepared)?;
            self.artifact_charge_intact()?;
            let d = self.validate_version(&p.source).map_err(owner_error)?;
            if !d.serialization.valid() {
                return Err(ResourceError::Faulted);
            }
            if d.serialization != p.serialization {
                return Err(ResourceError::Stale);
            }
            let limits = self.resources.limits.ok_or(ResourceError::Unsupported)?;
            if self.resources.artifacts.len() >= limits.max_issued_artifacts
                || self.resources.artifacts.iter().flatten().count() >= limits.max_live_artifacts
                || self.resources.held_artifact_bytes > limits.max_held_artifact_bytes
                || p.bytes.0.len() > limits.max_blob_bytes
                || self.resources.artifacts.capacity() <= self.resources.artifacts.len()
            {
                return Err(ResourceError::Faulted); // a prepaid credit was stolen
            }
            self.resources
                .held_artifact_bytes
                .checked_sub(p.bytes.0.len())
                .ok_or(ResourceError::Faulted)?;
            let extra = catalog
                .as_ref()
                .map(|c| c.native_bytes())
                .transpose()?
                .unwrap_or(0);
            let held = self
                .resources
                .held_artifact_bytes
                .checked_add(extra)
                .filter(|n| *n <= limits.max_held_artifact_bytes)
                .ok_or(ResourceError::Limit)?;
            let p = self
                .resources
                .source_preparation
                .take()
                .expect("validated original preparation");
            let id = ResourceArtifactId {
                owner: self.namespace.clone(),
                slot: self.resources.artifacts.len(),
            };
            self.resources.artifacts.push(Some(ArtifactRow {
                catalog,
                loaded_condition: None,
                basis: ArtifactBasis::Source {
                    source: p.source,
                    serialization: p.serialization,
                },
                bytes: p.bytes,
                stopped: false,
            }));
            self.resources.held_artifact_bytes = held;
            // Charge transfers from temporary to retained; never add it twice.
            Ok(id)
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceArtifactRetention, &out);
        out
    }
    pub(super) fn source_preparation(
        &self,
        id: &PreparedSourceArtifactIdV1,
    ) -> Result<&SourcePreparation, ResourceError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        self.resources
            .source_preparation
            .as_ref()
            .filter(|p| Arc::ptr_eq(&p.id.marker, &id.marker))
            .ok_or(ResourceError::Unavailable)
    }
    /// Protected metadata-only discharge; available under an editing fault.
    /// Does not retire artifacts, cancel effects or refund lifetime issuance.
    pub fn discard_source_artifact_preparation(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let out = (|| {
            let p = self.source_preparation(prepared)?;
            self.artifact_charge_intact()?;
            let held = self
                .resources
                .held_artifact_bytes
                .checked_sub(p.bytes.0.len())
                .ok_or(ResourceError::Faulted)?;
            self.resources.source_preparation = None;
            self.resources.held_artifact_bytes = held;
            Ok(())
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture(r, Boundary::SourceArtifactDiscard, &out);
        out
    }
}
