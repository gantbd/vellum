//! Closed trusted-native source arrays/read conjunction/scopes; no wire resolver.
use super::*;
use crate::capture::InertValueV1 as D;
pub const BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1: &str = "bounded-explicit-source-batch/1";
static ARGUMENTS: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "exact-source-batch/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "exact-batch",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("operation", "ClosedSourceVariant"),
            ("invocation", "InvocationIdentity"),
            ("profile", "ExecutionProfileRef"),
            ("sources", "NativeExplicitSourceWrites"),
            ("reads", "NativeReadConjunctionV3"),
            ("scopes", "NativeSourceScopesV1"),
        ],
    },
};
#[derive(Clone, Copy)]
pub enum ExactBatchEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, ExactBatchEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Invocation(&'h InvocationId),
    Sources(&'a [DocumentEdits]),
    Reads(&'a ReadDependencyRequestV3),
    Scopes(&'a [SourceOperationScope], SourceScopeBudget),
}
impl std::fmt::Debug for ExactBatchEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ExactBatchEnvelope(<redacted>)")
    }
}
pub struct NormalizedExactBatchArgumentsV1<'h> {
    pub(in crate::coordinator) invocation: &'h InvocationId,
    pub(in crate::coordinator) source: SourceIntent,
    pub(in crate::coordinator) reads: read_dependency::ReadCapture,
    pub(in crate::coordinator) scopes: scope::ScopeCapture,
    pub(in crate::coordinator) actor: SettingsExecutionContextV1,
    pub(in crate::coordinator) limits: CaptureLimitsV1,
}
impl std::fmt::Debug for NormalizedExactBatchArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedExactBatchArguments(<redacted>)")
    }
}
impl<'h> NormalizedExactBatchArgumentsV1<'h> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &ARGUMENTS;
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn execution_context(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn sources(&self) -> &[DocumentEdits] {
        &self.source.writes
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &ARGUMENTS
    }
}
type V<'a, 'h> = ExactBatchEnvelopeValueV1<'a, 'h>;
type Result<T> = std::result::Result<T, InvocationError>;
fn field<'a, 'h>(rows: &'a [(&str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(InvocationError::MalformedIntent)
}
fn text<'a>(v: V<'a, '_>) -> Result<&'a str> {
    if let V::Data(D::Text(s)) = v {
        Ok(s)
    } else {
        Err(InvocationError::MalformedIntent)
    }
}
fn natural(v: V<'_, '_>) -> Result<u64> {
    if let V::Data(D::Number(n)) = v
        && n.is_finite()
        && n >= 0.0
        && n.fract() == 0.0
        && n <= 9_007_199_254_740_991.0
    {
        return Ok(n as u64);
    }
    Err(InvocationError::MalformedIntent)
}
pub(in crate::coordinator) fn intent_bytes(
    source: &SourceIntent,
    reads: &read_dependency::ReadCapture,
    scopes: &scope::ScopeCapture,
    actor: &SettingsExecutionContextV1,
) -> Result<usize> {
    let mut n = size_of::<ExactBatchIntent>();
    let mut add = |bytes: usize| -> Result<()> {
        n = n.checked_add(bytes).ok_or(InvocationError::Limit)?;
        Ok(())
    };
    for label in basis::labels(actor) {
        add(label.len())?;
    }
    for w in &source.writes {
        add(size_of::<DocumentEdits>())?;
        for e in &w.edits {
            add(size_of::<Replacement>())?;
            add(e.expected.len())?;
            add(e.inserted.len())?;
        }
    }
    // Inline capture headers already occur in ExactBatchIntent.
    add(reads
        .logical_bytes()
        .ok_or(InvocationError::Limit)?
        .checked_sub(size_of::<read_dependency::ReadCapture>())
        .ok_or(InvocationError::Limit)?)?;
    add(scope::ScopeCapture::retained_bytes(&scopes.scopes)
        .ok_or(InvocationError::Limit)?
        .checked_sub(size_of::<scope::ScopeCapture>())
        .ok_or(InvocationError::Limit)?)?;
    Ok(n)
}
/// Count the closed native value graph before owned copies. Strings count as
/// scalar text (not wire byte arrays); opaque native identities count as leaves.
/// Byte units are native headers/records/payload, not JSON, allocator or RSS.
pub(in crate::coordinator) fn preflight(
    owner: &SourceCoordinator,
    sources: &[DocumentEdits],
    reads: &ReadDependencyRequestV3,
    scopes: &[SourceOperationScope],
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
) -> Result<(usize, usize, usize)> {
    let max = limits
        .max_bytes
        .min(owner.invocations.limits.max_intent_bytes);
    let mut bytes = size_of::<NormalizedExactBatchArgumentsV1<'_>>();
    // Request/context/capture-limit bundle plus the two native interpretation
    // budget records. Opaque identities are leaves, not serialized referents.
    let mut nodes = 28usize;
    let mut depth = 3usize;
    let mut add = |b: usize, n: usize| -> Result<()> {
        bytes = bytes
            .checked_add(b)
            .filter(|v| *v <= max)
            .ok_or(InvocationError::Limit)?;
        nodes = nodes
            .checked_add(n)
            .filter(|v| *v <= limits.max_nodes)
            .ok_or(InvocationError::Limit)?;
        Ok(())
    };
    add(0, 0)?;
    if sources.len() > owner.limits.max_participants
        || reads.predicates.len() > reads.budget.max_predicates
    {
        return Err(InvocationError::Limit);
    }
    for s in basis::labels(actor) {
        if s.is_empty() {
            return Err(InvocationError::MalformedIntent);
        }
        add(s.len(), 0)?;
    }
    for w in sources {
        depth = depth.max(5);
        add(size_of::<DocumentEdits>(), 5)?;
        if w.edits.len() > owner.limits.source.max_edits {
            return Err(InvocationError::Limit);
        }
        for e in &w.edits {
            depth = depth.max(7);
            if e.start > e.end || e.expected.len() != e.end - e.start {
                return Err(InvocationError::MalformedIntent);
            }
            add(size_of::<Replacement>(), 5)?;
            add(e.expected.len(), 0)?;
            add(e.inserted.len(), 0)?;
        }
    }
    for predicate in &reads.predicates {
        let (count, d) = match predicate {
            ReadDependencyV3::Base(_) => (4, 5),
            ReadDependencyV3::ConfigurationDomain { .. } => (9, 6),
            ReadDependencyV3::ConfigurationState { .. } => (8, 6),
        };
        depth = depth.max(d);
        add(size_of::<ReadDependencyV3>(), count)?;
    }
    for s in scopes {
        depth = depth.max(6);
        add(size_of::<SourceOperationScope>(), 6)?;
        for _ in &s.replacement_ranges {
            depth = depth.max(7);
            add(size_of::<ByteRange>(), 3)?;
        }
        for _ in &s.insertion_sites {
            depth = depth.max(7);
            add(size_of::<InsertionSiteRange>(), 3)?;
        }
    }
    if depth > limits.max_depth {
        return Err(InvocationError::Limit);
    }
    Ok((bytes, nodes, depth))
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn normalize_exact_batch<'a, 'h>(
        &self,
        value: V<'a, 'h>,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
    ) -> Result<NormalizedExactBatchArgumentsV1<'h>> {
        let V::Object(rows) = value else {
            return Err(InvocationError::MalformedIntent);
        };
        if !ARGUMENTS.fields_match(rows.iter().map(|(k, _)| *k))
            || text(field(rows, "contract")?)? != ARGUMENTS.contract
            || text(field(rows, "operation")?)? != ARGUMENTS.variant.operation
            || text(field(rows, "profile")?)? != BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1
        {
            return Err(InvocationError::MalformedIntent);
        }
        let V::Invocation(invocation) = field(rows, "invocation")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Sources(sources) = field(rows, "sources")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Reads(reads) = field(rows, "reads")? else {
            return Err(InvocationError::MalformedIntent);
        };
        let V::Scopes(scopes, budget) = field(rows, "scopes")? else {
            return Err(InvocationError::MalformedIntent);
        };
        preflight(self, sources, reads, scopes, actor, limits)?;
        let source = self.normalize_source_parts(sources, &[], false)?;
        let reads = read_dependency::ReadInput::V3(reads)
            .copy(&[])
            .map_err(|e| match e {
                ReadDependencyError::Capacity => InvocationError::Capacity,
                ReadDependencyError::Limit => InvocationError::Limit,
                _ => InvocationError::MalformedIntent,
            })?;
        let scopes = scope::ScopeCapture::copy(scopes, budget).map_err(|e| match e {
            SourceScopeError::Capacity => InvocationError::Capacity,
            SourceScopeError::Limit => InvocationError::Limit,
            _ => InvocationError::MalformedIntent,
        })?;
        let actor = basis::copy_context(actor).map_err(|_| InvocationError::Capacity)?;
        Ok(NormalizedExactBatchArgumentsV1 {
            invocation,
            source,
            reads,
            scopes,
            actor,
            limits,
        })
    }
    /// Native role projection only. Reads configuration limits but performs no
    /// current document lookup, private stamp capture or mutation authorization.
    pub fn normalize_exact_batch_arguments_v1<'a, 'h>(
        &self,
        value: V<'a, 'h>,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
        r: &mut Recorder,
    ) -> Result<NormalizedExactBatchArgumentsV1<'h>> {
        let timer = r.begin_service_timing();
        let result = self.normalize_exact_batch(value, actor, limits);
        capture_timed(
            r,
            Boundary::ExactBatchArgumentNormalization,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn normalize_exact_batch_envelope_v1<'a, 'h>(
        &self,
        value: V<'a, 'h>,
        actor: &SettingsExecutionContextV1,
        limits: CaptureLimitsV1,
        r: &mut Recorder,
    ) -> Result<NormalizedExactBatchArgumentsV1<'h>> {
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
            {
                return Err(InvocationError::MalformedIntent);
            }
            if (route == "typed" && natural(field(header, key)?)? != 10)
                || (route == "dynamic" && text(field(header, key)?)? != "change-source")
                || text(field(header, "variant")?)? != "exact-batch"
                || natural(field(header, "version")?)? != 1
            {
                return Err(InvocationError::MalformedIntent);
            }
            self.normalize_exact_batch(field(header, "arguments")?, actor, limits)
        })();
        capture_timed(
            r,
            Boundary::ExactBatchArgumentNormalization,
            &result,
            timer.finish(),
        );
        result
    }
}
