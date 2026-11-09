use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};

pub fn reference_profile() -> vellum_model::ReferenceSessionConfig {
    vellum_model::reference_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 16,
            max_bytes: 65536,
            max_stage_attempts: 16,
            max_metadata_bytes: 65536,
        })
        .erase(),
        Box::new(vellum_adapter_annotation_json::JsonAnnotationCodec),
    )
}
