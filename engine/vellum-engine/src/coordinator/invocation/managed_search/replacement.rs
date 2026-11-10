//! Returned-result F11 admission reuses the managed-search plan and publisher.
use super::*;
use crate::capture::CaptureLimitsV1;
use crate::capture::{CaptureErrorV1, InertCaptureV1, InertValueV1 as D};
use std::mem::size_of;
mod arguments;
pub use arguments::*;

#[derive(Debug)]
pub(in crate::coordinator::invocation) struct Capture {
    pub(in crate::coordinator::invocation) consumer: Arc<ConsumerCapture>,
    invocation: InvocationId,
    limits: CaptureLimitsV1,
    canonical: InertCaptureV1,
    reply_bytes: usize,
    canonical_hold: retention::Reservation,
    reply_hold: retention::Reservation,
}
impl Capture {
    pub(in crate::coordinator::invocation) fn reserved_bytes(&self) -> Option<usize> {
        self.canonical
            .usage()
            .retained_bytes
            .checked_add(self.reply_bytes)
    }
    pub(in crate::coordinator::invocation) fn intact(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        ledger: &retention::RetentionState,
    ) -> bool {
        Arc::ptr_eq(owner, &self.consumer.owner)
            && self.invocation == *id
            && self.canonical.usage().peak_bytes <= self.limits.max_bytes
            && self.canonical.usage().nodes <= self.limits.max_nodes
            && self.canonical.usage().depth <= self.limits.max_depth
            && self.reserved_bytes().is_some()
            && ledger
                .validate_exact(&self.canonical_hold, self.canonical.usage().retained_bytes)
                .is_ok()
            && ledger
                .validate_exact(&self.reply_hold, self.reply_bytes)
                .is_ok()
            && self
                .reserved_bytes()
                .is_some_and(|n| ledger.check(0).is_ok_and(|held| held >= n))
    }
}
fn parent(e: RetentionErrorV1) -> InvocationError {
    match e {
        RetentionErrorV1::Faulted => InvocationError::Faulted,
        RetentionErrorV1::Limit => InvocationError::Limit,
        RetentionErrorV1::Unavailable => InvocationError::Unavailable,
    }
}
fn number(n: usize) -> Result<f64, InvocationError> {
    if n as u128 > 9_007_199_254_740_991 {
        return Err(InvocationError::Limit);
    }
    Ok(n as f64)
}
/// Canonical native projection of the adopted four request fields, all seven
/// actor labels and all three input limits. Opaque result/ticket identities stay
/// in the owning native capture; no fake name, query rerun or wire codec.
fn canonical(
    command: &SourceSearchConsumerV1,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    max_intent: usize,
    r: &mut Recorder,
) -> Result<InertCaptureV1, InvocationError> {
    let (op, text) = match command {
        SourceSearchConsumerV1::ReplaceAll { replacement, .. } => ("replace-all", replacement),
        SourceSearchConsumerV1::ReplaceCurrent { replacement, .. } => {
            ("replace-current", replacement)
        }
        _ => return Err(InvocationError::MalformedIntent),
    };
    canonical_fields(op, command.index(), text, actor, limits, max_intent, r)
}
fn canonical_fields(
    op: &str,
    index: Option<usize>,
    text: &str,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    max_intent: usize,
    r: &mut Recorder,
) -> Result<InertCaptureV1, InvocationError> {
    if crate::coordinator::settings::basis::labels(actor)
        .iter()
        .any(|s| s.is_empty())
    {
        return Err(InvocationError::MalformedIntent);
    }
    let index = match index {
        Some(i) => D::Number(number(i)?),
        None => D::Null,
    };
    let request = [
        ("operation", D::Text(op)),
        ("index", index),
        ("replacement", D::Text(text)),
        ("invocation", D::Null),
    ];
    let context = [
        ("authority", D::Text(&actor.authority)),
        ("engine", D::Text(&actor.engine)),
        ("recipient", D::Text(&actor.recipient)),
        ("purpose", D::Text(&actor.purpose)),
        ("destination", D::Text(&actor.destination)),
        ("profile", D::Text(&actor.profile)),
        ("continuity", D::Text(&actor.continuity)),
    ];
    let original_limits = [
        ("maxRequestBytes", D::Number(number(limits.max_bytes)?)),
        ("maxInputNodes", D::Number(number(limits.max_nodes)?)),
        ("maxInputDepth", D::Number(number(limits.max_depth)?)),
    ];
    let opaque = [D::Text("literal-search-result")];
    let fields = [
        ("request", D::Object(&request)),
        ("context", D::Object(&context)),
        ("limits", D::Object(&original_limits)),
        ("opaqueBindings", D::Array(&opaque)),
    ];
    InertCaptureV1::copy(
        &D::Object(&fields),
        CaptureLimitsV1 {
            max_bytes: limits.max_bytes.min(max_intent),
            ..limits
        },
        r,
    )
    .map_err(|e| match e {
        CaptureErrorV1::Capacity => InvocationError::Capacity,
        CaptureErrorV1::Limit => InvocationError::Limit,
        _ => InvocationError::MalformedIntent,
    })
}
impl SourceCoordinator {
    /// F11 trusted-native replacement from an actually released original search
    /// result. Current read/query/mutation facts are separately required again by
    /// `run_managed_search_consumer`. Result identity itself grants no authority.
    /// Failed admission leaves the ticket unbound and refunds both provisional
    /// charges; admitted stop/outcome forgetting do not release either charge.
    #[allow(clippy::too_many_arguments)]
    pub fn submit_returned_search_replacement_v1(
        &mut self,
        id: &InvocationId,
        command: &SourceSearchConsumerV1,
        scope: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        limits: CaptureLimitsV1,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        let timer = r.begin_service_timing();
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let mut provisional: Option<(retention::Reservation, retention::Reservation)> = None;
        let result = (|| {
            self.guard_shutdown_invocation_admission(id, r)?;
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            let slot = self.invocation_slot(id)?;
            if let Some(original) = &slot.intent {
                let BoundIntent::ReturnedSearchReplacement(c) = original else {
                    return Err(InvocationError::IntentMismatch);
                };
                if c.consumer.command != *command
                    || c.consumer.scope != *scope
                    || c.consumer.request != *f.request
                    || sf.scope() != scope
                    || !c.consumer.read_matches(sf)
                    || cf.scope != scope
                    || cf.command != command
                    || c.limits != limits
                {
                    return Err(InvocationError::IntentMismatch);
                }
                if !c.intact(&self.namespace, id, &self.retention) {
                    return Err(InvocationError::Faulted);
                }
                return slot.phase.state();
            }
            let canonical = canonical(
                command,
                &scope.context,
                limits,
                self.invocations.limits.max_intent_bytes,
                r,
            )?;
            let reply_bytes = self
                .returned_search_frame_bytes(command.result())
                .map_err(error)?;
            let charge = canonical
                .usage()
                .retained_bytes
                .checked_add(reply_bytes)
                .ok_or(InvocationError::Limit)?;
            // One serialized parent owner: checking the sum precedes either
            // reservation and all native producer work. No capture-only admission.
            self.retention.check(charge).map_err(parent)?;
            let mut first = self
                .retention
                .reserve(canonical.usage().retained_bytes)
                .map_err(parent)?;
            let second = match self.retention.reserve(reply_bytes) {
                Ok(v) => v,
                Err(e) => {
                    self.retention.release(&mut first).map_err(parent)?;
                    return Err(parent(e));
                }
            };
            provisional = Some((first, second));
            let (consumer, plan) = self
                .capture_source_search_consumer(
                    command,
                    scope,
                    sf,
                    f,
                    cf,
                    self.invocations.limits.max_intent_bytes,
                    r,
                )
                .map_err(error)?;
            if let ConsumerPlan::Source(Err(e)) = &plan {
                return Err(match e {
                    CoordinatorError::Faulted
                    | CoordinatorError::History(HistoryError::Faulted) => InvocationError::Faulted,
                    CoordinatorError::Limit => InvocationError::Limit,
                    CoordinatorError::Capacity => InvocationError::Capacity,
                    _ => InvocationError::Unavailable,
                });
            }
            let envelope = consumer
                .replacement_envelope_bytes(&plan)
                .map_err(error)?
                .checked_add(size_of::<Capture>())
                .and_then(|n| n.checked_add(canonical.usage().retained_bytes))
                .ok_or(InvocationError::Limit)?;
            if envelope > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            let (canonical_hold, reply_hold) =
                provisional.take().expect("both provisional charges");
            let capture = Box::new(Capture {
                consumer: consumer.clone(),
                invocation: id.clone(),
                limits,
                canonical,
                reply_bytes,
                canonical_hold,
                reply_hold,
            });
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued replacement");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slot is issued")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::ReturnedSearchReplacement(capture),
                Phase::ManagedSearchPending(Pending {
                    capture: consumer,
                    plan,
                    reserved,
                }),
            );
            Ok(InvocationState::Pending)
        })();
        let mut result = result;
        if let Some((mut first, mut second)) = provisional {
            let capture_released = self.retention.release(&mut first);
            let reply_released = self.retention.release(&mut second);
            if capture_released.is_err() || reply_released.is_err() {
                result = Err(InvocationError::Faulted);
            }
        }
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::ReturnedSearchReplacementAdmission
            },
            &result,
            timer.finish(),
        );
        result
    }
    /// Protected retirement of this invocation's private plan/capture only.
    /// The original query result, issuer leases and caller copies are independent.
    pub fn retire_returned_search_replacement_v1(
        &mut self,
        id: &InvocationId,
        f: &InvocationObservationPolicyControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), InvocationError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if [
                f.owner_control_allowed,
                f.retention_released,
                f.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(InvocationError::Unavailable);
            }
            let slot = self.invocation_slot(id)?;
            let Some(BoundIntent::ReturnedSearchReplacement(c)) = &slot.intent else {
                return Err(InvocationError::Unavailable);
            };
            if c.consumer.scope.context != *f.context {
                return Err(InvocationError::Unavailable);
            }
            if !c.intact(&self.namespace, id, &self.retention) {
                return Err(InvocationError::Faulted);
            }
            if !(matches!(slot.phase, Phase::Terminal(_))
                || self.faulted
                    && matches!(slot.phase, Phase::Running | Phase::ManagedSearchPending(_)))
            {
                return Err(InvocationError::Pending);
            }
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("original funded slot");
            let Some(BoundIntent::ReturnedSearchReplacement(c)) = &mut slot.intent else {
                unreachable!()
            };
            self.retention
                .release(&mut c.canonical_hold)
                .map_err(parent)?;
            self.retention.release(&mut c.reply_hold).map_err(parent)?;
            slot.intent = Some(BoundIntent::RetiredSearchReplacement);
            if matches!(slot.phase, Phase::Terminal(_)) {
                slot.phase = Phase::Terminal(None);
            } else {
                slot.phase = Phase::Running;
            }
            Ok(())
        })();
        if result == Err(InvocationError::Faulted) {
            self.faulted = true;
        }
        capture_timed(
            r,
            Boundary::ReturnedSearchReplacementRetirement,
            &result,
            timer.finish(),
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{facts, mutation, query, ready, request};
    use super::*;

    fn limits() -> CaptureLimitsV1 {
        CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 128,
            max_depth: 8,
        }
    }
    fn retire(context: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
        InvocationObservationPolicyControlV1 {
            context,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        }
    }
    #[test]
    fn only_returned_results_admit_and_replay_cannot_borrow_the_legacy_ingress() {
        let (mut o, mut r, d, s, c) = ready();
        let q = request();
        let id = o.issue_invocation(&mut r).unwrap();
        let baseline = o.retention_usage().reserved;
        assert_eq!(
            o.submit_returned_search_replacement_v1(
                &id,
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &mutation(&c, &s),
                limits(),
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        assert_eq!(o.retention_usage().reserved, baseline);
        o.release_source_search(c.result(), &s, &facts(&s), &query(&q), &mut r)
            .unwrap();
        assert_eq!(
            o.submit_returned_search_replacement_v1(
                &id,
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &mutation(&c, &s),
                limits(),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        let funded = o.retention_usage().reserved;
        assert!(funded > baseline);
        assert_eq!(
            o.submit_returned_search_replacement_v1(
                &id,
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &mutation(&c, &s),
                limits(),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.retention_usage().reserved, funded);
        assert_eq!(
            o.submit_source_search_consumer(
                &id,
                &c,
                &s,
                &facts(&s),
                &query(&q),
                &mutation(&c, &s),
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        let mut changed = limits();
        changed.max_nodes += 1;
        assert_eq!(
            o.submit_returned_search_replacement_v1(
                &id,
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &mutation(&c, &s),
                changed,
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.run_source_search_consumer(
                &id,
                &s,
                &facts(&s),
                &query(&q),
                &mutation(&c, &s),
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            o.documents[d.document.slot].as_ref().unwrap().source.text(),
            "ZbZ"
        );
        assert_eq!(
            o.retire_invocation(&id, &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_returned_search_replacement_v1(&id, &retire(&s.context), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, baseline);
        assert_eq!(o.source_search_usage().results, 1);
        assert_eq!(
            o.retire_returned_search_replacement_v1(&id, &retire(&s.context), &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(o.retention_usage().reserved, baseline);
        o.retire_invocation(&id, &mut r).unwrap();
    }
    #[test]
    fn producer_refusal_refunds_and_current_authority_loss_cannot_publish() {
        let (mut o, mut r, d, s, c) = ready();
        let q = request();
        o.release_source_search(c.result(), &s, &facts(&s), &query(&q), &mut r)
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let baseline = o.retention_usage().reserved;
        let mut denied = mutation(&c, &s);
        denied.mutation_allowed = Some(false);
        assert_eq!(
            o.submit_returned_search_replacement_v1(
                &id,
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &denied,
                limits(),
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(o.retention_usage().reserved, baseline);
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        o.submit_returned_search_replacement_v1(
            &id,
            &c,
            &s,
            SearchReadFactsV1::Source(&facts(&s)),
            &query(&q),
            &mutation(&c, &s),
            limits(),
            &mut r,
        )
        .unwrap();
        o.run_source_search_consumer(&id, &s, &facts(&s), &query(&q), &denied, &mut r)
            .unwrap();
        assert_eq!(
            o.documents[d.document.slot].as_ref().unwrap().source.text(),
            "aba"
        );
        assert!(o.retention_usage().reserved > baseline);
        o.retire_returned_search_replacement_v1(&id, &retire(&s.context), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, baseline);
    }
    #[test]
    fn sum_headroom_is_required_before_preparation_and_exact_capacity_admits() {
        let (mut o, mut r, _, s, c) = ready();
        let q = request();
        o.release_source_search(c.result(), &s, &facts(&s), &query(&q), &mut r)
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let capture_bytes = canonical(
            &c,
            &s.context,
            limits(),
            o.invocations.limits.max_intent_bytes,
            &mut r,
        )
        .unwrap()
        .usage()
        .retained_bytes;
        let frame_bytes = o.returned_search_frame_bytes(c.result()).unwrap();
        let charge = capture_bytes + frame_bytes;
        let baseline = o.retention_usage().reserved;
        for headroom in [capture_bytes, charge - 1, charge] {
            let mut pressure = o
                .retention
                .reserve(usize::MAX - baseline - headroom)
                .unwrap();
            let before = o.retention_usage().reserved;
            let result = o.submit_returned_search_replacement_v1(
                &id,
                &c,
                &s,
                SearchReadFactsV1::Source(&facts(&s)),
                &query(&q),
                &mutation(&c, &s),
                limits(),
                &mut r,
            );
            if headroom < charge {
                assert_eq!(result, Err(InvocationError::Limit));
                assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
                assert_eq!(o.retention_usage().reserved, before);
            } else {
                assert_eq!(result, Ok(InvocationState::Pending));
                assert_eq!(o.retention_usage().reserved, usize::MAX);
            }
            o.retention.release(&mut pressure).unwrap();
        }
        assert_eq!(o.retention_usage().reserved, baseline + charge);
        assert!(o.stop_invocation(&id, &mut r).is_ok());
        assert_eq!(o.retention_usage().reserved, baseline + charge);
        o.retire_returned_search_replacement_v1(&id, &retire(&s.context), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, baseline);
    }
    #[test]
    fn private_plan_fault_keeps_funding_until_protected_cleanup_and_inventory_accounts_for_it() {
        let (mut o, mut r, d, s, c) = ready();
        let q = request();
        o.release_source_search(c.result(), &s, &facts(&s), &query(&q), &mut r)
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let baseline = o.retention_usage().reserved;
        o.submit_returned_search_replacement_v1(
            &id,
            &c,
            &s,
            SearchReadFactsV1::Source(&facts(&s)),
            &query(&q),
            &mutation(&c, &s),
            limits(),
            &mut r,
        )
        .unwrap();
        let inventory = crate::coordinator::shutdown::tests::inspect(&o, &mut r);
        let read = crate::coordinator::shutdown::tests::inspect_read(&o, &inventory, &mut r);
        assert!(read.coverage.all_owner_obligations);
        assert_eq!(read.funding.unrepresented_parent_bytes, 0);
        assert_eq!(
            o.retire_returned_search_replacement_v1(&id, &retire(&s.context), &mut r),
            Err(InvocationError::Pending)
        );
        let Phase::ManagedSearchPending(p) =
            &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
        else {
            panic!()
        };
        let ConsumerPlan::Source(Ok(plan)) = &mut p.plan else {
            panic!()
        };
        plan.candidates[0].edits[0].inserted = "unrelated".into();
        assert_eq!(
            o.run_source_search_consumer(
                &id,
                &s,
                &facts(&s),
                &query(&q),
                &mutation(&c, &s),
                &mut r
            ),
            Err(InvocationError::Faulted)
        );
        assert!(o.retention_usage().reserved > baseline);
        assert_eq!(
            o.documents[d.document.slot].as_ref().unwrap().source.text(),
            "aba"
        );
        let mut denied = retire(&s.context);
        denied.delivery_drained = Some(false);
        assert_eq!(
            o.retire_returned_search_replacement_v1(&id, &denied, &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_returned_search_replacement_v1(&id, &retire(&s.context), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, baseline);
        assert_eq!(
            o.run_source_search_consumer(
                &id,
                &s,
                &facts(&s),
                &query(&q),
                &mutation(&c, &s),
                &mut r
            ),
            Err(InvocationError::Faulted)
        );
    }
}
