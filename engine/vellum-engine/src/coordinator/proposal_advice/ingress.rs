//! Closed borrowed native routes; not a name resolver or wire codec.
use super::*;
use crate::capture::InertValueV1 as D;
#[cfg(test)]
pub(in crate::coordinator) mod tests;
#[derive(Clone, Copy)]
pub enum CompoundAdviceEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, CompoundAdviceEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Plan(&'h CompoundIntent),
    Proposal(&'h CompoundAdviceRequestV1<'h>),
}
impl std::fmt::Debug for CompoundAdviceEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CompoundAdviceEnvelopeV1(<redacted>)")
    }
}
type V<'a, 'h> = CompoundAdviceEnvelopeValueV1<'a, 'h>;
fn fields<'a, 'h>(v: &'a V<'a, 'h>, keys: &[&str]) -> Option<&'a [(&'a str, V<'a, 'h>)]> {
    let V::Object(rows) = v else {
        return None;
    };
    if rows.len() != keys.len()
        || rows.iter().any(|(k, _)| !keys.contains(k))
        || rows
            .iter()
            .enumerate()
            .any(|(i, (k, _))| rows[..i].iter().any(|(old, _)| old == k))
    {
        return None;
    }
    Some(rows)
}
fn field<'a, 'h>(rows: &'a [(&'a str, V<'a, 'h>)], key: &str) -> Option<&'a V<'a, 'h>> {
    rows.iter().find(|(k, _)| *k == key).map(|(_, value)| value)
}
fn text<'a>(value: &V<'a, '_>) -> Option<&'a str> {
    match value {
        V::Data(D::Text(text)) => Some(text),
        _ => None,
    }
}
pub(in crate::coordinator) fn recognize<'a, 'h>(
    envelope: &'a V<'a, 'h>,
) -> Option<&'h CompoundAdviceRequestV1<'h>> {
    // Each traversal is length-preflighted to at most four rows. No payload
    // copying, native lookup, fact application or allocation occurs here.
    let outer = fields(envelope, &["route", "request", "proposal"])?;
    let route = text(field(outer, "route")?)?;
    let keys = match route {
        "typed" => ["family", "variant", "version", "arguments"],
        "dynamic" => ["name", "variant", "version", "arguments"],
        _ => return None,
    };
    let header = fields(field(outer, "request")?, &keys)?;
    if !matches!(
        (route, field(header, keys[0])?),
        ("typed", V::Data(D::Number(9.0))) | ("dynamic", V::Data(D::Text("validate-proposal")))
    ) || !matches!(field(header, "version")?, V::Data(D::Number(1.0)))
        || text(field(header, "variant")?)? != "advisory-validation"
    {
        return None;
    }
    let args = fields(
        field(header, "arguments")?,
        &["contract", "operation", "plan"],
    )?;
    if text(field(args, "contract")?)? != "proposal-validation/1"
        || text(field(args, "operation")?)? != "advisory-validation"
    {
        return None;
    }
    let V::Plan(plan) = field(args, "plan")? else {
        return None;
    };
    let V::Proposal(proposal) = field(outer, "proposal")? else {
        return None;
    };
    std::ptr::eq(*plan, proposal.plan).then_some(*proposal)
}
impl SourceCoordinator {
    /// Header constants become compiled meaning; original native roles stay
    /// borrowed. The existing query funds bounded copying and candidate work.
    pub fn query_decoded_compound_advice_v1(
        &mut self,
        envelope: &CompoundAdviceEnvelopeValueV1<'_, '_>,
        facts: &TrustedCompoundAdviceFactsV1<'_>,
        r: &mut Recorder,
    ) -> CompoundAdviceReplyV1 {
        let timing = r.begin_service_timing();
        let request = recognize(envelope);
        r.record(Observation {
            boundary: Boundary::CompoundAdviceArgumentRecognition,
            disposition: if request.is_some() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: None,
        });
        let out = request.map_or(CompoundAdviceReplyV1::Unavailable, |request| {
            self.query_compound_advice_v1(request, facts, r)
        });
        self.record_compound_advice_dispatch(
            r,
            Boundary::CompoundAdviceDecodedDispatch,
            &out,
            timing.finish(),
        );
        out
    }
    pub(in crate::coordinator) fn record_compound_advice_dispatch(
        &self,
        r: &mut Recorder,
        boundary: Boundary,
        out: &CompoundAdviceReplyV1,
        service_time: Option<std::time::Duration>,
    ) {
        r.record(Observation {
            boundary,
            disposition: if self.is_faulted() {
                Disposition::Unknown
            } else if matches!(out, CompoundAdviceReplyV1::Advice { .. }) {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time,
        });
    }
}
