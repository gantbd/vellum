//! Asynchronous, zero-authority client boundary for the bounded semantic service.
//!
//! A client reference is supplied by trusted composition and partitions replies;
//! it is not authentication evidence or an authority grant. Constructing this
//! facade never admits a client. Provider driving and deadline evidence require
//! the separate runtime-driver capability. Neither capability chooses a
//! transport, discovers a provider, or controls a provider process.
#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;

use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticProviderReadinessProjectionV1,
    SemanticSessionCommandV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    SemanticSessionPendingV1, SemanticSessionRolesV1,
};
use vellum_semantic_session_client::{
    SemanticReplyValidationV1, SemanticSessionClientRefusalV1, SemanticSessionClientV1,
    SemanticSessionExchangePortV1,
};

pub use vellum_host::semantic::{
    MAX_SEMANTIC_DETAIL_BYTES_V1, MAX_SEMANTIC_LABEL_BYTES_V1, MAX_SEMANTIC_QUERY_BYTES_V1,
    SemanticBuildErrorV1, SemanticCapabilitySetV1, SemanticCapabilityV1,
    SemanticDiagnosticSeverityV1, SemanticDiagnosticV1, SemanticDocumentRefV1,
    SemanticIndexGenerationV1, SemanticLimitsV1, SemanticLocationV1, SemanticOutcomeV1,
    SemanticProviderProvenanceV1, SemanticProviderRefV1, SemanticQueryKindV1, SemanticQueryV1,
    SemanticRefusalV1, SemanticReplyV1, SemanticRequestRefV1, SemanticRequestV1, SemanticResultV1,
    SemanticSourceRevisionV1, SemanticSpanV1, SemanticSymbolKindV1, SemanticSymbolV1,
    SemanticTargetV1, SemanticTextEditV1, SemanticTextV1,
};
pub use vellum_semantic_service::{
    SemanticAdmissionOutcomeV1, SemanticClientRefV1, SemanticDeliveryOutcomeV1, SemanticDeliveryV1,
    SemanticProviderReadinessV1, SemanticProviderStateV1, SemanticServiceRefusalV1,
    SemanticSnapshotV1, SemanticSubmitOutcomeV1,
};

/// Transport-neutral asynchronous result used by the public semantic ports.
/// `Send` permits executor composition but implies no byte or remote protocol.
pub type SemanticFutureV1<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Client-owned, content-safe observation of the semantic provider product.
///
/// Provider identity, profile, and version remain composition-private. This
/// value reports only the readiness and finite capabilities needed to decide
/// whether a query can currently be attempted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticProviderObservationV1 {
    readiness: SemanticProviderReadinessV1,
    capabilities: SemanticCapabilitySetV1,
}

#[cfg(test)]
mod session_query_tests;

impl SemanticProviderObservationV1 {
    /// Constructs a transport projection from already composition-owned state.
    /// This value is observation only and carries no provider or query authority.
    pub const fn from_parts(
        readiness: SemanticProviderReadinessV1,
        capabilities: SemanticCapabilitySetV1,
    ) -> Self {
        Self {
            readiness,
            capabilities,
        }
    }

    pub const fn readiness(self) -> SemanticProviderReadinessV1 {
        self.readiness
    }

    pub const fn capabilities(self) -> SemanticCapabilitySetV1 {
        self.capabilities
    }

    fn from_state(state: SemanticProviderStateV1) -> Self {
        Self::from_parts(state.readiness, state.capabilities)
    }
}

