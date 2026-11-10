//! Closed borrowed F06 observation input over the existing native observer.
//! No owned decoder, label registry, callback, timer, producer action or new grant.
use super::*;
use crate::capture::InertValueV1 as D;

#[derive(Clone, Copy)]
pub enum InvocationObservationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, InvocationObservationEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Context(&'h InvocationObservationContextIdV1),
    After(&'h InvocationObservationReferenceV1),
}
/// Current native release facts arrive independently of recipient input.
/// Minimal control requires neither ordinary readiness nor rich-result capacity.
pub enum InvocationObservationReadPortV1<'a> {
    Ordinary {
        current: &'a InvocationObservationPolicyV1,
        facts: &'a TrustedInvocationOrdinaryObservationReleaseV1<'a>,
    },
    MinimalControl {
        current: &'a InvocationObservationPolicyV1,
        facts: &'a TrustedInvocationObservationReleaseV1<'a>,
    },
    Wait {
        current: &'a InvocationObservationPolicyV1,
        facts: &'a TrustedInvocationWaitFactsV1<'a>,
        clock: InvocationWaitClockV1,
    },
}
#[derive(Debug, PartialEq, Eq)]
pub enum InvocationObservationReadReplyV1 {
    Ordinary(InvocationObservationReplyV1),
    MinimalControl(InvocationControlObservationV1),
    Waiting(InvocationWaitIdV1),
}
impl std::fmt::Debug for InvocationObservationEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObservationEnvelope(<redacted>)")
    }
}
impl std::fmt::Debug for InvocationObservationReadPortV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationObservationReadPort(<redacted>)")
    }
}
type V<'a, 'h> = InvocationObservationEnvelopeValueV1<'a, 'h>;
enum Parsed<'h> {
    Ordinary(InvocationObservationQueryV1),
    MinimalControl,
    Wait {
        after: &'h InvocationObservationReferenceV1,
        budget: usize,
    },
}
fn fields<'a, 'h>(
    v: &V<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, V<'a, 'h>)], InvocationError> {
    let V::Object(rows) = v else {
        return Err(InvocationError::Unavailable);
    };
    if rows.len() != keys.len()
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| !keys.contains(k) || rows[..i].iter().any(|(prior, _)| prior == k))
    {
        return Err(InvocationError::Unavailable);
    }
    Ok(rows)
}
fn get<'a, 'h>(rows: &'a [(&'a str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>, InvocationError> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::Unavailable)
}
fn text(v: V<'_, '_>, expected: &str) -> bool {
    matches!(v, V::Data(D::Text(actual)) if actual == expected)
}
fn parse<'h>(
    envelope: &V<'_, 'h>,
    observer: &InvocationObservationContextV1,
) -> Result<Parsed<'h>, InvocationError> {
    let outer = fields(envelope, &["route", "request"])?;
    let typed = text(get(outer, "route")?, "typed");
    if !typed && !text(get(outer, "route")?, "dynamic") {
        return Err(InvocationError::Unavailable);
    }
    let h = fields(
        &get(outer, "request")?,
        &[
            if typed { "family" } else { "name" },
            "variant",
            "version",
            "arguments",
        ],
    )?;
    if !(if typed {
        matches!(get(h, "family")?, V::Data(D::Number(5.0)))
    } else {
        text(get(h, "name")?, "inspect-execution")
    }) || !matches!(get(h, "version")?, V::Data(D::Number(1.0)))
    {
        return Err(InvocationError::Unavailable);
    }
    let variant = get(h, "variant")?;
    let operation = if text(variant, "status") {
        "status"
    } else if text(variant, "retained-outcome") {
        "retained-outcome"
    } else if text(variant, "minimal-control-status") {
        "minimal-control-status"
    } else if text(variant, "bounded-observation-wait") {
        "bounded-observation-wait"
    } else {
        return Err(InvocationError::Unavailable);
    };
    let waiting = operation == "bounded-observation-wait";
    let a = fields(
        &get(h, "arguments")?,
        if waiting {
            &[
                "contract",
                "operation",
                "invocation",
                "context",
                "profile",
                "after",
                "budget",
            ]
        } else {
            &["contract", "operation", "invocation", "context", "profile"]
        },
    )?;
    if !text(get(a, "contract")?, "execution-observation/1")
        || !text(get(a, "operation")?, operation)
        || !matches!(get(a,"invocation")?, V::Invocation(id) if *id == observer.invocation)
        || !matches!(get(a,"context")?, V::Context(id) if id.matches_context(observer))
        || !text(get(a, "profile")?, &observer.context.profile)
    {
        return Err(InvocationError::Unavailable);
    }
    Ok(match operation {
        "status" => Parsed::Ordinary(InvocationObservationQueryV1::Status),
        "retained-outcome" => Parsed::Ordinary(InvocationObservationQueryV1::RetainedOutcome),
        "minimal-control-status" => Parsed::MinimalControl,
        "bounded-observation-wait" => {
            let V::After(after) = get(a, "after")? else {
                return Err(InvocationError::Unavailable);
            };
            let V::Data(D::Number(n)) = get(a, "budget")? else {
                return Err(InvocationError::Unavailable);
            };
            if !(1.0..=9_007_199_254_740_991.0).contains(&n) || n.fract() != 0.0 {
                return Err(InvocationError::Unavailable);
            }
            Parsed::Wait {
                after,
                budget: usize::try_from(n as u64).map_err(|_| InvocationError::Limit)?,
            }
        }
        _ => return Err(InvocationError::Unavailable),
    })
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn recognize_native_invocation_observation_request_v1(
        &self,
        observer: &InvocationObservationContextV1,
        envelope: &InvocationObservationEnvelopeValueV1<'_, '_>,
        port: &InvocationObservationReadPortV1<'_>,
    ) -> Result<&'static str, InvocationError> {
        match (parse(envelope, observer)?, port) {
            (
                Parsed::Ordinary(InvocationObservationQueryV1::Status),
                InvocationObservationReadPortV1::Ordinary { .. },
            ) => Ok("status"),
            (
                Parsed::Ordinary(InvocationObservationQueryV1::RetainedOutcome),
                InvocationObservationReadPortV1::Ordinary { .. },
            ) => Ok("retained-outcome"),
            (Parsed::MinimalControl, InvocationObservationReadPortV1::MinimalControl { .. }) => {
                Ok("minimal-control-status")
            }
            (Parsed::Wait { .. }, InvocationObservationReadPortV1::Wait { .. }) => {
                Ok("bounded-observation-wait")
            }
            _ => Err(InvocationError::Unavailable),
        }
    }
    /// Exactly one native read or bounded wait admission. There is no producer
    /// execution, blocking loop, clock read or ordinary-acquisition fallback.
    /// Wait continuation and protected discharge keep their existing native APIs.
    pub fn query_decoded_invocation_observation_v1(
        &self,
        observer: &mut InvocationObservationContextV1,
        envelope: &InvocationObservationEnvelopeValueV1<'_, '_>,
        port: InvocationObservationReadPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationObservationReadReplyV1, InvocationError> {
        let timing = r.begin_service_timing();
        let result = parse(envelope, observer).and_then(|query| match (query, port) {
            (
                Parsed::Ordinary(query),
                InvocationObservationReadPortV1::Ordinary { current, facts },
            ) => self
                .query_invocation_observation_v1(observer, query, current, facts, r)
                .map(InvocationObservationReadReplyV1::Ordinary),
            (
                Parsed::MinimalControl,
                InvocationObservationReadPortV1::MinimalControl { current, facts },
            ) => self
                .minimal_invocation_control_status_v1(observer, current, facts, r)
                .map(InvocationObservationReadReplyV1::MinimalControl),
            (
                Parsed::Wait { after, budget },
                InvocationObservationReadPortV1::Wait {
                    current,
                    facts,
                    clock,
                },
            ) => self
                .start_invocation_observation_wait_v1(
                    observer,
                    InvocationWaitRequestV1 {
                        after,
                        budget,
                        clock,
                    },
                    current,
                    facts,
                    r,
                )
                .map(InvocationObservationReadReplyV1::Waiting),
            _ => Err(InvocationError::Unavailable),
        });
        capture_timed(
            r,
            Boundary::InvocationObservationIngress,
            &result,
            timing.finish(),
        );
        result
    }
}
