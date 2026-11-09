//! Native scoped queries. Stage selectors and the entire reply before issuance.
use super::*;
use crate::capture::CapturedValueV1 as Value;
pub(super) fn literal_search_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    d: &NativeArgumentDeclarationV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeLiteralSearchV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        d.contract,
        d.variant.operation,
        DiscoveryExecutionKindV1::Query,
    )
}
mod results;
pub(super) fn native_operation_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    d: &NativeArgumentDeclarationV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeOperationObservationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        d.contract,
        d.variant.operation,
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn discovery_query_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: DiscoveryOperationV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeDiscoveryQueryV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        operation.declaration().contract,
        operation.name(),
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn diagnostic_drain_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeDiagnosticDrainV1 {
        return Err(Error::Unavailable);
    }
    let d = native_diagnostic_drain_argument_declaration_v1();
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        d.contract,
        d.variant.operation,
        DiscoveryExecutionKindV1::EffectRequest,
    )
}
pub(super) fn replacement_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    declaration: &NativeArgumentDeclarationV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeReplacementV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        declaration.contract,
        declaration.variant.operation,
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn atomic_authoring_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeAtomicAuthoringV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "atomic-source-annotation/1",
        "exact-batch",
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn exact_batch_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeExactBatchV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "exact-source-batch/1",
        "exact-batch",
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn composition_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeCompositionV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "composition-commit/1",
        "accepted-composition-commit",
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn compound_advice_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeCompoundAdviceV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "proposal-validation/1",
        "advisory-validation",
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn selection_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    declaration: &NativeArgumentDeclarationV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeSelectionV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        declaration.contract,
        declaration.variant.operation,
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn inspection_read_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeInspectionReadV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        contract,
        operation,
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn source_read_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeSourceReadV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        contract,
        operation,
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn catalog_read_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeCatalogReadV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "resource-catalog-query/1",
        "capabilities",
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn settings_read_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeSettingsReadV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "configuration-query/1",
        operation,
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn invocation_observation_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeInvocationObservationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        contract,
        operation,
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn observation_control_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeObservationControlV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "read-context/1",
        operation,
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn collection_publication_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
    kind: DiscoveryExecutionKindV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeCollectionPublicationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(c, entry, variant, contract, operation, kind)
}

pub(super) fn invocation_cancellation_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeInvocationCancellationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        INVOCATION_CONTROL_PROFILE_V1,
        "request-cancellation",
        DiscoveryExecutionKindV1::LocalCommand,
    )
}

pub(super) fn shutdown_disposition_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeShutdownDispositionV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        SHUTDOWN_DISPOSITION_PROFILE_V1,
        "shutdown-disposition",
        DiscoveryExecutionKindV1::LocalCommand,
    )
}

