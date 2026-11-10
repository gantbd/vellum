//! Fixed F13/F14 declaration source. Visibility/advice are independent facts;
//! metadata cannot register, replace or invoke a handler.
use super::*;
use InertValueV1 as D;
mod context;
pub use context::*;
mod editing;
pub use editing::*;
mod lifecycle;
pub use lifecycle::*;
mod ordinary_dispatch;
pub use ordinary_dispatch::*;
mod annotation_dispatch;
pub use annotation_dispatch::*;
mod reversal_dispatch;
pub use reversal_dispatch::*;
mod context_dispatch;
pub use context_dispatch::*;
mod annotation_load;
mod containers;
mod ordinary_save;
mod read_admission;
pub use read_admission::{
    DiscoveredReadOperationAdmissionV1, DiscoveredReadOperationErrorV1,
    NativeDiscoveredReadCommandV1,
};
mod atomic_authoring;
mod catalog_read;
mod composition;
mod compound_advice;
mod diagnostic_drain;
mod discovery_query;
mod exact_batch;
mod literal_search;
pub use literal_search::DiscoveredLiteralSearchErrorV1;
mod inspection_read;
mod replacement;
pub use diagnostic_drain::DiscoveredDiagnosticDrainErrorV1;
mod selection;
mod source_read;
pub use source_read::DiscoveredSourceReadErrorV1;
mod invocation_cancellation;
mod shutdown_disposition;
pub use shutdown_disposition::DiscoveredShutdownDispositionErrorV1;
mod invocation_observation;
mod native_operation;
pub use native_operation::DiscoveredNativeOperationErrorV1;
mod settings_read;
pub use catalog_read::DiscoveredCatalogReadErrorV1;
pub use settings_read::DiscoveredSettingsReadErrorV1;
mod observation_control;
pub use invocation_observation::DiscoveredInvocationObservationErrorV1;
mod observation_revocation;
pub use invocation_cancellation::DiscoveredInvocationCancellationErrorV1;
pub use observation_control::DiscoveredObservationControlErrorV1;
mod registry_command;
mod source_publication;
pub use source_publication::*;
mod facet_publication;
pub use facet_publication::*;
mod collection_publication;
pub use collection_publication::*;
mod registry_receipt;
pub use observation_revocation::DiscoveredObservationRevocationErrorV1;
pub use registry_command::DiscoveredRegistryCommandErrorV1;
mod retained_release;
pub use registry_receipt::DiscoveredRegistryReceiptErrorV1;
mod source_load;
mod source_rebind;
mod source_save_as;
pub use ordinary_save::{DiscoveredSaveOperationErrorV1, NativeDiscoveredOrdinarySaveCommandV1};
#[cfg(test)]
mod tests;

