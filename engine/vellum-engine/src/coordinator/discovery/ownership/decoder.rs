//! Explicit context-owned native staging. No wire parser or implicit decoder grant.
use super::*;
mod shutdown_inventory;
use std::panic::{AssertUnwindSafe, catch_unwind};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryDecoderLimitsV1 {
    pub input: CaptureLimitsV1,
    pub max_held_bytes: usize,
}
impl DiscoveryDecoderLimitsV1 {
    pub fn minimum_input_bytes() -> usize {
        arguments::template_bytes()
    }
    /// Eight bounded native argument/fact/owned/template frames plus fixed state.
    /// Reply, packet, selectors and context retain their independent sponsorship.
    pub fn reserved_bytes(self) -> Result<usize, Error> {
        add(
            mul(8, self.input.max_bytes)?,
            add(size_of::<State>(), size_of::<Decoder>())?,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryDecoderUsageV1 {
    pub input: CaptureLimitsV1,
    pub reserved_bytes: usize,
    pub busy: bool,
    pub faulted: bool,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Decoder>),
}
pub(super) struct Decoder {
    limits: DiscoveryDecoderLimitsV1,
    reservation: retention::Reservation,
    charge: usize,
    busy: bool,
    handoff: bool,
    faulted: bool,
    #[cfg(test)]
    panic: bool,
    #[cfg(test)]
    panic_after: bool,
}
impl State {
    pub(super) fn busy(&self) -> bool {
        matches!(self, Self::Active(d) if d.busy)
    }
    pub(super) fn blocks_native_service(&self) -> bool {
        matches!(self, Self::Active(d) if d.busy && !d.handoff)
    }
    pub(super) fn validate(&self, ledger: &retention::RetentionState) -> Result<(), Error> {
        if let Self::Active(d) = self {
            ledger.validate(&d.reservation).map_err(parent)?;
            if !arguments::valid_limits(d.limits.input)
                || d.limits.input.max_bytes < arguments::template_bytes()
                || d.charge != d.limits.reserved_bytes().map_err(|_| Error::Faulted)?
                || d.charge > d.limits.max_held_bytes
                || (d.handoff && !d.busy)
                || (d.faulted && d.busy)
            {
                return Err(Error::Faulted);
            }
        }
        Ok(())
    }
    pub(super) fn release(&mut self, ledger: &mut retention::RetentionState) -> Result<(), Error> {
        self.validate(ledger)?;
        if self.busy() {
            return Err(Error::Unavailable);
        }
        if let Self::Active(d) = self {
            ledger.release(&mut d.reservation).map_err(parent)?;
        }
        Ok(())
    }
}
fn facts_copy(
    f: &TrustedDiscoveryFactsV1,
    l: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<TrustedDiscoveryFactsV1, Error> {
    let flag = |v: Option<bool>| v.map_or(InertValueV1::Null, InertValueV1::Boolean);
    let fields = [
        ("authorized", flag(f.authorized)),
        ("scope", flag(f.scope)),
        ("continuous", flag(f.continuous)),
        ("live", flag(f.live)),
        ("notStopped", flag(f.not_stopped)),
        ("discoveryAllowed", flag(f.discovery_allowed)),
        ("interpretationCurrent", flag(f.interpretation_current)),
        ("retained", flag(f.retained)),
        ("transferReady", flag(f.transfer_ready)),
        ("replyReady", flag(f.reply_ready)),
        ("resourcesReady", flag(f.resources_ready)),
        ("outsidePublicationGuard", flag(f.outside_publication_guard)),
        ("adviceAttested", flag(f.advice_attested)),
        ("targetCurrent", flag(f.target_current)),
        (
            "targetEqualityObservable",
            flag(f.target_equality_observable),
        ),
    ];
    let _captured = arguments::copied(&InertValueV1::Object(&fields), l, r)?;
    Ok(TrustedDiscoveryFactsV1 {
        authorized: f.authorized,
        scope: f.scope,
        continuous: f.continuous,
        live: f.live,
        not_stopped: f.not_stopped,
        discovery_allowed: f.discovery_allowed,
        interpretation_current: f.interpretation_current,
        retained: f.retained,
        transfer_ready: f.transfer_ready,
        reply_ready: f.reply_ready,
        resources_ready: f.resources_ready,
        outside_publication_guard: f.outside_publication_guard,
        advice_attested: f.advice_attested,
        target_current: f.target_current,
        target_equality_observable: f.target_equality_observable,
        ..Default::default()
    })
}
impl SourceCoordinator {
    fn discovery_decoder_mut(&mut self, id: &DiscoveryContextIdV1) -> Result<&mut Decoder, Error> {
        self.discovery_context(id)?;
        let c = self.discovery_store_mut()?.contexts[id.slot]
            .as_mut()
            .expect("original context");
        match &mut c.decoder {
            State::Active(d) => Ok(d),
            _ => Err(Error::Unavailable),
        }
    }
    pub fn enable_discovery_decoder_v1(
        &mut self,
        id: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        limits: DiscoveryDecoderLimitsV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = (|| {
            let c = self.discovery_context(id)?;
            if c.scope != *scope
                || c.stopped
                || c.decoder.busy()
                || f.owner_control_allowed != Some(true)
            {
                return Err(Error::Unavailable);
            }
            self.validate_discovery()?;
            let s = self.discovery_store()?;
            if self.faulted || s.faulted || s.stopped {
                return Err(Error::Unavailable);
            }
            let c = self.discovery_context(id)?;
            if let State::Active(d) = &c.decoder {
                return if d.limits == limits && !d.faulted {
                    Ok(())
                } else {
                    Err(Error::Unavailable)
                };
            }
            if !arguments::valid_limits(limits.input) {
                return Err(Error::InvalidRequest);
            }
            if limits.input.max_bytes < arguments::template_bytes() {
                return Err(Error::Limit);
            }
            let charge = limits.reserved_bytes()?;
            if charge > limits.max_held_bytes {
                return Err(Error::Limit);
            }
            self.retention.check(charge).map_err(parent)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            self.discovery_store_mut()?.contexts[id.slot]
                .as_mut()
                .expect("original context")
                .decoder = State::Active(Box::new(Decoder {
                limits,
                reservation,
                charge,
                busy: false,
                handoff: false,
                faulted: false,
                #[cfg(test)]
                panic: false,
                #[cfg(test)]
                panic_after: false,
            }));
            Ok(())
        })();
        self.discovery_finish(r, Boundary::DiscoveryDecoderConfiguration, &result);
        result
    }
    /// Trusted-owner workspace gauges; no descriptor/schema or execution authority.
    pub fn discovery_decoder_usage_v1(
        &self,
        id: &DiscoveryContextIdV1,
    ) -> Result<DiscoveryDecoderUsageV1, Error> {
        let c = self.discovery_context(id)?;
        c.decoder.validate(&self.retention)?;
        let State::Active(d) = &c.decoder else {
            return Err(Error::Unavailable);
        };
        Ok(DiscoveryDecoderUsageV1 {
            input: d.limits.input,
            reserved_bytes: d.charge,
            busy: d.busy,
            faulted: d.faulted,
        })
    }
    pub fn query_decoded_discovery_v1(
        &mut self,
        id: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        cut: &DiscoveryCutIdV1,
        envelope: &DiscoveryEnvelopeValueV1<'_, '_>,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryReplyV1, Error> {
        let result = self.decoded_discovery(id, scope, cut, envelope, f, r);
        self.discovery_finish(r, Boundary::DiscoveryDecoderQuery, &result);
        result
    }
    fn decoded_discovery(
        &mut self,
        id: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        cut: &DiscoveryCutIdV1,
        envelope: &DiscoveryEnvelopeValueV1<'_, '_>,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryReplyV1, Error> {
        let c = self.discovery_context(id)?;
        if c.scope != *scope || c.stopped || c.decoder.busy() {
            return Err(Error::Unavailable);
        }
        self.validate_discovery()?;
        if self.faulted || self.discovery_store()?.faulted {
            return Err(Error::Unavailable);
        }
        let d = self.discovery_decoder_mut(id)?;
        if d.faulted {
            return Err(Error::Unavailable);
        }
        let l = d.limits.input;
        d.busy = true;
        let result = catch_unwind(AssertUnwindSafe(|| {
            #[cfg(test)]
            if self.discovery_decoder_mut(id)?.panic {
                panic!("injected native discovery decoder failure");
            }
            let n = normalize_discovery_arguments_v1(envelope, scope, l, l.max_bytes, r)?;
            // Shape cannot retarget authority, apply loss to another profile or
            // replace an original role by copied ordinary metadata.
            if n.context() != id || n.profile() != n.scope().context.profile {
                return Err(Error::Unavailable);
            }
            let f = facts_copy(f, l, r)?;
            self.discovery_decoder_mut(id)?.handoff = true;
            let result = self.query_discovery_v1(n.context(), n.scope(), cut, n.query(), &f, r);
            #[cfg(test)]
            if self.discovery_decoder_mut(id)?.panic_after {
                panic!("injected post-native discovery delivery failure");
            }
            result
        }))
        .unwrap_or(Err(Error::Faulted));
        let d = self.discovery_decoder_mut(id).map_err(|_| Error::Faulted)?;
        if !d.busy {
            return Err(Error::Faulted);
        }
        d.handoff = false;
        d.busy = false;
        if matches!(result, Err(Error::Faulted)) {
            d.faulted = true;
        }
        self.discovery_context(id)?
            .decoder
            .validate(&self.retention)?;
        result
    }
}
