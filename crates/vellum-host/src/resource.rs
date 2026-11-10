//! Synchronous resource-effect algebra used by the current native host.
//!
//! Admission, dispatch, physical execution and evidence intake remain distinct.
//! Returning an error never proves that a dispatched effect did not apply.
use std::{any::Any, convert::Infallible, path::Path, sync::Arc};
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::{
    AnnotationReadDispatch, AnnotationReadEvidence, AnnotationReadTarget, OrdinaryPairCheck,
    OrdinaryPairEvidence, ProviderPort, ResourceRead, SaveDispatch, SaveEvidence,
};

/// Closed, content-free failure classification shared by SDK consumers.
/// Adapter-specific errors may retain narrower variants, but must map each one
/// into this vocabulary without paths, bytes or platform error payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterFailure {
    Io,
    Limit,
    Capacity,
    Unavailable,
    Conflict,
    UnsupportedMetadata,
    UnsupportedResource,
    Provider,
}

pub trait ClassifiedAdapterError {
    fn classification(&self) -> AdapterFailure;
}

impl ClassifiedAdapterError for Infallible {
    fn classification(&self) -> AdapterFailure {
        match *self {}
    }
}

impl ClassifiedAdapterError for AdapterFailure {
    fn classification(&self) -> AdapterFailure {
        *self
    }
}

/// Synchronous execution result. Receipts are original engine evidence; the
/// optional adapter failure and cleanup fact cannot relabel those receipts.
#[derive(Debug)]
pub struct WriteCompletion<E> {
    pub written_observation: Option<ResourceRead>,
    pub result: SaveEvidence,
    pub no_future_effects: SaveEvidence,
    pub quiescence: SaveEvidence,
    pub failure: Option<E>,
    pub cleanup_failed: bool,
}

/// Captured read receipts and quiescence remain explicit and separate from an
/// adapter failure. Receipt order is the provider's original bounded order.
#[derive(Debug)]
pub struct AnnotationReadCompletion<E> {
    pub receipts: Vec<AnnotationReadEvidence>,
    pub quiescence: AnnotationReadEvidence,
    pub failure: Option<E>,
}

/// Interpretation of Vellum resource effects.
///
/// Associated locator, handle and probe types keep platform identities outside
/// the engine and prevent the SDK from pretending every resource is a path.
/// Implementations must bound all retained state through their accepted
/// configuration; this trait supplies no deployment defaults.
pub trait ResourceAdapter {
    type Locator: ?Sized;
    type Handle: Clone;
    type Probe;
    type Error: ClassifiedAdapterError;

    fn open(
        &mut self,
        locator: &Self::Locator,
        recorder: &mut Recorder,
    ) -> Result<(Self::Handle, ResourceRead), Self::Error>;

    fn probe(
        &self,
        handle: &Self::Handle,
        recorder: &mut Recorder,
    ) -> Result<(ResourceRead, Self::Probe), Self::Error>;

    fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, Self::Error>;

    fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error>;