/// Zero-authority operations available to one composition-provided partition.
///
/// Implementations preserve the client reference exactly for every mutation
/// and delivery. Observation is readiness/capability state, not a health or
/// provenance attestation.
pub trait SemanticServicePortV1 {
    fn observe_provider(&self) -> SemanticFutureV1<'_, SemanticProviderObservationV1>;
    fn open_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1>;
    fn change_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1>;
    fn close_snapshot(
        &mut self,
        target: SemanticTargetV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1>;
    fn submit(
        &mut self,
        request: SemanticRequestV1,
    ) -> SemanticFutureV1<'_, SemanticSubmitOutcomeV1>;
    fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>>;
    fn take_delivery(
        &mut self,
    ) -> SemanticFutureV1<'_, Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1>>;
}

mod sovereign_validation {
    pub trait Sealed {}
}

/// Port whose deliveries have crossed the sovereign semantic-service
/// partition and nested-result validation gate.
///
/// This trait is sealed: arbitrary transport or test ports can implement the
/// ordinary client port, but cannot claim that foreign-document membership,
/// source bounds, UTF-8 boundaries, or rename-range separation were validated.
/// A future remote adapter must validate those laws before entering this
/// boundary rather than forwarding untrusted delivery values directly.
pub trait SovereignValidatedSemanticServicePortV1:
    SemanticServicePortV1 + sovereign_validation::Sealed
{
}

/// Trusted runtime operations deliberately absent from a semantic client.
///
/// `expire` consumes exact deadline evidence selected by composition; it owns
/// no clock. `drive_once` performs one service-defined bounded pump. Admission,
/// provider replacement, launch, and shutdown are not members of this port.
pub trait SemanticRuntimeDriverPortV1 {
    fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>>;
    fn drive_once(&mut self) -> SemanticFutureV1<'_, ()>;
}

/// Composition-owned binding from one sovereign service partition to the
/// ordinary client port. Its fields are private so an ordinary client cannot
/// retarget a live handle after composition has fixed its partition.
struct SemanticClientPartitionV1<'a, P> {
    client: SemanticClientRefV1,
    service: &'a mut vellum_semantic_service::SemanticServiceV1<P>,
}

impl<P> sovereign_validation::Sealed for SemanticClientPartitionV1<'_, P> {}
impl<P> SovereignValidatedSemanticServicePortV1 for SemanticClientPartitionV1<'_, P> where
    Self: SemanticServicePortV1
{
}

/// Binds one already-selected correlation partition without admitting it.
/// Possession of the sovereign mutable service is the composition boundary;
/// the returned ordinary port contains no operation for changing partitions.
pub fn bind_semantic_client_v1<P: vellum_semantic_service::SemanticProviderPortV1 + Send>(
    service: &mut vellum_semantic_service::SemanticServiceV1<P>,
    client: SemanticClientRefV1,
) -> SemanticClientV1<impl SovereignValidatedSemanticServicePortV1 + '_> {
    SemanticClientV1::new(SemanticClientPartitionV1 { client, service })
}

impl<P: vellum_semantic_service::SemanticProviderPortV1 + Send> SemanticServicePortV1
    for SemanticClientPartitionV1<'_, P>
{
    fn observe_provider(&self) -> SemanticFutureV1<'_, SemanticProviderObservationV1> {
        Box::pin(std::future::ready(
            SemanticProviderObservationV1::from_state(self.service.provider_state()),
        ))
    }

    fn open_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(async move { self.service.open_snapshot(self.client, snapshot) })
    }

    fn change_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(async move { self.service.change_snapshot(self.client, snapshot) })
    }

    fn close_snapshot(
        &mut self,
        target: SemanticTargetV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(async move { self.service.close_snapshot(self.client, target) })
    }

    fn submit(
        &mut self,
        request: SemanticRequestV1,
    ) -> SemanticFutureV1<'_, SemanticSubmitOutcomeV1> {
        Box::pin(async move { self.service.submit(self.client, request) })
    }

    fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>> {
        Box::pin(async move { self.service.cancel(self.client, request) })
    }

    fn take_delivery(
        &mut self,
    ) -> SemanticFutureV1<'_, Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1>> {
        Box::pin(async move { self.service.take_delivery(self.client) })
    }
}

impl<P: vellum_semantic_service::SemanticProviderPortV1 + Send> SemanticRuntimeDriverPortV1
    for vellum_semantic_service::SemanticServiceV1<P>
{
    fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>> {
        Box::pin(async move {
            vellum_semantic_service::SemanticServiceV1::expire(self, client, request)
        })
    }

    fn drive_once(&mut self) -> SemanticFutureV1<'_, ()> {
        Box::pin(async move {
            vellum_semantic_service::SemanticServiceV1::pump(self);
        })
    }
}

