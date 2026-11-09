//! The native history direction vocabulary owns the reversal declaration.
use super::*;
const FIELDS: &[(&str, &str)] = &[
    ("contract", "RegisteredContractVersion"),
    ("operation", "ClosedHistoryReversalVariant"),
    ("invocation", "InvocationIdentity"),
    ("authority", "EngineAuthorityIncarnation"),
    ("target", "HistoryReversalTarget"),
    ("profile", "ExecutionProfileRef"),
    ("dependencies", "BoundedContextDependencyVector"),
];
static DECLARATION: NativeHandlerDeclarationV1 = NativeHandlerDeclarationV1 {
    family: "F14",
    contract: "history-reversal/1",
    variants: &[
        NativeHandlerVariantDeclarationV1 {
            operation: "undo",
            fields: FIELDS,
        },
        NativeHandlerVariantDeclarationV1 {
            operation: "redo",
            fields: FIELDS,
        },
    ],
    outcomes: &[("applied", "applied"), ("not-published", "not-published")],
    admission: &[
        "authorized",
        "bounded",
        "notStopped",
        "interpretationCurrent",
        "companionsReady",
        "versionsAvailable",
        "historyReady",
        "invocationFresh",
    ],
    current_predicates: "group-frontier-participant-view-and-owner-specific",
    limits: &[
        "maxDependencies",
        "maxParticipants",
        "maxChanges",
        "maxPlanBytes",
    ],
    limit_values: "current-execution-and-view-profiles",
    version_capacity: None,
};
pub fn native_history_reversal_declaration_v1() -> &'static NativeHandlerDeclarationV1 {
    &DECLARATION
}
impl HistoryDirection {
    pub fn declaration_v1(self) -> &'static NativeHandlerVariantDeclarationV1 {
        &DECLARATION.variants[match self {
            Self::Undo => 0,
            Self::Redo => 1,
        }]
    }
}
