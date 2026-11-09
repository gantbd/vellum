//! Borrowed in-process F06 receipt input. No wire IDs, heap capture, callbacks,
//! authority synthesis, command replay or new receipt/result issuer.
use super::*;
use crate::capture::InertValueV1 as D;

#[derive(Clone, Copy)]
pub enum InvocationReceiptEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, InvocationReceiptEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Context(&'h InvocationObservationContextIdV1),
}
impl std::fmt::Debug for InvocationReceiptEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InvocationReceiptEnvelope(<redacted>)")
    }
}
type V<'a, 'h> = InvocationReceiptEnvelopeValueV1<'a, 'h>;
fn fields<'a, 'h>(
    v: &V<'a, 'h>,
    keys: &[&str],
) -> Result<&'a [(&'a str, V<'a, 'h>)], InvocationError> {
    let V::Object(rows) = v else {
        return Err(InvocationError::Unavailable);
    };
    // Exact cardinality bounds all later scans, including malformed input.
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
fn index_matches(v: V<'_, '_>, index: usize) -> bool {
    matches!(v, V::Data(D::Number(n)) if (0.0..=9_007_199_254_740_991.0).contains(&n)
        && n.fract() == 0.0 && n as u128 == index as u128)
}

/// Closed native selector mapping. Older executable enums remain exhaustive;
/// future additions must supply their own meaning here rather than fall through.
fn selector(q: InvocationReceiptRequestV10<'_>) -> (&'static str, Option<usize>) {
    let q = match q {
        InvocationReceiptRequestV10::Existing(v) => v,
        InvocationReceiptRequestV10::SourceSaveAs(_) => return ("lifecycle", None),
    };
    let q = match q {
        InvocationReceiptRequestV9::Existing(v) => v,
        InvocationReceiptRequestV9::AnnotationLoad(_) => return ("lifecycle", None),
    };
    let q = match q {
        InvocationReceiptRequestV8::Existing(v) => v,
        InvocationReceiptRequestV8::SourceLoad(_) => return ("lifecycle", None),
    };
    let q = match q {
        InvocationReceiptRequestV7::Existing(v) => v,
        InvocationReceiptRequestV7::SourceRebind(_) => return ("lifecycle", None),
    };
    let q = match q {
        InvocationReceiptRequestV6::Existing(v) => v,
        InvocationReceiptRequestV6::Scratch(_) => return ("lifecycle", None),
    };
    let q = match q {
        InvocationReceiptRequestV5::Existing(v) => v,
        InvocationReceiptRequestV5::LiveRelation(_) => return ("relation", None),
    };
    let q = match q {
        InvocationReceiptRequestV4::Existing(v) => v,
        InvocationReceiptRequestV4::AtomicAnnotation { index, .. } => {
            return ("atomic-annotation", Some(index));
        }
    };
    let q = match q {
        InvocationReceiptRequestV3::Existing(v) => v,
        InvocationReceiptRequestV3::RetainedRelease(_) => return ("lifecycle", None),
    };
    let q = match q {
        InvocationReceiptRequestV2::Existing(v) => v,
        InvocationReceiptRequestV2::RetainedDisposal(_) => return ("lifecycle", None),
    };
    match q {
        InvocationReceiptRequestV1::Annotation(_) => ("annotation", None),
        InvocationReceiptRequestV1::AtomicAnnotation { index } => {
            ("atomic-annotation", Some(index))
        }
        InvocationReceiptRequestV1::Reversal(_) => ("reversal", None),
        InvocationReceiptRequestV1::Relation(_) => ("relation", None),
        InvocationReceiptRequestV1::Lifecycle => ("lifecycle", None),
        InvocationReceiptRequestV1::HostRegistration(_) => ("host-registration", None),
        InvocationReceiptRequestV1::ScopedRegistration(_) => ("scoped-registration", None),
        InvocationReceiptRequestV1::CreatedSettings(_) => ("created-settings", None),
    }
}
fn validate(
    envelope: &V<'_, '_>,
    observer: &InvocationObservationContextV1,
    native: InvocationReceiptRequestV10<'_>,
) -> Result<(), InvocationError> {
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
    }) || !text(get(h, "variant")?, "retained-outcome")
        || !matches!(get(h, "version")?, V::Data(D::Number(1.0)))
    {
        return Err(InvocationError::Unavailable);
    }
    let a = fields(
        &get(h, "arguments")?,
        &[
            "contract",
            "operation",
            "invocation",
            "context",
            "profile",
            "facet",
        ],
    )?;
    if !text(get(a, "contract")?, "execution-receipt/1")
        || !text(get(a, "operation")?, "retained-outcome")
        || !matches!(get(a,"invocation")?, V::Invocation(id) if *id == observer.invocation)
        || !matches!(get(a,"context")?, V::Context(id) if id.matches_context(observer))
        || !text(get(a, "profile")?, &observer.context.profile)
    {
        return Err(InvocationError::Unavailable);
    }
    let (kind, index) = selector(native);
    let facet = get(a, "facet")?;
    let facet = fields(
        &facet,
        if index.is_some() {
            &["kind", "index"]
        } else {
            &["kind"]
        },
    )?;
    if !text(get(facet, "kind")?, kind) {
        return Err(InvocationError::Unavailable);
    }
    if let Some(index) = index
        && !index_matches(get(facet, "index")?, index)
    {
        return Err(InvocationError::Unavailable);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
impl SourceCoordinator {
    pub(in crate::coordinator) fn recognize_native_invocation_receipt_request_v10(
        &self,
        observer: &InvocationObservationContextV1,
        envelope: &InvocationReceiptEnvelopeValueV1<'_, '_>,
        native: InvocationReceiptRequestV10<'_>,
    ) -> Result<(), InvocationError> {
        validate(envelope, observer, native)
    }
    /// One native retained-result release after closed grammar and original-role
    /// validation. The trusted native request and current authority are separate
    /// inputs, never derived from recipient data. No new admission or execution.
    /// Fixed borrowed frames require no retained decoder allocation; this is not
    /// the reference model's copied/owned decoder or a wire deserializer.
    pub fn query_decoded_invocation_receipt_v10<'s>(
        &'s self,
        observer: &mut InvocationObservationContextV1,
        envelope: &InvocationReceiptEnvelopeValueV1<'_, '_>,
        native: InvocationReceiptRequestV10<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV10<'s>, InvocationError> {
        let timing = r.begin_service_timing();
        let result = self
            .recognize_native_invocation_receipt_request_v10(observer, envelope, native)
            .and_then(|()| self.query_invocation_receipt_v10(observer, native, current, facts, r));
        capture_timed(
            r,
            Boundary::InvocationReceiptIngress,
            &result,
            timing.finish(),
        );
        result
    }
}
