//! Heterogeneous native event delivery, not subscriber callbacks or another verb.
use super::*;
use std::mem::size_of;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObservationEventTargetV1<'a> {
    Source(&'a SourcePublicationStreamIdV1),
    NamedFacets(&'a FacetPublicationStreamIdV1),
    Collection(&'a CollectionPublicationStreamIdV1),
    ResourceRegistry(&'a ResourceRegistryStreamIdV1),
}
/// First-party lowering. Facts and historical certificates remain independently
/// attributable native authority, never values derived from an event target.
pub enum ObservationEventPortV1<'a> {
    Source {
        stream: &'a SourcePublicationStreamIdV1,
        scope: &'a SourceReadScopeV1,
        facts: &'a TrustedSourcePublicationFactsV1<'a>,
    },
    NamedFacets {
        stream: &'a FacetPublicationStreamIdV1,
        scope: &'a FacetPublicationScopeV1,
        facts: &'a TrustedFacetPublicationFactsV1<'a>,
    },
    Collection {
        stream: &'a CollectionPublicationStreamIdV1,
        scope: &'a CollectionPublicationScopeV1,
        facts: &'a TrustedCollectionPublicationFactsV1<'a>,
    },
    ResourceRegistry {
        stream: &'a ResourceRegistryStreamIdV1,
        scope: &'a ResourceRegistryObservationScopeV1,
        certificate: Option<&'a ResourceRegistryPreparationIdV1>,
        facts: &'a TrustedResourceRegistryObservationFactsV1<'a>,
    },
}
/// Original issuer-scoped before/after identities and payloads are not flattened
/// into a global cursor. Empty and Gap remain distinct native products.
pub enum ObservationEventDeliveryV1 {
    Source(SourcePublicationDeliveryV1),
    NamedFacets(FacetPublicationDeliveryV1),
    Collection(CollectionPublicationDeliveryV1),
    ResourceRegistry(ResourceRegistryDeliveryV1),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationEventErrorV1 {
    Unavailable,
    Source(SourcePublicationErrorV1),
    NamedFacets(FacetPublicationErrorV1),
    Collection(CollectionPublicationErrorV1),
    ResourceRegistry(crate::resource::ResourceError),
}
impl ObservationEventPortV1<'_> {
    fn matches(&self, target: ObservationEventTargetV1<'_>) -> bool {
        match (self, target) {
            (Self::Source { stream, .. }, ObservationEventTargetV1::Source(t)) => *stream == t,
            (Self::NamedFacets { stream, .. }, ObservationEventTargetV1::NamedFacets(t)) => {
                *stream == t
            }
            (Self::Collection { stream, .. }, ObservationEventTargetV1::Collection(t)) => {
                *stream == t
            }
            (
                Self::ResourceRegistry { stream, .. },
                ObservationEventTargetV1::ResourceRegistry(t),
            ) => *stream == t,
            _ => false,
        }
    }
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<observation-event:redacted>") }
})+ }; }
redacted!(
    ObservationEventTargetV1<'_>,
    ObservationEventPortV1<'_>,
    ObservationEventDeliveryV1
);
impl SourceCoordinator {
    /// Exactly one existing native release. Wrapper storage fits inside that
    /// owner's existing reply ceiling before any queue/certificate consumption.
    /// No new retention, preparation, native identity or ordinary grant is issued.
    pub fn release_observation_event_v1(
        &mut self,
        target: ObservationEventTargetV1<'_>,
        port: ObservationEventPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<ObservationEventDeliveryV1, ObservationEventErrorV1> {
        let timer = r.begin_service_timing();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if !port.matches(target) {
                return Err(ObservationEventErrorV1::Unavailable);
            }
            let frame = size_of::<ObservationEventDeliveryV1>();
            match port {
                ObservationEventPortV1::Source {
                    stream,
                    scope,
                    facts,
                } => self
                    .release_source_publication_event_frame_v1(
                        stream,
                        scope,
                        facts,
                        frame.saturating_sub(size_of::<SourcePublicationDeliveryV1>()),
                        r,
                    )
                    .map(ObservationEventDeliveryV1::Source)
                    .map_err(ObservationEventErrorV1::Source),
                ObservationEventPortV1::NamedFacets {
                    stream,
                    scope,
                    facts,
                } => self
                    .release_facet_publication_event_frame_v1(
                        stream,
                        scope,
                        facts,
                        frame.saturating_sub(size_of::<FacetPublicationDeliveryV1>()),
                        r,
                    )
                    .map(ObservationEventDeliveryV1::NamedFacets)
                    .map_err(ObservationEventErrorV1::NamedFacets),
                ObservationEventPortV1::Collection {
                    stream,
                    scope,
                    facts,
                } => self
                    .release_collection_publication_event_frame_v1(
                        stream,
                        scope,
                        facts,
                        frame.saturating_sub(size_of::<CollectionPublicationDeliveryV1>()),
                        r,
                    )
                    .map(ObservationEventDeliveryV1::Collection)
                    .map_err(ObservationEventErrorV1::Collection),
                ObservationEventPortV1::ResourceRegistry {
                    stream,
                    scope,
                    certificate,
                    facts,
                } => self
                    .release_resource_registry_event_frame_v1(
                        stream,
                        scope,
                        certificate,
                        facts,
                        frame.saturating_sub(size_of::<ResourceRegistryDeliveryV1>()),
                        r,
                    )
                    .map(ObservationEventDeliveryV1::ResourceRegistry)
                    .map_err(ObservationEventErrorV1::ResourceRegistry),
            }
        }));
        let result = match result {
            Ok(result) => result,
            Err(cause) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationEventDelivery,
                    disposition: Disposition::Unknown,
                    service_time: timer.finish(),
                });
                std::panic::resume_unwind(cause)
            }
        };
        r.record(Observation {
            boundary: Boundary::ObservationEventDelivery,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(
                    ObservationEventErrorV1::Source(SourcePublicationErrorV1::Faulted)
                    | ObservationEventErrorV1::NamedFacets(FacetPublicationErrorV1::Faulted)
                    | ObservationEventErrorV1::Collection(CollectionPublicationErrorV1::Faulted)
                    | ObservationEventErrorV1::ResourceRegistry(
                        crate::resource::ResourceError::Faulted,
                    ),
                ) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }
}