impl<T: SemanticServicePortV1 + ?Sized> SemanticServicePortV1 for &mut T {
    fn observe_provider(&self) -> SemanticFutureV1<'_, SemanticProviderObservationV1> {
        (**self).observe_provider()
    }

    fn open_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        (**self).open_snapshot(snapshot)
    }

    fn change_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        (**self).change_snapshot(snapshot)
    }

    fn close_snapshot(
        &mut self,
        target: SemanticTargetV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        (**self).close_snapshot(target)
    }

    fn submit(
        &mut self,
        request: SemanticRequestV1,
    ) -> SemanticFutureV1<'_, SemanticSubmitOutcomeV1> {
        (**self).submit(request)
    }

    fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>> {
        (**self).cancel(request)
    }

    fn take_delivery(
        &mut self,
    ) -> SemanticFutureV1<'_, Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1>> {
        (**self).take_delivery()
    }
}

impl<T: SovereignValidatedSemanticServicePortV1 + ?Sized> sovereign_validation::Sealed for &mut T {}
impl<T: SovereignValidatedSemanticServicePortV1 + ?Sized> SovereignValidatedSemanticServicePortV1
    for &mut T
{
}

impl<T: SemanticRuntimeDriverPortV1 + ?Sized> SemanticRuntimeDriverPortV1 for &mut T {
    fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>> {
        (**self).expire(client, request)
    }

    fn drive_once(&mut self) -> SemanticFutureV1<'_, ()> {
        (**self).drive_once()
    }
}

/// Application-facing handle fixed to one composition-provided partition.
pub struct SemanticClientV1<P> {
    port: P,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionQueryPortBuildRefusalV1 {
    Unbound,
    RoleDenied,
    Unsealed,
    InvalidCorrelationSeed,
}

/// Query-only projection of a witness-validated persistent semantic session.
///
/// Construction rechecks the granted query role and the session client's
/// source/UTF-8 witness seal. Synchronization remains composition-owned and is
/// deliberately absent from this port. A transport-unknown Submit is retained
/// by the session client and may be replayed only by an identical later Submit.
pub struct SemanticSessionQueryPortV1<P> {
    session: SemanticSessionClientV1<P>,
    next_exchange: u128,
    next_operation: u128,
    pending_request: Option<SemanticRequestV1>,
    collecting: VecDeque<SemanticRequestRefV1>,
}

impl<P> sovereign_validation::Sealed for SemanticSessionQueryPortV1<P> where
    P: SemanticSessionExchangePortV1 + Send
{
}
impl<P> SovereignValidatedSemanticServicePortV1 for SemanticSessionQueryPortV1<P> where
    P: SemanticSessionExchangePortV1 + Send
{
}

pub fn bind_semantic_session_query_v1<P: SemanticSessionExchangePortV1 + Send>(
    session: SemanticSessionClientV1<P>,
    correlation_seed: u128,
) -> Result<SemanticClientV1<SemanticSessionQueryPortV1<P>>, SemanticSessionQueryPortBuildRefusalV1>
{
    let binding = session
        .binding()
        .ok_or(SemanticSessionQueryPortBuildRefusalV1::Unbound)?;
    if !binding.roles.contains(SemanticSessionRolesV1::query()) {
        return Err(SemanticSessionQueryPortBuildRefusalV1::RoleDenied);
    }
    if session.validation_state() != SemanticReplyValidationV1::WitnessValidated {
        return Err(SemanticSessionQueryPortBuildRefusalV1::Unsealed);
    }
    if !valid_semantic_session_correlation_seed(correlation_seed) {
        return Err(SemanticSessionQueryPortBuildRefusalV1::InvalidCorrelationSeed);
    }
    Ok(SemanticClientV1::new(SemanticSessionQueryPortV1 {
        session,
        next_exchange: correlation_seed,
        next_operation: correlation_seed + 1,
        pending_request: None,
        collecting: VecDeque::new(),
    }))
}

const fn valid_semantic_session_correlation_seed(seed: u128) -> bool {
    seed != 0 && seed <= u128::MAX - 3
}

impl<P: SemanticSessionExchangePortV1> SemanticSessionQueryPortV1<P> {
    fn binding(
        &self,
    ) -> Result<
        vellum_semantic_session_client::SemanticSessionClientBindingV1,
        SemanticServiceRefusalV1,
    > {
        self.session
            .binding()
            .ok_or(SemanticServiceRefusalV1::UnknownClient)
    }

    fn exchange(&mut self) -> Result<SemanticExchangeRefV1, SemanticServiceRefusalV1> {
        let value = self.next_exchange;
        self.next_exchange = self
            .next_exchange
            .checked_add(2)
            .ok_or(SemanticServiceRefusalV1::Capacity)?;
        SemanticExchangeRefV1::new(value.to_be_bytes()).ok_or(SemanticServiceRefusalV1::Capacity)
    }

