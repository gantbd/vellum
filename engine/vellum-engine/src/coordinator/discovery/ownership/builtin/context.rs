//! Fixed /2 source: the unchanged F13/F14 prefix plus the existing F16 product.
use super::*;
use crate::coordinator::handler_declaration::{
    context_workspace_bytes_v1, with_context_descriptor_v1,
};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBuiltinDiscoveryVisibilityV2 {
    pub legacy: NativeBuiltinDiscoveryVisibilityV1,
    pub context: NativeBuiltinEntryVisibilityV1<2>,
}
/// Trusted-owner identities only. Not a recipient reference or handler grant.
pub struct NativeBuiltinDiscoveryRowV2 {
    pub authority: String,
    pub command: String,
    pub incarnation: u64,
    pub family: &'static str,
    pub contract: &'static str,
}
impl std::fmt::Debug for NativeBuiltinDiscoveryRowV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeBuiltinDiscoveryRowV2(<owner metadata>)")
    }
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    // In addition to the simultaneous legacy frame. Inspection's six owned
    // strings conservatively also cover preparation's four owned strings.
    [
        mul(6, maximum)?,
        size_of::<[NativeBuiltinDiscoveryRowV2; 3]>(),
        size_of::<[String; 4]>(),
        context_workspace_bytes_v1(),
        size_of::<[DiscoveryVariantInputV1<'_>; 2]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 3]>(),
        size_of::<NativeBuiltinDiscoveryVisibilityV2>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn capacity(l: DiscoveryLimitsV1) -> Result<(), Error> {
    catalog_capacity(l)?;
    if l.max_entries < 3 {
        return Err(Error::Limit);
    }
    Ok(())
}
impl SourceCoordinator {
    pub fn native_builtin_discovery_rows_v2(
        &mut self,
        scope: &DiscoveryScopeV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<[NativeBuiltinDiscoveryRowV2; 3], Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V2 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            if f.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let s = self.discovery_store()?;
            capacity(s.limits)?;
            let identity = |family, contract| {
                Ok(NativeBuiltinDiscoveryRowV2 {
                    authority: text(&s.authority)?,
                    command: command(&s.engine, family, s.limits.max_context_bytes)?,
                    incarnation: 1,
                    family,
                    contract,
                })
            };
            let [annotation, history] = declarations();
            Ok([
                identity(annotation.family, annotation.contract)?,
                identity(history.family, history.contract)?,
                identity("F16", "engine-context-declaration/1")?,
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
    /// Same owner/bounds/issuer as /1. Only fixed declaration-source origin is
    /// established here; visibility, authority and advisory facts stay strict.
    pub fn prepare_native_builtin_discovery_cut_v2(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinDiscoveryVisibilityV2,
        target: Option<DiscoveryTargetInputV1<'_>>,
        advice_present: bool,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V2 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            let s = self.discovery_store()?;
            capacity(s.limits)?;
            let commands = [
                command(&s.engine, "F13", s.limits.max_context_bytes)?,
                command(&s.engine, "F14", s.limits.max_context_bytes)?,
            ];
            let context_command = command(&s.engine, "F16", s.limits.max_context_bytes)?;
            let authority = text(&s.authority)?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*f
            };
            with_inputs_v2(
                &authority,
                &commands,
                &context_command,
                visibility,
                target,
                advice_present,
                |input| {
                    self.prepare_discovery_cut_from_source(
                        scope,
                        input,
                        &facts,
                        Origin::BuiltinV2,
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

pub(super) fn property<'a>(value: D<'a>, name: &str) -> Result<D<'a>, Error> {
    let D::Object(rows) = value else {
        return Err(Error::Faulted);
    };
    rows.iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
        .ok_or(Error::Faulted)
}
pub(super) fn variant<'a>(
    value: D<'a>,
    visibility: NativeBuiltinVariantVisibilityV1,
) -> Result<DiscoveryVariantInputV1<'a>, Error> {
    let D::Text(id) = property(value, "id")? else {
        return Err(Error::Faulted);
    };
    Ok(DiscoveryVariantInputV1 {
        id,
        input: property(value, "input")?,
        outcomes: property(value, "outcomes")?,
        requirements: property(value, "requirements")?,
        limits: property(value, "limits")?,
        cancellation: property(value, "cancellation")?,
        order: visibility.order,
        enabled: visibility.enabled,
    })
}
pub(super) fn entry<'a>(e: &DiscoveryEntryInputV1<'a>) -> DiscoveryEntryInputV1<'a> {
    DiscoveryEntryInputV1 {
        command: e.command,
        contract: e.contract,
        version: e.version,
        kind: e.kind,
        variants: e.variants,
        presentation: e.presentation,
        order: e.order,
        schema_visible: e.schema_visible,
        presentation_visible: e.presentation_visible,
        advice_visible: e.advice_visible,
    }
}
// Private immediate handoff only. No copied intermediate schema tree or public
// callback. The complete nested legacy/context workspace is prepaid by owner.
pub(super) fn with_inputs_v2<T>(
    authority: &str,
    commands: &[String; 2],
    context_command: &str,
    visibility: &NativeBuiltinDiscoveryVisibilityV2,
    target: Option<DiscoveryTargetInputV1<'_>>,
    advice_present: bool,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> Result<T, Error> {
    with_inputs(
        authority,
        commands,
        &visibility.legacy,
        target,
        advice_present,
        |legacy| {
            with_context_descriptor_v1(|descriptor| {
                let D::Array(values) = property(descriptor, "variants")? else {
                    return Err(Error::Faulted);
                };
                let [a, b] = values else {
                    return Err(Error::Faulted);
                };
                let variants = [
                    variant(*a, visibility.context.variants[0])?,
                    variant(*b, visibility.context.variants[1])?,
                ];
                let D::Text(contract) = property(descriptor, "contract")? else {
                    return Err(Error::Faulted);
                };
                let rows = [
                    entry(&legacy.entries[0]),
                    entry(&legacy.entries[1]),
                    DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority,
                            id: context_command,
                            incarnation: 1,
                        },
                        contract,
                        version: 1,
                        kind: DiscoveryExecutionKindV1::LocalCommand,
                        variants: &variants,
                        presentation: property(descriptor, "presentation")?,
                        order: visibility.context.order,
                        schema_visible: visibility.context.schema,
                        presentation_visible: visibility.context.presentation,
                        advice_visible: visibility.context.advice,
                    },
                ];
                Ok(consume(DiscoveryCutInputV1 {
                    entries: &rows,
                    target: legacy.target,
                    advice_present: legacy.advice_present,
                }))
            })
            .map_err(|_| Error::Faulted)?
        },
    )
}