// Independently prepaid logical staging, not allocator/physical-stack or RSS.
// Four bounded owned inspection strings plus the complete preparation frame.
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(4, maximum)?,
        size_of::<[String; 4]>(),
        size_of::<[NativeBuiltinDiscoveryRowV1; 2]>(),
        size_of::<[[(&str, D<'_>); 9]; 6]>(),
        size_of::<[[(&str, D<'_>); 3]; 6]>(),
        size_of::<[[D<'_>; 8]; 2]>() * 2,
        size_of::<[[(&str, D<'_>); 3]; 2]>(),
        size_of::<[[(&str, D<'_>); 2]; 2]>(),
        size_of::<[(&str, D<'_>); 3]>(),
        size_of::<[D<'_>; 2]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 6]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 2]>(),
        size_of::<[&NativeHandlerVariantDeclarationV1; 6]>(),
        size_of::<[&NativeHandlerDeclarationV1; 2]>(),
        size_of::<TrustedDiscoveryFactsV1>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        // These preparations are synchronous and owner-exclusive. Their fixed
        // stack frames cannot coexist or invoke external callbacks.
        context::workspace_bytes(maximum)?
            .max(invocation_observation::workspace_bytes(maximum)?)
            .max(settings_read::workspace_bytes(maximum)?)
            .max(catalog_read::workspace_bytes(maximum)?)
            .max(source_read::workspace_bytes(maximum)?)
            .max(selection::workspace_bytes(maximum)?)
            .max(discovery_query::workspace_bytes(maximum)?)
            .max(native_operation::workspace_bytes(maximum)?),
        editing::workspace_bytes(maximum)?,
        lifecycle::workspace_bytes(maximum)?,
        containers::workspace_bytes(maximum)?,
        retained_release::workspace_bytes(maximum)?,
        registry_receipt::workspace_bytes(maximum)?,
        registry_command::workspace_bytes(maximum)?,
        // Synchronous source/facet/collection preparations share one owner-exclusive frame.
        source_publication::workspace_bytes(maximum)?
            .max(facet_publication::workspace_bytes(maximum)?)
            .max(collection_publication::workspace_bytes(maximum)?)
            .max(observation_control::workspace_bytes(maximum)?),
        // Both are synchronous first-party preparations under exclusive owner
        // access, with no external callback or nested preparation. Their staging
        // frames cannot coexist; prepay the larger, not two simultaneous frames.
        observation_revocation::workspace_bytes(maximum)?
            .max(invocation_cancellation::workspace_bytes(maximum)?)
            .max(shutdown_disposition::workspace_bytes(maximum)?),
        source_rebind::workspace_bytes(maximum)?,
        source_save_as::workspace_bytes(maximum)?,
        ordinary_save::workspace_bytes(maximum)?,
        read_admission::workspace_bytes(maximum)?,
        source_load::workspace_bytes(maximum)?,
        annotation_load::workspace_bytes(maximum)?,
    ]
    .into_iter()
    .try_fold(0, add)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBuiltinVariantVisibilityV1 {
    pub order: Option<usize>,
    pub enabled: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBuiltinEntryVisibilityV1<const N: usize> {
    pub order: Option<usize>,
    pub schema: bool,
    pub presentation: bool,
    pub advice: bool,
    pub variants: [NativeBuiltinVariantVisibilityV1; N],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBuiltinDiscoveryVisibilityV1 {
    pub annotation: NativeBuiltinEntryVisibilityV1<4>,
    pub history: NativeBuiltinEntryVisibilityV1<2>,
}
/// Trusted-owner inspection, not an executable reference or recipient product.
pub struct NativeBuiltinDiscoveryRowV1 {
    pub authority: String,
    pub command: String,
    pub incarnation: u64,
    pub declaration: &'static NativeHandlerDeclarationV1,
}
impl std::fmt::Debug for NativeBuiltinDiscoveryRowV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeBuiltinDiscoveryRowV1(<owner metadata>)")
    }
}
fn declarations() -> [&'static NativeHandlerDeclarationV1; 2] {
    [
        native_annotation_authoring_declaration_v1(),
        native_history_reversal_declaration_v1(),
    ]
}
fn catalog_capacity(l: DiscoveryLimitsV1) -> Result<(), Error> {
    if l.max_entries < 2 || l.max_variants < 4 {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
fn command(engine: &str, family: &str, maximum: usize) -> Result<String, Error> {
    // Owner-local spelling only, not the reference model's JSON or a wire ID.
    let size = add(add(engine.len(), "\0builtin\0".len())?, family.len())?;
    if size > maximum {
        return Err(Error::Limit);
    }
    let mut out = String::new();
    out.try_reserve_exact(size).map_err(|_| Error::Capacity)?;
    out.push_str(engine);
    out.push_str("\0builtin\0");
    out.push_str(family);
    Ok(out)
}
impl SourceCoordinator {
    pub fn native_builtin_discovery_rows_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<[NativeBuiltinDiscoveryRowV1; 2], Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            if f.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let s = self.discovery_store()?;
            catalog_capacity(s.limits)?;
            let [a, h] = declarations();
            Ok([
                NativeBuiltinDiscoveryRowV1 {
                    authority: text(&s.authority)?,
                    command: command(&s.engine, a.family, s.limits.max_context_bytes)?,
                    incarnation: 1,
                    declaration: a,
                },
                NativeBuiltinDiscoveryRowV1 {
                    authority: text(&s.authority)?,
                    command: command(&s.engine, h.family, s.limits.max_context_bytes)?,
                    incarnation: 1,
                    declaration: h,
                },
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

    /// Discharges only accepted-source and declaration-origin premises. All
    /// authority/visibility/resource/target facts remain independent and strict.
    pub fn prepare_native_builtin_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinDiscoveryVisibilityV1,
        target: Option<DiscoveryTargetInputV1<'_>>,
        advice_present: bool,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            let s = self.discovery_store()?;
            catalog_capacity(s.limits)?;
            let rows = [
                command(&s.engine, "F13", s.limits.max_context_bytes)?,
                command(&s.engine, "F14", s.limits.max_context_bytes)?,
            ];
            let authority = text(&s.authority)?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*f
            };
            with_inputs(
                &authority,
                &rows,
                visibility,
                target,
                advice_present,
                |input| {
                    self.prepare_discovery_cut_from_source(
                        scope,
                        input,
                        &facts,
                        Origin::BuiltinV1,
                        r,
                    )
                },
            )
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

// Fixed six-variant workspace; no caller-sized metadata vector or executable
// callback. `consume` is this module's private immediate handoff to the copier.
fn with_inputs<T>(
    authority: &str,
    commands: &[String; 2],
    visibility: &NativeBuiltinDiscoveryVisibilityV1,
    target: Option<DiscoveryTargetInputV1<'_>>,
    advice_present: bool,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let ds = declarations();
    let variants: [&NativeHandlerVariantDeclarationV1; 6] = std::array::from_fn(|i| {
        if i < 4 {
            &ds[0].variants[i]
        } else {
            &ds[1].variants[i - 4]
        }
    });
    let fields: [[(&str, D<'_>); 9]; 6] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            variants[i]
                .fields
                .get(j)
                .map_or(("", D::Null), |(k, v)| (*k, D::Text(v)))
        })
    });
    let inputs: [[(&str, D<'_>); 3]; 6] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(ds[usize::from(i >= 4)].contract)),
            ("variant", D::Text(variants[i].operation)),
            ("fields", D::Object(&fields[i][..variants[i].fields.len()])),
        ]
    });
    let gates: [[D<'_>; 8]; 2] = std::array::from_fn(|i| {
        std::array::from_fn(|j| ds[i].admission.get(j).map_or(D::Null, |v| D::Text(v)))
    });
    let requirements: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("admission", D::Array(&gates[i][..ds[i].admission.len()])),
            ("currentPredicates", D::Text(ds[i].current_predicates)),
            (
                "versionCapacity",
                ds[i].version_capacity.map_or(D::Null, D::Text),
            ),
        ]
    });
    let names: [[D<'_>; 8]; 2] = std::array::from_fn(|i| {
        std::array::from_fn(|j| ds[i].limits.get(j).map_or(D::Null, |v| D::Text(v)))
    });
    let limits: [[(&str, D<'_>); 2]; 2] = std::array::from_fn(|i| {
        [
            ("names", D::Array(&names[i][..ds[i].limits.len()])),
            ("values", D::Text(ds[i].limit_values)),
        ]
    });
    let annotation_outcomes: [(&str, D<'_>); 3] =
        std::array::from_fn(|i| (ds[0].outcomes[i].0, D::Text(ds[0].outcomes[i].1)));
    let history_outcomes: [D<'_>; 2] = std::array::from_fn(|i| D::Text(ds[1].outcomes[i].0));
    let inputs: [DiscoveryVariantInputV1<'_>; 6] = std::array::from_fn(|i| {
        let family = usize::from(i >= 4);
        let v = if family == 0 {
            visibility.annotation.variants[i]
        } else {
            visibility.history.variants[i - 4]
        };
        DiscoveryVariantInputV1 {
            id: variants[i].operation,
            input: D::Object(&inputs[i]),
            outcomes: if family == 0 {
                D::Object(&annotation_outcomes)
            } else {
                D::Array(&history_outcomes)
            },
            requirements: D::Object(&requirements[family][..if family == 0 { 3 } else { 2 }]),
            limits: D::Object(&limits[family]),
            cancellation: D::Text(
                "owned-prepublication-stop; accepted-publication-is-not-retracted",
            ),
            order: v.order,
            enabled: v.enabled,
        }
    });
    let entries = [
        DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority,
                id: &commands[0],
                incarnation: 1,
            },
            contract: ds[0].contract,
            version: 1,
            kind: DiscoveryExecutionKindV1::LocalCommand,
            variants: &inputs[..4],
            presentation: D::Object(&[]),
            order: visibility.annotation.order,
            schema_visible: visibility.annotation.schema,
            presentation_visible: visibility.annotation.presentation,
            advice_visible: visibility.annotation.advice,
        },
        DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority,
                id: &commands[1],
                incarnation: 1,
            },
            contract: ds[1].contract,
            version: 1,
            kind: DiscoveryExecutionKindV1::LocalCommand,
            variants: &inputs[4..],
            presentation: D::Object(&[]),
            order: visibility.history.order,
            schema_visible: visibility.history.schema,
            presentation_visible: visibility.history.presentation,
            advice_visible: visibility.history.advice,
        },
    ];
    consume(DiscoveryCutInputV1 {
        entries: &entries,
        target,
        advice_present,
    })
}
