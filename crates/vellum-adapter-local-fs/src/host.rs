use crate::{
    AnnotationReadCompletion as NativeAnnotationReadCompletion, FilesystemError, FilesystemLimits,
    LocalFilesystem, ResourceHandle, WriteCompletion as NativeWriteCompletion, WriteProbe,
};
use std::{convert::Infallible, path::Path};
use vellum_engine::{
    instrumentation::Recorder,
    resource::{
        AnnotationReadDispatch, AnnotationReadTarget, OrdinaryPairCheck, OrdinaryPairEvidence,
        ProviderPort, ResourceRead, SaveDispatch,
    },
};
use vellum_host::resource::{
    AdapterFailure, AnnotationReadCompletion, ClassifiedAdapterError,
    ErasedPathResourceAdapterFactory, ResourceAdapter, ResourceAdapterFactory, WriteCompletion,
};

impl ClassifiedAdapterError for FilesystemError {
    fn classification(&self) -> AdapterFailure {
        match self {
            Self::Io => AdapterFailure::Io,
            Self::Limit => AdapterFailure::Limit,
            Self::Capacity => AdapterFailure::Capacity,
            Self::Unavailable => AdapterFailure::Unavailable,
            Self::Conflict => AdapterFailure::Conflict,
            Self::UnsupportedMetadata => AdapterFailure::UnsupportedMetadata,
            Self::UnsupportedFile => AdapterFailure::UnsupportedResource,
            Self::Resource(_) => AdapterFailure::Provider,
        }
    }
}

impl From<NativeWriteCompletion> for WriteCompletion<FilesystemError> {
    fn from(value: NativeWriteCompletion) -> Self {
        Self {
            written_observation: value.written_observation,
            result: value.result,
            no_future_effects: value.no_future_effects,
            quiescence: value.quiescence,
            failure: value.failure,
            cleanup_failed: value.cleanup_failed,
        }
    }
}

impl From<NativeAnnotationReadCompletion> for AnnotationReadCompletion<FilesystemError> {
    fn from(value: NativeAnnotationReadCompletion) -> Self {
        Self {
            receipts: value.receipts,
            quiescence: value.quiescence,
            failure: value.failure,
        }
    }
}

impl ResourceAdapter for LocalFilesystem {
    type Locator = Path;
    type Handle = ResourceHandle;
    type Probe = WriteProbe;
    type Error = FilesystemError;

    fn open(
        &mut self,
        locator: &Self::Locator,
        recorder: &mut Recorder,
    ) -> Result<(Self::Handle, ResourceRead), Self::Error> {
        LocalFilesystem::open(self, locator, recorder)
    }

    fn probe(
        &self,
        handle: &Self::Handle,
        recorder: &mut Recorder,
    ) -> Result<(ResourceRead, Self::Probe), Self::Error> {
        LocalFilesystem::probe(self, handle, recorder)
    }

    fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, Self::Error> {
        LocalFilesystem::compare_ordinary_pair(self, check, recorder)
    }

    fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error> {
        LocalFilesystem::write(self, dispatch, probe, recorder).map(Into::into)
    }

    fn write_observed(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error> {
        LocalFilesystem::write_observed(self, dispatch, probe, recorder).map(Into::into)
    }

    fn annotation_read_target(
        &self,
        handle: &Self::Handle,
    ) -> Result<AnnotationReadTarget, Self::Error> {
        LocalFilesystem::annotation_read_target(self, handle)
    }

    fn read_annotation(
        &self,
        dispatch: &AnnotationReadDispatch,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadCompletion<Self::Error>, Self::Error> {
        LocalFilesystem::read_annotation(self, dispatch, recorder).map(Into::into)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LocalFilesystemFactory {
    limits: FilesystemLimits,
}

impl LocalFilesystemFactory {
    pub fn new(limits: FilesystemLimits) -> Self {
        Self { limits }
    }

    /// Composition-time budget adjustment before the factory is consumed.
    pub fn limits_mut(&mut self) -> &mut FilesystemLimits {
        &mut self.limits
    }

    pub fn erase(self) -> ErasedPathResourceAdapterFactory {
        ErasedPathResourceAdapterFactory::new(self)
    }
}

impl ResourceAdapterFactory for LocalFilesystemFactory {
    type Adapter = LocalFilesystem;
    type Error = Infallible;

    fn build(self, port: ProviderPort) -> Result<Self::Adapter, Self::Error> {
        Ok(LocalFilesystem::new(port, self.limits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_contract<A>()
    where
        A: ResourceAdapter<Locator = Path, Handle = ResourceHandle, Error = FilesystemError>,
    {
    }

    #[test]
    fn local_filesystem_closes_the_host_resource_contract() {
        assert_contract::<LocalFilesystem>();
        assert_eq!(
            FilesystemError::UnsupportedFile.classification(),
            AdapterFailure::UnsupportedResource
        );
        assert_eq!(
            FilesystemError::Conflict.classification(),
            AdapterFailure::Conflict
        );
    }
}
