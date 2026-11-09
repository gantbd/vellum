//! Passive configured-line binding, never a settings resolver or UI policy.
use super::*;
use crate::capture::InertValueV1 as D;
use crate::coordinator::invocation::literal::arguments::bind_header;
use std::mem::size_of;
pub const CONFIGURED_PREFIX_LINE_PROFILE_V1: &str = "configured-prefix-line/1";
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "configured-line-edit/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "ordinary-edit",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLineVariant"),
            ("action", "ClosedLineAction"),
            ("invocation", "InvocationIdentity"),
            ("document", "DocumentIdentity"),
            ("view", "LogicalViewIdentity"),
            ("sourceRevision", "SafeNatural"),
            ("expectedSelection", "SafeNatural"),
            ("profile", "ExecutionProfileRef"),
            ("configuration", "NativeTargetAndClosedKeyRoles"),
        ],
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfiguredLineArgumentLimitsV1 {
    pub max_key_bytes: usize,
    /// Normalized header plus two owned key names. Excludes borrowed role
    /// referents, caller input, allocator/RSS and later native interpretation.
    pub max_owned_bytes: usize,
}
#[derive(Clone, Copy)]
pub enum ConfiguredLineEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ConfiguredLineEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    View(&'h LogicalViewId),
    Document(&'h DocumentId),
    /// Native configuration object: target is an actual opaque token, never
    /// resolved from ordinary labels. Keys are still bounded inert data.
    Configuration {
        target: &'h SettingsTargetId,
        keys: D<'a>,
    },
}
impl std::fmt::Debug for ConfiguredLineEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConfiguredLineEnvelope(<redacted>)")
    }
}
pub struct NormalizedConfiguredLineArgumentsV1<'h> {
    invocation: &'h InvocationId,
    context: &'h LogicalViewContext,
    request: ConfiguredLineEditV1,
}
impl std::fmt::Debug for NormalizedConfiguredLineArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedConfiguredLineArguments(<redacted>)")
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
type V<'a, 'h> = ConfiguredLineEnvelopeValueV1<'a, 'h>;
impl<'h> NormalizedConfiguredLineArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn context(&self) -> &'h LogicalViewContext {
        self.context
    }
    pub fn request(&self) -> &ConfiguredLineEditV1 {
        &self.request
    }
    pub fn required_owned_bytes(first: usize, second: usize) -> Result<usize> {
        size_of::<Self>()
            .checked_add(first)
            .and_then(|n| n.checked_add(second))
            .ok_or(InvocationError::Limit)
    }
    pub fn owned_bytes(&self) -> usize {
        let (_, a, b) = self.request.parts();
        size_of::<Self>() + a.len() + b.len()
    }
}
fn field<'a, 'h>(rows: &[(&str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
        .ok_or(InvocationError::MalformedIntent)
}
fn key<'a>(rows: &[(&str, D<'a>)], name: &str) -> Result<&'a str> {
    match rows
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
    {
        Some(D::Text(value)) if !value.is_empty() => Ok(value),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn copy(value: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(value.len())
        .map_err(|_| InvocationError::Capacity)?;
    out.push_str(value);
    Ok(out)
}
fn normalize<'a, 'h>(
    envelope: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: ConfiguredLineArgumentLimitsV1,
) -> Result<NormalizedConfiguredLineArgumentsV1<'h>> {
    let V::Object(rows) = envelope else {
        return Err(InvocationError::MalformedIntent);
    };
    if rows.len() != 10 {
        return Err(InvocationError::MalformedIntent);
    }
    // Fixed, private projection of common roles only. Specialized values are
    // not interpreted by this header guard; configuration is checked below.
    let header: [(&str, LiteralEditEnvelopeValueV1<'_, '_>); 10] = std::array::from_fn(|i| {
        use LiteralEditEnvelopeValueV1 as H;
        (
            rows[i].0,
            match rows[i].1 {
                V::Data(v) => H::Data(v),
                V::Invocation(v) => H::Invocation(v),
                V::View(v) => H::View(v),
                V::Document(v) => H::Document(v),
                _ => H::Data(D::Null),
            },
        )
    });
    let invocation = bind_header(
        &header,
        context,
        NormalizedConfiguredLineArgumentsV1::ARGUMENT_DECLARATION,
        CONFIGURED_PREFIX_LINE_PROFILE_V1,
    )?;
    let V::Data(D::Text(action)) = field(rows, "action")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let names = match action {
        "indent-lines" | "outdent-lines" => ["unit", "tabWidth"],
        "insert-newline" => ["delimiter", "autoIndent"],
        _ => return Err(InvocationError::MalformedIntent),
    };
    let V::Configuration {
        target,
        keys: D::Object(keys),
    } = field(rows, "configuration")?
    else {
        return Err(InvocationError::MalformedIntent);
    };
    if keys.len() != 2
        || names
            .iter()
            .any(|name| keys.iter().filter(|(key, _)| key == name).count() != 1)
    {
        return Err(InvocationError::MalformedIntent);
    }
    let a = key(keys, names[0])?;
    let b = key(keys, names[1])?;
    if a.len() > limits.max_key_bytes
        || b.len() > limits.max_key_bytes
        || NormalizedConfiguredLineArgumentsV1::required_owned_bytes(a.len(), b.len())?
            > limits.max_owned_bytes
    {
        return Err(InvocationError::Limit);
    }
    let a = copy(a)?;
    let b = copy(b)?;
    let target = target.clone();
    let request = match action {
        "indent-lines" => ConfiguredLineEditV1::Indent {
            target,
            unit_key: a,
            tab_width_key: b,
        },
        "outdent-lines" => ConfiguredLineEditV1::Outdent {
            target,
            unit_key: a,
            tab_width_key: b,
        },
        "insert-newline" => ConfiguredLineEditV1::Newline {
            target,
            delimiter_key: a,
            auto_indent_key: b,
        },
        _ => return Err(InvocationError::MalformedIntent),
    };
    Ok(NormalizedConfiguredLineArgumentsV1 {
        invocation,
        context,
        request,
    })
}
/// Passive native configuration binding, not a public identity encoding or
/// settings snapshot/lease. Actual target/domain/currentness is checked by the
/// original configured-line preparation and service under the privileged owner.
pub fn normalize_configured_line_arguments_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: ConfiguredLineArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedConfiguredLineArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, context, limits);
    capture_timed(
        r,
        Boundary::ConfiguredLineArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    pub fn submit_configured_line_arguments_v1(
        &mut self,
        q: &NormalizedConfiguredLineArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result = self.submit_configured_line_edit(q.invocation, q.context, &q.request, r);
        capture_timed(
            r,
            Boundary::ConfiguredLineDecodedAdmission,
            &result,
            timing.finish(),
        );
        result
    }
}