    fn operation(&mut self) -> Result<SemanticSessionOperationRefV1, SemanticServiceRefusalV1> {
        let value = self.next_operation;
        self.next_operation = self
            .next_operation
            .checked_add(2)
            .ok_or(SemanticServiceRefusalV1::Capacity)?;
        SemanticSessionOperationRefV1::new(value.to_be_bytes())
            .ok_or(SemanticServiceRefusalV1::Capacity)
    }

    fn map_refusal(refusal: SemanticSessionClientRefusalV1) -> SemanticServiceRefusalV1 {
        match refusal {
            SemanticSessionClientRefusalV1::RoleDenied
            | SemanticSessionClientRefusalV1::UnsealedValidation => {
                SemanticServiceRefusalV1::AuthorityDenied
            }
            SemanticSessionClientRefusalV1::Capacity => SemanticServiceRefusalV1::Capacity,
            SemanticSessionClientRefusalV1::RequestState => {
                SemanticServiceRefusalV1::DuplicateRequest
            }
            _ => SemanticServiceRefusalV1::ClientIsolation,
        }
    }

    fn map_session_refusal(
        refusal: vellum_semantic_protocol::SemanticSessionRefusalV1,
    ) -> SemanticServiceRefusalV1 {
        use vellum_semantic_protocol::SemanticSessionRefusalV1 as R;
        match refusal {
            R::RoleDenied => SemanticServiceRefusalV1::AuthorityDenied,
            R::InputLimit => SemanticServiceRefusalV1::InputLimit,
            R::Capacity => SemanticServiceRefusalV1::Capacity,
            R::AbsentOrForeign => SemanticServiceRefusalV1::UnknownDocument,
            R::ClientRetiring => SemanticServiceRefusalV1::ClientRetiring,
            R::ProviderRefused => SemanticServiceRefusalV1::ProviderRefused,
            R::ProtocolViolation => SemanticServiceRefusalV1::MalformedProviderReply,
            R::SynchronizationRequired => SemanticServiceRefusalV1::SynchronizationRequired,
            R::InvalidSession | R::UnknownMembership => SemanticServiceRefusalV1::UnknownClient,
            R::SynchronizationConflict => SemanticServiceRefusalV1::ClientIsolation,
        }
    }
}

impl<P: SemanticSessionExchangePortV1 + Send> SemanticServicePortV1
    for SemanticSessionQueryPortV1<P>
{
    fn observe_provider(&self) -> SemanticFutureV1<'_, SemanticProviderObservationV1> {
        // This is the last witness-validated Describe projection, not a live
        // provider poll. Zero-TTL callers must refresh the session description
        // before constructing a replacement port; every query is still checked
        // by the authoritative session and may return stale/unavailable.
        let observed = self.session.description().map(|description| {
            let provider = description.provider();
            let readiness = match provider.readiness() {
                SemanticProviderReadinessProjectionV1::Unavailable => {
                    SemanticProviderReadinessV1::Unavailable
                }
                SemanticProviderReadinessProjectionV1::Indexing => {
                    SemanticProviderReadinessV1::Indexing
                }
                SemanticProviderReadinessProjectionV1::Ready { generation } => {
                    SemanticProviderReadinessV1::Ready { generation }
                }
            };
            SemanticProviderObservationV1::from_parts(readiness, provider.capabilities())
        });
        Box::pin(std::future::ready(observed.unwrap_or_else(|| {
            SemanticProviderObservationV1::from_parts(
                SemanticProviderReadinessV1::Unavailable,
                SemanticCapabilitySetV1::bottom(),
            )
        })))
    }

    fn open_snapshot(
        &mut self,
        _snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(std::future::ready(SemanticAdmissionOutcomeV1::Refused(
            SemanticServiceRefusalV1::AuthorityDenied,
        )))
    }

    fn change_snapshot(
        &mut self,
        _snapshot: SemanticSnapshotV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(std::future::ready(SemanticAdmissionOutcomeV1::Refused(
            SemanticServiceRefusalV1::AuthorityDenied,
        )))
    }