    fn write_observed(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error>;

    fn annotation_read_target(
        &self,
        handle: &Self::Handle,
    ) -> Result<AnnotationReadTarget, Self::Error>;

    fn read_annotation(
        &self,
        dispatch: &AnnotationReadDispatch,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadCompletion<Self::Error>, Self::Error>;
}

/// Deferred adapter construction after the SDK host has created the original
/// engine provider port. A factory is configuration, not resource authority.
pub trait ResourceAdapterFactory {
    type Adapter: ResourceAdapter;
    type Error: ClassifiedAdapterError;

    fn build(self, port: ProviderPort) -> Result<Self::Adapter, Self::Error>;
}

/// Opaque cloneable resource identity for path-locator applications. The
/// concrete native identity remains owned by the selected adapter.
#[derive(Clone)]
pub struct ErasedResourceHandle(Arc<dyn Any>);

pub struct ErasedResourceProbe(Box<dyn Any>);

trait DynPathResourceAdapter {
    fn open(
        &mut self,
        locator: &Path,
        recorder: &mut Recorder,
    ) -> Result<(ErasedResourceHandle, ResourceRead), AdapterFailure>;
    fn probe(
        &self,
        handle: &ErasedResourceHandle,
        recorder: &mut Recorder,
    ) -> Result<(ResourceRead, ErasedResourceProbe), AdapterFailure>;
    fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, AdapterFailure>;
    fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: ErasedResourceProbe,
        recorder: &mut Recorder,
        observed: bool,
    ) -> Result<WriteCompletion<AdapterFailure>, AdapterFailure>;
    fn annotation_read_target(
        &self,
        handle: &ErasedResourceHandle,
    ) -> Result<AnnotationReadTarget, AdapterFailure>;
    fn read_annotation(
        &self,
        dispatch: &AnnotationReadDispatch,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadCompletion<AdapterFailure>, AdapterFailure>;
}

fn erase_write<E: ClassifiedAdapterError>(
    completion: WriteCompletion<E>,
) -> WriteCompletion<AdapterFailure> {
    WriteCompletion {
        written_observation: completion.written_observation,
        result: completion.result,
        no_future_effects: completion.no_future_effects,
        quiescence: completion.quiescence,
        failure: completion
            .failure
            .as_ref()
            .map(ClassifiedAdapterError::classification),
        cleanup_failed: completion.cleanup_failed,
    }
}

impl<A> DynPathResourceAdapter for A
where
    A: ResourceAdapter<Locator = Path> + 'static,
    A::Handle: 'static,
    A::Probe: 'static,
{
    fn open(
        &mut self,
        locator: &Path,
        recorder: &mut Recorder,
    ) -> Result<(ErasedResourceHandle, ResourceRead), AdapterFailure> {
        ResourceAdapter::open(self, locator, recorder)
            .map(|(handle, read)| (ErasedResourceHandle(Arc::new(handle)), read))
            .map_err(|error| error.classification())
    }

    fn probe(
        &self,
        handle: &ErasedResourceHandle,
        recorder: &mut Recorder,
    ) -> Result<(ResourceRead, ErasedResourceProbe), AdapterFailure> {
        let handle = handle
            .0
            .downcast_ref::<A::Handle>()
            .ok_or(AdapterFailure::UnsupportedResource)?;
        ResourceAdapter::probe(self, handle, recorder)
            .map(|(read, probe)| (read, ErasedResourceProbe(Box::new(probe))))
            .map_err(|error| error.classification())
    }

    fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, AdapterFailure> {
        ResourceAdapter::compare_ordinary_pair(self, check, recorder)
            .map_err(|error| error.classification())
    }

    fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: ErasedResourceProbe,
        recorder: &mut Recorder,
        observed: bool,
    ) -> Result<WriteCompletion<AdapterFailure>, AdapterFailure> {
        let probe = probe
            .0
            .downcast::<A::Probe>()
            .map_err(|_| AdapterFailure::UnsupportedResource)?;
        let result = if observed {
            ResourceAdapter::write_observed(self, dispatch, *probe, recorder)
        } else {
            ResourceAdapter::write(self, dispatch, *probe, recorder)
        };
        result
            .map(erase_write)
            .map_err(|error| error.classification())
    }

    fn annotation_read_target(
        &self,
        handle: &ErasedResourceHandle,
    ) -> Result<AnnotationReadTarget, AdapterFailure> {
        let handle = handle
            .0
            .downcast_ref::<A::Handle>()
            .ok_or(AdapterFailure::UnsupportedResource)?;
        ResourceAdapter::annotation_read_target(self, handle)
            .map_err(|error| error.classification())
    }

    fn read_annotation(
        &self,
        dispatch: &AnnotationReadDispatch,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadCompletion<AdapterFailure>, AdapterFailure> {
        ResourceAdapter::read_annotation(self, dispatch, recorder)
            .map(|completion| AnnotationReadCompletion {
                receipts: completion.receipts,
                quiescence: completion.quiescence,
                failure: completion
                    .failure
                    .as_ref()
                    .map(ClassifiedAdapterError::classification),
            })
            .map_err(|error| error.classification())
    }
}

/// Type-erased path adapter selected by an application composition root.
pub struct ErasedPathResourceAdapter {
    inner: Box<dyn DynPathResourceAdapter>,
}

impl ResourceAdapter for ErasedPathResourceAdapter {
    type Locator = Path;
    type Handle = ErasedResourceHandle;
    type Probe = ErasedResourceProbe;
    type Error = AdapterFailure;

    fn open(
        &mut self,
        locator: &Path,
        recorder: &mut Recorder,
    ) -> Result<(Self::Handle, ResourceRead), Self::Error> {
        self.inner.open(locator, recorder)
    }

    fn probe(
        &self,
        handle: &Self::Handle,
        recorder: &mut Recorder,
    ) -> Result<(ResourceRead, Self::Probe), Self::Error> {
        self.inner.probe(handle, recorder)
    }

    fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, Self::Error> {
        self.inner.compare_ordinary_pair(check, recorder)
    }

    fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error> {
        self.inner.write(dispatch, probe, recorder, false)
    }

    fn write_observed(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error> {
        self.inner.write(dispatch, probe, recorder, true)
    }

    fn annotation_read_target(
        &self,
        handle: &Self::Handle,
    ) -> Result<AnnotationReadTarget, Self::Error> {
        self.inner.annotation_read_target(handle)
    }

    fn read_annotation(
        &self,
        dispatch: &AnnotationReadDispatch,
        recorder: &mut Recorder,
    ) -> Result<AnnotationReadCompletion<Self::Error>, Self::Error> {
        self.inner.read_annotation(dispatch, recorder)
    }
}

trait DynPathResourceAdapterFactory {
    fn build(
        self: Box<Self>,
        port: ProviderPort,
    ) -> Result<ErasedPathResourceAdapter, AdapterFailure>;
}

struct Factory<F>(F);

impl<F> DynPathResourceAdapterFactory for Factory<F>
where
    F: ResourceAdapterFactory + 'static,
    F::Adapter: ResourceAdapter<Locator = Path> + 'static,
    <F::Adapter as ResourceAdapter>::Handle: 'static,
    <F::Adapter as ResourceAdapter>::Probe: 'static,
{
    fn build(
        self: Box<Self>,
        port: ProviderPort,
    ) -> Result<ErasedPathResourceAdapter, AdapterFailure> {
        let Self(factory) = *self;
        let adapter = factory
            .build(port)
            .map_err(|error| error.classification())?;
        Ok(ErasedPathResourceAdapter {
            inner: Box::new(adapter),
        })
    }
}

/// Deferred type-erased construction for a path-locator application.
pub struct ErasedPathResourceAdapterFactory {
    inner: Box<dyn DynPathResourceAdapterFactory>,
}

impl ErasedPathResourceAdapterFactory {
    pub fn new<F>(factory: F) -> Self
    where
        F: ResourceAdapterFactory + 'static,
        F::Adapter: ResourceAdapter<Locator = Path> + 'static,
        <F::Adapter as ResourceAdapter>::Handle: 'static,
        <F::Adapter as ResourceAdapter>::Probe: 'static,
    {
        Self {
            inner: Box::new(Factory(factory)),
        }
    }
}

impl ResourceAdapterFactory for ErasedPathResourceAdapterFactory {
    type Adapter = ErasedPathResourceAdapter;
    type Error = AdapterFailure;

    fn build(self, port: ProviderPort) -> Result<Self::Adapter, Self::Error> {
        self.inner.build(port)
    }
}
