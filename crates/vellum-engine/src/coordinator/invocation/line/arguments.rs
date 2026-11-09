//! Closed prefix/newline data over original borrowed native view roles.
use super::*;
use crate::capture::InertValueV1 as D;
use crate::coordinator::invocation::literal::arguments::{bind_header, field, natural, string};
use std::mem::size_of;
pub type LineEditEnvelopeValueV1<'a, 'h> = LiteralEditEnvelopeValueV1<'a, 'h>;
pub const EXPLICIT_PREFIX_LINE_EDIT_PROFILE_V1: &str = "explicit-prefix-line-edit/1";
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "line-edit/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "ordinary-edit",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLineVariant"),
            ("action", "ClosedLineAction"),
            ("invocation", "InvocationIdentity"),
            ("view", "LogicalViewIdentity"),
            ("document", "DocumentIdentity"),
            ("sourceRevision", "SafeNatural"),
            ("expectedSelection", "SafeNatural"),
            ("profile", "ExecutionProfileRef"),
            ("settings", "ClosedExplicitLineSettings"),
        ],
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEditArgumentLimitsV1 {
    pub max_unit_bytes: usize,
    /// Normalized header plus owned unit bytes, not borrowed native referents,
    /// caller input, allocator/RSS or later native job/preparation storage.
    pub max_owned_bytes: usize,
}
pub struct NormalizedLineEditArgumentsV1<'h> {
    invocation: &'h InvocationId,
    context: &'h LogicalViewContext,
    request: LineEditV1,
}
impl std::fmt::Debug for NormalizedLineEditArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedLineEditArguments(<redacted>)")
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
impl<'h> NormalizedLineEditArgumentsV1<'h> {
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
    pub fn request(&self) -> &LineEditV1 {
        &self.request
    }
    pub fn required_owned_bytes(unit_bytes: usize) -> Result<usize> {
        size_of::<Self>()
            .checked_add(unit_bytes)
            .ok_or(InvocationError::Limit)
    }
    pub fn owned_bytes(&self) -> usize {
        size_of::<Self>() + self.request.unit_bytes()
    }
}
fn object<'a>(value: D<'a>, keys: &[&str]) -> Result<&'a [(&'a str, D<'a>)]> {
    let D::Object(rows) = value else {
        return Err(InvocationError::MalformedIntent);
    };
    if rows.len() != keys.len()
        || keys
            .iter()
            .any(|key| rows.iter().filter(|(name, _)| name == key).count() != 1)
    {
        return Err(InvocationError::MalformedIntent);
    }
    Ok(rows)
}
fn data_field<'a>(rows: &'a [(&str, D<'a>)], key: &str) -> Result<D<'a>> {
    rows.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
        .ok_or(InvocationError::MalformedIntent)
}
fn text(value: D<'_>) -> Result<&str> {
    match value {
        D::Text(v) => Ok(v),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn normalize<'a, 'h>(
    envelope: LineEditEnvelopeValueV1<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: LineEditArgumentLimitsV1,
) -> Result<NormalizedLineEditArgumentsV1<'h>> {
    use LineEditEnvelopeValueV1 as V;
    let V::Object(rows) = envelope else {
        return Err(InvocationError::MalformedIntent);
    };
    let invocation = bind_header(
        rows,
        context,
        NormalizedLineEditArgumentsV1::ARGUMENT_DECLARATION,
        EXPLICIT_PREFIX_LINE_EDIT_PROFILE_V1,
    )?;
    let action = string(field(rows, "action")?)?;
    let V::Data(settings) = field(rows, "settings")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let request = match action {
        "indent-lines" | "outdent-lines" => {
            let settings = object(settings, &["unit", "tabWidth"])?;
            let unit = text(data_field(settings, "unit")?)?;
            if unit.len() > limits.max_unit_bytes
                || NormalizedLineEditArgumentsV1::required_owned_bytes(unit.len())?
                    > limits.max_owned_bytes
            {
                return Err(InvocationError::Limit);
            }
            if unit.is_empty() || (unit != "\t" && !unit.bytes().all(|v| v == b' ')) {
                return Err(InvocationError::MalformedIntent);
            }
            let tab_width = usize::try_from(natural(V::Data(data_field(settings, "tabWidth")?))?)
                .map_err(|_| InvocationError::Limit)?;
            if tab_width == 0 {
                return Err(InvocationError::MalformedIntent);
            }
            let mut owned = String::new();
            owned
                .try_reserve_exact(unit.len())
                .map_err(|_| InvocationError::Capacity)?;
            owned.push_str(unit);
            if action == "indent-lines" {
                LineEditV1::Indent {
                    unit: owned,
                    tab_width,
                }
            } else {
                LineEditV1::Outdent {
                    unit: owned,
                    tab_width,
                }
            }
        }
        "insert-newline" => {
            let settings = object(settings, &["delimiter", "autoIndent"])?;
            let delimiter = match text(data_field(settings, "delimiter")?)? {
                "LF" => GeneratedNewlineV1::Lf,
                "CRLF" => GeneratedNewlineV1::CrLf,
                "CR" => GeneratedNewlineV1::Cr,
                _ => return Err(InvocationError::MalformedIntent),
            };
            let indent = match text(data_field(settings, "autoIndent")?)? {
                "none" => NewlineIndentV1::None,
                "copy-prefix" => NewlineIndentV1::CopyPrefix,
                _ => return Err(InvocationError::MalformedIntent),
            };
            if NormalizedLineEditArgumentsV1::required_owned_bytes(0)? > limits.max_owned_bytes {
                return Err(InvocationError::Limit);
            }
            LineEditV1::Newline { delimiter, indent }
        }
        _ => return Err(InvocationError::MalformedIntent),
    };
    Ok(NormalizedLineEditArgumentsV1 {
        invocation,
        context,
        request,
    })
}
/// Passive native projection only. Full original view context stays a separate
/// native binding; no lookup, settings provider, newline default or UI policy.
pub fn normalize_line_edit_arguments_v1<'a, 'h>(
    envelope: LineEditEnvelopeValueV1<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: LineEditArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedLineEditArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, context, limits);
    capture_timed(
        r,
        Boundary::LineArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    pub fn submit_line_edit_arguments_v1(
        &mut self,
        q: &NormalizedLineEditArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result = self.submit_line_edit(q.invocation, q.context, &q.request, r);
        capture_timed(r, Boundary::LineDecodedAdmission, &result, timing.finish());
        result
    }
}
