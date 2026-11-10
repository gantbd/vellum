//! Closed native F11 projection of the adopted four replacement fields.
use super::*;
use crate::coordinator::invocation::literal::arguments::{field, natural, string};
pub type ReplacementEnvelopeValueV1<'a, 'h> = LiteralEditEnvelopeValueV1<'a, 'h>;
type V<'a, 'h> = ReplacementEnvelopeValueV1<'a, 'h>;
const FIELDS: &[(&str, &str)] = &[
    ("operation", "ClosedReplacementVariant"),
    ("index", "SafeNaturalOrNull"),
    ("replacement", "LiteralText"),
    ("invocation", "InvocationIdentity"),
];
static DECLARATIONS: [NativeArgumentDeclarationV1; 2] = [
    NativeArgumentDeclarationV1 {
        contract: "literal-search/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "replace-current",
            fields: FIELDS,
        },
    },
    NativeArgumentDeclarationV1 {
        contract: "literal-search/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "replace-all",
            fields: FIELDS,
        },
    },
];
/// Original native roles plus immutable literal/context/limits. This descriptor
/// is not a read/mutation grant, source-name resolver or serialized identity.
pub struct NormalizedReplacementArgumentsV1<'h> {
    invocation: &'h InvocationId,
    command: SourceSearchConsumerV1,
    actor: SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
}
impl std::fmt::Debug for NormalizedReplacementArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedReplacementArguments(<redacted>)")
    }
}
impl<'h> NormalizedReplacementArgumentsV1<'h> {
    pub fn invocation(&self) -> &'h InvocationId {
        self.invocation
    }
    pub fn command(&self) -> &SourceSearchConsumerV1 {
        &self.command
    }
    pub fn execution_context(&self) -> &SettingsExecutionContextV1 {
        &self.actor
    }
    pub fn limits(&self) -> CaptureLimitsV1 {
        self.limits
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &DECLARATIONS[usize::from(self.command.index().is_none())]
    }
}
pub fn native_replacement_argument_declarations_v1() -> &'static [NativeArgumentDeclarationV1; 2] {
    &DECLARATIONS
}
fn normalize<'a, 'h>(
    value: V<'a, 'h>,
    result: &SourceSearchResultId,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedReplacementArgumentsV1<'h>, InvocationError> {
    let V::Object(rows) = value else {
        return Err(InvocationError::MalformedIntent);
    };
    if rows.len() != FIELDS.len()
        || FIELDS
            .iter()
            .any(|(k, _)| rows.iter().filter(|(name, _)| name == k).count() != 1)
    {
        return Err(InvocationError::MalformedIntent);
    }
    let op = string(field(rows, "operation")?)?;
    let index = match (op, field(rows, "index")?) {
        ("replace-all", V::Data(D::Null)) => None,
        ("replace-current", v) => {
            Some(usize::try_from(natural(v)?).map_err(|_| InvocationError::Limit)?)
        }
        _ => return Err(InvocationError::MalformedIntent),
    };
    let text = string(field(rows, "replacement")?)?;
    let V::Invocation(invocation) = field(rows, "invocation")? else {
        return Err(InvocationError::MalformedIntent);
    };
    // Bound the complete canonical native graph before copying literal/context.
    // The submission owner separately checks its native plan and parent budget.
    canonical_fields(op, index, text, actor, limits, limits.max_bytes, r)?;
    let command = SourceSearchConsumerV1::replacement_value(result, index, text).map_err(error)?;
    let actor = crate::coordinator::settings::basis::copy_context(actor)
        .map_err(|_| InvocationError::Capacity)?;
    Ok(NormalizedReplacementArgumentsV1 {
        invocation,
        command,
        actor,
        limits,
    })
}
pub fn normalize_replacement_arguments_v1<'a, 'h>(
    value: V<'a, 'h>,
    result: &SourceSearchResultId,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedReplacementArgumentsV1<'h>, InvocationError> {
    let timer = r.begin_service_timing();
    let out = normalize(value, result, actor, limits, r);
    capture_timed(
        r,
        Boundary::ReplacementArgumentNormalization,
        &out,
        timer.finish(),
    );
    out
}
pub fn normalize_replacement_envelope_v1<'a, 'h>(
    value: V<'a, 'h>,
    result: &SourceSearchResultId,
    actor: &SettingsExecutionContextV1,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedReplacementArgumentsV1<'h>, InvocationError> {
    let timer = r.begin_service_timing();
    let out = (|| {
        let V::Object(outer) = value else {
            return Err(InvocationError::MalformedIntent);
        };
        if outer.len() != 2
            || ["route", "request"]
                .iter()
                .any(|k| outer.iter().filter(|(name, _)| name == k).count() != 1)
        {
            return Err(InvocationError::MalformedIntent);
        }
        let route = string(field(outer, "route")?)?;
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
            || (route == "dynamic" && string(field(header, key)?)? != "change-source")
            || natural(field(header, "version")?)? != 1
        {
            return Err(InvocationError::MalformedIntent);
        }
        let args = normalize(field(header, "arguments")?, result, actor, limits, r)?;
        if string(field(header, "variant")?)? != args.argument_declaration_v1().variant.operation {
            return Err(InvocationError::MalformedIntent);
        }
        Ok(args)
    })();
    capture_timed(
        r,
        Boundary::ReplacementArgumentNormalization,
        &out,
        timer.finish(),
    );
    out
}
impl SourceCoordinator {
    #[allow(clippy::too_many_arguments)]
    pub fn submit_replacement_arguments_v1(
        &mut self,
        q: &NormalizedReplacementArgumentsV1<'_>,
        scope: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        if q.actor != scope.context {
            let refused = Err(InvocationError::Unavailable);
            capture(r, Boundary::ReturnedSearchReplacementAdmission, &refused);
            return refused;
        }
        self.submit_returned_search_replacement_v1(
            q.invocation,
            &q.command,
            scope,
            sf,
            f,
            cf,
            q.limits,
            r,
        )
    }
}
