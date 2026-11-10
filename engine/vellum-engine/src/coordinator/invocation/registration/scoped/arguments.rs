//! Passive already-decoded native data. No JSON parser, wire-token codec, getters
//! or authority acquisition. Basis references stay in the separate native array.
use super::*;
use crate::capture::{
    CaptureErrorV1, CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1,
    InertValueV1,
};
#[cfg(test)]
mod tests;
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "scoped-registration-invocation/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "activate-context",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedRegistrationVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "RegistrationExecutionProfile"),
            ("request", "ScopedResourceCatalogRequest"),
        ],
    },
};
#[derive(Clone, Copy)]
pub enum ScopedRegistrationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ScopedRegistrationEnvelopeValueV1<'a, 'h>)]),
    Data(InertValueV1<'a>),
    Invocation(&'h InvocationId),
}
impl std::fmt::Debug for ScopedRegistrationEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScopedRegistrationEnvelope(<redacted>)")
    }
}
pub struct NormalizedScopedRegistrationArgumentsV1<'h> {
    invocation: &'h InvocationId,
    profile: ResourceCatalogLimitsV1,
    basis_count: usize,
    changes: Vec<ScopedResourceRegistrationChangeV1>,
    usage: CaptureUsageV1,
    owned_bytes: usize,
    peak_bytes: usize,
}
impl std::fmt::Debug for NormalizedScopedRegistrationArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedScopedRegistration(<redacted>)")
    }
}
impl<'h> NormalizedScopedRegistrationArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn changes(&self) -> &[ScopedResourceRegistrationChangeV1] {
        &self.changes
    }
    pub fn native_profile(&self) -> ResourceCatalogLimitsV1 {
        self.profile
    }
    pub fn basis_count(&self) -> usize {
        self.basis_count
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    /// Native logical output header/rows/string payload, not allocator/RSS bytes.
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
    /// Maximum of inert traversal and simultaneous inert/typed value envelopes.
    /// Excludes caller ingress, program stack and global caller-held sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak_bytes
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
const COMMON: &[&str] = &["identityDomain", "identityKey", "identityKnown", "locator"];
const STRICT: &[&str] = &[
    "identityDomain",
    "identityKey",
    "identityKnown",
    "locator",
    "profile",
    "atomicRead",
    "insertionLineBreak",
];
const ANNOTATIONS: &[&str] = &[
    "identityDomain",
    "identityKey",
    "identityKnown",
    "locator",
    "profile",
    "atomicRead",
];
const WRITE: &[&str] = &[
    "identityDomain",
    "identityKey",
    "identityKnown",
    "locator",
    "procedure",
    "logicalContinuity",
    "syncSupported",
    "conditionalWrite",
    "causalReceipts",
    "nonReuseTokens",
    "exclusiveCreate",
];
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or(InvocationError::Limit)
}
fn capacity(_: std::collections::TryReserveError) -> InvocationError {
    InvocationError::Capacity
}
pub(in crate::coordinator::invocation::registration) fn copied(s: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len()).map_err(capacity)?;
    out.push_str(s);
    Ok(out)
}
pub(in crate::coordinator::invocation::registration) fn capture_error(
    e: CaptureErrorV1,
) -> InvocationError {
    match e {
        CaptureErrorV1::Limit => InvocationError::Limit,
        CaptureErrorV1::Capacity => InvocationError::Capacity,
        CaptureErrorV1::NonfiniteNumber
        | CaptureErrorV1::DuplicateField
        | CaptureErrorV1::Cycle => InvocationError::MalformedIntent,
    }
}
pub(in crate::coordinator::invocation::registration) fn outer_field<'a, 'h>(
    rows: &'a [(&str, ScopedRegistrationEnvelopeValueV1<'a, 'h>)],
    key: &str,
) -> Result<&'a ScopedRegistrationEnvelopeValueV1<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
        .ok_or(InvocationError::MalformedIntent)
}
pub(in crate::coordinator::invocation::registration) fn constant(
    v: &ScopedRegistrationEnvelopeValueV1<'_, '_>,
    expected: &str,
) -> Result<()> {
    if matches!(v, ScopedRegistrationEnvelopeValueV1::Data(InertValueV1::Text(s)) if *s == expected)
    {
        Ok(())
    } else {
        bad()
    }
}
fn object(v: &C) -> Result<&[(String, usize)]> {
    if let C::Object(rows) = v {
        Ok(rows)
    } else {
        bad()
    }
}
pub(in crate::coordinator::invocation::registration) fn exact(v: &C, keys: &[&str]) -> Result<()> {
    let rows = object(v)?;
    if rows.len() == keys.len() && rows.iter().all(|(k, _)| keys.contains(&k.as_str())) {
        Ok(())
    } else {
        bad()
    }
}
pub(in crate::coordinator::invocation::registration) fn field<'a>(
    capture: &'a InertCaptureV1,
    v: &'a C,
    key: &str,
) -> Result<&'a C> {
    let index = object(v)?
        .iter()
        .find(|(k, _)| k == key)
        .ok_or(InvocationError::MalformedIntent)?
        .1;
    capture.node(index).ok_or(InvocationError::Faulted)
}
pub(in crate::coordinator::invocation::registration) fn text(v: &C) -> Result<&str> {
    if let C::Text(s) = v { Ok(s) } else { bad() }
}
pub(in crate::coordinator::invocation::registration) fn label(
    v: &C,
    l: ResourceCatalogLimitsV1,
) -> Result<&str> {
    let s = text(v)?;
    if !s.is_empty() && s.len() <= l.max_tag_bytes {
        Ok(s)
    } else {
        bad()
    }
}
fn boolean(v: &C) -> Result<bool> {
    if let C::Boolean(b) = v { Ok(*b) } else { bad() }
}
fn index(v: &C, count: usize) -> Result<usize> {
    let C::Number(n) = v else {
        return bad();
    };
    if !n.is_finite() || *n < 0.0 || n.fract() != 0.0 || *n > 9_007_199_254_740_991.0 {
        return bad();
    }
    let n = usize::try_from(*n as u64).map_err(|_| InvocationError::MalformedIntent)?;
    if n < count { Ok(n) } else { bad() }
}
enum Cap<'a> {
    Strict(bool, CatalogInsertionLineBreakV1),
    Annotation(bool),
    Write {
        procedure: &'a str,
        flags: [bool; 6],
    },
}
pub(in crate::coordinator::invocation::registration) struct Config<'a> {
    domain: &'a str,
    key: &'a str,
    known: bool,
    locator: &'a str,
    cap: Cap<'a>,
}
impl Config<'_> {
    pub(in crate::coordinator::invocation::registration) fn valid_for(
        &self,
        table: ResourceCatalogTableV1,
    ) -> bool {
        matches!(
            (table, &self.cap),
            (
                ResourceCatalogTableV1::Read,
                Cap::Strict(..) | Cap::Annotation(_)
            ) | (ResourceCatalogTableV1::Write, Cap::Write { .. })
        )
    }
    pub(in crate::coordinator::invocation::registration) fn payload(&self) -> Result<usize> {
        let n = [&self.domain, &self.key, &self.locator]
            .into_iter()
            .try_fold(0, |n, s| add(n, s.len()))?;
        if let Cap::Write { procedure, .. } = self.cap {
            add(n, procedure.len())
        } else {
            Ok(n)
        }
    }
    pub(in crate::coordinator::invocation::registration) fn copy(
        &self,
    ) -> Result<ResourceCatalogConfigurationV1> {
        let capabilities = match self.cap {
            Cap::Strict(atomic_read, insertion_line_break) => {
                ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
                    atomic_read,
                    insertion_line_break,
                }
            }
            Cap::Annotation(atomic_read) => {
                ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read }
            }
            Cap::Write {
                procedure,
                flags:
                    [
                        logical_continuity,
                        synchronization_supported,
                        conditional_write,
                        causal_receipts,
                        non_reuse_tokens,
                        exclusive_create,
                    ],
            } => ResourceCatalogCapabilitiesV1::Write {
                procedure: copied(procedure)?,
                logical_continuity,
                synchronization_supported,
                conditional_write,
                causal_receipts,
                non_reuse_tokens,
                exclusive_create,
            },
        };
        Ok(ResourceCatalogConfigurationV1 {
            identity_domain: copied(self.domain)?,
            identity_key: copied(self.key)?,
            identity_known: self.known,
            locator: copied(self.locator)?,
            capabilities,
        })
    }
}
pub(in crate::coordinator::invocation::registration) fn config<'a>(
    c: &'a InertCaptureV1,
    v: &'a C,
    l: ResourceCatalogLimitsV1,
) -> Result<Config<'a>> {
    let rows = object(v)?;
    let cap = if rows.iter().any(|(k, _)| k == "profile") {
        match text(field(c, v, "profile")?)? {
            "strict-utf8-one-bom/1" => {
                exact(v, STRICT)?;
                let line_break = match text(field(c, v, "insertionLineBreak")?)? {
                    "LF" => CatalogInsertionLineBreakV1::Lf,
                    "CRLF" => CatalogInsertionLineBreakV1::CrLf,
                    "CR" => CatalogInsertionLineBreakV1::Cr,
                    _ => return bad(),
                };
                Cap::Strict(boolean(field(c, v, "atomicRead")?)?, line_break)
            }
            "vellum.annotations/1" => {
                exact(v, ANNOTATIONS)?;
                Cap::Annotation(boolean(field(c, v, "atomicRead")?)?)
            }
            _ => return bad(),
        }
    } else {
        exact(v, WRITE)?;
        let flags = [
            "logicalContinuity",
            "syncSupported",
            "conditionalWrite",
            "causalReceipts",
            "nonReuseTokens",
            "exclusiveCreate",
        ];
        let mut values = [false; 6];
        for (out, name) in values.iter_mut().zip(flags) {
            *out = boolean(field(c, v, name)?)?;
        }
        Cap::Write {
            procedure: label(field(c, v, "procedure")?, l)?,
            flags: values,
        }
    };
    Ok(Config {
        domain: label(field(c, v, COMMON[0])?, l)?,
        key: label(field(c, v, COMMON[1])?, l)?,
        known: boolean(field(c, v, COMMON[2])?)?,
        locator: label(field(c, v, COMMON[3])?, l)?,
        cap,
    })
}
struct Change<'a> {
    basis: usize,
    config: Option<Config<'a>>,
}
fn change<'a>(
    c: &'a InertCaptureV1,
    v: &'a C,
    count: usize,
    l: ResourceCatalogLimitsV1,
) -> Result<Change<'a>> {
    let action = text(field(c, v, "action")?)?;
    let config = match action {
        "remove" => {
            exact(v, &["basis", "action"])?;
            None
        }
        "install" => {
            exact(v, &["basis", "action", "configuration"])?;
            Some(config(c, field(c, v, "configuration")?, l)?)
        }
        _ => return bad(),
    };
    Ok(Change {
        basis: index(field(c, v, "basis")?, count)?,
        config,
    })
}
/// Pure schema normalization. Caller-selected native logical bounds are not a
/// reservation, grant or promise of admission. Original roles are borrowed only.
pub fn normalize_scoped_registration_envelope_v1<'a, 'h>(
    envelope: &'a ScopedRegistrationEnvelopeValueV1<'a, 'h>,
    basis_count: usize,
    profile: ResourceCatalogLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedRegistrationArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope, basis_count, profile, limits, maximum, r);
    capture_timed(
        r,
        Boundary::ScopedRegistrationArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
fn normalize<'a, 'h>(
    envelope: &'a ScopedRegistrationEnvelopeValueV1<'a, 'h>,
    basis_count: usize,
    profile: ResourceCatalogLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedScopedRegistrationArgumentsV1<'h>> {
    let ScopedRegistrationEnvelopeValueV1::Object(rows) = envelope else {
        return bad();
    };
    let declaration = NormalizedScopedRegistrationArgumentsV1::ARGUMENT_DECLARATION;
    if !declaration.fields_match(rows.iter().map(|(key, _)| *key)) {
        return bad();
    }
    constant(outer_field(rows, "contract")?, declaration.contract)?;
    constant(
        outer_field(rows, "operation")?,
        declaration.variant.operation,
    )?;
    constant(
        outer_field(rows, "profile")?,
        SCOPED_RESOURCE_REGISTRATION_PROFILE_V1,
    )?;
    let ScopedRegistrationEnvelopeValueV1::Invocation(invocation) =
        outer_field(rows, "invocation")?
    else {
        return bad();
    };
    let ScopedRegistrationEnvelopeValueV1::Data(request) = outer_field(rows, "request")? else {
        return bad();
    };
    if basis_count > profile.max_relations {
        return bad();
    }
    let c = InertCaptureV1::copy(
        request,
        CaptureLimitsV1 {
            max_bytes: limits.max_bytes.min(maximum),
            ..limits
        },
        r,
    )
    .map_err(capture_error)?;
    exact(c.root(), &["changes"])?;
    let C::Array(entries) = field(&c, c.root(), "changes")? else {
        return bad();
    };
    if entries.len() != basis_count {
        return bad();
    }
    let mut bytes = add(
        size_of::<NormalizedScopedRegistrationArgumentsV1<'_>>(),
        entries
            .len()
            .checked_mul(size_of::<ScopedResourceRegistrationChangeV1>())
            .ok_or(InvocationError::Limit)?,
    )?;
    for (i, node) in entries.iter().enumerate() {
        let v = change(
            &c,
            c.node(*node).ok_or(InvocationError::Faulted)?,
            basis_count,
            profile,
        )?;
        for old in &entries[..i] {
            if change(
                &c,
                c.node(*old).ok_or(InvocationError::Faulted)?,
                basis_count,
                profile,
            )?
            .basis
                == v.basis
            {
                return bad();
            }
        }
        if let Some(config) = v.config {
            bytes = add(bytes, config.payload()?)?;
        }
    }
    let peak_bytes = add(c.usage().retained_bytes, bytes)?.max(c.usage().peak_bytes);
    if peak_bytes > maximum {
        return Err(InvocationError::Limit);
    }
    let mut changes = Vec::new();
    changes.try_reserve_exact(entries.len()).map_err(capacity)?;
    for node in entries {
        let v = change(
            &c,
            c.node(*node).ok_or(InvocationError::Faulted)?,
            basis_count,
            profile,
        )?;
        changes.push(ScopedResourceRegistrationChangeV1 {
            basis: v.basis,
            action: match v.config {
                Some(config) => ResourceCatalogActionV1::Install(config.copy()?),
                None => ResourceCatalogActionV1::Remove,
            },
        });
    }
    Ok(NormalizedScopedRegistrationArgumentsV1 {
        invocation,
        profile,
        basis_count,
        changes,
        usage: c.usage(),
        owned_bytes: bytes,
        peak_bytes,
    })
}
impl SourceCoordinator {
    pub fn submit_normalized_scoped_registration_v1(
        &mut self,
        q: &NormalizedScopedRegistrationArgumentsV1<'_>,
        bases: &[ResourceRegistrationBasisIdV1],
        c: &SettingsExecutionContextV1,
        f: &TrustedScopedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.profile != f.basis.native_profile {
            Err(InvocationError::Unavailable)
        } else if q.basis_count != bases.len() {
            Err(InvocationError::MalformedIntent)
        } else {
            self.submit_scoped_resource_registration_v1(q.invocation, &q.changes, bases, c, f, r)
        };
        capture_timed(
            r,
            Boundary::ScopedRegistrationDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
