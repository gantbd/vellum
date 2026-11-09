//! Explicit context-owned eight-frame native decoder; no wire or fallback route.
use super::*;
#[cfg(test)]
mod tests;
use crate::capture::{CaptureErrorV1, CaptureLimitsV1, InertCaptureV1, InertValueV1};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationDecoderLimitsV1 {
    pub input: CaptureLimitsV1,
    pub max_held_bytes: usize,
}
impl NativeOperationDecoderLimitsV1 {
    pub fn reserved_bytes(self) -> Result<usize, Error> {
        add(
            self.input.max_bytes.checked_mul(8).ok_or(Error::Limit)?,
            add(size_of::<State>(), size_of::<Decoder>())?,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeOperationDecoderUsageV1 {
    pub busy: bool,
    pub faulted: bool,
    pub reserved_bytes: usize,
    pub input: CaptureLimitsV1,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeOperationDecodedReplyV1 {
    Query(NativeOperationObservationReplyV1),
    Waiting(NativeOperationWaitIdV1),
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Decoder>),
}
pub(super) struct Decoder {
    limits: NativeOperationDecoderLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    busy: bool,
    native_handoff: bool,
    faulted: bool,
    #[cfg(test)]
    pub(super) panic: bool,
}
impl State {
    pub(super) fn shutdown_reservation(
        &self,
        ledger: &retention::RetentionState,
    ) -> Result<usize, CoordinatorError> {
        let Self::Active(d) = self else {
            return Ok(0);
        };
        self.validate(ledger)
            .map_err(|_| CoordinatorError::Faulted)?;
        Ok(d.charge)
    }
    pub(super) fn busy(&self) -> bool {
        matches!(self, Self::Active(d) if d.busy)
    }
    pub(super) fn blocks_native_service(&self) -> bool {
        matches!(self, Self::Active(d) if d.busy && !d.native_handoff)
    }
    pub(super) fn validate(&self, ledger: &retention::RetentionState) -> Result<(), Error> {
        if let Self::Active(d) = self {
            ledger
                .validate_exact(&d.reservation, d.charge)
                .map_err(parent)?;
            if d.charge != d.limits.reserved_bytes()? || d.charge > d.limits.max_held_bytes {
                return Err(Error::Faulted);
            }
        }
        Ok(())
    }
    pub(super) fn release(&mut self, ledger: &mut retention::RetentionState) -> Result<(), Error> {
        if let Self::Active(d) = self {
            ledger.release(&mut d.reservation).map_err(parent)?;
        }
        Ok(())
    }
}
impl Target {
    fn native_kind(&self) -> NativeOperationKindV1 {
        match self {
            Self::SourceLoad(_) => NativeOperationKindV1::SourceLoad,
            Self::AnnotationLoad(_) => NativeOperationKindV1::AnnotationLoad,
            Self::Save(_, NativeSaveKindV1::SourceSave) => NativeOperationKindV1::SourceSave,
            Self::Save(_, NativeSaveKindV1::AnnotationSave) => {
                NativeOperationKindV1::AnnotationSave
            }
        }
    }
    fn task_matches(&self, task: NativeOperationTaskV1<'_>) -> bool {
        match (self, task) {
            (Self::SourceLoad(a), NativeOperationTaskV1::SourceLoad(b)) => a == b,
            (Self::AnnotationLoad(a), NativeOperationTaskV1::AnnotationLoad(b)) => a == b,
            (Self::Save(a, _), NativeOperationTaskV1::Save(b)) => a == b,
            _ => false,
        }
    }
}
fn measured(
    v: &InertValueV1<'_>,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<InertCaptureV1, Error> {
    InertCaptureV1::copy(v, l, r).map_err(|e| match e {
        CaptureErrorV1::Limit => Error::Limit,
        CaptureErrorV1::Capacity => Error::Capacity,
        _ => Error::InvalidRequest,
    })
}
fn flag(v: Option<bool>) -> InertValueV1<'static> {
    v.map_or(InertValueV1::Null, InertValueV1::Boolean)
}
fn inputs<'a>(
    p: &NativeOperationObservationPolicyV1,
    f: &NativeOperationObservationFactsV1<'_>,
    actor: &'a SettingsExecutionContextV1,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<
    (
        NativeOperationObservationPolicyV1,
        NativeOperationObservationFactsV1<'a>,
    ),
    Error,
> {
    // Lossless fixed-width native measurement shadow, not a wire epoch codec.
    let epoch = [
        InertValueV1::Number((p.epoch >> 32) as f64),
        InertValueV1::Number((p.epoch as u32) as f64),
    ];
    let policy = [
        ("recipient", InertValueV1::Text(&p.recipient)),
        ("epoch", InertValueV1::Array(&epoch)),
        ("grant", InertValueV1::Boolean(p.grant)),
        ("publication", InertValueV1::Boolean(p.publication)),
        ("control", InertValueV1::Boolean(p.control)),
        ("effect", InertValueV1::Boolean(p.effect)),
        ("reasons", InertValueV1::Boolean(p.reasons)),
        ("details", InertValueV1::Boolean(p.details)),
        ("recovery", InertValueV1::Boolean(p.recovery)),
    ];
    let facts = [
        ("authorized", flag(f.authorized)),
        ("scope", flag(f.scope)),
        ("continuous", flag(f.continuous)),
        ("live", flag(f.live)),
        ("retained", flag(f.retained)),
        ("transferReady", flag(f.transfer_ready)),
        ("resourcesReady", flag(f.resources_ready)),
        ("preparationReserved", flag(f.preparation_reserved)),
        ("registryReady", flag(f.registry_ready)),
        ("retentionReady", flag(f.retention_ready)),
        ("controlAllowed", flag(f.control_allowed)),
        ("controlReady", flag(f.control_ready)),
        ("ordinaryReady", flag(f.ordinary_ready)),
        (
            "operationReceiptAdmissionAllowed",
            flag(f.operation_receipt_admission_allowed),
        ),
        ("operationReceiptAllowed", flag(f.operation_receipt_allowed)),
        ("sourceReadable", flag(f.source_readable)),
        ("annotationReadable", flag(f.annotation_readable)),
        ("dispositionReadable", flag(f.disposition_readable)),
        ("readAllowed", flag(f.read_allowed)),
        ("replyReady", flag(f.reply_ready)),
        ("waitCapacity", flag(f.wait_capacity)),
        ("waitClockValid", flag(f.wait_clock_valid)),
        ("ownerControlAllowed", flag(f.owner_control_allowed)),
    ];
    let _policy = measured(&InertValueV1::Object(&policy), l, r)?;
    let _facts = measured(&InertValueV1::Object(&facts), l, r)?;
    let p = NativeOperationObservationPolicyV1 {
        recipient: copy_text(&p.recipient)?,
        epoch: p.epoch,
        grant: p.grant,
        publication: p.publication,
        control: p.control,
        effect: p.effect,
        reasons: p.reasons,
        details: p.details,
        recovery: p.recovery,
    };
    let f = NativeOperationObservationFactsV1 {
        context: actor,
        authorized: f.authorized,
        scope: f.scope,
        continuous: f.continuous,
        live: f.live,
        retained: f.retained,
        transfer_ready: f.transfer_ready,
        resources_ready: f.resources_ready,
        preparation_reserved: f.preparation_reserved,
        registry_ready: f.registry_ready,
        retention_ready: f.retention_ready,
        control_allowed: f.control_allowed,
        control_ready: f.control_ready,
        ordinary_ready: f.ordinary_ready,
        operation_receipt_admission_allowed: f.operation_receipt_admission_allowed,
        operation_receipt_allowed: f.operation_receipt_allowed,
        source_readable: f.source_readable,
        annotation_readable: f.annotation_readable,
        disposition_readable: f.disposition_readable,
        read_allowed: f.read_allowed,
        reply_ready: f.reply_ready,
        wait_capacity: f.wait_capacity,
        wait_clock_valid: f.wait_clock_valid,
        owner_control_allowed: f.owner_control_allowed,
    };
    Ok((p, f))
}
impl SourceCoordinator {
    pub fn enable_native_operation_decoder_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        limits: NativeOperationDecoderLimitsV1,
        f: &NativeOperationObservationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let pool = self.observation_pool()?;
            self.retention.validate(&pool.reservation).map_err(parent)?;
            if pool.stopped {
                return Err(Error::Unavailable);
            }
            let row = self.observation_row(id)?;
            if !row.live
                || row.busy
                || row.context != *f.context
                || f.owner_control_allowed != Some(true)
            {
                return Err(Error::Unavailable);
            }
            row.decoder.validate(&self.retention)?;
            self.retention.validate(&row.reservation).map_err(parent)?;
            self.native_operation_control(&row.target, &row.context)?;
            if let State::Active(d) = &row.decoder {
                return if d.limits == limits && !d.faulted && !d.busy {
                    Ok(())
                } else {
                    Err(Error::Unavailable)
                };
            }
            if !arguments::valid_limits(limits.input) {
                return Err(Error::InvalidRequest);
            }
            if limits.input.max_bytes
                < size_of::<[(&str, InertValueV1<'_>); 32]>() + size_of::<[InertValueV1<'_>; 2]>()
            {
                return Err(Error::Limit);
            }
            let charge = limits.reserved_bytes()?;
            if charge > limits.max_held_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(charge).map_err(parent)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.observation_row_mut(id)?.decoder = State::Active(Box::new(Decoder {
                limits,
                reservation,
                charge,
                busy: false,
                native_handoff: false,
                faulted: false,
                #[cfg(test)]
                panic: false,
            }));
            Ok(())
        })();
        observe(r, Boundary::NativeOperationDecoderConfiguration, &result);
        result
    }
    pub fn native_operation_decoder_usage_v1(
        &self,
        id: &NativeOperationObservationIdV1,
    ) -> Result<NativeOperationDecoderUsageV1, Error> {
        let row = self.observation_row(id)?;
        row.decoder.validate(&self.retention)?;
        let State::Active(d) = &row.decoder else {
            return Err(Error::Unavailable);
        };
        Ok(NativeOperationDecoderUsageV1 {
            busy: d.busy,
            faulted: d.faulted,
            reserved_bytes: d.charge,
            input: d.limits.input,
        })
    }
    pub fn query_decoded_native_operation_v1(
        &mut self,
        id: &NativeOperationObservationIdV1,
        envelope: &NativeOperationEnvelopeValueV1<'_, '_>,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        clock: Option<NativeOperationWaitClockV1>,
        r: &mut Recorder,
    ) -> Result<NativeOperationDecodedReplyV1, Error> {
        let result = self.decoded_native_operation_inner(id, envelope, p, f, clock, r);
        observe(r, Boundary::NativeOperationDecoderQuery, &result);
        result
    }
    fn decoded_native_operation_inner(
        &mut self,
        id: &NativeOperationObservationIdV1,
        envelope: &NativeOperationEnvelopeValueV1<'_, '_>,
        p: &NativeOperationObservationPolicyV1,
        f: &NativeOperationObservationFactsV1<'_>,
        clock: Option<NativeOperationWaitClockV1>,
        r: &mut Recorder,
    ) -> Result<NativeOperationDecodedReplyV1, Error> {
        let row = self.observation_row(id)?;
        row.decoder.validate(&self.retention)?;
        let State::Active(d) = &row.decoder else {
            return Err(Error::Unavailable);
        };
        if d.faulted || d.busy || row.busy || !row.live {
            return Err(Error::Unavailable);
        }
        let limits = d.limits.input;
        let State::Active(d) = &mut self.observation_row_mut(id)?.decoder else {
            unreachable!()
        };
        d.busy = true;
        let mut native_called = false;
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            if matches!(&self.observation_row(id)?.decoder, State::Active(d) if d.panic) {
                panic!("injected native decoder fault");
            }
            let n = normalize_native_operation_envelope_v1(
                envelope,
                f.context,
                limits,
                limits.max_bytes,
                r,
            )?;
            let row = self.observation_row(id)?;
            if n.context != id
                || n.actor != row.context
                || n.profile != row.context.profile
                || !row.target.task_matches(n.task)
                || matches!(n.query, NormalizedNativeOperationQueryV1::RetainedOutcome(k) if k != row.target.native_kind())
            {
                return Err(Error::Unavailable);
            }
            if !matches!(n.query, NormalizedNativeOperationQueryV1::Wait { .. }) && clock.is_some()
            {
                return Err(Error::InvalidRequest);
            }
            let (policy, facts) = inputs(p, f, &n.actor, limits, r)?;
            native_called = true;
            let State::Active(d) = &mut self.observation_row_mut(id)?.decoder else {
                unreachable!()
            };
            d.native_handoff = true;
            match n.query {
                NormalizedNativeOperationQueryV1::Wait { budget } => self
                    .start_native_operation_observation_wait_v1(
                        id,
                        NativeOperationWaitRequestV1 {
                            after: n.after.ok_or(Error::InvalidRequest)?,
                            budget,
                            clock: clock.ok_or(Error::Unavailable)?,
                        },
                        &policy,
                        &facts,
                        r,
                    )
                    .map(NativeOperationDecodedReplyV1::Waiting),
                q => self
                    .query_native_operation_observation_v1(
                        id,
                        match q {
                            NormalizedNativeOperationQueryV1::Status => {
                                NativeOperationObservationQueryV1::Status
                            }
                            NormalizedNativeOperationQueryV1::MinimalControlStatus => {
                                NativeOperationObservationQueryV1::MinimalControlStatus
                            }
                            NormalizedNativeOperationQueryV1::RetainedOutcome(_) => {
                                NativeOperationObservationQueryV1::RetainedOutcome
                            }
                            _ => unreachable!(),
                        },
                        &policy,
                        &facts,
                        r,
                    )
                    .map(NativeOperationDecodedReplyV1::Query),
            }
        }));
        let local_fault =
            outcome.is_err() || (!native_called && matches!(&outcome, Ok(Err(Error::Faulted))));
        if let Ok(row) = self.observation_row_mut(id)
            && let State::Active(d) = &mut row.decoder
        {
            d.busy = false;
            d.native_handoff = false;
            if local_fault {
                d.faulted = true;
            }
        }
        outcome.unwrap_or(Err(Error::Faulted))
    }
}
