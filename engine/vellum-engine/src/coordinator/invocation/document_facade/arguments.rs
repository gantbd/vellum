//! Bounded in-process native argument guards. Opaque roles are never decoded
//! from strings. No transport, name resolution, policy decision or publication.
use super::*;
use crate::capture::InertValueV1 as D;
use crate::selection::Selection;
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub const NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1: &str = "native-document-lifecycle/1";
const HEADER: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "lifecycle-invocation/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "change-document-lifecycle",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLifecycleOperation"),
            ("invocation", "OriginalOpaqueInvocation"),
            ("profile", "ExecutionProfileRef"),
            ("request", "ClosedNativeLifecycleRequest"),
        ],
    },
};
const fn declaration(
    operation: &'static str,
    fields: &'static [(&'static str, &'static str)],
) -> NativeArgumentDeclarationV1 {
    NativeArgumentDeclarationV1 {
        contract: "document-lifecycle/1",
        variant: NativeHandlerVariantDeclarationV1 { operation, fields },
    }
}
static FORMS: [NativeArgumentDeclarationV1; 5] = [
    declaration(
        "create-scratch",
        &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLifecycleVariant"),
            ("source", "BoundedSourceBytes"),
            ("serialization", "NativeSourceSerialization"),
            ("insertionLineBreak", "GeneratedNewline"),
        ],
    ),
    declaration(
        "attach-view",
        &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLifecycleVariant"),
            ("document", "OriginalOpaqueDocument"),
            ("expectedLifecycle", "OriginalNativeLifecycle"),
            ("view", "OriginalOpaqueView"),
            ("expectedAttachment", "OriginalNativeViewStamp"),
            ("sourceRevision", "OriginalNativeSourceVersion"),
            ("selections", "BoundedNativeSelections"),
            ("primary", "NativeSelectionIndex"),
        ],
    ),
    declaration(
        "detach-view",
        &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLifecycleVariant"),
            ("document", "OriginalOpaqueDocument"),
            ("expectedLifecycle", "OriginalNativeLifecycle"),
            ("view", "OriginalOpaqueView"),
            ("expectedAttachment", "OriginalNativeViewStamp"),
        ],
    ),
    declaration(
        "membership",
        &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLifecycleVariant"),
            ("document", "OriginalOpaqueDocument"),
            ("expectedLifecycle", "OriginalNativeLifecycle"),
            ("workspace", "OriginalOpaqueWorkspace"),
            ("expectedWorkspace", "OriginalNativeWorkspaceContext"),
            ("present", "Boolean"),
        ],
    ),
    declaration(
        "dispose",
        &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedLifecycleVariant"),
            ("document", "OriginalOpaqueDocument"),
            ("expectedLifecycle", "OriginalNativeLifecycle"),
            ("expectedSource", "OriginalNativeSourceVersion"),
            ("expectedAnnotations", "NativeGeneration"),
            ("expectedSerialization", "NativeGeneration"),
            ("expectedBinding", "NativeOptionalBindingGeneration"),
            ("sourceDisposition", "ClosedDisposalDisposition"),
            ("annotationDisposition", "ClosedDisposalDisposition"),
        ],
    ),
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeDocumentLifecycleArgumentLimitsV1 {
    /// Logical owned output only; not borrowed inputs/referents, stack, RSS or a
    /// shared parent reservation. The caller owns/funds this normalization result.
    pub max_owned_bytes: usize,
    pub max_source_bytes: usize,
    pub max_selections: usize,
}
#[derive(Clone, Copy)]
pub enum NativeDocumentLifecycleEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, Self)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Document(&'h DocumentId),
    Lifecycle(LifecycleState),
    View(&'h LogicalViewId),
    ViewStamp(LogicalViewStamp),
    SourceVersion(&'h DocumentVersion),
    Selections(&'a [Selection]),
    Index(usize),
    Workspace(&'h WorkspaceId),
    WorkspaceContext(&'h WorkspaceContext),
    Generation(u64),
    BindingGeneration(Option<u64>),
}
impl std::fmt::Debug for NativeDocumentLifecycleEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeLifecycleEnvelope(<redacted>)")
    }
}
enum Body {
    Scratch {
        source: Vec<u8>,
        serialization: SourceSerializationV1,
        newline: GeneratedNewlineV1,
    },
    Live(LiveRelationRequestV1),
    Disposal(ExplicitDisposalRequestV1),
}
pub struct NormalizedDocumentLifecycleArgumentsV1<'h> {
    invocation: &'h InvocationId,
    body: Body,
    form: usize,
    owned: usize,
}
impl std::fmt::Debug for NormalizedDocumentLifecycleArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedNativeLifecycleArguments(<redacted>)")
    }
}
impl<'h> NormalizedDocumentLifecycleArgumentsV1<'h> {
    pub const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 = &HEADER;
    pub const REQUEST_DECLARATIONS: &'static [NativeArgumentDeclarationV1; 5] = &FORMS;
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn operation(&self) -> &'static str {
        FORMS[self.form].variant.operation
    }
    pub fn owned_bytes(&self) -> usize {
        self.owned
    }
    pub const fn required_header_bytes() -> usize {
        size_of::<Self>()
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &HEADER
    }
    pub fn request_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &FORMS[self.form]
    }
    pub fn source_bytes(&self) -> Option<&[u8]> {
        match &self.body {
            Body::Scratch { source, .. } => Some(source),
            _ => None,
        }
    }
    pub fn live_relation(&self) -> Option<&LiveRelationRequestV1> {
        match &self.body {
            Body::Live(q) => Some(q),
            _ => None,
        }
    }
    pub fn explicit_disposal(&self) -> Option<&ExplicitDisposalRequestV1> {
        match &self.body {
            Body::Disposal(q) => Some(q),
            _ => None,
        }
    }
}
type V<'a, 'h> = NativeDocumentLifecycleEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
fn bad<T>() -> Result<T> {
    Err(InvocationError::MalformedIntent)
}
fn field<'a, 'h>(rows: &[(&str, V<'a, 'h>)], name: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::MalformedIntent)
}
fn text<'a>(v: V<'a, '_>) -> Result<&'a str> {
    match v {
        V::Data(D::Text(s)) => Ok(s),
        _ => bad(),
    }
}
fn object<'a, 'h>(
    v: V<'a, 'h>,
    d: &NativeArgumentDeclarationV1,
) -> Result<&'a [(&'a str, V<'a, 'h>)]> {
    let V::Object(rows) = v else { return bad() };
    if !d.fields_match(rows.iter().map(|(k, _)| *k))
        || text(field(rows, "contract")?)? != d.contract
        || text(field(rows, "operation")?)? != d.variant.operation
    {
        return bad();
    }
    Ok(rows)
}
fn generation(v: V<'_, '_>) -> Result<u64> {
    match v {
        V::Generation(n) => Ok(n),
        _ => bad(),
    }
}
fn disposition(v: V<'_, '_>) -> Result<DisposalDispositionV1> {
    match text(v)? {
        "retain" => Ok(DisposalDispositionV1::Retain),
        "discard" => Ok(DisposalDispositionV1::Discard),
        _ => bad(),
    }
}
fn serialization(v: V<'_, '_>) -> Result<SourceSerializationV1> {
    let V::Data(D::Object(rows)) = v else {
        return bad();
    };
    if rows.len() != 2
        || rows.iter().filter(|(k, _)| *k == "profile").count() != 1
        || rows.iter().filter(|(k, _)| *k == "bom").count() != 1
    {
        return bad();
    }
    let get = |key| {
        rows.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .ok_or(InvocationError::MalformedIntent)
    };
    if !matches!(get("profile")?, D::Text("strict-utf8-one-bom/1")) {
        return bad();
    }
    let D::Boolean(bom) = get("bom")? else {
        return bad();
    };
    Ok(SourceSerializationV1 {
        profile: SourceSerializationProfileV1::StrictUtf8OneBom,
        bom,
    })
}
fn owned_bytes(extra: usize, l: NativeDocumentLifecycleArgumentLimitsV1) -> Result<usize> {
    let n = NormalizedDocumentLifecycleArgumentsV1::required_header_bytes()
        .checked_add(extra)
        .ok_or(InvocationError::Limit)?;
    if n > l.max_owned_bytes {
        return Err(InvocationError::Limit);
    }
    Ok(n)
}
fn normalize<'h>(
    envelope: V<'_, 'h>,
    l: NativeDocumentLifecycleArgumentLimitsV1,
) -> Result<NormalizedDocumentLifecycleArgumentsV1<'h>> {
    let header = object(envelope, &HEADER)?;
    if text(field(header, "profile")?)? != NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1 {
        return bad();
    }
    let V::Invocation(invocation) = field(header, "invocation")? else {
        return bad();
    };
    let V::Object(raw) = field(header, "request")? else {
        return bad();
    };
    // Bound scanning before selecting a declaration; opaque referents are never walked.
    if raw.len() > 10 {
        return bad();
    }
    let operation = text(field(raw, "operation")?)?;
    let form = FORMS
        .iter()
        .position(|d| d.variant.operation == operation)
        .ok_or(InvocationError::MalformedIntent)?;
    let rows = object(V::Object(raw), &FORMS[form])?;
    let mut owned = owned_bytes(0, l)?;
    let body = if form == 0 {
        let V::Data(D::Array(bytes)) = field(rows, "source")? else {
            return bad();
        };
        if bytes.len() > l.max_source_bytes {
            return Err(InvocationError::Limit);
        }
        owned = owned_bytes(bytes.len(), l)?;
        let serialization = serialization(field(rows, "serialization")?)?;
        let newline = match text(field(rows, "insertionLineBreak")?)? {
            "LF" => GeneratedNewlineV1::Lf,
            "CRLF" => GeneratedNewlineV1::CrLf,
            _ => return bad(),
        };
        let mut source = Vec::new();
        source
            .try_reserve_exact(bytes.len())
            .map_err(|_| InvocationError::Capacity)?;
        for byte in bytes {
            let D::Number(n) = byte else { return bad() };
            if !n.is_finite() || n.fract() != 0.0 || !(0.0..=255.0).contains(n) {
                return bad();
            }
            source.push(*n as u8);
        }
        Body::Scratch {
            source,
            serialization,
            newline,
        }
    } else {
        let V::Document(document) = field(rows, "document")? else {
            return bad();
        };
        let V::Lifecycle(lifecycle) = field(rows, "expectedLifecycle")? else {
            return bad();
        };
        if form == 4 {
            let V::SourceVersion(source) = field(rows, "expectedSource")? else {
                return bad();
            };
            if source.document != *document {
                return bad();
            }
            let V::BindingGeneration(binding_generation) = field(rows, "expectedBinding")? else {
                return bad();
            };
            Body::Disposal(ExplicitDisposalRequestV1 {
                source: source.clone(),
                annotation_generation: generation(field(rows, "expectedAnnotations")?)?,
                lifecycle,
                serialization_generation: generation(field(rows, "expectedSerialization")?)?,
                binding_generation,
                source_disposition: disposition(field(rows, "sourceDisposition")?)?,
                annotation_disposition: disposition(field(rows, "annotationDisposition")?)?,
            })
        } else {
            let operation = if form == 3 {
                let (
                    V::Workspace(workspace),
                    V::WorkspaceContext(expected),
                    V::Data(D::Boolean(present)),
                ) = (
                    field(rows, "workspace")?,
                    field(rows, "expectedWorkspace")?,
                    field(rows, "present")?,
                )
                else {
                    return bad();
                };
                if *workspace != expected.workspace {
                    return bad();
                }
                LiveRelationOperationV1::Membership {
                    workspace: expected.clone(),
                    present,
                }
            } else {
                let (V::View(view), V::ViewStamp(expected)) =
                    (field(rows, "view")?, field(rows, "expectedAttachment")?)
                else {
                    return bad();
                };
                if form == 2 {
                    LiveRelationOperationV1::Detach {
                        view: view.clone(),
                        expected,
                    }
                } else {
                    let (V::SourceVersion(source), V::Selections(input), V::Index(primary)) = (
                        field(rows, "sourceRevision")?,
                        field(rows, "selections")?,
                        field(rows, "primary")?,
                    ) else {
                        return bad();
                    };
                    if source.document != *document {
                        return bad();
                    }
                    if input.len() > l.max_selections {
                        return Err(InvocationError::Limit);
                    }
                    owned = owned_bytes(
                        input
                            .len()
                            .checked_mul(size_of::<Selection>())
                            .ok_or(InvocationError::Limit)?,
                        l,
                    )?;
                    let mut selections = Vec::new();
                    selections
                        .try_reserve_exact(input.len())
                        .map_err(|_| InvocationError::Capacity)?;
                    selections.extend_from_slice(input);
                    LiveRelationOperationV1::Attach {
                        view: view.clone(),
                        expected,
                        source: source.clone(),
                        selections,
                        primary,
                    }
                }
            };
            Body::Live(LiveRelationRequestV1 {
                document: document.clone(),
                expected_lifecycle: lifecycle,
                operation,
            })
        }
    };
    Ok(NormalizedDocumentLifecycleArgumentsV1 {
        invocation,
        body,
        form,
        owned,
    })
}
/// Exact in-process native-role shape and bounded copy only, not authority,
/// UTF-8/selection validity, currentness, shared sponsorship or a wire decoder.
pub fn normalize_native_document_lifecycle_arguments_v1<'h>(
    envelope: V<'_, 'h>,
    limits: NativeDocumentLifecycleArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedDocumentLifecycleArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, limits);
    capture_timed(
        r,
        Boundary::DocumentLifecycleArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// Original native owners perform all applicable authorization, funding,
    /// replay/currentness and final publication checks. Normalization is inert.
    pub fn submit_native_document_lifecycle_arguments_v1(
        &mut self,
        n: &NormalizedDocumentLifecycleArgumentsV1<'_>,
        a: NativeDocumentLifecycleAdmissionV2<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        match &n.body {
            Body::Scratch {
                source,
                serialization,
                newline,
            } => {
                let q = ScratchDocumentRequestV1 {
                    source,
                    serialization: *serialization,
                    insertion_policy: *newline,
                };
                self.submit_native_document_lifecycle_request_v1(
                    n.invocation,
                    NativeDocumentLifecycleRequestV1::Scratch(&q),
                    a,
                    r,
                )
            }
            Body::Live(q) => self.submit_native_document_lifecycle_request_v1(
                n.invocation,
                NativeDocumentLifecycleRequestV1::LiveRelation(q),
                a,
                r,
            ),
            Body::Disposal(q) => self.submit_native_document_lifecycle_request_v1(
                n.invocation,
                NativeDocumentLifecycleRequestV1::ExplicitDisposal(q),
                a,
                r,
            ),
        }
    }
}
