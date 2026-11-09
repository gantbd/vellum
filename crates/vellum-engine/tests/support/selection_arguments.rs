use SelectionEnvelopeValueV1 as V;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinates::CoordinateBudget;
use vellum_engine::coordinator::*;
pub fn limits() -> SelectionArgumentLimitsV1 {
    SelectionArgumentLimitsV1 {
        max_input_selections: 8,
        max_owned_bytes: 16384,
        endpoint: CoordinateBudget {
            max_scan_bytes: 4096,
        },
    }
}
pub fn header<'h>(
    id: &'h InvocationId,
    c: &'h LogicalViewContext,
    operation: &'h str,
) -> Vec<(&'h str, V<'h, 'h>)> {
    vec![
        ("contract", V::Data(D::Text("selection-edit/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("invocation", V::Invocation(id)),
        ("view", V::View(&c.view)),
        ("document", V::Document(&c.source.document)),
        (
            "sourceRevision",
            V::Data(D::Number(c.source.revision as f64)),
        ),
        (
            "expectedSelection",
            V::Data(D::Number(c.stamp.selection_generation as f64)),
        ),
        (
            "profile",
            V::Data(D::Text("explicit-positional-selection/1")),
        ),
    ]
}
