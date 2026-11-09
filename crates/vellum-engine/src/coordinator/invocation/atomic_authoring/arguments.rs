//! Eight-field native mixed data. Current component bindings are a separate port.
use super::*;
use crate::capture::InertValueV1 as D;
pub const BOUNDED_ATOMIC_AUTHORING_PROFILE_V1: &str = "bounded-atomic-authoring/1";
#[derive(Clone, Copy)]
pub struct AtomicAnnotationComponentV1<'a> {
    pub document: &'a DocumentId,
    pub mutation: &'a AnnotationMutation,
    pub dependencies: &'a ReadDependencyRequestV3,
}
#[derive(Clone, Copy)]
pub enum AtomicAuthoringEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, AtomicAuthoringEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Sources(&'a [DocumentEdits]),
    Annotations(&'a [AtomicAnnotationComponentV1<'a>]),
    Reads(&'a ReadDependencyRequestV3),
    Scopes(&'a [SourceOperationScope], SourceScopeBudget),
}
pub(super) struct OriginalComponent {
    pub(super) document: DocumentId,
    pub(super) mutation: AnnotationMutation,
    pub(super) reads: read_dependency::ReadCapture,
}
pub struct NormalizedAtomicAuthoringArgumentsV1<'h> {
    pub(super) source: NormalizedExactBatchArgumentsV1<'h>,
    pub(super) components: Vec<OriginalComponent>,
    pub(super) bytes: usize,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("AtomicAuthoringArguments(<redacted>)") }
})+ }; }
redacted!(
    AtomicAnnotationComponentV1<'_>,
    AtomicAuthoringEnvelopeValueV1<'_, '_>,
    NormalizedAtomicAuthoringArgumentsV1<'_>
);
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "atomic-source-annotation/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "exact-batch",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSourceVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("sources", "NativeExplicitSourceWrites"),
            ("annotations", "NativeOrderedAtomicAnnotationComponents"),
            ("reads", "NativeReadConjunctionV3"),
            ("scopes", "NativeSourceScopesV1"),
        ],
    },
};
impl<'h> NormalizedAtomicAuthoringArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn invocation(&self) -> &'h InvocationId {
        self.source.invocation
    }
    pub fn execution_context(&self) -> &SettingsExecutionContextV1 {
        &self.source.actor
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &ARGUMENTS
    }
}
type V<'a, 'h> = AtomicAuthoringEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
fn field<'a, 'h>(rows: &'a [(&str, V<'a, 'h>)], k: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(name, _)| *name == k)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::MalformedIntent)
}
fn text<'a>(v: V<'a, '_>) -> Result<&'a str> {
    match v {
        V::Data(D::Text(s)) => Ok(s),
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn number(v: V<'_, '_>) -> Result<u64> {
    match v {
        V::Data(D::Number(n))
            if n.is_finite() && n >= 0.0 && n.fract() == 0.0 && n <= 9_007_199_254_740_991.0 =>
        {
            Ok(n as u64)
        }
        _ => Err(InvocationError::MalformedIntent),
    }
}
fn same_document(c: &AtomicAnnotationComponentV1<'_>) -> bool {
    match c.mutation {
        AnnotationMutation::Create { target, .. } => c.document == &target.source.document,
        AnnotationMutation::UpdateBody {
            annotation,
            expected_content,
            ..
        } => c.document == &annotation.document && *expected_content > 0,
        AnnotationMutation::Reattach {
            annotation,
            expected_target,
            target,
        } => {
            c.document == &annotation.document
                && c.document == &target.source.document
                && *expected_target > 0
        }
        AnnotationMutation::Delete {
            annotation,
            expected_content,
            expected_target,
        } => c.document == &annotation.document && *expected_content > 0 && *expected_target > 0,
    }
}
impl SourceCoordinator {
    fn normalize_atomic_authoring<'a, 'h>(
        &self,
        value: V<'a, 'h>,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
    ) -> Result<NormalizedAtomicAuthoringArgumentsV1<'h>> {
        let V::Object(rows) = value else {
            return Err(InvocationError::MalformedIntent);
        };
        if !ARGUMENTS.fields_match(rows.iter().map(|(k, _)| *k))
            || text(field(rows, "contract")?)? != ARGUMENTS.contract
            || text(field(rows, "operation")?)? != "exact-batch"
            || text(field(rows, "profile")?)? != BOUNDED_ATOMIC_AUTHORING_PROFILE_V1
        {
            return Err(InvocationError::MalformedIntent);
        }
        let V::Invocation(invocation) = field(rows, "invocation")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Sources(writes) = field(rows, "sources")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Annotations(components) = field(rows, "annotations")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Reads(reads) = field(rows, "reads")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Scopes(scopes, budget) = field(rows, "scopes")? else {
            return Err(InvocationError::MalformedIntent);
        };
        if components.is_empty() || components.len() > self.limits.max_participants {
            return Err(InvocationError::Limit);
        }
        let (mut bytes, mut nodes, mut depth) =
            exact_batch::arguments::preflight(self, writes, reads, scopes, actor, limits)?;
        // Additional mixed header and ordered annotation array. Native identities
        // are leaves, not public ID labels; text is scalar UTF-8, not wire arrays.
        bytes = add(
            bytes,
            size_of::<Vec<OriginalComponent>>() + size_of::<usize>(),
        )?;
        nodes = add(nodes, 1)?;
        for c in components {
            depth = depth.max(6); // component fields and native dependency budget
            if !same_document(c) {
                return Err(InvocationError::MalformedIntent);
            }
            let target_nodes = match c.mutation {
                AnnotationMutation::Create { target, .. }
                | AnnotationMutation::Reattach { target, .. } => {
                    depth = depth.max(7);
                    5 + match target.target {
                        crate::attachment::AttachmentTarget::Document => 2,
                        crate::attachment::AttachmentTarget::Point { .. } => 5,
                        crate::attachment::AttachmentTarget::ContentRange { .. } => 4,
                    }
                }
                _ => 0,
            };
            nodes = add(nodes, add(10, target_nodes)?)?;
            bytes = add(
                bytes,
                add(size_of::<OriginalComponent>(), body_bytes(c.mutation))?,
            )?;
            if c.dependencies.predicates.len() > c.dependencies.budget.max_predicates {
                return Err(InvocationError::Limit);
            }
            for read in &c.dependencies.predicates {
                let n = match read {
                    ReadDependencyV3::Base(_) => {
                        depth = depth.max(7);
                        4
                    }
                    ReadDependencyV3::ConfigurationDomain { .. } => {
                        depth = depth.max(8);
                        9
                    }
                    ReadDependencyV3::ConfigurationState { .. } => {
                        depth = depth.max(8);
                        8
                    }
                };
                nodes = add(nodes, n)?;
                bytes = add(bytes, size_of::<ReadDependencyV3>())?;
            }
            if bytes
                > limits
                    .max_bytes
                    .min(self.invocations.limits.max_intent_bytes)
                || nodes > limits.max_nodes
            {
                return Err(InvocationError::Limit);
            }
        }
        if depth > limits.max_depth {
            return Err(InvocationError::Limit);
        }
        use ExactBatchEnvelopeValueV1 as E;
        let source_fields = [
            ("contract", E::Data(D::Text("exact-source-batch/1"))),
            ("operation", E::Data(D::Text("exact-batch"))),
            ("invocation", E::Invocation(invocation)),
            (
                "profile",
                E::Data(D::Text(BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1)),
            ),
            ("sources", E::Sources(writes)),
            ("reads", E::Reads(reads)),
            ("scopes", E::Scopes(scopes, budget)),
        ];
        let source = self.normalize_exact_batch(E::Object(&source_fields), actor, limits)?;
        if source
            .source
            .writes
            .len()
            .checked_add(components.len())
            .is_none_or(|n| n > self.limits.max_participants)
        {
            return Err(InvocationError::Limit);
        }
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(components.len())
            .map_err(|_| InvocationError::Capacity)?;
        for c in components {
            owned.push(OriginalComponent {
                document: c.document.clone(),
                mutation: annotation_scoped::copy_mutation(c.mutation)?,
                reads: read_dependency::ReadInput::V3(c.dependencies)
                    .copy(&[])
                    .map_err(read_error)?,
            });
        }
        // Sponsor canonical owned public data, not the incidental partitioning
        // of raw source rows. Raw byte/node/depth caps were checked above.
        let mut bytes = exact_batch::arguments::intent_bytes(
            &source.source,
            &source.reads,
            &source.scopes,
            &source.actor,
        )?;
        bytes = add(
            bytes,
            size_of::<Vec<OriginalComponent>>() + size_of::<usize>(),
        )?;
        for c in &owned {
            bytes = add(
                bytes,
                add(size_of::<OriginalComponent>(), body_bytes(&c.mutation))?,
            )?;
            bytes = add(
                bytes,
                c.reads
                    .logical_bytes()
                    .and_then(|n| n.checked_sub(size_of::<read_dependency::ReadCapture>()))
                    .ok_or(InvocationError::Limit)?,
            )?;
        }
        if bytes
            > limits
                .max_bytes
                .min(self.invocations.limits.max_intent_bytes)
        {
            return Err(InvocationError::Limit);
        }
        Ok(NormalizedAtomicAuthoringArgumentsV1 {
            source,
            components: owned,
            bytes,
        })
    }
    pub fn normalize_atomic_authoring_arguments_v1<'a, 'h>(
        &self,
        value: V<'a, 'h>,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
        r: &mut Recorder,
    ) -> Result<NormalizedAtomicAuthoringArgumentsV1<'h>> {
        let timer = r.begin_service_timing();
        let result = self.normalize_atomic_authoring(value, actor, limits);
        capture_timed(
            r,
            Boundary::AtomicAuthoringArgumentNormalization,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn normalize_atomic_authoring_envelope_v1<'a, 'h>(
        &self,
        value: V<'a, 'h>,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
        r: &mut Recorder,
    ) -> Result<NormalizedAtomicAuthoringArgumentsV1<'h>> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let V::Object(outer) = value else {
                return Err(InvocationError::MalformedIntent);
            };
            if outer.len() != 2
                || outer.iter().filter(|(k, _)| *k == "route").count() != 1
                || outer.iter().filter(|(k, _)| *k == "request").count() != 1
            {
                return Err(InvocationError::MalformedIntent);
            }
            let route = text(field(outer, "route")?)?;
            let key = match route {
                "typed" => "family",
                "dynamic" => "name",
                _ => return Err(InvocationError::MalformedIntent),
            };
            let V::Object(header) = field(outer, "request")? else {
                return Err(InvocationError::MalformedIntent);
            };
            if header.len() != 4
                || [key, "variant", "version", "arguments"]
                    .iter()
                    .any(|k| header.iter().filter(|(name, _)| name == k).count() != 1)
                || (route == "typed" && number(field(header, key)?)? != 10)
                || (route == "dynamic" && text(field(header, key)?)? != "change-source")
                || text(field(header, "variant")?)? != "exact-batch"
                || number(field(header, "version")?)? != 1
            {
                return Err(InvocationError::MalformedIntent);
            }
            self.normalize_atomic_authoring(field(header, "arguments")?, actor, limits)
        })();
        capture_timed(
            r,
            Boundary::AtomicAuthoringArgumentNormalization,
            &result,
            timer.finish(),
        );
        result
    }
}
