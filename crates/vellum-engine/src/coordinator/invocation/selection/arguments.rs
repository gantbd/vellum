//! Four closed F12 native envelopes; original roles, no UI or wire/name resolver.
use super::*;
use crate::attachment::Affinity;
use crate::capture::InertValueV1 as D;
use crate::coordinates::{BytePosition, CoordinateBudget};
use crate::coordinator::invocation::literal::arguments::bind_header;
use crate::selection::Selection;
use std::mem::size_of;

const COMMON: [(&str, &str); 8] = [
    ("contract", "RegisteredContractVersion"),
    ("operation", "ClosedSelectionVariant"),
    ("invocation", "InvocationIdentity"),
    ("view", "LogicalViewIdentity"),
    ("document", "DocumentIdentity"),
    ("sourceRevision", "SafeNatural"),
    ("expectedSelection", "SafeNatural"),
    ("profile", "ExecutionProfileRef"),
];
const SET: &[(&str, &str)] = &[
    COMMON[0],
    COMMON[1],
    COMMON[2],
    COMMON[3],
    COMMON[4],
    COMMON[5],
    COMMON[6],
    COMMON[7],
    ("selections", "OrderedRawSelectionList"),
    ("primary", "SafeNatural"),
];
const ENDPOINT: &[(&str, &str)] = &[
    COMMON[0],
    COMMON[1],
    COMMON[2],
    COMMON[3],
    COMMON[4],
    COMMON[5],
    COMMON[6],
    COMMON[7],
    ("unit", "ClosedLogicalEndpoint"),
    ("extend", "Boolean"),
];
const OCCURRENCE: &[(&str, &str)] = &[
    COMMON[0],
    COMMON[1],
    COMMON[2],
    COMMON[3],
    COMMON[4],
    COMMON[5],
    COMMON[6],
    COMMON[7],
    ("match", "OriginalOpaqueSearchMatch"),
    ("direction", "ClosedSelectionDirection"),
    ("affinity", "ClosedAffinity"),
    ("selectionMode", "ClosedSearchSelectionMode"),
];
const UNICODE: &[(&str, &str)] = &[
    COMMON[0],
    COMMON[1],
    COMMON[2],
    COMMON[3],
    COMMON[4],
    COMMON[5],
    COMMON[6],
    COMMON[7],
    ("boundary", "BoundaryEvidenceIdentity"),
    ("unit", "ClosedLogicalUnicodeUnit"),
    ("direction", "ClosedLogicalDirection"),
    ("count", "PositiveSafeNativeInteger"),
    ("extend", "Boolean"),
];
const fn declaration(
    contract: &'static str,
    operation: &'static str,
    fields: &'static [(&'static str, &'static str)],
) -> NativeArgumentDeclarationV1 {
    NativeArgumentDeclarationV1 {
        contract,
        variant: NativeHandlerVariantDeclarationV1 { operation, fields },
    }
}
static DECLARATIONS: [NativeArgumentDeclarationV1; 4] = [
    declaration("selection-edit/1", "set-selection", SET),
    declaration("selection-edit/1", "logical-movement", ENDPOINT),
    declaration("selection-edit/1", "select-search-match", OCCURRENCE),
    declaration("boundary-command/1", "logical-movement", UNICODE),
];
/// Owning native argument declarations, not executable handles or authority.
pub fn native_selection_argument_declarations_v1() -> [&'static NativeArgumentDeclarationV1; 4] {
    std::array::from_fn(|i| &DECLARATIONS[i])
}
#[derive(Clone, Copy, Debug)]
pub struct SelectionArgumentLimitsV1 {
    pub max_input_selections: usize,
    /// Normalized native header, raw selection rows and any retained occurrence
    /// descriptor/referents. Not caller ingress, allocator/RSS or handler staging.
    pub max_owned_bytes: usize,
    /// Explicit interpretation bound, captured as native endpoint intent.
    pub endpoint: CoordinateBudget,
}
#[derive(Clone, Copy)]
pub enum SelectionEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, SelectionEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    View(&'h LogicalViewId),
    Document(&'h DocumentId),
    Boundary(&'h BoundaryEvidenceId),
    Match(&'h SearchMatch),
}
enum Request<'h> {
    Selection(SelectionCommandV1),
    Unicode {
        evidence: &'h BoundaryEvidenceId,
        request: UnicodeCommandV1,
    },
}
pub struct NormalizedSelectionArgumentsV1<'h> {
    invocation: &'h InvocationId,
    context: &'h LogicalViewContext,
    request: Request<'h>,
    declaration: &'static NativeArgumentDeclarationV1,
    bytes: usize,
}
impl std::fmt::Debug for SelectionEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SelectionEnvelope(<redacted>)")
    }
}
impl std::fmt::Debug for NormalizedSelectionArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedSelectionArguments(<redacted>)")
    }
}
impl<'h> NormalizedSelectionArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn context(&self) -> &'h LogicalViewContext {
        self.context
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        self.declaration
    }
    pub fn selection_request(&self) -> Option<&SelectionCommandV1> {
        match &self.request {
            Request::Selection(q) => Some(q),
            Request::Unicode { .. } => None,
        }
    }
    pub fn boundary_request(&self) -> Option<(&'h BoundaryEvidenceId, UnicodeCommandV1)> {
        match &self.request {
            Request::Unicode { evidence, request } => Some((evidence, *request)),
            Request::Selection(_) => None,
        }
    }
    pub fn owned_bytes(&self) -> usize {
        self.bytes
    }
    pub const fn minimum_owned_bytes() -> usize {
        size_of::<Self>()
    }
}
type V<'a, 'h> = SelectionEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
fn field<'a, 'h>(rows: &[(&str, V<'a, 'h>)], name: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::MalformedIntent)
}
fn text<'a>(v: V<'a, '_>) -> Result<&'a str> {
    match v {
        V::Data(D::Text(s)) => Ok(s),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn boolean(v: V<'_, '_>) -> Result<bool> {
    match v {
        V::Data(D::Boolean(b)) => Ok(b),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn natural(v: D<'_>) -> Result<usize> {
    usize::try_from(super::super::literal::arguments::natural(
        LiteralEditEnvelopeValueV1::Data(v),
    )?)
    .map_err(|_| InvocationError::MalformedIntent)
}
fn number(v: V<'_, '_>) -> Result<usize> {
    match v {
        V::Data(d) => natural(d),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn affinity(v: &str) -> Result<Affinity> {
    match v {
        "before" => Ok(Affinity::Before),
        "after" => Ok(Affinity::After),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn selection(v: &D<'_>) -> Result<Selection> {
    let D::Object(rows) = v else {
        return Err(InvocationError::MalformedIntent);
    };
    if rows.len() != 3
        || ["anchor", "head", "affinity"]
            .iter()
            .any(|name| rows.iter().filter(|(key, _)| key == name).count() != 1)
    {
        return Err(InvocationError::MalformedIntent);
    }
    let get = |name| {
        rows.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, v)| *v)
            .ok_or(InvocationError::MalformedIntent)
    };
    let D::Text(a) = get("affinity")? else {
        return Err(InvocationError::MalformedIntent);
    };
    Ok(Selection {
        anchor: BytePosition(natural(get("anchor")?)?),
        head: BytePosition(natural(get("head")?)?),
        affinity: affinity(a)?,
    })
}
fn charge(payload: usize, limits: SelectionArgumentLimitsV1) -> Result<usize> {
    size_of::<NormalizedSelectionArgumentsV1<'_>>()
        .checked_add(payload)
        .filter(|n| *n <= limits.max_owned_bytes)
        .ok_or(InvocationError::Limit)
}
fn normalize<'a, 'h>(
    value: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: SelectionArgumentLimitsV1,
) -> Result<NormalizedSelectionArgumentsV1<'h>> {
    let V::Object(rows) = value else {
        return Err(InvocationError::MalformedIntent);
    };
    if !matches!(rows.len(), 10 | 12 | 13) {
        return Err(InvocationError::MalformedIntent);
    }
    let index = match (
        text(field(rows, "contract")?)?,
        text(field(rows, "operation")?)?,
    ) {
        ("selection-edit/1", "set-selection") => 0,
        ("selection-edit/1", "logical-movement") => 1,
        ("selection-edit/1", "select-search-match") => 2,
        ("boundary-command/1", "logical-movement") => 3,
        _ => return Err(InvocationError::MalformedIntent),
    };
    let declaration = &DECLARATIONS[index];
    // Bounded stack shadow only; opaque match/boundary fields are validated below,
    // never reconstructed from inert data or looked up by caller labels.
    let header: [(&str, LiteralEditEnvelopeValueV1<'_, '_>); 13] = std::array::from_fn(|i| {
        use LiteralEditEnvelopeValueV1 as H;
        rows.get(i).map_or(("", H::Data(D::Null)), |(name, value)| {
            (
                *name,
                match value {
                    V::Data(d) => H::Data(*d),
                    V::Invocation(v) => H::Invocation(v),
                    V::View(v) => H::View(v),
                    V::Document(v) => H::Document(v),
                    _ => H::Data(D::Null),
                },
            )
        })
    });
    let invocation = bind_header(
        &header[..rows.len()],
        context,
        declaration,
        if index == 3 {
            UNICODE_LOGICAL_EDIT_PROFILE_V1
        } else {
            EXPLICIT_POSITIONAL_SELECTION_PROFILE_V1
        },
    )?;
    let mut bytes = charge(0, limits)?;
    let request = match index {
        0 => {
            let V::Data(D::Array(raw)) = field(rows, "selections")? else {
                return Err(InvocationError::MalformedIntent);
            };
            if raw.len() > limits.max_input_selections {
                return Err(InvocationError::Limit);
            }
            bytes = charge(
                raw.len()
                    .checked_mul(size_of::<Selection>())
                    .ok_or(InvocationError::Limit)?,
                limits,
            )?;
            let primary = number(field(rows, "primary")?)?;
            for value in raw {
                selection(value)?;
            }
            let mut selections = Vec::new();
            selections
                .try_reserve_exact(raw.len())
                .map_err(|_| InvocationError::Capacity)?;
            for value in raw {
                selections.push(selection(value)?);
            }
            Request::Selection(SelectionCommandV1::Set {
                selections,
                primary,
            })
        }
        1 => {
            let endpoint = match text(field(rows, "unit")?)? {
                "line-start" => LogicalEndpointV1::LineStart,
                "line-end" => LogicalEndpointV1::LineEnd,
                "document-start" => LogicalEndpointV1::DocumentStart,
                "document-end" => LogicalEndpointV1::DocumentEnd,
                _ => return Err(InvocationError::MalformedIntent),
            };
            Request::Selection(SelectionCommandV1::Endpoint {
                endpoint,
                extend: boolean(field(rows, "extend")?)?,
                budget: limits.endpoint,
            })
        }
        2 => {
            let V::Match(occurrence) = field(rows, "match")? else {
                return Err(InvocationError::MalformedIntent);
            };
            bytes = charge(
                occurrence.retained_bytes().ok_or(InvocationError::Limit)?,
                limits,
            )?;
            let direction = match text(field(rows, "direction")?)? {
                "forward" => SelectionDirectionV1::Forward,
                "backward" => SelectionDirectionV1::Backward,
                _ => return Err(InvocationError::MalformedIntent),
            };
            let affinity = affinity(text(field(rows, "affinity")?)?)?;
            let mode = match text(field(rows, "selectionMode")?)? {
                "replace-all" => SearchSelectionModeV1::ReplaceAll,
                "replace-primary" => SearchSelectionModeV1::ReplacePrimary,
                "add-primary" => SearchSelectionModeV1::AddPrimary,
                _ => return Err(InvocationError::MalformedIntent),
            };
            Request::Selection(SelectionCommandV1::SearchMatch {
                occurrence: occurrence.clone(),
                direction,
                affinity,
                mode,
            })
        }
        3 => {
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
            let count = number(field(rows, "count")?)?;
            if count == 0 {
                return Err(InvocationError::MalformedIntent);
            }
            Request::Unicode {
                evidence,
                request: UnicodeCommandV1 {
                    unit,
                    direction,
                    count,
                    extend: boolean(field(rows, "extend")?)?,
                },
            }
        }
        _ => unreachable!("closed declaration index"),
    };
    Ok(NormalizedSelectionArgumentsV1 {
        invocation,
        context,
        request,
        declaration,
        bytes,
    })
}
pub fn normalize_selection_arguments_v1<'a, 'h>(
    value: V<'a, 'h>,
    context: &'h LogicalViewContext,
    limits: SelectionArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedSelectionArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(value, context, limits);
    capture_timed(
        r,
        Boundary::SelectionArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// Trusted native adapter, not the complete recipient-authorized F12 facade.
    pub fn submit_selection_arguments_v1(
        &mut self,
        q: &NormalizedSelectionArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result = match &q.request {
            Request::Selection(request) => {
                self.submit_selection_command(q.invocation, q.context, request, r)
            }
            Request::Unicode { evidence, request } => self.submit_unicode_command(
                q.invocation,
                q.context,
                evidence,
                UnicodeOperationV1::Movement,
                *request,
                r,
            ),
        };
        capture_timed(
            r,
            Boundary::SelectionDecodedAdmission,
            &result,
            timing.finish(),
        );
        result
    }
}