    fn close_snapshot(
        &mut self,
        _target: SemanticTargetV1,
    ) -> SemanticFutureV1<'_, SemanticAdmissionOutcomeV1> {
        Box::pin(std::future::ready(SemanticAdmissionOutcomeV1::Refused(
            SemanticServiceRefusalV1::AuthorityDenied,
        )))
    }

    fn submit(
        &mut self,
        request: SemanticRequestV1,
    ) -> SemanticFutureV1<'_, SemanticSubmitOutcomeV1> {
        Box::pin(async move {
            let replaying = self.pending_request.is_some();
            let result = if let Some(pending) = self.pending_request.as_ref() {
                if pending != &request {
                    return SemanticSubmitOutcomeV1::Refused(
                        SemanticServiceRefusalV1::DuplicateRequest,
                    );
                }
                let exchange = match self.exchange() {
                    Ok(exchange) => exchange,
                    // Correlation exhaustion prevents this replay attempt, but
                    // cannot make the earlier exchange definitive.
                    Err(_) => return SemanticSubmitOutcomeV1::OutcomeUnknown,
                };
                match self.session.replay_pending(exchange) {
                    Ok(disposition) => Ok(disposition),
                    // The session ledger still owns the uncertain mutation.
                    // A local preflight refusal cannot turn it into a terminal
                    // application outcome.
                    Err(_) => return SemanticSubmitOutcomeV1::OutcomeUnknown,
                }
            } else {
                let binding = match self.binding() {
                    Ok(binding) => binding,
                    Err(refusal) => return SemanticSubmitOutcomeV1::Refused(refusal),
                };
                let exchange = match self.exchange() {
                    Ok(exchange) => exchange,
                    Err(refusal) => return SemanticSubmitOutcomeV1::Refused(refusal),
                };
                let operation = match self.operation() {
                    Ok(operation) => operation,
                    Err(refusal) => return SemanticSubmitOutcomeV1::Refused(refusal),
                };
                self.session.attempt(SemanticSessionCommandV1::Submit {
                    exchange,
                    operation,
                    incarnation: binding.incarnation,
                    membership: binding.membership,
                    request: request.clone(),
                })
            };
            let disposition = match result {
                Ok(disposition) => disposition,
                Err(refusal) => {
                    return SemanticSubmitOutcomeV1::Refused(Self::map_refusal(refusal));
                }
            };
            match disposition {
                SemanticExchangeDispositionV1::NotSent if replaying => {
                    // The earlier exchange may still have applied. A replay
                    // that was definitely not sent cannot erase that fact.
                    SemanticSubmitOutcomeV1::OutcomeUnknown
                }
                SemanticExchangeDispositionV1::NotSent => SemanticSubmitOutcomeV1::Unavailable,
                SemanticExchangeDispositionV1::OutcomeUnknown => {
                    self.pending_request = Some(request);
                    SemanticSubmitOutcomeV1::OutcomeUnknown
                }
                SemanticExchangeDispositionV1::Reply(reply) => {
                    self.pending_request = None;
                    match reply.outcome() {
                        SemanticSessionOutcomeV1::Accepted => {
                            if !self.collecting.contains(&request.request) {
                                self.collecting.push_back(request.request);
                            }
                            SemanticSubmitOutcomeV1::Accepted
                        }
                        SemanticSessionOutcomeV1::Unsupported(kind) => {
                            SemanticSubmitOutcomeV1::Unsupported(*kind)
                        }
                        SemanticSessionOutcomeV1::Stale => SemanticSubmitOutcomeV1::Stale,
                        SemanticSessionOutcomeV1::Unavailable => {
                            SemanticSubmitOutcomeV1::Unavailable
                        }
                        SemanticSessionOutcomeV1::Refused(refusal) => {
                            SemanticSubmitOutcomeV1::Refused(Self::map_session_refusal(*refusal))
                        }
                        SemanticSessionOutcomeV1::RequestConflict => {
                            SemanticSubmitOutcomeV1::Refused(
                                SemanticServiceRefusalV1::DuplicateRequest,
                            )
                        }
                        SemanticSessionOutcomeV1::ReplayCapacity => {
                            SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::Capacity)
                        }
                        _ => SemanticSubmitOutcomeV1::Refused(
                            SemanticServiceRefusalV1::MalformedProviderReply,
                        ),
                    }
                }
            }
        })
    }

    fn cancel(
        &mut self,
        _request: SemanticRequestRefV1,
    ) -> SemanticFutureV1<'_, Result<bool, SemanticServiceRefusalV1>> {
        Box::pin(std::future::ready(Err(
            SemanticServiceRefusalV1::AuthorityDenied,
        )))
    }

    fn take_delivery(
        &mut self,
    ) -> SemanticFutureV1<'_, Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1>> {
        Box::pin(async move {
            let Some(request) = self.collecting.pop_front() else {
                return Ok(None);
            };
            let binding = self.binding()?;
            let command = SemanticSessionCommandV1::Collect {
                exchange: self.exchange()?,
                incarnation: binding.incarnation,
                membership: binding.membership,
                request,
            };
            let disposition = self.session.attempt(command).map_err(Self::map_refusal)?;
            let reply = match disposition {
                SemanticExchangeDispositionV1::NotSent
                | SemanticExchangeDispositionV1::OutcomeUnknown => {
                    self.collecting.push_back(request);
                    return Ok(None);
                }
                SemanticExchangeDispositionV1::Reply(reply) => reply,
            };
            let outcome = match reply.outcome() {
                SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(pending))
                    if *pending == request =>
                {
                    self.collecting.push_back(request);
                    return Ok(None);
                }
                SemanticSessionOutcomeV1::Complete(reply) => {
                    SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(reply.clone()))
                }
                SemanticSessionOutcomeV1::Unsupported(kind) => {
                    SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Unsupported(*kind))
                }
                SemanticSessionOutcomeV1::Stale => {
                    SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Stale)
                }
                SemanticSessionOutcomeV1::Unavailable => {
                    SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Unavailable)
                }
                SemanticSessionOutcomeV1::Cancelled => SemanticDeliveryOutcomeV1::Cancelled,
                SemanticSessionOutcomeV1::TimedOut => SemanticDeliveryOutcomeV1::TimedOut,
                SemanticSessionOutcomeV1::PeerRefused(refusal) => {
                    SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Refused(*refusal))
                }
                SemanticSessionOutcomeV1::Refused(refusal) => {
                    return Err(Self::map_session_refusal(*refusal));
                }
                _ => return Err(SemanticServiceRefusalV1::MalformedProviderReply),
            };
            Ok(Some(SemanticDeliveryV1 { request, outcome }))
        })
    }
}

