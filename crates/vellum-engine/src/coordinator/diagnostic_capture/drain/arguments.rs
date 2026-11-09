//! Four-field original-task request. Already-decoded native values only; no
//! label registry, callback, input payload allocation or identity wire codec.
use super::*;
use crate::capture::InertValueV1 as D;
#[derive(Clone, Copy)]
pub enum DiagnosticDrainEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, DiagnosticDrainEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Task(&'h DiagnosticDrainTaskV1),
    Generation(&'h DiagnosticGenerationV1),
}
type V<'a, 'h> = DiagnosticDrainEnvelopeValueV1<'a, 'h>;
static DECLARATION: NativeArgumentDeclarationV1 = NativeArgumentDeclarationV1 {
    contract: "host-diagnostic-drain/1",
    variant: NativeHandlerVariantDeclarationV1 {
        operation: "authorized-diagnostic-drain",
        fields: &[
            ("contract", "RegisteredContractVersion"),
            ("task", "OriginalDiagnosticDrainTask"),
            ("generation", "OriginalDiagnosticObservationGeneration"),
            ("limit", "SafePositiveNatural"),
        ],
    },
};
pub struct NormalizedDiagnosticDrainArgumentsV1<'h> {
    task: &'h DiagnosticDrainTaskV1,
    generation: &'h DiagnosticGenerationV1,
    limit: usize,
}
impl std::fmt::Debug for DiagnosticDrainEnvelopeValueV1<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiagnosticDrainEnvelope(<redacted>)")
    }
}
impl std::fmt::Debug for NormalizedDiagnosticDrainArgumentsV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NormalizedDiagnosticDrain(<redacted>)")
    }
}
impl<'h> NormalizedDiagnosticDrainArgumentsV1<'h> {
    pub fn task(&self) -> &'h DiagnosticDrainTaskV1 {
        self.task
    }
    pub fn generation(&self) -> &'h DiagnosticGenerationV1 {
        self.generation
    }
    pub fn limit(&self) -> usize {
        self.limit
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        &DECLARATION
    }
}
pub fn native_diagnostic_drain_argument_declaration_v1() -> &'static NativeArgumentDeclarationV1 {
    &DECLARATION
}
fn fields<'a, 'h>(v: V<'a, 'h>, names: &[&str]) -> Result<&'a [(&'a str, V<'a, 'h>)]> {
    let V::Object(rows) = v else {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    };
    if rows.len() != names.len()
        || names
            .iter()
            .any(|k| rows.iter().filter(|(name, _)| name == k).count() != 1)
    {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    }
    Ok(rows)
}
fn get<'a, 'h>(rows: &'a [(&'a str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(DiagnosticDrainErrorV1::Unavailable)
}
fn text(v: V<'_, '_>, expected: &str) -> bool {
    matches!(v, V::Data(D::Text(s)) if s == expected)
}
fn normalize<'a, 'h>(value: V<'a, 'h>) -> Result<NormalizedDiagnosticDrainArgumentsV1<'h>> {
    let rows = fields(value, &["contract", "task", "generation", "limit"])?;
    if !text(get(rows, "contract")?, DECLARATION.contract) {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    }
    let V::Task(task) = get(rows, "task")? else {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    };
    let V::Generation(generation) = get(rows, "generation")? else {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    };
    let V::Data(D::Number(n)) = get(rows, "limit")? else {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    };
    if !(1.0..=9_007_199_254_740_991.0).contains(&n) || n.fract() != 0.0 {
        return Err(DiagnosticDrainErrorV1::Unavailable);
    }
    let limit = usize::try_from(n as u64).map_err(|_| DiagnosticDrainErrorV1::Limit)?;
    Ok(NormalizedDiagnosticDrainArgumentsV1 {
        task,
        generation,
        limit,
    })
}
pub fn normalize_diagnostic_drain_arguments_v1<'a, 'h>(
    value: V<'a, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedDiagnosticDrainArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = normalize(value);
    record(
        r,
        Boundary::DiagnosticDrainArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
pub fn normalize_diagnostic_drain_envelope_v1<'a, 'h>(
    value: V<'a, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedDiagnosticDrainArgumentsV1<'h>> {
    let timer = r.begin_service_timing();
    let result = (|| {
        let outer = fields(value, &["route", "request"])?;
        let route = get(outer, "route")?;
        let typed = text(route, "typed");
        if !typed && !text(route, "dynamic") {
            return Err(DiagnosticDrainErrorV1::Unavailable);
        }
        let key = if typed { "family" } else { "name" };
        let header = fields(
            get(outer, "request")?,
            &[key, "variant", "version", "arguments"],
        )?;
        if (typed && !matches!(get(header, key)?, V::Data(D::Number(25.0))))
            || (!typed && !text(get(header, key)?, "request-host-facility"))
            || !text(get(header, "variant")?, DECLARATION.variant.operation)
            || !matches!(get(header, "version")?, V::Data(D::Number(1.0)))
        {
            return Err(DiagnosticDrainErrorV1::Unavailable);
        }
        normalize(get(header, "arguments")?)
    })();
    record(
        r,
        Boundary::DiagnosticDrainArgumentNormalization,
        &result,
        timer.finish(),
    );
    result
}
impl SourceCoordinator {
    pub fn submit_diagnostic_drain_arguments_v1(
        &mut self,
        q: &NormalizedDiagnosticDrainArgumentsV1<'_>,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainPhaseV1> {
        self.submit_diagnostic_drain_v1(q.task, q.generation, q.limit, scope, f, r)
    }
}
