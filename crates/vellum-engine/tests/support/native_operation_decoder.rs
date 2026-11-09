use NativeOperationEnvelopeValueV1 as V;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
use vellum_engine::coordinator::*;
pub fn limits() -> NativeOperationDecoderLimitsV1 {
    NativeOperationDecoderLimitsV1 {
        input: CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 256,
            max_depth: 8,
        },
        max_held_bytes: 262144,
    }
}
#[allow(clippy::too_many_arguments)] // Exact native envelope fixture fields.
pub fn envelope<T>(
    route: &str,
    variant: &str,
    task: NativeOperationTaskV1<'_>,
    context: &NativeOperationObservationIdV1,
    profile: &str,
    kind: &str,
    after: Option<&NativeOperationProgressIdV1>,
    budget: f64,
    run: impl FnOnce(&V<'_, '_>) -> T,
) -> T {
    let facet = [("kind", D::Text(kind))];
    let mut args = [
        (
            "contract",
            V::Data(D::Text(if variant == "retained-outcome" {
                "native-operation-receipt/1"
            } else {
                "native-operation-observation/1"
            })),
        ),
        ("operation", V::Data(D::Text(variant))),
        ("task", V::Task(task)),
        ("context", V::Context(context)),
        ("profile", V::Data(D::Text(profile))),
        ("", V::Data(D::Null)),
        ("", V::Data(D::Null)),
    ];
    let count = if variant == "retained-outcome" {
        args[5] = ("facet", V::Data(D::Object(&facet)));
        6
    } else if variant == "bounded-observation-wait" {
        args[5] = ("after", after.map_or(V::Data(D::Null), V::Progress));
        args[6] = ("budget", V::Data(D::Number(budget)));
        7
    } else {
        5
    };
    let header = [
        (
            if route == "typed" { "family" } else { "name" },
            V::Data(if route == "typed" {
                D::Number(5.0)
            } else {
                D::Text("inspect-execution")
            }),
        ),
        ("variant", V::Data(D::Text(variant))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args[..count])),
    ];
    let outer = [
        ("route", V::Data(D::Text(route))),
        ("request", V::Object(&header)),
    ];
    run(&V::Object(&outer))
}
