//! F11 ordinary boundary deletion only, not segmentation or visual movement.
use super::*;
use crate::capture::InertValueV1 as D;
use crate::coordinator::invocation::literal::arguments::{bind_header, natural};
use std::mem::size_of;
pub const UNICODE_LOGICAL_EDIT_PROFILE_V1: &str = "unicode-logical-edit/1";
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "boundary-command/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "ordinary-edit",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedBoundaryVariant"),
            ("invocation", "InvocationIdentity"),
            ("view", "LogicalViewIdentity"),
            ("document", "DocumentIdentity"),
            ("sourceRevision", "SafeNatural"),
            ("expectedSelection", "SafeNatural"),
            ("profile", "ExecutionProfileRef"),
            ("boundary", "BoundaryEvidenceIdentity"),
            ("unit", "ClosedLogicalUnicodeUnit"),
            ("direction", "ClosedLogicalDirection"),
            ("count", "PositiveSafeNativeInteger"),
            ("extend", "FalseForOrdinaryDeletion"),
        ],
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryEditArgumentLimitsV1 {
    /// Fixed normalized header only; borrowed roles/tables, caller input,
    /// allocator/RSS and later native preparation remain separately bounded.
    pub max_owned_bytes: usize,
}
#[derive(Clone, Copy)]
pub enum BoundaryEditEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, BoundaryEditEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    View(&'h LogicalViewId),
    Document(&'h DocumentId),
    Boundary(&'h BoundaryEvidenceId),
}
impl std::fmt::Debug for BoundaryEditEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BoundaryEditEnvelope(<redacted>)")
    }
}
pub struct NormalizedBoundaryEditArgumentsV1<'h> {
    invocation: &'h InvocationId,
    context: &'h LogicalViewContext,
    evidence: &'h BoundaryEvidenceId,
    request: UnicodeCommandV1,
}
impl std::fmt::Debug for NormalizedBoundaryEditArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedBoundaryEditArguments(<redacted>)")
    }
}
type Result<T> = std::result::Result<T, InvocationError>;
type V<'a, 'h> = BoundaryEditEnvelopeValueV1<'a, 'h>;
impl<'h> NormalizedBoundaryEditArgumentsV1<'h> {
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
    pub fn evidence(&self) -> &'h BoundaryEvidenceId {
        self.evidence
    }
    pub fn request(&self) -> UnicodeCommandV1 {
        self.request
    }
    pub fn required_owned_bytes() -> usize {
        size_of::<Self>()
    }
    pub fn owned_bytes(&self) -> usize {
        Self::required_owned_bytes()
    }
}
fn field<'a, 'h>(rows: &[(&str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
        .ok_or(InvocationError::MalformedIntent)
}
fn text<'a>(value: V<'a, '_>) -> Result<&'a str> {
    match value {
        V::Data(D::Text(value)) => Ok(value),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn normalize<'a, 'h>(
    envelope: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: BoundaryEditArgumentLimitsV1,
) -> Result<NormalizedBoundaryEditArgumentsV1<'h>> {
    let V::Object(rows) = envelope else {
        return Err(InvocationError::MalformedIntent);
    };
    if rows.len() != 13 {
        return Err(InvocationError::MalformedIntent);
    }
    let header: [(&str, LiteralEditEnvelopeValueV1<'_, '_>); 13] = std::array::from_fn(|i| {
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
        NormalizedBoundaryEditArgumentsV1::ARGUMENT_DECLARATION,
        UNICODE_LOGICAL_EDIT_PROFILE_V1,
    )?;
    let V::Boundary(evidence) = field(rows, "boundary")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let unit = match text(field(rows, "unit")?)? {
        "grapheme" => UnicodeUnitV1::Grapheme,
        "word-start" => UnicodeUnitV1::WordStart,
        "word-end" => UnicodeUnitV1::WordEnd,
        _ => return Err(InvocationError::MalformedIntent),
    };
    let direction = match text(field(rows, "direction")?)? {
        "forward" => UnicodeDirectionV1::Forward,
        "backward" => UnicodeDirectionV1::Backward,
        _ => return Err(InvocationError::MalformedIntent),
    };
    let V::Data(count) = field(rows, "count")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let count = usize::try_from(natural(LiteralEditEnvelopeValueV1::Data(count))?)
        .map_err(|_| InvocationError::Limit)?;
    if count == 0 || !matches!(field(rows, "extend")?, V::Data(D::Boolean(false))) {
        return Err(InvocationError::MalformedIntent);
    }
    if NormalizedBoundaryEditArgumentsV1::required_owned_bytes() > limits.max_owned_bytes {
        return Err(InvocationError::Limit);
    }
    Ok(NormalizedBoundaryEditArgumentsV1 {
        invocation,
        context,
        evidence,
        request: UnicodeCommandV1 {
            unit,
            direction,
            count,
            extend: false,
        },
    })
}
/// Original evidence/view roles only. No table copying/resolution, segmentation,
/// provider qualification, currentness assertion or public identity codec.
pub fn normalize_boundary_edit_arguments_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: BoundaryEditArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedBoundaryEditArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, context, limits);
    capture_timed(
        r,
        Boundary::BoundaryEditArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    pub fn submit_boundary_edit_arguments_v1(
        &mut self,
        q: &NormalizedBoundaryEditArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result = self.submit_unicode_command(
            q.invocation,
            q.context,
            q.evidence,
            UnicodeOperationV1::Deletion,
            q.request,
            r,
        );
        capture_timed(
            r,
            Boundary::BoundaryEditDecodedAdmission,
            &result,
            timing.finish(),
        );
        result
    }
}
