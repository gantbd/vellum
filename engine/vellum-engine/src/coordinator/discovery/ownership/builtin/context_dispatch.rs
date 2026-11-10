//! The ten existing guarded F16 inputs, not another mutation owner or evaluator.
use super::*;
/// Exhaustive V1 native/scoped forms with their matching independent facts.
/// Basis roles stay opaque. Borrowing this sum grants no execution authority.
#[derive(Clone, Copy)]
pub enum NativeDiscoveredContextCommandV1<'a, 'h> {
    NativeValues {
        arguments: &'a NormalizedNativeSettingsArgumentsV1<'h>,
        facts: &'a TrustedNativeSettingsInvocationV1<'a>,
    },
    ScopedValues {
        arguments: &'a NormalizedScopedSettingsArgumentsV1<'h>,
        bases: &'a [SettingsWriteBasisId],
        facts: &'a TrustedSettingsMutationFactsV1<'a>,
    },
    NativeLifecycle {
        arguments: &'a NormalizedNativeLifecycleArgumentsV1<'h>,
        facts: &'a TrustedNativeSettingsLifecycleInvocationV1<'a>,
    },
    ScopedLifecycle {
        arguments: &'a NormalizedScopedLifecycleArgumentsV1<'h>,
        basis: &'a SettingsLifecycleBasisId,
        facts: &'a TrustedSettingsLifecycleMutationFactsV1<'a>,
    },
    NativeSerialization {
        arguments: &'a NormalizedSourceSerializationArgumentsV1<'h>,
        facts: &'a TrustedSerializationFactsV1<'a>,
    },
    ScopedSerialization {
        arguments: &'a NormalizedScopedSerializationArgumentsV1<'h>,
        basis: &'a SerializationBasisId,
        facts: &'a TrustedSerializationBasisFactsV1<'a>,
    },
    NativeDefinition {
        arguments: &'a NormalizedNativeDefinitionArgumentsV1<'h>,
        facts: &'a TrustedNativeSettingsDefinitionInvocationV1<'a>,
    },
    ScopedDefinition {
        arguments: &'a NormalizedScopedDefinitionArgumentsV1<'h>,
        basis: &'a SettingsDefinitionBasisId,
        facts: &'a TrustedSettingsDefinitionMutationFactsV1<'a>,
    },
    NativeRegistration {
        arguments: &'a NormalizedResourceRegistrationArgumentsV1<'h>,
        facts: &'a TrustedResourceRegistrationInvocationV1<'a>,
    },
    ScopedRegistration {
        arguments: &'a NormalizedScopedRegistrationArgumentsV1<'h>,
        bases: &'a [ResourceRegistrationBasisIdV1],
        facts: &'a TrustedScopedResourceRegistrationInvocationV1<'a>,
    },
}
impl NativeDiscoveredContextCommandV1<'_, '_> {
    pub const fn operation(&self) -> &'static str {
        match self {
            Self::NativeValues { .. }
            | Self::ScopedValues { .. }
            | Self::NativeLifecycle { .. }
            | Self::ScopedLifecycle { .. }
            | Self::NativeSerialization { .. }
            | Self::ScopedSerialization { .. } => "semantic-settings",
            Self::NativeDefinition { .. }
            | Self::ScopedDefinition { .. }
            | Self::NativeRegistration { .. }
            | Self::ScopedRegistration { .. } => "supported-registry",
        }
    }
}
impl SourceCoordinator {
    /// Same installed semantic-settings operation, with original issued-only
    /// result admission. Discovery visibility does not authorize creation or release.
    pub fn submit_discovered_native_lifecycle_with_created_results_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        arguments: &NormalizedNativeLifecycleArgumentsV1<'_>,
        context: &SettingsExecutionContextV1,
        facts: &TrustedNativeSettingsLifecycleInvocationV1<'_>,
        result: &TrustedCreatedLayerResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        self.validate_discovered_context_route(route, "semantic-settings", r)?;
        self.submit_normalized_native_lifecycle_with_created_results_v1(
            arguments, context, facts, result, r,
        )
        .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
    fn validate_discovered_context_route(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        operation: &str,
        r: &mut Recorder,
    ) -> Result<(), DiscoveredNativeCommandErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::context_selector_matches(c, route.entry, route.variant, operation)
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized.map_err(DiscoveredNativeCommandErrorV1::Discovery)?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )
        .map_err(DiscoveredNativeCommandErrorV1::Discovery)?;
        Ok(())
    }
    /// Fresh discovery release is separate from owning admission permission.
    /// Each arm uses its existing context/profile/basis/current-grant guards,
    /// parent funding and original replay, with no extra preparation or jobs.
    pub fn submit_discovered_context_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        context: &SettingsExecutionContextV1,
        command: NativeDiscoveredContextCommandV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        self.validate_discovered_context_route(route, command.operation(), r)?;
        match command {
            NativeDiscoveredContextCommandV1::NativeValues { arguments, facts } => {
                self.submit_normalized_native_settings_v1(arguments, context, facts, r)
            }
            NativeDiscoveredContextCommandV1::ScopedValues {
                arguments,
                bases,
                facts,
            } => self.submit_normalized_scoped_settings_v1(arguments, bases, context, facts, r),
            NativeDiscoveredContextCommandV1::NativeLifecycle { arguments, facts } => {
                self.submit_normalized_native_lifecycle_v1(arguments, context, facts, r)
            }
            NativeDiscoveredContextCommandV1::ScopedLifecycle {
                arguments,
                basis,
                facts,
            } => self.submit_normalized_scoped_lifecycle_v1(arguments, basis, context, facts, r),
            NativeDiscoveredContextCommandV1::NativeSerialization { arguments, facts } => {
                self.submit_normalized_source_serialization_v1(arguments, context, facts, r)
            }
            NativeDiscoveredContextCommandV1::ScopedSerialization {
                arguments,
                basis,
                facts,
            } => {
                self.submit_normalized_scoped_serialization_v1(arguments, basis, context, facts, r)
            }
            NativeDiscoveredContextCommandV1::NativeDefinition { arguments, facts } => {
                self.submit_normalized_native_definition_v1(arguments, context, facts, r)
            }
            NativeDiscoveredContextCommandV1::ScopedDefinition {
                arguments,
                basis,
                facts,
            } => self.submit_normalized_scoped_definition_v1(arguments, basis, context, facts, r),
            NativeDiscoveredContextCommandV1::NativeRegistration { arguments, facts } => {
                self.submit_normalized_resource_registration_v1(arguments, context, facts, r)
            }
            NativeDiscoveredContextCommandV1::ScopedRegistration {
                arguments,
                bases,
                facts,
            } => self.submit_normalized_scoped_registration_v1(arguments, bases, context, facts, r),
        }
        .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
