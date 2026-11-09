//! Flat inert literal fields with original borrowed native roles. No UI or wire.
use super::*;
use crate::capture::InertValueV1 as D;
use std::mem::size_of;

pub const EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1: &str = "explicit-positional-selection/1";
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "selection-edit/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "ordinary-edit",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSelectionVariant"),
            ("invocation", "InvocationIdentity"),
            ("view", "LogicalViewIdentity"),
            ("document", "DocumentIdentity"),
            ("sourceRevision", "SafeNatural"),
            ("expectedSelection", "SafeNatural"),
            ("profile", "ExecutionProfileRef"),
            ("text", "LiteralText"),
            ("selectionAfter", "ClosedPostSelectionIntent"),
        ],
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiteralEditArgumentLimitsV1 {
    pub max_text_bytes: usize,
    /// Logical normalized header plus owned literal bytes; not borrowed native
    /// role referents, caller ingress, allocator capacity, stack or RSS.
    pub max_owned_bytes: usize,
}
#[derive(Clone, Copy)]
pub enum LiteralEditEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, LiteralEditEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    View(&'h LogicalViewId),
    Document(&'h DocumentId),
}
impl std::fmt::Debug for LiteralEditEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LiteralEditEnvelope(<redacted>)")
    }
}
pub struct NormalizedLiteralEditArgumentsV1<'h> {
    invocation: &'h InvocationId,
    context: &'h LogicalViewContext,
    text: String,
    after: SelectionAfterV1,
}
impl std::fmt::Debug for NormalizedLiteralEditArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedLiteralEditArguments(<redacted>)")
    }
}
impl<'h> NormalizedLiteralEditArgumentsV1<'h> {
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
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn selection_after(&self) -> SelectionAfterV1 {
        self.after
    }
    pub fn required_owned_bytes(text_bytes: usize) -> Result<usize> {
        size_of::<Self>()
            .checked_add(text_bytes)
            .ok_or(InvocationError::Limit)
    }
    pub fn owned_bytes(&self) -> usize {
        size_of::<Self>() + self.text.len()
    }
}
type V<'a, 'h> = LiteralEditEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
pub(in crate::coordinator) fn field<'a, 'h>(
    rows: &'a [(&str, V<'a, 'h>)],
    key: &str,
) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::MalformedIntent)
}
pub(in crate::coordinator) fn string<'a>(value: V<'a, '_>) -> Result<&'a str> {
    match value {
        V::Data(D::Text(v)) => Ok(v),
        _ => Err(InvocationError::MalformedIntent),
    }
}
pub(in crate::coordinator) fn natural(value: V<'_, '_>) -> Result<u64> {
    match value {
        V::Data(D::Number(v))
            if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= 9_007_199_254_740_991.0 =>
        {
            Ok(v as u64)
        }
        _ => Err(InvocationError::MalformedIntent),
    }
}
// Shared native view binding, not a caller-selected handler or live lookup.
pub(in crate::coordinator) fn bind_header<'a, 'h>(
    rows: &'a [(&str, V<'a, 'h>)],
    context: &'h LogicalViewContext,
    d: &NativeArgumentDeclarationV1,
    profile: &str,
) -> Result<&'h InvocationId> {
    if !d.fields_match(rows.iter().map(|(key, _)| *key)) {
        return Err(InvocationError::MalformedIntent);
    }
    if string(field(rows, "contract")?)? != d.contract
        || string(field(rows, "operation")?)? != d.variant.operation
        || string(field(rows, "profile")?)? != profile
    {
        return Err(InvocationError::MalformedIntent);
    }
    let V::Invocation(invocation) = field(rows, "invocation")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let V::View(view) = field(rows, "view")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let V::Document(document) = field(rows, "document")? else {
        return Err(InvocationError::MalformedIntent);
    };
    if view != &context.view
        || document != &context.source.document
        || natural(field(rows, "sourceRevision")?)? != context.source.revision
        || natural(field(rows, "expectedSelection")?)? != context.stamp.selection_generation
    {
        return Err(InvocationError::MalformedIntent);
    }
    Ok(invocation)
}
fn normalize<'a, 'h>(
    value: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: LiteralEditArgumentLimitsV1,
) -> Result<NormalizedLiteralEditArgumentsV1<'h>> {
    let V::Object(rows) = value else {
        return Err(InvocationError::MalformedIntent);
    };
    let invocation = bind_header(
        rows,
        context,
        NormalizedLiteralEditArgumentsV1::ARGUMENT_DECLARATION,
        EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1,
    )?;
    let after = match string(field(rows, "selectionAfter")?)? {
        "mapped" => SelectionAfterV1::Mapped,
        "carets-after" => SelectionAfterV1::CaretsAfter,
        _ => return Err(InvocationError::MalformedIntent),
    };
    let text = string(field(rows, "text")?)?;
    if text.len() > limits.max_text_bytes
        || NormalizedLiteralEditArgumentsV1::required_owned_bytes(text.len())?
            > limits.max_owned_bytes
    {
        return Err(InvocationError::Limit);
    }
    // All accepted data is scalar: reject nested values without walking them.
    // One bounded text copy, no intermediate capture/schema tree or context lookup.
    let mut owned = String::new();
    owned
        .try_reserve_exact(text.len())
        .map_err(|_| InvocationError::Capacity)?;
    owned.push_str(text);
    Ok(NormalizedLiteralEditArgumentsV1 {
        invocation,
        context,
        text: owned,
        after,
    })
}
/// Passive native projection of selection-edit/1 ordinary-edit. The full
/// original view context is supplied separately, never rebuilt from the current
/// view or inferred from numeric fields. Header roles must match it exactly.
/// Normalization validates representation, not current authority/state or a
/// public identity codec. Wider trusted-native counters retain their own API.
pub fn normalize_literal_edit_arguments_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: LiteralEditArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedLiteralEditArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, context, limits);
    capture_timed(
        r,
        Boundary::LiteralArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    pub fn submit_literal_edit_arguments_v1(
        &mut self,
        q: &NormalizedLiteralEditArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result = self.submit_literal_edit(q.invocation, q.context, &q.text, q.after, r);
        capture_timed(
            r,
            Boundary::LiteralDecodedAdmission,
            &result,
            timing.finish(),
        );
        result
    }
}
