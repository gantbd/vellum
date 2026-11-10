//! Argument declarations belong to the actual decoders, not a host registry.
//! They describe only these inner products, not full F16 enablement or authority.
use super::*;

#[cfg(test)]
mod tests;

/// Compatibility name for the shared native inner argument description.
pub type NativeSettingsArgumentDeclarationV1 = NativeArgumentDeclarationV1;

pub(super) static VALUES: NativeSettingsArgumentDeclarationV1 =
    NativeSettingsArgumentDeclarationV1 {
        contract: "semantic-settings-activation/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "semantic-settings",
            fields: &[
                ("contract", "RegisteredContractVersion"),
                ("operation", "ClosedSettingsVariant"),
                ("invocation", "InvocationIdentity"),
                ("profile", "ExecutionProfileRef"),
                ("changes", "BoundedNativeSettingChanges"),
                ("dependencies", "BoundedContextDependencyVector"),
            ],
        },
    };
pub(super) static DEFINITION: NativeSettingsArgumentDeclarationV1 =
    NativeSettingsArgumentDeclarationV1 {
        contract: "engine-settings-definition/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "supported-registry",
            fields: &[
                ("contract", "RegisteredContractVersion"),
                ("operation", "ClosedSettingsVariant"),
                ("invocation", "InvocationIdentity"),
                ("profile", "ExecutionProfileRef"),
                ("expected", "NativeSettingsInterpretationEpochs"),
                ("definition", "NativeSettingsDefinition"),
                ("dependencies", "BoundedContextDependencyVector"),
            ],
        },
    };
pub(super) static LIFECYCLE: NativeSettingsArgumentDeclarationV1 =
    NativeSettingsArgumentDeclarationV1 {
        contract: "engine-settings-layers/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "semantic-settings",
            fields: &[
                ("contract", "RegisteredContractVersion"),
                ("operation", "ClosedSettingsVariant"),
                ("invocation", "InvocationIdentity"),
                ("profile", "ExecutionProfileRef"),
                ("expected", "NativeSettingsLayerBasis"),
                ("create", "BoundedNativeLayerCreations"),
                ("remove", "BoundedNativeLayerRemovals"),
                ("targets", "CompleteNativeTargetAssociations"),
                ("dependencies", "BoundedContextDependencyVector"),
            ],
        },
    };

/// Fixed-size, allocation-free outer guard shared by declaration and decoder.
/// Borrowed invocation/dependency roles and all nested semantics are checked by
/// the owning decoder; descriptive field kinds cannot manufacture capabilities.
pub(super) fn header(
    rows: &[(&str, NativeSettingsEnvelopeValueV1<'_, '_>)],
    declaration: &NativeSettingsArgumentDeclarationV1,
) -> Result<(), InvocationError> {
    use crate::capture::InertValueV1 as D;
    use NativeSettingsEnvelopeValueV1 as V;
    if !declaration.fields_match(rows.iter().map(|(key, _)| *key))
        || !rows.iter().any(|(key, value)| {
            *key == "contract" && matches!(value, V::Data(D::Text(v)) if *v == declaration.contract)
        })
        || !rows.iter().any(|(key, value)| {
            *key == "operation"
                && matches!(value, V::Data(D::Text(v)) if *v == declaration.variant.operation)
        })
    {
        return Err(InvocationError::MalformedIntent);
    }
    Ok(())
}

impl NormalizedNativeSettingsArgumentsV1<'_> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &VALUES;
    pub fn argument_declaration_v1(&self) -> &'static NativeSettingsArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
}
impl NormalizedNativeDefinitionArgumentsV1<'_> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &DEFINITION;
    pub fn argument_declaration_v1(&self) -> &'static NativeSettingsArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
}
impl NormalizedNativeLifecycleArgumentsV1<'_> {
    pub(in crate::coordinator) const ARGUMENT_DECLARATION: &'static NativeArgumentDeclarationV1 =
        &LIFECYCLE;
    pub fn argument_declaration_v1(&self) -> &'static NativeSettingsArgumentDeclarationV1 {
        Self::ARGUMENT_DECLARATION
    }
}
