//! Passive exact native registration schema; no parser, authority or provider I/O.
use super::scoped::arguments as shared;
use super::*;
#[cfg(test)]
mod tests;
use crate::capture::{CaptureLimitsV1, CaptureUsageV1, CapturedValueV1 as C, InertCaptureV1};
use shared::{capture_error, constant, copied, exact, field, label, outer_field, text};
pub type ResourceRegistrationEnvelopeValueV1<'a, 'h> = ScopedRegistrationEnvelopeValueV1<'a, 'h>;
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "registration-invocation/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "activate-context",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedRegistrationVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "RegistrationExecutionProfile"),
            ("request", "NativeResourceCatalogRequest"),
        ],
    },
};
pub struct NormalizedResourceRegistrationArgumentsV1<'h> {
    invocation: &'h InvocationId,
    profile: ResourceCatalogLimitsV1,
    changes: Vec<ResourceCatalogChangeV1>,
    usage: CaptureUsageV1,
    owned_bytes: usize,
    peak_bytes: usize,
}
impl std::fmt::Debug for NormalizedResourceRegistrationArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedResourceRegistration(<redacted>)")
    }
}
impl<'h> NormalizedResourceRegistrationArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn native_profile(&self) -> ResourceCatalogLimitsV1 {
        self.profile
    }
    pub fn changes(&self) -> &[ResourceCatalogChangeV1] {
        &self.changes
    }
    pub fn capture_usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned_bytes
    }
    /// Joint inert/typed logical peak, not wire/allocator/RSS/global sponsorship.
    pub fn normalization_peak_bytes(&self) -> usize {
        self.peak_bytes
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or(InvocationError::Limit)
}
fn natural(v: &C) -> Result<u64> {
    let C::Number(n) = v else {
        return bad();
    };
    if !n.is_finite() || *n < 0.0 || n.fract() != 0.0 || *n > 9_007_199_254_740_991.0 {
        return bad();
    }
    Ok(*n as u64)
}
#[derive(PartialEq, Eq)]
struct Identity<'a> {
    authority: &'a str,
    id: &'a str,
    incarnation: u64,
}
impl Identity<'_> {
    fn payload(&self) -> Result<usize> {
        add(self.authority.len(), self.id.len())
    }
    fn copy(&self) -> Result<ResourceCatalogIdentityV1> {
        Ok(ResourceCatalogIdentityV1 {
            authority: copied(self.authority)?,
            id: copied(self.id)?,
            incarnation: self.incarnation,
        })
    }
}
fn identity<'a>(
    c: &'a InertCaptureV1,
    v: &'a C,
    p: ResourceCatalogLimitsV1,
) -> Result<Identity<'a>> {
    exact(v, &["authority", "id", "incarnation"])?;
    Ok(Identity {
        authority: label(field(c, v, "authority")?, p)?,
        id: label(field(c, v, "id")?, p)?,
        incarnation: natural(field(c, v, "incarnation")?)?,
    })
}
struct Change<'a> {
    table: ResourceCatalogTableV1,
    provider: Identity<'a>,
    resource: Identity<'a>,
    expected: CatalogExpectationV1,
    config: Option<shared::Config<'a>>,
}
impl Change<'_> {
    fn same_slot(&self, b: &Self) -> bool {
        self.table == b.table && self.provider == b.provider && self.resource == b.resource
    }
    fn payload(&self) -> Result<usize> {
        add(
            add(self.provider.payload()?, self.resource.payload()?)?,
            self.config
                .as_ref()
                .map(|c| c.payload())
                .transpose()?
                .unwrap_or(0),
        )
    }
    fn copy(&self) -> Result<ResourceCatalogChangeV1> {
        Ok(ResourceCatalogChangeV1 {
            address: ResourceCatalogAddressV1 {
                table: self.table,
                provider: self.provider.copy()?,
                resource: self.resource.copy()?,
            },
            expected: self.expected,
            action: match &self.config {
                Some(c) => ResourceCatalogActionV1::Install(c.copy()?),
                None => ResourceCatalogActionV1::Remove,
            },
        })
    }
}
fn change<'a>(c: &'a InertCaptureV1, v: &'a C, p: ResourceCatalogLimitsV1) -> Result<Change<'a>> {
    let install = match text(field(c, v, "action")?)? {
        "install" => true,
        "remove" => false,
        _ => return bad(),
    };
    exact(
        v,
        if install {
            &[
                "table",
                "action",
                "provider",
                "resource",
                "expected",
                "configuration",
            ]
        } else {
            &["table", "action", "provider", "resource", "expected"]
        },
    )?;
    let table = match text(field(c, v, "table")?)? {
        "read" => ResourceCatalogTableV1::Read,
        "write" => ResourceCatalogTableV1::Write,
        _ => return bad(),
    };
    let e = field(c, v, "expected")?;
    let expected = match text(field(c, e, "state")?)? {
        "absent" => {
            exact(e, &["state"])?;
            CatalogExpectationV1::Absent
        }
        "present" => {
            exact(e, &["state", "generation"])?;
            CatalogExpectationV1::Present {
                generation: natural(field(c, e, "generation")?)?,
            }
        }
        _ => return bad(),
    };
    let config = if install {
        let cfg = shared::config(c, field(c, v, "configuration")?, p)?;
        if !cfg.valid_for(table) {
            return bad();
        }
        Some(cfg)
    } else {
        None
    };
    Ok(Change {
        table,
        provider: identity(c, field(c, v, "provider")?, p)?,
        resource: identity(c, field(c, v, "resource")?, p)?,
        expected,
        config,
    })
}
pub fn normalize_resource_registration_envelope_v1<'a, 'h>(
    envelope: &'a ResourceRegistrationEnvelopeValueV1<'a, 'h>,
    profile: ResourceCatalogLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedResourceRegistrationArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(envelope, profile, limits, maximum, r);
    capture_timed(
        r,
        Boundary::RegistrationArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
fn normalize<'a, 'h>(
    envelope: &'a ResourceRegistrationEnvelopeValueV1<'a, 'h>,
    profile: ResourceCatalogLimitsV1,
    limits: CaptureLimitsV1,
    maximum: usize,
    r: &mut Recorder,
) -> Result<NormalizedResourceRegistrationArgumentsV1<'h>> {
    use ResourceRegistrationEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return bad();
    };
    let declaration = NormalizedResourceRegistrationArgumentsV1::ARGUMENT_DECLARATION;
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
        NATIVE_RESOURCE_REGISTRATION_PROFILE_V1,
    )?;
    let V::Invocation(invocation) = outer_field(rows, "invocation")? else {
        return bad();
    };
    let V::Data(request) = outer_field(rows, "request")? else {
        return bad();
    };
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
    if entries.len() > profile.max_relations {
        return Err(InvocationError::Limit);
    }
    let node = |n| c.node(n).ok_or(InvocationError::Faulted);
    let mut bytes = add(
        size_of::<NormalizedResourceRegistrationArgumentsV1<'_>>(),
        entries
            .len()
            .checked_mul(size_of::<ResourceCatalogChangeV1>())
            .ok_or(InvocationError::Limit)?,
    )?;
    for (i, n) in entries.iter().enumerate() {
        let v = change(&c, node(*n)?, profile)?;
        for old in &entries[..i] {
            if v.same_slot(&change(&c, node(*old)?, profile)?) {
                return bad();
            }
        }
        bytes = add(bytes, v.payload()?)?;
    }
    let peak_bytes = add(c.usage().retained_bytes, bytes)?.max(c.usage().peak_bytes);
    if peak_bytes > maximum {
        return Err(InvocationError::Limit);
    }
    let mut changes = Vec::new();
    changes
        .try_reserve_exact(entries.len())
        .map_err(|_| InvocationError::Capacity)?;
    for n in entries {
        changes.push(change(&c, node(*n)?, profile)?.copy()?);
    }
    Ok(NormalizedResourceRegistrationArgumentsV1 {
        invocation,
        profile,
        changes,
        usage: c.usage(),
        owned_bytes: bytes,
        peak_bytes,
    })
}
impl SourceCoordinator {
    pub fn submit_normalized_resource_registration_v1(
        &mut self,
        q: &NormalizedResourceRegistrationArgumentsV1<'_>,
        c: &SettingsExecutionContextV1,
        f: &TrustedResourceRegistrationInvocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timer = r.begin_service_timing();
        let result = if q.profile != f.profile {
            Err(InvocationError::Unavailable)
        } else {
            self.submit_resource_registration_v1(q.invocation, &q.changes, c, f, r)
        };
        capture_timed(
            r,
            Boundary::RegistrationDecodedAdmission,
            &result,
            timer.finish(),
        );
        result
    }
}