pub(super) fn observation_revocation_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeObservationRevocationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "read-context/1",
        "revoke-observation-grant",
        DiscoveryExecutionKindV1::LocalCommand,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Entry(usize),
    Variant { entry: usize, index: usize },
    Cursor { limit: usize, start: usize },
}
pub(super) fn facet_publication_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
    kind: DiscoveryExecutionKindV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeFacetPublicationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(c, entry, variant, contract, operation, kind)
}
pub(super) fn source_publication_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
    kind: DiscoveryExecutionKindV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeSourcePublicationV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(c, entry, variant, contract, operation, kind)
}
pub(super) fn registry_command_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeRegistryCommandV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        RESOURCE_REGISTRY_COMMAND_PROFILE_V1,
        operation,
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
pub(super) fn registry_receipt_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeRegistryReceiptV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        RESOURCE_REGISTRY_RECEIPT_PROFILE_V1,
        "retained-outcome",
        DiscoveryExecutionKindV1::Query,
    )
}
pub(super) fn read_admission_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeReadAdmissionV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "engine-read-admission-declaration/1",
        operation,
        DiscoveryExecutionKindV1::EffectRequest,
    )
}
pub(super) fn ordinary_save_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeOrdinarySaveV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        "engine-ordinary-save-declaration/1",
        operation,
        DiscoveryExecutionKindV1::EffectRequest,
    )
}
pub(super) fn source_save_as_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeSaveAsV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        SourceSaveAsRequestV1::argument_declaration_v1().contract,
        "activate",
    )
}
pub(super) fn annotation_load_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeAnnotationLoadV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        native_annotation_load_argument_declarations_v1()[0].contract,
        operation,
    )
}
pub(super) fn source_load_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeSourceLoadV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        native_source_load_argument_declaration_v1().contract,
        "activate-loaded-candidate",
    )
}
pub(super) fn rebind_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeRebindV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        SourceRebindRequestV1::argument_declaration_v1().contract,
        "rebind",
    )
}
pub(super) fn retained_release_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if c.origin != Origin::NativeRetainedReleaseV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        RetainedReleaseOperationV1::SourceBinding
            .argument_declaration_v1()
            .contract,
        "dispose",
    )
}
pub(super) fn container_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::NativeContainersV1 {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(c, entry, variant, "relation-invocation/1", operation)
}
pub(super) fn ordinary_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
) -> Result<(), Error> {
    if !matches!(c.origin, Origin::BuiltinV3 | Origin::BuiltinV4) {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        crate::coordinator::handler_declaration::ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1,
        "ordinary-edit",
    )
}
pub(super) fn annotation_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if !matches!(
        c.origin,
        Origin::BuiltinV1 | Origin::BuiltinV2 | Origin::BuiltinV3 | Origin::BuiltinV4
    ) {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        native_annotation_authoring_declaration_v1().contract,
        operation,
    )
}
pub(super) fn reversal_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    direction: HistoryDirection,
) -> Result<(), Error> {
    if !matches!(
        c.origin,
        Origin::BuiltinV1 | Origin::BuiltinV2 | Origin::BuiltinV3 | Origin::BuiltinV4
    ) {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(
        c,
        entry,
        variant,
        native_history_reversal_declaration_v1().contract,
        direction.declaration_v1().operation,
    )
}
pub(super) fn context_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if !matches!(
        c.origin,
        Origin::BuiltinV2 | Origin::BuiltinV3 | Origin::BuiltinV4
    ) {
        return Err(Error::Unavailable);
    }
    installed_selector_matches(c, entry, variant, "engine-context-declaration/1", operation)
}
fn installed_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
) -> Result<(), Error> {
    installed_selector_matches_kind(
        c,
        entry,
        variant,
        contract,
        operation,
        DiscoveryExecutionKindV1::LocalCommand,
    )
}
fn installed_selector_matches_kind(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    contract: &str,
    operation: &str,
    kind: DiscoveryExecutionKindV1,
) -> Result<(), Error> {
    let Kind::Entry(index) = original(c, &entry.owner, &entry.marker, entry.slot)? else {
        return Err(Error::Unavailable);
    };
    let Kind::Variant {
        entry: vi,
        index: vv,
    } = original(c, &variant.owner, &variant.marker, variant.slot)?
    else {
        return Err(Error::Unavailable);
    };
    let row = c.rows.get(index).ok_or(Error::Unavailable)?;
    if vi != index
        || row.contract != contract
        || row.kind != kind
        || row.variants.get(vv).is_none_or(|v| v.id != operation)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
pub(super) fn lifecycle_selector_matches(
    c: &Context,
    entry: &DiscoveryEntryIdV1,
    variant: &DiscoveryVariantIdV1,
    operation: &str,
) -> Result<(), Error> {
    if c.origin != Origin::BuiltinV4 {
        return Err(Error::Unavailable);
    }
    let Kind::Entry(index) = original(c, &entry.owner, &entry.marker, entry.slot)? else {
        return Err(Error::Unavailable);
    };
    let Kind::Variant {
        entry: vi,
        index: vv,
    } = original(c, &variant.owner, &variant.marker, variant.slot)?
    else {
        return Err(Error::Unavailable);
    };
    let row = c.rows.get(index).ok_or(Error::Unavailable)?;
    let expected = match operation {
        "create-scratch" => "create-scratch",
        "attach-view" | "detach-view" => "attach-detach-view",
        "membership" => "membership",
        "dispose" => "dispose",
        _ => return Err(Error::Unavailable),
    };
    if vi != index
        || row.contract != builtin::NATIVE_LIFECYCLE_DISCOVERY_CONTRACT_V1
        || row.kind != DiscoveryExecutionKindV1::LocalCommand
        || row.variants.get(vv).is_none_or(|v| v.id != expected)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
pub(super) struct Selector {
    marker: Arc<()>,
    kind: Kind,
}
pub(super) fn validate_selectors(s: &Store) -> Result<(), Error> {
    let mut count = 0;
    for c in s.contexts.iter().flatten() {
        count = add(count, c.selectors.len())?;
        for (i, v) in c.selectors.iter().enumerate() {
            let valid = match v.kind {
                Kind::Entry(index) => index < c.rows.len(),
                Kind::Variant { entry, index } => {
                    c.rows.get(entry).is_some_and(|r| index < r.variants.len())
                }
                Kind::Cursor { limit, start } => {
                    limit > 0
                        && limit <= s.limits.max_page_rows
                        && start > 0
                        && start < c.rows.len()
                }
            };
            if !valid
                || size_of::<Selector>() > s.limits.max_selector_bytes
                || c.selectors[..i]
                    .iter()
                    .any(|p| p.kind == v.kind || Arc::ptr_eq(&p.marker, &v.marker))
            {
                return Err(Error::Faulted);
            }
        }
    }
    if count > s.limits.max_selectors || count > s.issued {
        return Err(Error::Faulted);
    }
    Ok(())
}

// Captures are immutable, finite acyclic trees admitted by the native copier.
// Object member order is irrelevant; array order is significant. No Debug,
// recursive serializer, callback, executable schema, or identity hash is used.
fn metadata_equal(a: &InertCaptureV1, b: &InertCaptureV1, budget: usize) -> Result<bool, Error> {
    if a.usage().nodes != b.usage().nodes {
        return Ok(false);
    }
    if mul(a.usage().nodes, size_of::<(usize, usize)>())? > budget {
        return Err(Error::Limit);
    }
    let mut work = Vec::new();
    work.try_reserve_exact(a.usage().nodes)
        .map_err(|_| Error::Capacity)?;
    work.push((0, 0));
    while let Some((ai, bi)) = work.pop() {
        let (Some(av), Some(bv)) = (a.node(ai), b.node(bi)) else {
            return Err(Error::Faulted);
        };
        match (av, bv) {
            (Value::Null, Value::Null) => {}
            (Value::Boolean(a), Value::Boolean(b)) if a == b => {}
            (Value::Number(a), Value::Number(b)) if a.to_bits() == b.to_bits() => {}
            (Value::Text(a), Value::Text(b)) if a == b => {}
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                work.extend(a.iter().copied().zip(b.iter().copied()))
            }
            (Value::Object(a), Value::Object(b)) if a.len() == b.len() => {
                for (key, ai) in a {
                    let Some((_, bi)) = b.iter().find(|(k, _)| k == key) else {
                        return Ok(false);
                    };
                    work.push((*ai, *bi));
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
fn same_rows(c: &Context, p: &Packet, budget: usize) -> Result<bool, Error> {
    for old in &c.rows {
        let Some(now) = p.rows.iter().find(|v| {
            same_key(
                &v.key,
                DiscoveryCommandV1 {
                    authority: &old.key.authority,
                    id: &old.key.id,
                    incarnation: old.key.incarnation,
                },
            )
        }) else {
            return Ok(false);
        };
        if now.order.is_none() || old.version != now.version || old.kind != now.kind {
            return Ok(false);
        }
        if let Some(old) = &old.presentation {
            let Some(now) = now
                .presentation
                .as_ref()
                .filter(|_| now.presentation_visible)
            else {
                return Ok(false);
            };
            if !metadata_equal(old, now, budget)? {
                return Ok(false);
            }
        }
        for v in &old.variants {
            let Some(nv) = now
                .variants
                .iter()
                .find(|nv| nv.id == v.id && nv.order.is_some())
            else {
                return Ok(false);
            };
            if let Some(old) = &v.schema {
                let Some(now) = nv.schema.as_ref().filter(|_| now.schema_visible) else {
                    return Ok(false);
                };
                if !metadata_equal(old, now, budget)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
fn loss(f: &TrustedDiscoveryFactsV1) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.discovery_allowed,
        f.interpretation_current,
        f.retained,
    ]
    .into_iter()
    .any(|v| v == Some(false))
}
fn stop(c: &mut Context) {
    c.stopped = true;
    c.rows.clear();
    c.target = None;
    c.selectors.clear();
}
struct Stage<'a> {
    context: &'a Context,
    limits: DiscoveryLimitsV1,
    issued: usize,
    live: usize,
    selectors: Vec<Selector>,
}
fn original(c: &Context, owner: &Arc<()>, marker: &Arc<()>, slot: usize) -> Result<Kind, Error> {
    if !Arc::ptr_eq(owner, &c.marker) {
        return Err(Error::Unavailable);
    }
    let s = c.selectors.get(slot).ok_or(Error::Unavailable)?;
    if !Arc::ptr_eq(marker, &s.marker) {
        return Err(Error::Unavailable);
    }
    Ok(s.kind)
}
// Recognition only: no allocation, lookup by ordinary label, current grant,
// metadata comparison, selector issuance or loss effect. These checks must run
// before a request can invalidate the original whose roles it claims to carry.
fn recognize_query(
    c: &Context,
    id: &DiscoveryContextIdV1,
    namespace: &Arc<()>,
    query: DiscoveryQueryV1<'_>,
) -> Result<(), Error> {
    let entry_index = |e: &DiscoveryEntryIdV1| {
        let Kind::Entry(index) = original(c, &e.owner, &e.marker, e.slot)? else {
            return Err(Error::Unavailable);
        };
        if index >= c.rows.len() {
            return Err(Error::Faulted);
        }
        Ok(index)
    };
    match query {
        DiscoveryQueryV1::List {
            limit,
            continuation,
        } => {
            if limit == 0 || (limit as u128) > 9_007_199_254_740_991 {
                return Err(Error::InvalidRequest);
            }
            if let Some(cur) = continuation {
                let Kind::Cursor {
                    limit: original_limit,
                    start,
                } = original(c, &cur.owner, &cur.marker, cur.slot)?
                else {
                    return Err(Error::Unavailable);
                };
                if original_limit != limit {
                    return Err(Error::Unavailable);
                }
                if start == 0 || start >= c.rows.len() {
                    return Err(Error::Faulted);
                }
            }
        }
        DiscoveryQueryV1::Inspect { entry } => {
            entry_index(entry)?;
        }
        DiscoveryQueryV1::AdvisoryEnablement {
            entry,
            variant,
            target,
        } => {
            let index = entry_index(entry)?;
            let Kind::Variant {
                entry: ei,
                index: vi,
            } = original(c, &variant.owner, &variant.marker, variant.slot)?
            else {
                return Err(Error::Unavailable);
            };
            if ei != index {
                return Err(Error::Unavailable);
            }
            if vi >= c.rows[index].variants.len() {
                return Err(Error::Faulted);
            }
            let Some((marker, _)) = &c.target else {
                return Err(Error::Unavailable);
            };
            if !Arc::ptr_eq(&target.owner, namespace)
                || target.slot != id.slot
                || !Arc::ptr_eq(&target.marker, marker)
            {
                return Err(Error::Unavailable);
            }
        }
    }
    Ok(())
}
impl Stage<'_> {
    fn candidate(&self, owner: &Arc<()>, marker: &Arc<()>, slot: usize) -> Option<Kind> {
        if !Arc::ptr_eq(owner, &self.context.marker) {
            return None;
        }
        let s = if slot < self.context.selectors.len() {
            self.context.selectors.get(slot)
        } else {
            self.selectors
                .get(slot.checked_sub(self.context.selectors.len())?)
        }?;
        Arc::ptr_eq(marker, &s.marker).then_some(s.kind)
    }
    fn mint(&mut self, kind: Kind) -> Result<(usize, Arc<()>), Error> {
        if let Some((slot, s)) = self
            .context
            .selectors
            .iter()
            .chain(&self.selectors)
            .enumerate()
            .find(|(_, s)| s.kind == kind)
        {
            return Ok((slot, s.marker.clone()));
        }
        if add(self.live, self.selectors.len())? >= self.limits.max_selectors
            || add(self.issued, self.selectors.len())? >= self.limits.max_issued
            || size_of::<Selector>() > self.limits.max_selector_bytes
        {
            return Err(Error::Limit);
        }
        self.selectors
            .try_reserve_exact(1)
            .map_err(|_| Error::Capacity)?;
        let slot = add(self.context.selectors.len(), self.selectors.len())?;
        let marker = Arc::new(());
        self.selectors.push(Selector {
            marker: marker.clone(),
            kind,
        });
        Ok((slot, marker))
    }
    fn original(&self, owner: &Arc<()>, marker: &Arc<()>, slot: usize) -> Result<Kind, Error> {
        original(self.context, owner, marker, slot)
    }
}
fn reply_bytes(v: &DiscoveryReplyV1) -> Result<usize, Error> {
    let mut bytes = size_of::<DiscoveryReplyV1>();
    match v {
        DiscoveryReplyV1::Page { entries, .. } => {
            bytes = add(
                bytes,
                mul(entries.len(), size_of::<DiscoveryListEntryV1>())?,
            )?
        }
        DiscoveryReplyV1::Descriptor { descriptor: d, .. } => {
            bytes = add(
                bytes,
                mul(d.variants.len(), size_of::<DiscoveryVariantDescriptorV1>())?,
            )?;
            bytes = add(
                bytes,
                d.presentation
                    .as_ref()
                    .map_or(0, |v| v.usage().retained_bytes),
            )?;
            for v in &d.variants {
                bytes = add(
                    bytes,
                    v.schema.as_ref().map_or(0, |v| v.usage().retained_bytes),
                )?;
            }
        }
        DiscoveryReplyV1::Advice { .. } => {}
    }
    Ok(bytes)
}
impl SourceCoordinator {
    /// Read-only discovery over an original context and fresh native current-root
    /// cut. This profile stamps native-semantic-root/1, not an external accepted
    /// command registry, builtin schema truth, public facade or wire codec.
    pub fn query_discovery_v1(
        &mut self,
        id: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        cut: &DiscoveryCutIdV1,
        query: DiscoveryQueryV1<'_>,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryReplyV1, Error> {
        let timing = r.begin_service_timing();
        let result = self.discovery_query(id, scope, cut, query, f, r);
        self.discovery_finish_timed(r, Boundary::DiscoveryQuery, &result, timing.finish());
        result
    }
    fn discovery_query(
        &mut self,
        id: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        cut: &DiscoveryCutIdV1,
        query: DiscoveryQueryV1<'_>,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryReplyV1, Error> {
        // Scope/origin first: a foreign caller cannot stop someone else's capture.
        let c = self.discovery_context(id)?;
        if c.scope != *scope
            || c.stopped
            || c.decoder.blocks_native_service()
            || f.outside_publication_guard != Some(true)
        {
            return Err(Error::Unavailable);
        }
        // Recognize the complete original cut and its private source before
        // observed loss can stop this context. Matching descriptor bytes are
        // not evidence of builtin origin.
        let p = self
            .discovery_store()?
            .packets
            .get(cut.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if !Arc::ptr_eq(&cut.owner, &self.namespace)
            || !Arc::ptr_eq(&cut.marker, &p.marker)
            || p.scope != *scope
            || p.origin != c.origin
        {
            return Err(Error::Unavailable);
        }
        let recognized = recognize_query(c, id, &self.namespace, query);
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized?;
        if self.faulted || loss(f) {
            stop(
                self.discovery_store_mut()?.contexts[id.slot]
                    .as_mut()
                    .expect("original context"),
            );
            return Err(Error::Unavailable);
        }
        self.discovery_current(scope, f, r)?;
        if [
            f.retained,
            f.transfer_ready,
            f.reply_ready,
            f.resources_ready,
        ]
        .into_iter()
        .any(|v| v != Some(true))
        {
            return Err(Error::Unavailable);
        }
        let s = self.discovery_store()?;
        let p = s
            .packets
            .get(cut.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if !Arc::ptr_eq(&cut.owner, &self.namespace)
            || !Arc::ptr_eq(&cut.marker, &p.marker)
            || p.scope != *scope
        {
            return Err(Error::Unavailable);
        }
        self.discovery_root_current(&p.root, r)?;
        // Consumption is durable within the owner even if later delivery refuses.
        let p = self.discovery_store_mut()?.packets[cut.slot]
            .take()
            .expect("original cut");
        let s = self.discovery_store()?;
        let c = self.discovery_context(id)?;
        if !same_rows(c, &p, s.limits.max_capture_bytes)? {
            stop(
                self.discovery_store_mut()?.contexts[id.slot]
                    .as_mut()
                    .expect("original context"),
            );
            return Err(Error::Unavailable);
        }
        let mut stage = Stage {
            context: c,
            limits: s.limits,
            issued: s.issued,
            live: s.contexts.iter().flatten().map(|c| c.selectors.len()).sum(),
            selectors: Vec::new(),
        };
        let reply = match query {
            DiscoveryQueryV1::List {
                limit,
                continuation,
            } => {
                if limit == 0
                    || limit > s.limits.max_page_rows
                    || (limit as u128) > 9_007_199_254_740_991
                {
                    return Err(Error::InvalidRequest);
                }
                let start = if let Some(cur) = continuation {
                    match stage.original(&cur.owner, &cur.marker, cur.slot)? {
                        Kind::Cursor { limit: old, start } if old == limit => start,
                        _ => return Err(Error::Unavailable),
                    }
                } else {
                    0
                };
                let end = add(start, limit)?.min(c.rows.len());
                let mut entries = Vec::new();
                let count = end.checked_sub(start).ok_or(Error::Faulted)?;
                if add(
                    size_of::<DiscoveryReplyV1>(),
                    mul(count, size_of::<DiscoveryListEntryV1>())?,
                )? > s.limits.max_reply_bytes
                {
                    return Err(Error::Limit);
                }
                entries
                    .try_reserve_exact(count)
                    .map_err(|_| Error::Capacity)?;
                for index in start..end {
                    let (slot, marker) = stage.mint(Kind::Entry(index))?;
                    entries.push(DiscoveryListEntryV1 {
                        reference: DiscoveryEntryIdV1 {
                            owner: c.marker.clone(),
                            marker,
                            slot,
                        },
                        version: c.rows[index].version,
                        kind: c.rows[index].kind,
                    });
                }
                let continuation = if end < c.rows.len() {
                    let (slot, marker) = stage.mint(Kind::Cursor { limit, start: end })?;
                    Some(DiscoveryCursorIdV1 {
                        owner: c.marker.clone(),
                        marker,
                        slot,
                    })
                } else {
                    None
                };
                DiscoveryReplyV1::Page {
                    context: id.clone(),
                    coverage: DiscoveryCoverageV1::CapturedPermitted,
                    entries,
                    continuation,
                }
            }
            DiscoveryQueryV1::Inspect { entry } => {
                let Kind::Entry(index) = stage.original(&entry.owner, &entry.marker, entry.slot)?
                else {
                    return Err(Error::Unavailable);
                };
                let row = &c.rows[index];
                let mut variants = Vec::new();
                // Account the complete reply before allocating or minting roles.
                let mut bytes = add(
                    size_of::<DiscoveryReplyV1>(),
                    mul(
                        row.variants.len(),
                        size_of::<DiscoveryVariantDescriptorV1>(),
                    )?,
                )?;
                bytes = add(
                    bytes,
                    row.presentation
                        .as_ref()
                        .map_or(0, |v| v.usage().retained_bytes),
                )?;
                for v in &row.variants {
                    bytes = add(
                        bytes,
                        v.schema.as_ref().map_or(0, |v| v.usage().retained_bytes),
                    )?;
                }
                if bytes > s.limits.max_reply_bytes {
                    return Err(Error::Limit);
                }
                variants
                    .try_reserve_exact(row.variants.len())
                    .map_err(|_| Error::Capacity)?;
                for (vi, v) in row.variants.iter().enumerate() {
                    let (slot, marker) = stage.mint(Kind::Variant {
                        entry: index,
                        index: vi,
                    })?;
                    variants.push(DiscoveryVariantDescriptorV1 {
                        reference: DiscoveryVariantIdV1 {
                            owner: c.marker.clone(),
                            marker,
                            slot,
                        },
                        schema: v.schema.clone(),
                    });
                }
                DiscoveryReplyV1::Descriptor {
                    context: id.clone(),
                    descriptor: DiscoveryDescriptorV1 {
                        reference: entry.clone(),
                        version: row.version,
                        kind: row.kind,
                        variants,
                        presentation: row.presentation.clone(),
                    },
                }
            }
            DiscoveryQueryV1::AdvisoryEnablement {
                entry,
                variant,
                target,
            } => {
                let Kind::Entry(index) = stage.original(&entry.owner, &entry.marker, entry.slot)?
                else {
                    return Err(Error::Unavailable);
                };
                let Kind::Variant {
                    entry: ei,
                    index: vi,
                } = stage.original(&variant.owner, &variant.marker, variant.slot)?
                else {
                    return Err(Error::Unavailable);
                };
                let Some((marker, data)) = &c.target else {
                    return Err(Error::Unavailable);
                };
                if ei != index
                    || !Arc::ptr_eq(&target.owner, &self.namespace)
                    || target.slot != id.slot
                    || !Arc::ptr_eq(&target.marker, marker)
                    || !p.target_known
                    || !p.advice_known
                    || [
                        f.advice_attested,
                        f.target_current,
                        f.target_equality_observable,
                    ]
                    .into_iter()
                    .any(|v| v != Some(true))
                {
                    return Err(Error::Unavailable);
                }
                let Some(now_target) = &p.target else {
                    return Err(Error::Faulted);
                };
                if !metadata_equal(data, now_target, s.limits.max_capture_bytes)? {
                    return Err(Error::Unavailable);
                }
                let row = &c.rows[index];
                let now = p
                    .rows
                    .iter()
                    .find(|n| {
                        same_key(
                            &n.key,
                            DiscoveryCommandV1 {
                                authority: &row.key.authority,
                                id: &row.key.id,
                                incarnation: row.key.incarnation,
                            },
                        )
                    })
                    .ok_or(Error::Faulted)?;
                if !row.advice_visible || !now.advice_visible {
                    return Err(Error::Unavailable);
                }
                let enabled = now
                    .variants
                    .iter()
                    .find(|v| v.id == row.variants[vi].id)
                    .and_then(|v| v.enabled)
                    .ok_or(Error::Unavailable)?;
                DiscoveryReplyV1::Advice {
                    context: id.clone(),
                    entry: entry.clone(),
                    variant: variant.clone(),
                    target: target.clone(),
                    enabled,
                }
            }
        };
        if reply_bytes(&reply)? > s.limits.max_reply_bytes {
            return Err(Error::Limit);
        }
        let additions = stage.selectors;
        self.finish_discovery_reply(id, query, reply, additions, r)
    }
    fn finish_discovery_reply(
        &mut self,
        id: &DiscoveryContextIdV1,
        query: DiscoveryQueryV1<'_>,
        reply: DiscoveryReplyV1,
        additions: Vec<Selector>,
        r: &mut Recorder,
    ) -> Result<DiscoveryReplyV1, Error> {
        let s = self.discovery_store()?;
        let stage = Stage {
            context: self.discovery_context(id)?,
            limits: s.limits,
            issued: s.issued,
            live: s.contexts.iter().flatten().map(|c| c.selectors.len()).sum(),
            selectors: additions,
        };
        // Validate the actual whole result and tentative original-role map, not
        // merely its constructor or a detached post-publication copy.
        let checked = results::validate(&stage, id, query, &reply);
        let issued = add(s.issued, stage.selectors.len());
        let additions = stage.selectors;
        self.discovery_finish(r, Boundary::DiscoveryResultValidation, &checked);
        checked?;
        let issued = issued?;
        let s = self.discovery_store_mut()?;
        let c = s.contexts[id.slot].as_mut().expect("original context");
        c.selectors
            .try_reserve_exact(additions.len())
            .map_err(|_| Error::Capacity)?;
        c.selectors.extend(additions);
        s.issued = issued;
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        SourceCoordinator,
        Recorder,
        DiscoveryScopeV1,
        DiscoveryContextIdV1,
        DiscoveryCutIdV1,
    ) {
        fixture_with_facets(false)
    }
    fn fixture_with_facets(
        facets: bool,
    ) -> (
        SourceCoordinator,
        Recorder,
        DiscoveryScopeV1,
        DiscoveryContextIdV1,
        DiscoveryCutIdV1,
    ) {
        let (mut owner, mut recorder, _) = crate::coordinator::serialization::tests::setup();
        let scope = super::super::tests::scope();
        let limits = super::super::tests::limits();
        let mut facts = super::super::tests::facts();
        facts.target_binding_attested = Some(true);
        facts.target_equality_observable = Some(true);
        facts.advice_attested = Some(true);
        facts.target_current = Some(true);
        owner
            .configure_retention_budget(1048576, &mut recorder)
            .unwrap();
        owner
            .configure_discovery_v1(&scope, limits, &facts, &mut recorder)
            .unwrap();
        let fields = [("title", InertValueV1::Text("literal metadata"))];
        let input_fields = [("operation", InertValueV1::Text("status"))];
        let input = [
            ("contract", InertValueV1::Text("example/1")),
            ("variant", InertValueV1::Text("status")),
            ("fields", InertValueV1::Object(&input_fields)),
        ];
        let variants = [DiscoveryVariantInputV1 {
            id: "status",
            input: InertValueV1::Object(&input),
            outcomes: InertValueV1::Null,
            requirements: InertValueV1::Null,
            limits: InertValueV1::Null,
            cancellation: InertValueV1::Null,
            order: Some(0),
            enabled: Some(false),
        }];
        let rows = [DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority: "authority",
                id: "example",
                incarnation: 1,
            },
            contract: "example/1",
            version: 1,
            kind: DiscoveryExecutionKindV1::Query,
            variants: if facets { &variants } else { &[] },
            presentation: InertValueV1::Object(&fields),
            order: Some(0),
            schema_visible: facets,
            presentation_visible: facets,
            advice_visible: facets,
        }];
        let input = || DiscoveryCutInputV1 {
            entries: &rows,
            target: facets.then_some(DiscoveryTargetInputV1 {
                identity: "target",
                basis: InertValueV1::Null,
            }),
            advice_present: facets,
        };
        let cut = owner
            .prepare_discovery_cut_v1(&scope, input(), &facts, &mut recorder)
            .unwrap();
        let context = owner
            .acquire_discovery_v1(&scope, &cut, &facts, &mut recorder)
            .unwrap()
            .context;
        let cut = owner
            .prepare_discovery_cut_v1(&scope, input(), &facts, &mut recorder)
            .unwrap();
        (owner, recorder, scope, context, cut)
    }
    #[test]
    fn recognized_corrupt_selector_faults_locally_instead_of_becoming_a_loss_cleanup_or_known_refusal()
     {
        let (mut owner, mut r, scope, context, cut) = fixture();
        let marker = Arc::new(());
        let s = owner.discovery_store_mut().unwrap();
        let c = s.contexts[context.slot].as_mut().unwrap();
        let entry = DiscoveryEntryIdV1 {
            owner: c.marker.clone(),
            marker: marker.clone(),
            slot: 0,
        };
        c.selectors.push(Selector {
            marker,
            kind: Kind::Entry(usize::MAX),
        });
        s.issued += 1;
        let issued = s.issued;
        let mut g = super::super::tests::facts();
        g.authorized = Some(false);
        let generation = owner.generation();
        assert!(matches!(
            owner.query_discovery_v1(
                &context,
                &scope,
                &cut,
                DiscoveryQueryV1::Inspect { entry: &entry },
                &g,
                &mut r
            ),
            Err(Error::Faulted)
        ));
        let s = owner.discovery_store().unwrap();
        assert!(s.faulted);
        assert_eq!(s.issued, issued);
        assert!(s.packets[cut.slot].is_some());
        let c = s.contexts[context.slot].as_ref().unwrap();
        assert!(!c.stopped);
        assert_eq!(c.selectors.len(), 1);
        assert_eq!(owner.generation(), generation);
        assert!(!owner.is_faulted());
    }

    #[test]
    fn actual_discovery_result_guard_fences_a_bad_producer_before_installing_tentative_selectors() {
        let (mut owner, mut r, scope, context, cut) = fixture();
        // Test-only private corruption models a broken producer, not a public
        // malformed-request route or a callback in authoritative execution.
        let store = owner.discovery_store_mut().unwrap();
        store.contexts[context.slot].as_mut().unwrap().rows[0].version = 0;
        store.packets[cut.slot].as_mut().unwrap().rows[0].version = 0;
        let before = owner.discovery_usage_v1().unwrap();
        let debt = owner.retention_usage();
        let result = owner.query_discovery_v1(
            &context,
            &scope,
            &cut,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &super::super::tests::facts(),
            &mut r,
        );
        assert!(matches!(result, Err(Error::Faulted)));
        let after = owner.discovery_usage_v1().unwrap();
        assert!(after.faulted);
        assert_eq!(
            (after.selectors, after.issued_lifetime, after.preparations),
            (0, before.issued_lifetime, 0)
        );
        assert_eq!(owner.retention_usage(), debt);
        assert!(!owner.is_faulted());
        let f = super::super::tests::facts();
        owner
            .stop_discovery_context_v1(&context, &scope, &f, &mut r)
            .unwrap();
        owner
            .retire_discovery_context_v1(&context, &f, &mut r)
            .unwrap();
        assert_eq!(owner.retention_usage(), debt);
    }
    #[test]
    fn actual_page_guard_rejects_shape_origin_cursor_and_role_corruption_without_rollback_of_prior_issuance()
     {
        for change in 0..12 {
            let (mut owner, _, scope, context, cut) = fixture();
            let mut r = Recorder::try_with_capacity(64).unwrap();
            let q = DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            };
            let mut reply = owner
                .query_discovery_v1(
                    &context,
                    &scope,
                    &cut,
                    q,
                    &super::super::tests::facts(),
                    &mut r,
                )
                .unwrap();
            let before = owner.discovery_usage_v1().unwrap();
            let debt = owner.retention_usage();
            let DiscoveryReplyV1::Page {
                context: returned,
                entries,
                continuation,
                ..
            } = &mut reply
            else {
                panic!("page");
            };
            match change {
                0 => returned.marker = Arc::new(()),
                1 => returned.owner = Arc::new(()),
                2 => entries[0].version = 0,
                3 => entries[0].version = 9_007_199_254_740_992,
                4 => entries[0].version += 1,
                5 => entries[0].kind = DiscoveryExecutionKindV1::EffectRequest,
                6 => entries[0].reference.owner = Arc::new(()),
                7 => entries[0].reference.marker = Arc::new(()),
                8 => entries[0].reference.slot = usize::MAX,
                9 => {
                    *continuation = Some(DiscoveryCursorIdV1 {
                        owner: returned.marker.clone(),
                        marker: Arc::new(()),
                        slot: 0,
                    })
                }
                10 => entries.push(DiscoveryListEntryV1 {
                    reference: entries[0].reference.clone(),
                    version: 1,
                    kind: DiscoveryExecutionKindV1::Query,
                }),
                _ => {
                    reply = DiscoveryReplyV1::Descriptor {
                        context: context.clone(),
                        descriptor: DiscoveryDescriptorV1 {
                            reference: entries[0].reference.clone(),
                            version: 1,
                            kind: DiscoveryExecutionKindV1::Query,
                            variants: Vec::new(),
                            presentation: None,
                        },
                    }
                }
            }
            assert!(matches!(
                owner.finish_discovery_reply(&context, q, reply, Vec::new(), &mut r),
                Err(Error::Faulted)
            ));
            let after = owner.discovery_usage_v1().unwrap();
            assert!(after.faulted);
            assert_eq!(
                (after.selectors, after.issued_lifetime),
                (before.selectors, before.issued_lifetime)
            );
            assert_eq!(owner.retention_usage(), debt);
            assert!(!owner.is_faulted());
            let mut events = String::new();
            while let Some(event) = r.pop_front() {
                events.push_str(&format!("{event:?}"));
            }
            assert!(events.contains("DiscoveryResultValidation") && events.contains("Unknown"));
            for secret in ["example", "authority", "recipient"] {
                assert!(!events.contains(secret));
            }
        }
    }
    #[test]
    fn actual_descriptor_guard_rejects_unapproved_metadata_and_unpublished_or_unused_roles() {
        for change in 0..7 {
            let (mut owner, mut r, scope, context, cut) = fixture();
            let DiscoveryReplyV1::Page { entries, .. } = owner
                .query_discovery_v1(
                    &context,
                    &scope,
                    &cut,
                    DiscoveryQueryV1::List {
                        limit: 1,
                        continuation: None,
                    },
                    &super::super::tests::facts(),
                    &mut r,
                )
                .unwrap()
            else {
                panic!("page");
            };
            let e = entries[0].reference.clone();
            let mut d = DiscoveryDescriptorV1 {
                reference: e.clone(),
                version: 1,
                kind: DiscoveryExecutionKindV1::Query,
                variants: Vec::new(),
                presentation: None,
            };
            let mut additions = Vec::new();
            match change {
                0 => d.reference.marker = Arc::new(()),
                1 => d.version = 0,
                2 => d.kind = DiscoveryExecutionKindV1::EffectRequest,
                3 => d.variants.push(DiscoveryVariantDescriptorV1 {
                    reference: DiscoveryVariantIdV1 {
                        owner: e.owner.clone(),
                        marker: e.marker.clone(),
                        slot: e.slot,
                    },
                    schema: None,
                }),
                4 => {
                    d.presentation = Some(
                        meta(
                            &InertValueV1::Object(&[]),
                            super::super::tests::limits().metadata,
                            &mut r,
                        )
                        .unwrap(),
                    )
                }
                5 => additions.push(Selector {
                    marker: Arc::new(()),
                    kind: Kind::Variant { entry: 0, index: 0 },
                }),
                _ => {
                    let data = meta(
                        &InertValueV1::Object(&[(
                            "unknown-presentation-field",
                            InertValueV1::Null,
                        )]),
                        super::super::tests::limits().metadata,
                        &mut r,
                    )
                    .unwrap();
                    owner.discovery_store_mut().unwrap().contexts[context.slot]
                        .as_mut()
                        .unwrap()
                        .rows[0]
                        .presentation = Some(data.clone());
                    d.presentation = Some(data);
                }
            }
            let before = owner.discovery_usage_v1().unwrap();
            let reply = DiscoveryReplyV1::Descriptor {
                context: context.clone(),
                descriptor: d,
            };
            assert!(matches!(
                owner.finish_discovery_reply(
                    &context,
                    DiscoveryQueryV1::Inspect { entry: &e },
                    reply,
                    additions,
                    &mut r
                ),
                Err(Error::Faulted)
            ));
            let after = owner.discovery_usage_v1().unwrap();
            assert_eq!(
                (after.selectors, after.issued_lifetime),
                (before.selectors, before.issued_lifetime)
            );
            assert!(after.faulted && !owner.is_faulted());
        }
    }
    #[test]
    fn metadata_comparison_is_iterative_value_based_and_bounded_not_debug_or_member_order() {
        let mut r = Recorder::disabled();
        let l = CaptureLimitsV1 {
            max_bytes: 16384,
            max_nodes: 128,
            max_depth: 12,
        };
        let a = [
            ("first", InertValueV1::Text("value")),
            ("second", InertValueV1::Boolean(false)),
        ];
        let b = [a[1], a[0]];
        let ca = InertCaptureV1::copy(&InertValueV1::Object(&a), l, &mut r).unwrap();
        let cb = InertCaptureV1::copy(&InertValueV1::Object(&b), l, &mut r).unwrap();
        let bytes = ca.usage().nodes * size_of::<(usize, usize)>();
        assert!(metadata_equal(&ca, &cb, bytes).unwrap());
        assert!(metadata_equal(&ca, &cb, bytes + 1).unwrap());
        assert_eq!(metadata_equal(&ca, &cb, bytes - 1), Err(Error::Limit));
        let a = [InertValueV1::Text("a"), InertValueV1::Text("b")];
        let b = [a[1], a[0]];
        let ca = InertCaptureV1::copy(&InertValueV1::Array(&a), l, &mut r).unwrap();
        let cb = InertCaptureV1::copy(&InertValueV1::Array(&b), l, &mut r).unwrap();
        assert!(!metadata_equal(&ca, &cb, 16384).unwrap());
        // Redacted Debug deliberately cannot distinguish these values.
        assert_eq!(format!("{ca:?}"), format!("{cb:?}"));
        for (a, b, equal) in [
            (InertValueV1::Null, InertValueV1::Boolean(false), false),
            (InertValueV1::Number(1.0), InertValueV1::Number(2.0), false),
            (InertValueV1::Number(-0.0), InertValueV1::Number(0.0), false),
            (
                InertValueV1::Boolean(false),
                InertValueV1::Boolean(true),
                false,
            ),
            (InertValueV1::Text("a"), InertValueV1::Text("b"), false),
            (InertValueV1::Null, InertValueV1::Null, true),
        ] {
            let a = InertCaptureV1::copy(&a, l, &mut r).unwrap();
            let b = InertCaptureV1::copy(&b, l, &mut r).unwrap();
            assert_eq!(metadata_equal(&a, &b, 16384).unwrap(), equal);
        }
    }
    fn descriptor_candidate(
        owner: &SourceCoordinator,
        context: &DiscoveryContextIdV1,
        entry: &DiscoveryEntryIdV1,
    ) -> (DiscoveryReplyV1, Vec<Selector>, DiscoveryVariantIdV1) {
        let s = owner.discovery_store().unwrap();
        let c = owner.discovery_context(context).unwrap();
        let mut stage = Stage {
            context: c,
            limits: s.limits,
            issued: s.issued,
            live: s.contexts.iter().flatten().map(|v| v.selectors.len()).sum(),
            selectors: Vec::new(),
        };
        let (slot, marker) = stage.mint(Kind::Variant { entry: 0, index: 0 }).unwrap();
        let variant = DiscoveryVariantIdV1 {
            owner: c.marker.clone(),
            marker,
            slot,
        };
        let reply = DiscoveryReplyV1::Descriptor {
            context: context.clone(),
            descriptor: DiscoveryDescriptorV1 {
                reference: entry.clone(),
                version: c.rows[0].version,
                kind: c.rows[0].kind,
                variants: vec![DiscoveryVariantDescriptorV1 {
                    reference: variant.clone(),
                    schema: c.rows[0].variants[0].schema.clone(),
                }],
                presentation: c.rows[0].presentation.clone(),
            },
        };
        (reply, stage.selectors, variant)
    }
    #[test]
    fn actual_schema_guard_checks_all_facets_original_payload_caps_and_independent_visibility() {
        for change in 0..9 {
            let (mut owner, mut r, scope, context, cut) = fixture_with_facets(true);
            let DiscoveryReplyV1::Page { entries, .. } = owner
                .query_discovery_v1(
                    &context,
                    &scope,
                    &cut,
                    DiscoveryQueryV1::List {
                        limit: 1,
                        continuation: None,
                    },
                    &super::super::tests::facts(),
                    &mut r,
                )
                .unwrap()
            else {
                panic!("page");
            };
            let e = &entries[0].reference;
            if change == 2 || change == 6 {
                let many = [InertValueV1::Null; 130];
                let partial = [("input", InertValueV1::Null)];
                let excessive = [
                    ("input", InertValueV1::Array(&many)),
                    ("outcomes", InertValueV1::Null),
                    ("requirements", InertValueV1::Null),
                    ("limits", InertValueV1::Null),
                    ("cancellation", InertValueV1::Null),
                ];
                let copied = meta(
                    &InertValueV1::Object(if change == 2 { &partial } else { &excessive }),
                    CaptureLimitsV1 {
                        max_bytes: 32768,
                        max_nodes: 512,
                        max_depth: 12,
                    },
                    &mut r,
                )
                .unwrap();
                owner.discovery_store_mut().unwrap().contexts[context.slot]
                    .as_mut()
                    .unwrap()
                    .rows[0]
                    .variants[0]
                    .schema = Some(copied);
            }
            let (mut reply, additions, _) = descriptor_candidate(&owner, &context, e);
            let DiscoveryReplyV1::Descriptor { descriptor: d, .. } = &mut reply else {
                panic!("descriptor");
            };
            match change {
                1 => d.variants[0].schema = None,
                3 => {
                    d.presentation = Some(
                        meta(
                            &InertValueV1::Object(&[(
                                "title",
                                InertValueV1::Text("literal metadata"),
                            )]),
                            super::super::tests::limits().metadata,
                            &mut r,
                        )
                        .unwrap(),
                    )
                }
                4 => d.variants[0].reference.marker = Arc::new(()),
                5 => {
                    d.variants[0].reference = DiscoveryVariantIdV1 {
                        owner: e.owner.clone(),
                        marker: e.marker.clone(),
                        slot: e.slot,
                    }
                }
                7 => {
                    owner.discovery_store_mut().unwrap().contexts[context.slot]
                        .as_mut()
                        .unwrap()
                        .rows[0]
                        .presentation_visible = false
                }
                8 => {
                    owner.discovery_store_mut().unwrap().contexts[context.slot]
                        .as_mut()
                        .unwrap()
                        .rows[0]
                        .schema_visible = false
                }
                _ => {}
            }
            let before = owner.discovery_usage_v1().unwrap();
            let result = owner.finish_discovery_reply(
                &context,
                DiscoveryQueryV1::Inspect { entry: e },
                reply,
                additions,
                &mut r,
            );
            assert_eq!(result.is_ok(), change == 0);
            if change != 0 {
                assert!(matches!(result, Err(Error::Faulted)));
            }
            let after = owner.discovery_usage_v1().unwrap();
            let installed = usize::from(change == 0);
            assert_eq!(
                (after.issued_lifetime, after.selectors),
                (
                    before.issued_lifetime + installed,
                    before.selectors + installed
                )
            );
            assert_eq!(after.faulted, change != 0);
            assert!(!owner.is_faulted());
        }
    }
    #[test]
    fn actual_advice_guard_correlates_every_original_role_without_treating_boolean_shape_as_policy_truth()
     {
        for change in 0..7 {
            let (mut owner, mut r, scope, context, cut) = fixture_with_facets(true);
            let DiscoveryReplyV1::Page { entries, .. } = owner
                .query_discovery_v1(
                    &context,
                    &scope,
                    &cut,
                    DiscoveryQueryV1::List {
                        limit: 1,
                        continuation: None,
                    },
                    &super::super::tests::facts(),
                    &mut r,
                )
                .unwrap()
            else {
                panic!("page");
            };
            let e = &entries[0].reference;
            let (descriptor, additions, v) = descriptor_candidate(&owner, &context, e);
            owner
                .finish_discovery_reply(
                    &context,
                    DiscoveryQueryV1::Inspect { entry: e },
                    descriptor,
                    additions,
                    &mut r,
                )
                .unwrap();
            let target = DiscoveryTargetIdV1 {
                owner: owner.namespace.clone(),
                marker: owner
                    .discovery_context(&context)
                    .unwrap()
                    .target
                    .as_ref()
                    .unwrap()
                    .0
                    .clone(),
                slot: context.slot,
            };
            let mut reply = DiscoveryReplyV1::Advice {
                context: context.clone(),
                entry: e.clone(),
                variant: v.clone(),
                target: target.clone(),
                enabled: false,
            };
            let DiscoveryReplyV1::Advice {
                context: returned,
                entry,
                variant,
                target: returned_target,
                enabled,
            } = &mut reply
            else {
                panic!("advice");
            };
            match change {
                1 => returned.marker = Arc::new(()),
                2 => entry.marker = Arc::new(()),
                3 => variant.marker = Arc::new(()),
                4 => returned_target.marker = Arc::new(()),
                5 => returned_target.slot = usize::MAX,
                6 => *enabled = true, // Valid shape, not independently proven policy truth.
                _ => {}
            }
            let before = owner.discovery_usage_v1().unwrap();
            let result = owner.finish_discovery_reply(
                &context,
                DiscoveryQueryV1::AdvisoryEnablement {
                    entry: e,
                    variant: &v,
                    target: &target,
                },
                reply,
                Vec::new(),
                &mut r,
            );
            assert_eq!(result.is_ok(), change == 0 || change == 6);
            let after = owner.discovery_usage_v1().unwrap();
            assert_eq!(
                (after.selectors, after.issued_lifetime),
                (before.selectors, before.issued_lifetime)
            );
            assert!(!owner.is_faulted());
        }
    }
}
