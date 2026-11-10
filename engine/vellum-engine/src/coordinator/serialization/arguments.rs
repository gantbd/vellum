//! Passive serialization envelopes. Opaque roles are never decoded from data.
use super::*;
#[cfg(test)]
mod tests;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1 as D,
};
static NATIVE_ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "source-serialization/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "semantic-settings",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSerializationVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("document", "LiveDocumentRef"),
            ("expectedSerialization", "SerializationGeneration"),
            ("serialization", "NativeSourceSerialization"),
            ("dependencies", "BoundedContextDependencyVector"),
        ],
    },
};
static SCOPED_ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "scoped-source-serialization/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "semantic-settings",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSerializationVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("serialization", "NativeSourceSerialization"),
            ("dependencies", "BoundedContextDependencyVector"),
        ],
    },
};
#[derive(Clone, Copy)]
pub enum SerializationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, Self)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Document(&'h DocumentId),
    Dependencies(&'h ReadDependencyRequestV3),
}
impl std::fmt::Debug for SerializationEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SerializationEnvelope(<redacted>)")
    }
}
struct Body<'h> {
    profile: SourceSerializationLimitsV1,
    execution: String,
    serialization: SourceSerializationV1,
    dependencies: &'h ReadDependencyRequestV3,
    usage: CaptureUsageV1,
    owned: usize,
    peak: usize,
}
pub struct NormalizedSourceSerializationArgumentsV1<'h> {
    invocation: &'h InvocationId,
    document: &'h DocumentId,
    expected: u64,
    body: Body<'h>,
}
pub struct NormalizedScopedSerializationArgumentsV1<'h> {
    invocation: &'h InvocationId,
    body: Body<'h>,
}
macro_rules! accessors {
    ($t:ident) => {
        impl<'h> $t<'h> {
            pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
                Self::ARGUMENT_DECLARATION
            }
            pub fn invocation(&self) -> &'h InvocationId {
                self.invocation
            }
            pub fn execution_profile(&self) -> &str {
                &self.body.execution
            }
            pub fn native_profile(&self) -> SourceSerializationLimitsV1 {
                self.body.profile
            }
            pub fn serialization(&self) -> SourceSerializationV1 {
                self.body.serialization
            }
            pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
                self.body.dependencies
            }
            pub fn capture_usage(&self) -> CaptureUsageV1 {
                self.body.usage
            }
            pub fn owned_bytes(&self) -> usize {
                self.body.owned
            }
            /// Inert plus typed logical envelope; excludes borrowed role storage,
            /// caller ingress, stack, allocator/RSS and global sponsorship.
            pub fn normalization_peak_bytes(&self) -> usize {
                self.body.peak
            }
        }
        impl std::fmt::Debug for $t<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($t), "(<redacted>)"))
            }
        }
    };
}
accessors!(NormalizedSourceSerializationArgumentsV1);
accessors!(NormalizedScopedSerializationArgumentsV1);
impl<'h> NormalizedSourceSerializationArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &NATIVE_ARGUMENTS;
    pub fn request(&self) -> SourceSerializationRequestV1<'h> {
        SourceSerializationRequestV1 {
            document: self.document,
            expected: self.expected,
            serialization: self.body.serialization,
            dependencies: self.body.dependencies,
        }
    }
}
impl NormalizedScopedSerializationArgumentsV1<'_> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &SCOPED_ARGUMENTS;
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn capacity(_: std::collections::TryReserveError) -> InvocationError {
    InvocationError::Capacity
}
fn field<'a, 'h>(
    rows: &'a [(&str, SerializationEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a SerializationEnvelopeValueV1<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(InvocationError::MalformedIntent)
}
fn data<'a>(v: &SerializationEnvelopeValueV1<'a, '_>) -> Result<D<'a>> {
    if let SerializationEnvelopeValueV1::Data(v) = v {
        Ok(*v)
    } else {
        bad()
    }
}
fn get<'a>(c: &'a InertCaptureV1, v: &C, key: &str) -> Result<&'a C> {
    let C::Object(rows) = v else { return bad() };
    let n = rows
        .iter()
        .find(|(k, _)| k == key)
        .ok_or(InvocationError::MalformedIntent)?
        .1;
    c.node(n).ok_or(InvocationError::Faulted)
}
fn exact(v: &C, keys: &[&str]) -> Result<()> {
    let C::Object(rows) = v else { return bad() };
    if rows.len() == keys.len() && rows.iter().all(|(k, _)| keys.contains(&k.as_str())) {
        Ok(())
    } else {
        bad()
    }
}
struct Parts<'h> {
    invocation: &'h InvocationId,
    target: Option<(&'h DocumentId, u64)>,
    body: Body<'h>,
}
fn normalize<'a, 'h>(
    envelope: &'a SerializationEnvelopeValueV1<'a, 'h>,
    scoped: bool,
    profile: SourceSerializationLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<Parts<'h>> {
    use SerializationEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    let declaration = if scoped {
        NormalizedScopedSerializationArgumentsV1::ARGUMENT_DECLARATION
    } else {
        NormalizedSourceSerializationArgumentsV1::ARGUMENT_DECLARATION
    };
    if !declaration.fields_match(rows.iter().map(|(key, _)| *key)) {
        return bad();
    }
    if !matches!(data(field(rows,"contract")?)?,D::Text(s) if s == declaration.contract)
        || !matches!(
            data(field(rows, "operation")?)?,
            D::Text(s) if s == declaration.variant.operation
        )
    {
        return bad();
    }
    let V::Invocation(invocation) = field(rows, "invocation")? else {
        return bad();
    };
    let V::Dependencies(dependencies) = field(rows, "dependencies")? else {
        return bad();
    };
    if dependencies.predicates.len() > profile.max_dependencies
        || dependencies.predicates.len() > dependencies.budget.max_predicates
    {
        return Err(InvocationError::Limit);
    }
    let document = if scoped {
        None
    } else {
        let V::Document(d) = field(rows, "document")? else {
            return bad();
        };
        Some(*d)
    };
    let inert = [
        ("profile", data(field(rows, "profile")?)?),
        ("serialization", data(field(rows, "serialization")?)?),
        (
            "expectedSerialization",
            if scoped {
                D::Null
            } else {
                data(field(rows, "expectedSerialization")?)?
            },
        ),
    ];
    let c = InertCaptureV1::copy(
        &D::Object(if scoped { &inert[..2] } else { &inert }),
        CaptureLimitsV1 {
            max_bytes: limits.max_bytes.min(maximum),
            ..limits
        },
        r,
    )
    .map_err(|e| match e {
        CaptureErrorV1::Limit => InvocationError::Limit,
        CaptureErrorV1::Capacity => InvocationError::Capacity,
        _ => InvocationError::MalformedIntent,
    })?;
    let C::Text(execution) = get(&c, c.root(), "profile")? else {
        return bad();
    };
    if execution.is_empty() || execution.len() > profile.max_label_bytes {
        return bad();
    }
    let s = get(&c, c.root(), "serialization")?;
    exact(s, &["profile", "bom"])?;
    if !matches!(get(&c,s,"profile")?,C::Text(s) if s=="strict-utf8-one-bom/1") {
        return bad();
    }
    let C::Boolean(bom) = get(&c, s, "bom")? else {
        return bad();
    };
    let target = if let Some(document) = document {
        let C::Number(n) = get(&c, c.root(), "expectedSerialization")? else {
            return bad();
        };
        if !n.is_finite() || *n <= 0.0 || n.fract() != 0.0 || *n > 9_007_199_254_740_991.0 {
            return bad();
        }
        Some((document, *n as u64))
    } else {
        None
    };
    let header = if scoped {
        std::mem::size_of::<NormalizedScopedSerializationArgumentsV1<'_>>()
    } else {
        std::mem::size_of::<NormalizedSourceSerializationArgumentsV1<'_>>()
    };
    let owned = header
        .checked_add(execution.len())
        .ok_or(InvocationError::Limit)?;
    let peak = c
        .usage()
        .retained_bytes
        .checked_add(owned)
        .ok_or(InvocationError::Limit)?
        .max(c.usage().peak_bytes);
    if peak > maximum {
        return Err(InvocationError::Limit);
    }
    let mut execution_copy = String::new();
    execution_copy
        .try_reserve_exact(execution.len())
        .map_err(capacity)?;
    execution_copy.push_str(execution);
    Ok(Parts {
        invocation,
        target,
        body: Body {
            profile,
            execution: execution_copy,
            serialization: SourceSerializationV1 {
                profile: SourceSerializationProfileV1::StrictUtf8OneBom,
                bom: *bom,
            },
            dependencies,
            usage: c.usage(),
            owned,
            peak,
        },
    })
}
pub fn normalize_source_serialization_envelope_v1<'a, 'h>(
    envelope: &'a SerializationEnvelopeValueV1<'a, 'h>,
    profile: SourceSerializationLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedSourceSerializationArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope, false, profile, limits, maximum, r).and_then(|p| {
        let (document, expected) = p.target.ok_or(InvocationError::Faulted)?;
        Ok(NormalizedSourceSerializationArgumentsV1 {
            invocation: p.invocation,
            document,
            expected,
            body: p.body,
        })
    });
    record(
        r,
        Boundary::SerializationArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
pub fn normalize_scoped_serialization_envelope_v1<'a, 'h>(
    envelope: &'a SerializationEnvelopeValueV1<'a, 'h>,
    profile: SourceSerializationLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedSerializationArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope, true, profile, limits, maximum, r).map(|p| {
        NormalizedScopedSerializationArgumentsV1 {
            invocation: p.invocation,
            body: p.body,
        }
    });
    record(
        r,
        Boundary::SerializationArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
fn record<T>(
    r: &mut Recorder,
    boundary: Boundary,
    out: &Result<T>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match out {
            Ok(_) => Disposition::Completed,
            Err(InvocationError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
impl SourceCoordinator {
    pub fn submit_normalized_source_serialization_v1(
        &mut self,
        q: &NormalizedSourceSerializationArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.body.profile != f.profile || q.body.execution != c.profile {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_source_serialization(q.invocation, &q.request(), c, f, r)
        };
        record(
            r,
            Boundary::SerializationDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn submit_normalized_scoped_serialization_v1(
        &mut self,
        q: &NormalizedScopedSerializationArgumentsV1<'_>,
        basis: &SerializationBasisId,
        c: &SettingsExecutionContextV1,
        f: &TrustedSerializationBasisFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.body.profile != f.native.profile || q.body.execution != c.profile {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_scoped_source_serialization(
                q.invocation,
                &ScopedSourceSerializationRequestV1 {
                    basis,
                    serialization: q.body.serialization,
                    dependencies: q.body.dependencies,
                },
                c,
                f,
                r,
            )
        };
        record(
            r,
            Boundary::SerializationDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
