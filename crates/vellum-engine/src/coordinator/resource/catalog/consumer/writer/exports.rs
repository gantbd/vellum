//! Catalog profiles delegate to the existing artifact, writer and Save As owner.
use super::*;
impl SourceCoordinator {
    pub fn queue_catalog_annotation_export_v1(
        &mut self,
        artifact: &ResourceArtifactId,
        destination: ResourceRead,
        requirements: SaveRequirements,
        writer: &CatalogWriterBindingV1,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_annotation_export_checked(
            artifact,
            destination,
            requirements,
            None,
            Some(writer.capture.clone()),
            r,
        )
    }
    pub fn queue_catalog_bound_annotation_export_v1(
        &mut self,
        artifact: &ResourceArtifactId,
        destination: ResourceRead,
        requirements: SaveRequirements,
        writer: &CatalogWriterBindingV1,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_annotation_export_checked(
            artifact,
            destination,
            requirements,
            Some(pair.as_ref()),
            Some(writer.capture.clone()),
            r,
        )
    }
    pub fn queue_catalog_source_export_v1(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        destination: SourceExportDestinationV1,
        options: SourceExportOptionsV1,
        writer: &CatalogWriterBindingV1,
        facts: &TrustedSourceExportFactsV1,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_source_export_checked(
            (prepared, Some(writer.capture.clone())),
            destination,
            options,
            None,
            facts,
            r,
        )
    }
    pub fn queue_catalog_source_export_v2(
        &mut self,
        prepared: &PreparedSourceArtifactIdV1,
        destination: SourceExportDestinationV1,
        options: SourceExportOptionsV2,
        writer: &CatalogWriterBindingV1,
        facts: &TrustedSourceExportFactsV1,
        r: &mut Recorder,
    ) -> Result<SaveId> {
        self.queue_source_export_checked(
            (prepared, Some(writer.capture.clone())),
            destination,
            options.intent,
            Some(options.result_version_profile),
            facts,
            r,
        )
    }
}
