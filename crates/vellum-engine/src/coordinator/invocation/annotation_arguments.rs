//! Lossless F13 inner arguments, not profile activation, a live-reference issuer,
//! recipient authorizer, receipt owner or complete typed/dynamic invocation port.
use super::*;
use crate::capture::InertValueV1 as D;
use std::mem::size_of;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnotationAuthoringArgumentLimitsV1 {
    pub max_body_bytes: usize,
    pub max_profile_bytes: usize,
    pub max_dependencies: usize,
    /// Fixed normalized/mutation headers and owned body/profile payloads. Borrowed
    /// dependency referents, caller ingress, allocator/RSS and native preparation
    /// are separate; this does not sponsor a retained invocation capsule.
    pub max_owned_bytes: usize,
}
#[derive(Clone, Copy)]
pub enum AnnotationAuthoringEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, AnnotationAuthoringEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Document(&'h DocumentId),
    Annotation(&'h AnnotationId),
    Target(&'h SourceTarget),
    Dependencies(&'h ReadDependencyRequestV3),
    /// Actual native version value, including values above JSON's safe integer
    /// range. Inert numbers/decimal strings are deliberately not this role.
    Version(u64),
}
pub struct NormalizedAnnotationAuthoringArgumentsV1<'h> {
    invocation: &'h InvocationId,
    document: &'h DocumentId,
    dependencies: &'h ReadDependencyRequestV3,
    execution: String,
    mutation: AnnotationMutation,
    declaration: NativeArgumentDeclarationV1,
    bytes: usize,
    limits: AnnotationAuthoringArgumentLimitsV1,
}
impl std::fmt::Debug for AnnotationAuthoringEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationEnvelope(<redacted>)")
    }
}
impl std::fmt::Debug for NormalizedAnnotationAuthoringArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedAnnotationArguments(<redacted>)")
    }
}
impl<'h> NormalizedAnnotationAuthoringArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn document(&self) -> &'h DocumentId {
        self.document
    }
    pub fn dependencies(&self) -> &'h ReadDependencyRequestV3 {
        self.dependencies
    }
    /// An uninterpreted requested profile, not proof it is supported/current.
    pub fn execution_profile(&self) -> &str {
        &self.execution
    }
    pub fn mutation(&self) -> &AnnotationMutation {
        &self.mutation
    }
    pub fn argument_declaration_v1(&self) -> &NativeArgumentDeclarationV1 {
        &self.declaration
    }
    pub fn owned_bytes(&self) -> usize {
        self.bytes
    }
    pub fn limits(&self) -> AnnotationAuthoringArgumentLimitsV1 {
        self.limits
    }
    pub fn required_owned_bytes(body_bytes: usize, profile_bytes: usize) -> Result<usize> {
        size_of::<Self>()
            .checked_add(body_bytes)
            .and_then(|n| n.checked_add(profile_bytes))
            .ok_or(InvocationError::Limit)
    }
}
type V<'a, 'h> = AnnotationAuthoringEnvelopeValueV1<'a, 'h>;
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
fn version(v: V<'_, '_>) -> Result<u64> {
    match v {
        V::Version(n) if n > 0 => Ok(n),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn copied(s: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| InvocationError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
fn normalize<'a, 'h>(
    envelope: V<'a, 'h>,
    limits: AnnotationAuthoringArgumentLimitsV1,
) -> Result<NormalizedAnnotationAuthoringArgumentsV1<'h>> {
    let V::Object(rows) = envelope else {
        return Err(InvocationError::MalformedIntent);
    };
    if !matches!(rows.len(), 8 | 9) {
        return Err(InvocationError::MalformedIntent);
    }
    let operation = text(field(rows, "operation")?)?;
    let handler = native_annotation_authoring_declaration_v1();
    let variant = handler
        .variants
        .iter()
        .find(|v| v.operation == operation)
        .ok_or(InvocationError::MalformedIntent)?;
    // The actual mutation owner supplies both the closed vocabulary and fields;
    // this is not a second independently maintained descriptive schema.
    let declaration = NativeArgumentDeclarationV1 {
        contract: handler.contract,
        variant: *variant,
    };
    if !declaration.fields_match(rows.iter().map(|(key, _)| *key))
        || text(field(rows, "contract")?)? != declaration.contract
    {
        return Err(InvocationError::MalformedIntent);
    }
    let V::Invocation(invocation) = field(rows, "invocation")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let V::Document(document) = field(rows, "document")? else {
        return Err(InvocationError::MalformedIntent);
    };
    let V::Dependencies(dependencies) = field(rows, "dependencies")? else {
        return Err(InvocationError::MalformedIntent);
    };
    if dependencies.predicates.len() > limits.max_dependencies
        || dependencies.predicates.len() > dependencies.budget.max_predicates
    {
        return Err(InvocationError::Limit);
    }
    let execution = text(field(rows, "profile")?)?;
    if execution.is_empty() {
        return Err(InvocationError::MalformedIntent);
    }
    if execution.len() > limits.max_profile_bytes {
        return Err(InvocationError::Limit);
    }
    let body = if matches!(operation, "create" | "update-body") {
        text(field(rows, "body")?)?
    } else {
        ""
    };
    if body.len() > limits.max_body_bytes {
        return Err(InvocationError::Limit);
    }
    let bytes = NormalizedAnnotationAuthoringArgumentsV1::required_owned_bytes(
        body.len(),
        execution.len(),
    )?;
    if bytes > limits.max_owned_bytes {
        return Err(InvocationError::Limit);
    }
    let annotation = if operation == "create" {
        None
    } else {
        let V::Annotation(annotation) = field(rows, "annotation")? else {
            return Err(InvocationError::MalformedIntent);
        };
        if annotation.document != *document {
            return Err(InvocationError::MalformedIntent);
        }
        Some(annotation)
    };
    let target = if matches!(operation, "create" | "reattach") {
        let V::Target(target) = field(rows, "target")? else {
            return Err(InvocationError::MalformedIntent);
        };
        if target.source.document != *document {
            return Err(InvocationError::MalformedIntent);
        }
        Some(target)
    } else {
        None
    };
    // All ordinary fields/roles/versions are validated before the first payload
    // copy. Live membership, source currency, point/range semantics and profile
    // support belong to the future owning preparation, not this passive guard.
    let content = if matches!(operation, "update-body" | "delete") {
        Some(version(field(rows, "expectedContent")?)?)
    } else {
        None
    };
    let generation = if matches!(operation, "reattach" | "delete") {
        Some(version(field(rows, "expectedTarget")?)?)
    } else {
        None
    };
    let mutation = match operation {
        "create" => AnnotationMutation::Create {
            body: copied(body)?,
            target: target.expect("validated target").clone(),
        },
        "update-body" => AnnotationMutation::UpdateBody {
            annotation: annotation.expect("validated annotation").clone(),
            expected_content: content.expect("validated version"),
            body: copied(body)?,
        },
        "reattach" => AnnotationMutation::Reattach {
            annotation: annotation.expect("validated annotation").clone(),
            expected_target: generation.expect("validated version"),
            target: target.expect("validated target").clone(),
        },
        "delete" => AnnotationMutation::Delete {
            annotation: annotation.expect("validated annotation").clone(),
            expected_content: content.expect("validated version"),
            expected_target: generation.expect("validated version"),
        },
        _ => return Err(InvocationError::MalformedIntent),
    };
    Ok(NormalizedAnnotationAuthoringArgumentsV1 {
        invocation,
        document,
        dependencies,
        execution: copied(execution)?,
        mutation,
        declaration,
        bytes,
        limits,
    })
}
/// No current-source lookup, native identity allocation, preparation/publication,
/// policy/profile activation, dependency interpretation, callback or I/O.
pub fn normalize_annotation_authoring_arguments_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    limits: AnnotationAuthoringArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedAnnotationAuthoringArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, limits);
    capture_timed(
        r,
        Boundary::AnnotationAuthoringArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