impl<P: SemanticServicePortV1> SemanticClientV1<P> {
    /// Wraps an already partition-bound port. This performs no authentication,
    /// admission, authorization, or capability grant and cannot retarget it.
    pub const fn new(port: P) -> Self {
        Self { port }
    }

    pub async fn observe_provider(&self) -> SemanticProviderObservationV1 {
        self.port.observe_provider().await
    }

    pub async fn open_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticAdmissionOutcomeV1 {
        self.port.open_snapshot(snapshot).await
    }

    pub async fn change_snapshot(
        &mut self,
        snapshot: SemanticSnapshotV1,
    ) -> SemanticAdmissionOutcomeV1 {
        self.port.change_snapshot(snapshot).await
    }

    pub async fn close_snapshot(&mut self, target: SemanticTargetV1) -> SemanticAdmissionOutcomeV1 {
        self.port.close_snapshot(target).await
    }

    pub async fn submit(&mut self, request: SemanticRequestV1) -> SemanticSubmitOutcomeV1 {
        self.port.submit(request).await
    }

    pub async fn cancel(
        &mut self,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.port.cancel(request).await
    }

    pub async fn take_delivery(
        &mut self,
    ) -> Result<Option<SemanticDeliveryV1>, SemanticServiceRefusalV1> {
        self.port.take_delivery().await
    }

    pub fn into_port(self) -> P {
        self.port
    }
}

/// Composition-facing handle for bounded driving and explicit expiry only.
pub struct SemanticRuntimeDriverV1<P> {
    port: P,
}

impl<P: SemanticRuntimeDriverPortV1> SemanticRuntimeDriverV1<P> {
    pub const fn new(port: P) -> Self {
        Self { port }
    }

    pub async fn expire(
        &mut self,
        client: SemanticClientRefV1,
        request: SemanticRequestRefV1,
    ) -> Result<bool, SemanticServiceRefusalV1> {
        self.port.expire(client, request).await
    }

    pub async fn drive_once(&mut self) {
        self.port.drive_once().await;
    }

