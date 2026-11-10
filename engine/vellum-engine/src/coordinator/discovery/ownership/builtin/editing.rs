//! /3's fixed prefix and ordinary-edit row. No new executable registration.
use super::context::{entry, property, variant, with_inputs_v2};
use super::*;
#[cfg(test)]
mod tests;
use crate::coordinator::handler_declaration::{
    ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1, ordinary_workspace_bytes_v1, with_ordinary_descriptor_v1,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBuiltinDiscoveryVisibilityV3 {
    pub previous: NativeBuiltinDiscoveryVisibilityV2,
    pub ordinary: NativeBuiltinEntryVisibilityV1<1>,
}
/// Same trusted-owner identity product shape, not recipient references.
pub type NativeBuiltinDiscoveryRowV3 = NativeBuiltinDiscoveryRowV2;
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(8, maximum)?,
        size_of::<[NativeBuiltinDiscoveryRowV3; 4]>(),
        size_of::<[String; 5]>(),
        ordinary_workspace_bytes_v1(),
        size_of::<[DiscoveryVariantInputV1<'_>; 1]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 4]>(),
        size_of::<NativeBuiltinDiscoveryVisibilityV3>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn capacity(l: DiscoveryLimitsV1) -> Result<(), Error> {
    catalog_capacity(l)?;
    if l.max_entries < 4 {
        return Err(Error::Limit);
    }
    Ok(())
}
impl SourceCoordinator {
    pub fn native_builtin_discovery_rows_v3(
        &mut self,
        scope: &DiscoveryScopeV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<[NativeBuiltinDiscoveryRowV3; 4], Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V3 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            if f.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let s = self.discovery_store()?;
            capacity(s.limits)?;
            let identity = |family, contract| {
                Ok(NativeBuiltinDiscoveryRowV3 {
                    authority: text(&s.authority)?,
                    command: command(&s.engine, family, s.limits.max_context_bytes)?,
                    incarnation: 1,
                    family,
                    contract,
                })
            };
            let [a, h] = declarations();
            Ok([
                identity(a.family, a.contract)?,
                identity(h.family, h.contract)?,
                identity("F16", "engine-context-declaration/1")?,
                identity("F11", ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1)?,
            ])
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinInspection,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn prepare_native_builtin_discovery_cut_v3(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinDiscoveryVisibilityV3,
        target: Option<DiscoveryTargetInputV1<'_>>,
        advice_present: bool,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V3 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            let s = self.discovery_store()?;
            capacity(s.limits)?;
            let commands = [
                command(&s.engine, "F13", s.limits.max_context_bytes)?,
                command(&s.engine, "F14", s.limits.max_context_bytes)?,
                command(&s.engine, "F16", s.limits.max_context_bytes)?,
                command(&s.engine, "F11", s.limits.max_context_bytes)?,
            ];
            let authority = text(&s.authority)?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*f
            };
            with_inputs_v3(
                &authority,
                &commands,
                visibility,
                target,
                advice_present,
                |input| {
                    self.prepare_discovery_cut_from_source(
                        scope,
                        input,
                        &facts,
                        Origin::BuiltinV3,
                        r,
                    )
                },
            )?
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinPreparation,
            &result,
            timing.finish(),
        );
        result
    }
}
pub(super) fn with_inputs_v3<T>(
    authority: &str,
    commands: &[String; 4],
    visibility: &NativeBuiltinDiscoveryVisibilityV3,
    target: Option<DiscoveryTargetInputV1<'_>>,
    advice_present: bool,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> Result<T, Error> {
    let legacy_commands = [text(&commands[0])?, text(&commands[1])?];
    with_inputs_v2(
        authority,
        &legacy_commands,
        &commands[2],
        &visibility.previous,
        target,
        advice_present,
        |previous| {
            with_ordinary_descriptor_v1(|descriptor| {
                let D::Array(values) = property(descriptor, "variants")? else {
                    return Err(Error::Faulted);
                };
                let [value] = values else {
                    return Err(Error::Faulted);
                };
                let variants = [variant(*value, visibility.ordinary.variants[0])?];
                let rows = [
                    entry(&previous.entries[0]),
                    entry(&previous.entries[1]),
                    entry(&previous.entries[2]),
                    DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority,
                            id: &commands[3],
                            incarnation: 1,
                        },
                        contract: ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1,
                        version: 1,
                        kind: DiscoveryExecutionKindV1::LocalCommand,
                        variants: &variants,
                        presentation: property(descriptor, "presentation")?,
                        order: visibility.ordinary.order,
                        schema_visible: visibility.ordinary.schema,
                        presentation_visible: visibility.ordinary.presentation,
                        advice_visible: visibility.ordinary.advice,
                    },
                ];
                Ok(consume(DiscoveryCutInputV1 {
                    entries: &rows,
                    target: previous.target,
                    advice_present: previous.advice_present,
                }))
            })
            .map_err(|_| Error::Faulted)?
        },
    )?
}
