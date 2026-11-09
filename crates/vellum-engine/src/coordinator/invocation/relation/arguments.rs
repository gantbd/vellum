//! Closed native container envelopes. No naming, recipient grant, UI or wire codec.
use super::*;
use crate::capture::InertValueV1 as D;
use std::mem::size_of;
pub const NATIVE_RELATION_LIFECYCLE_PROFILE_V1: &str = "native-relation-lifecycle/1";
const CREATE_FIELDS: &[(&str, &str)] = &[
    ("contract", "RegisteredContractVersion"),
    ("operation", "ClosedContainerKind"),
    ("invocation", "OriginalOpaqueInvocation"),
    ("profile", "ExecutionProfileRef"),
    ("action", "ClosedContainerAction"),
];
const RETIRE_FIELDS: &[(&str, &str)] = &[
    ("contract", "RegisteredContractVersion"),
    ("operation", "ClosedContainerKind"),
    ("invocation", "OriginalOpaqueInvocation"),
    ("profile", "ExecutionProfileRef"),
    ("action", "ClosedContainerAction"),
    ("target", "OriginalOpaqueContainer"),
    ("expected", "OriginalNativeContainerExpectation"),
];
const fn declaration(
    operation: &'static str,
    fields: &'static [(&'static str, &'static str)],
) -> NativeArgumentDeclarationV1 {
    NativeArgumentDeclarationV1 {
        contract: "relation-invocation/1",
        variant: NativeHandlerVariantDeclarationV1 { operation, fields },
    }
}
static DECLARATIONS: [NativeArgumentDeclarationV1; 4] = [
    declaration("view-container", CREATE_FIELDS),
    declaration("view-container", RETIRE_FIELDS),
    declaration("workspace-container", CREATE_FIELDS),
    declaration("workspace-container", RETIRE_FIELDS),
];
/// Exact accepted native forms, not executable registration or authority.
pub fn native_relation_argument_declarations_v1() -> &'static [NativeArgumentDeclarationV1; 4] {
    &DECLARATIONS
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationArgumentLimitsV1 {
    /// Fixed normalized header/command. Opaque Arc-backed identities are cloned
    /// without a new identity or heap payload; excludes borrowed input/referents,
    /// allocator/RSS and the existing handler's separately bounded preparation.
    pub max_owned_bytes: usize,
}
#[derive(Clone, Copy)]
pub enum RelationEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, RelationEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    View(&'h LogicalViewId),
    ViewStamp(&'h LogicalViewStamp),
    Workspace(&'h WorkspaceId),
    WorkspaceContext(&'h WorkspaceContext),
}
pub struct NormalizedRelationArgumentsV1<'h> {
    invocation: &'h InvocationId,
    command: RelationCommandV1,
    declaration: &'static NativeArgumentDeclarationV1,
}
impl std::fmt::Debug for RelationEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RelationEnvelopeValue(<redacted>)")
    }
}
impl std::fmt::Debug for NormalizedRelationArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedRelationArguments(<redacted>)")
    }
}
impl<'h> NormalizedRelationArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn request(&self) -> &RelationCommandV1 {
        &self.command
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        self.declaration
    }
    pub const fn required_owned_bytes() -> usize {
        size_of::<Self>()
    }
    pub fn owned_bytes(&self) -> usize {
        Self::required_owned_bytes()
    }
}
type V<'a, 'h> = RelationEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
fn field<'a, 'h>(rows: &[(&str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(name, _)| *name == key)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::MalformedIntent)
}
fn text<'a>(value: V<'a, '_>) -> Result<&'a str> {
    match value {
        V::Data(D::Text(v)) => Ok(v),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn normalize<'a, 'h>(
    envelope: V<'a, 'h>,
    limits: RelationArgumentLimitsV1,
) -> Result<NormalizedRelationArgumentsV1<'h>> {
    let V::Object(rows) = envelope else {
        return Err(InvocationError::MalformedIntent);
    };
    if !matches!(rows.len(), 5 | 7) {
        return Err(InvocationError::MalformedIntent);
    }
    let (index, retire, workspace) = match (
        text(field(rows, "operation")?)?,
        text(field(rows, "action")?)?,
    ) {
        ("view-container", "create") => (0, false, false),
        ("view-container", "retire") => (1, true, false),
        ("workspace-container", "create") => (2, false, true),
        ("workspace-container", "retire") => (3, true, true),
        _ => return Err(InvocationError::MalformedIntent),
    };
    let declaration = &DECLARATIONS[index];
    if !declaration.fields_match(rows.iter().map(|(name, _)| *name))
        || text(field(rows, "contract")?)? != declaration.contract
        || text(field(rows, "profile")?)? != NATIVE_RELATION_LIFECYCLE_PROFILE_V1
    {
        return Err(InvocationError::MalformedIntent);
    }
    let V::Invocation(invocation) = field(rows, "invocation")? else {
        return Err(InvocationError::MalformedIntent);
    };
    if NormalizedRelationArgumentsV1::required_owned_bytes() > limits.max_owned_bytes {
        return Err(InvocationError::Limit);
    }
    let command = match (workspace, retire) {
        (false, false) => RelationCommandV1::CreateView,
        (true, false) => RelationCommandV1::CreateWorkspace,
        (false, true) => {
            let (V::View(view), V::ViewStamp(expected)) =
                (field(rows, "target")?, field(rows, "expected")?)
            else {
                return Err(InvocationError::MalformedIntent);
            };
            RelationCommandV1::RetireView {
                view: view.clone(),
                expected: *expected,
            }
        }
        (true, true) => {
            let (V::Workspace(workspace), V::WorkspaceContext(expected)) =
                (field(rows, "target")?, field(rows, "expected")?)
            else {
                return Err(InvocationError::MalformedIntent);
            };
            if *workspace != expected.workspace {
                return Err(InvocationError::IntentMismatch);
            }
            RelationCommandV1::RetireWorkspace {
                expected: expected.clone(),
            }
        }
    };
    Ok(NormalizedRelationArgumentsV1 {
        invocation,
        command,
        declaration,
    })
}
fn observe_result<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(InvocationError::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time,
    });
}
/// Keeps original native target identities and full native expectations. This
/// trusted in-process adapter is not a safe-number stamp/opaque-selector codec,
/// lookup, permission decision or complete recipient-facing F17 facade.
pub fn normalize_relation_arguments_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    limits: RelationArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedRelationArgumentsV1<'h>> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, limits);
    observe_result(
        r,
        Boundary::RelationArgumentNormalization,
        &result,
        timing.finish(),
    );
    result
}
impl SourceCoordinator {
    /// Same normalized request and original result owner; disclosure is opt-in,
    /// independent of trusted-host command execution and current result release.
    pub fn submit_relation_arguments_with_result_v1(
        &mut self,
        arguments: &NormalizedRelationArgumentsV1<'_>,
        facts: &TrustedRelationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result =
            self.submit_relation_with_result_v1(arguments.invocation, &arguments.command, facts, r);
        observe_result(
            r,
            Boundary::RelationDecodedAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    /// Delegates to the existing trusted-owner job. Original preparation,
    /// currentness, replay, service, cancellation and retention rules still apply;
    /// no recipient authority or filtered receipt disclosure is established.
    pub fn submit_relation_arguments_v1(
        &mut self,
        arguments: &NormalizedRelationArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState> {
        let timing = r.begin_service_timing();
        let result = self.submit_relation(arguments.invocation, &arguments.command, r);
        observe_result(
            r,
            Boundary::RelationDecodedAdmission,
            &result,
            timing.finish(),
        );
        result
    }
}