    pub fn into_port(self) -> P {
        self.port
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::{Context, Poll, Waker};
    use vellum_host::semantic::{
        SemanticCapabilitySetV1, SemanticDocumentRefV1, SemanticIndexGenerationV1,
        SemanticProviderProvenanceV1, SemanticProviderRefV1, SemanticQueryV1,
        SemanticSourceRevisionV1, SemanticTextV1,
    };
    use vellum_semantic_service::{
        SemanticDeliveryOutcomeV1, SemanticProviderAdmissionV1, SemanticProviderCompletionV1,
        SemanticProviderPortV1, SemanticProviderReadinessV1, SemanticProviderSubmissionV1,
        SemanticServiceLimitsV1, SemanticServiceV1, SemanticSnapshotUpdateV1,
    };

    fn run<F: Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("model future unexpectedly pending"),
        }
    }

    fn provenance() -> SemanticProviderProvenanceV1 {
        SemanticProviderProvenanceV1::try_new(
            SemanticProviderRefV1::new([19; 16]),
            "model-provider".into(),
            "1.0.0".into(),
        )
        .unwrap()
    }

    fn ready_state(generation: u64) -> SemanticProviderStateV1 {
        SemanticProviderStateV1 {
            provider: provenance(),
            readiness: SemanticProviderReadinessV1::Ready {
                generation: SemanticIndexGenerationV1::new(generation).unwrap(),
            },
            capabilities: SemanticCapabilitySetV1::read_only(),
        }
    }

    struct ModelProvider {
        state: SemanticProviderStateV1,
        state_after_poll: Option<SemanticProviderStateV1>,
    }

    impl SemanticProviderPortV1 for ModelProvider {
        fn state(&self) -> SemanticProviderStateV1 {
            self.state.clone()
        }

        fn synchronize(
            &mut self,
            _update: &SemanticSnapshotUpdateV1,
        ) -> SemanticProviderAdmissionV1 {
            SemanticProviderAdmissionV1::Accepted
        }

        fn submit(&mut self, _request: &SemanticRequestV1) -> SemanticProviderSubmissionV1 {
            SemanticProviderSubmissionV1::Accepted
        }

        fn cancel(&mut self, _request: SemanticRequestRefV1) {}

        fn poll(&mut self) -> Option<SemanticProviderCompletionV1> {
            if let Some(state) = self.state_after_poll.take() {
                self.state = state;
            }
            None
        }
    }

    fn client(tag: u8) -> SemanticClientRefV1 {
        SemanticClientRefV1::new([tag; 16])
    }

    fn target(tag: u8, revision: u64) -> SemanticTargetV1 {
        SemanticTargetV1 {
            document: SemanticDocumentRefV1::new([tag; 16]),
            source_revision: SemanticSourceRevisionV1::new(revision),
        }
    }

    fn snapshot(tag: u8, revision: u64) -> SemanticSnapshotV1 {
        SemanticSnapshotV1::try_new(
            target(tag, revision),
            "rust".into(),
            "fn bounded() {}".into(),
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap()
    }

    fn request(tag: u8, revision: u64, request_tag: u8) -> SemanticRequestV1 {
        SemanticRequestV1 {
            request: SemanticRequestRefV1::new([request_tag; 16]),
            target: target(tag, revision),
            expected_index: SemanticIndexGenerationV1::new(1),
            language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
            query: SemanticQueryV1::Diagnostics,
        }
    }

    fn service(transition: Option<SemanticProviderStateV1>) -> SemanticServiceV1<ModelProvider> {
        SemanticServiceV1::try_new(
            ModelProvider {
                state: ready_state(1),
                state_after_poll: transition,
            },
            SemanticServiceLimitsV1::reference(),
        )
        .unwrap()
    }

    #[test]
    fn constructing_a_client_neither_authenticates_nor_admits_it() {
        let mut service = service(None);
        let mut facade = bind_semantic_client_v1(&mut service, client(1));
        assert_eq!(
            run(facade.open_snapshot(snapshot(3, 1))),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::UnknownClient)
        );
    }

    #[test]
    fn partitions_cannot_change_or_close_each_others_snapshots() {
        let mut service = service(None);
        service.admit_client(client(1)).unwrap();
        service.admit_client(client(2)).unwrap();
        {
            let mut first = bind_semantic_client_v1(&mut service, client(1));
            assert_eq!(
                run(first.open_snapshot(snapshot(3, 1))),
                SemanticAdmissionOutcomeV1::Accepted
            );
        }
        let mut hostile = bind_semantic_client_v1(&mut service, client(2));
        assert_eq!(
            run(hostile.change_snapshot(snapshot(3, 2))),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::ClientIsolation)
        );
        assert_eq!(
            run(hostile.close_snapshot(target(3, 1))),
            SemanticAdmissionOutcomeV1::Refused(SemanticServiceRefusalV1::ClientIsolation)
        );
    }

    #[test]
    fn cancellation_and_composition_expiry_remain_distinct() {
        let mut service = service(None);
        service.admit_client(client(1)).unwrap();
        service.admit_client(client(2)).unwrap();
        {
            let mut facade = bind_semantic_client_v1(&mut service, client(1));
            assert_eq!(
                run(facade.open_snapshot(snapshot(3, 1))),
                SemanticAdmissionOutcomeV1::Accepted
            );
            assert_eq!(
                run(facade.submit(request(3, 1, 4))),
                SemanticSubmitOutcomeV1::Accepted
            );
            assert_eq!(
                run(facade.cancel(SemanticRequestRefV1::new([4; 16]))),
                Ok(true)
            );
            assert_eq!(
                run(facade.submit(request(3, 1, 5))),
                SemanticSubmitOutcomeV1::Accepted
            );
        }
        {
            let mut driver = SemanticRuntimeDriverV1::new(&mut service);
            assert_eq!(
                run(driver.expire(client(2), SemanticRequestRefV1::new([5; 16]))),
                Err(SemanticServiceRefusalV1::ClientIsolation)
            );
            assert_eq!(
                run(driver.expire(client(1), SemanticRequestRefV1::new([5; 16]))),
                Ok(true)
            );
        }
        let mut facade = bind_semantic_client_v1(&mut service, client(1));
        assert_eq!(
            run(facade.take_delivery()).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Cancelled
        );
        assert_eq!(
            run(facade.take_delivery()).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::TimedOut
        );
    }

    #[test]
    fn bounded_drive_makes_readiness_changes_observable() {
        let mut service = service(Some(ready_state(2)));
        service.admit_client(client(1)).unwrap();
        {
            let facade = bind_semantic_client_v1(&mut service, client(1));
            assert_eq!(
                run(facade.observe_provider()).readiness(),
                ready_state(1).readiness
            );
        }
        {
            let mut driver = SemanticRuntimeDriverV1::new(&mut service);
            run(driver.drive_once());
        }
        let facade = bind_semantic_client_v1(&mut service, client(1));
        assert_eq!(
            run(facade.observe_provider()).readiness(),
            ready_state(2).readiness
        );
    }

    #[test]
    fn client_observation_projects_only_readiness_and_capabilities() {
        let mut service = service(None);
        let facade = bind_semantic_client_v1(&mut service, client(1));
        let observation = run(facade.observe_provider());
        assert_eq!(observation.readiness(), ready_state(1).readiness);
        assert_eq!(
            observation.capabilities(),
            SemanticCapabilitySetV1::read_only()
        );
        let debug = format!("{observation:?}");
        assert!(!debug.contains("model-provider"));
        assert!(!debug.contains("1.0.0"));
    }

    #[test]
    fn bound_partition_cannot_drain_a_victims_delivery() {
        let mut service = service(None);
        service.admit_client(client(1)).unwrap();
        service.admit_client(client(2)).unwrap();
        {
            let mut victim = bind_semantic_client_v1(&mut service, client(1));
            assert_eq!(
                run(victim.open_snapshot(snapshot(3, 1))),
                SemanticAdmissionOutcomeV1::Accepted
            );
            assert_eq!(
                run(victim.submit(request(3, 1, 4))),
                SemanticSubmitOutcomeV1::Accepted
            );
            assert_eq!(
                run(victim.cancel(SemanticRequestRefV1::new([4; 16]))),
                Ok(true)
            );
        }
        {
            let mut hostile = bind_semantic_client_v1(&mut service, client(2));
            assert_eq!(run(hostile.take_delivery()), Ok(None));
        }
        let mut victim = bind_semantic_client_v1(&mut service, client(1));
        assert_eq!(
            run(victim.take_delivery()).unwrap().unwrap().outcome,
            SemanticDeliveryOutcomeV1::Cancelled
        );
    }
}
