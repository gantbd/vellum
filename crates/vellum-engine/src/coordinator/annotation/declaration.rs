//! The four native mutation variants own their semantic description.
use super::*;
use NativeHandlerVariantDeclarationV1 as Variant;
macro_rules! fields {
    ($($name:literal => $kind:literal),* $(,)?) => {
        &[("contract", "RegisteredContractVersion"),
          ("operation", "ClosedAnnotationVariant"),
          ("invocation", "InvocationIdentity"),
          ("document", "LiveDocumentRef"),
          ("profile", "ExecutionProfileRef"),
          ("dependencies", "BoundedContextDependencyVector"),
          $(($name, $kind)),*]
    };
}
static DECLARATION: NativeHandlerDeclarationV1 = NativeHandlerDeclarationV1 {
    family: "F13",
    contract: "annotation-authoring/1",
    variants: &[
        Variant {
            operation: "create",
            fields: fields!("body" => "ScalarText", "target" => "LiveSourceTarget"),
        },
        Variant {
            operation: "update-body",
            fields: fields!("annotation" => "LiveAnnotationRef", "expectedContent" => "ContentVersion", "body" => "ScalarText"),
        },
        Variant {
            operation: "reattach",
            fields: fields!("annotation" => "LiveAnnotationRef", "expectedTarget" => "TargetGeneration", "target" => "LiveSourceTarget"),
        },
        Variant {
            operation: "delete",
            fields: fields!("annotation" => "LiveAnnotationRef", "expectedContent" => "ContentVersion", "expectedTarget" => "TargetGeneration"),
        },
    ],
    outcomes: &[
        (
            "applied",
            "One coherent publication plus relevant live/retained-absent record and changed version facts; protected bounded receipt release",
        ),
        (
            "unchanged",
            "Validated complete semantic identity with current version facts and no publication bookkeeping",
        ),
        (
            "not-published",
            "Confirmed local prepublication refusal/stop and recipient-safe explanation",
        ),
    ],
    admission: &[
        "authorized",
        "bounded",
        "notStopped",
        "interpretationCurrent",
        "companionsReady",
        "historyReady",
        "annotationWritable",
    ],
    current_predicates: "operation-and-owner-specific",
    limits: &[
        "maxRootBytes",
        "maxDocuments",
        "maxRecords",
        "maxContexts",
        "maxDependencies",
        "maxBodyBytes",
        "maxRequestBytes",
        "maxPlanBytes",
    ],
    limit_values: "current-execution-profile",
    version_capacity: Some("required-for-changing-publication"),
};
/// Static logical metadata. Not a request validator or live authority decision.
pub fn native_annotation_authoring_declaration_v1() -> &'static NativeHandlerDeclarationV1 {
    &DECLARATION
}
impl AnnotationMutation {
    pub fn declaration_v1(&self) -> &'static NativeHandlerVariantDeclarationV1 {
        &DECLARATION.variants[match self {
            Self::Create { .. } => 0,
            Self::UpdateBody { .. } => 1,
            Self::Reattach { .. } => 2,
            Self::Delete { .. } => 3,
        }]
    }
}
