use super::*;
pub(super) fn scope_bytes(
    s: &ResourceRegistryObservationScopeV1,
    l: ResourceRegistryObservationLimitsV1,
) -> Result<usize> {
    if s.context.profile != RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1 {
        return Err(ResourceError::Unavailable);
    }
    let n = settings::basis::labels(&s.context)
        .into_iter()
        .chain([s.policy.as_str()])
        .try_fold(size_of::<ResourceRegistryObservationScopeV1>(), |n, v| {
            if v.is_empty() || v.len() > l.max_tag_bytes {
                return Err(ResourceError::Malformed);
            }
            add(n, v.len())
        })?;
    if n > l.max_scope_bytes {
        Err(ResourceError::Limit)
    } else {
        Ok(n)
    }
}
pub(super) fn copy_scope(
    s: &ResourceRegistryObservationScopeV1,
) -> Result<ResourceRegistryObservationScopeV1> {
    Ok(ResourceRegistryObservationScopeV1 {
        context: settings::basis::copy_context(&s.context).map_err(|_| ResourceError::Capacity)?,
        policy: text(&s.policy)?,
        read_descriptors: s.read_descriptors,
        write_descriptors: s.write_descriptors,
    })
}
fn descriptor_bytes(d: &ResourceCatalogCapabilitiesV1) -> Result<usize> {
    add(
        size_of::<ResourceCatalogCapabilitiesV1>(),
        match d {
            ResourceCatalogCapabilitiesV1::Write { procedure, .. } => procedure.len(),
            _ => 0,
        },
    )
}
fn descriptor_copy(d: &ResourceCatalogCapabilitiesV1) -> Result<ResourceCatalogCapabilitiesV1> {
    Ok(match d {
        ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
            atomic_read,
            insertion_line_break,
        } => ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
            atomic_read: *atomic_read,
            insertion_line_break: *insertion_line_break,
        },
        ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read } => {
            ResourceCatalogCapabilitiesV1::AnnotationsV1 {
                atomic_read: *atomic_read,
            }
        }
        ResourceCatalogCapabilitiesV1::Write {
            procedure,
            logical_continuity,
            synchronization_supported,
            conditional_write,
            causal_receipts,
            non_reuse_tokens,
            exclusive_create,
        } => ResourceCatalogCapabilitiesV1::Write {
            procedure: text(procedure)?,
            logical_continuity: *logical_continuity,
            synchronization_supported: *synchronization_supported,
            conditional_write: *conditional_write,
            causal_receipts: *causal_receipts,
            non_reuse_tokens: *non_reuse_tokens,
            exclusive_create: *exclusive_create,
        },
    })
}
pub(super) fn projection_bytes(p: &ResourceRegistryProjectionV1) -> Result<usize> {
    p.rows
        .iter()
        .try_fold(size_of::<ResourceRegistryProjectionV1>(), |n, r| {
            add(
                add(n, size_of::<ResourceRegistryObservationRowV1>())?,
                r.descriptor.as_ref().map_or(Ok(0), descriptor_bytes)?,
            )
        })
}
pub(super) fn frame_bytes(p: &ResourceRegistryProjectionV1) -> Result<usize> {
    add(
        size_of::<ResourceRegistryDeliveryV1>(),
        projection_bytes(p)?,
    )
}
pub(super) fn binding_bytes(b: &[Binding]) -> Result<usize> {
    b.iter().try_fold(0, |n, b| {
        add(add(n, size_of::<Binding>())?, b.address.payload()?)
    })
}
pub(super) fn packet_bytes(p: &Packet, l: ResourceRegistryObservationLimitsV1) -> Result<usize> {
    let mut n = add(size_of::<Packet>(), scope_bytes(&p.scope, l)?)?;
    match &p.value {
        PacketValue::Historical { .. } => {}
        PacketValue::ReceiptCertificate(_) => {}
        PacketValue::ReceiptContext(_) => {
            n = add(
                add(add(n, size_of::<receipts::Context>())?, l.max_reply_bytes)?,
                l.max_binding_bytes,
            )?;
        }
        PacketValue::Command(_) => {
            n = add(
                add(
                    add(
                        add(n, size_of::<commands::Record>())?,
                        size_of::<ResourceRegistryCommandIdV1>(),
                    )?,
                    l.max_reply_bytes,
                )?,
                l.max_binding_bytes,
            )?;
        }
        PacketValue::Snapshot(rows) => {
            for row in rows {
                n = add(
                    add(add(n, size_of::<SnapshotRow>())?, row.address.payload()?)?,
                    row.descriptor.as_ref().map_or(Ok(0), descriptor_bytes)?,
                )?;
            }
        }
    }
    if n > l.max_packet_bytes {
        Err(ResourceError::Limit)
    } else {
        Ok(n)
    }
}
pub(super) fn valid(
    o: &Observer,
    id: &ResourceRegistryStreamIdV1,
    l: ResourceRegistryObservationLimitsV1,
    issued: usize,
) -> bool {
    if o.generation.stream != *id
        || o.generation.serial >= issued
        || o.queue.len() > l.max_queue
        || o.bindings.len() > l.max_bindings
        || scope_bytes(&o.scope, l).is_err()
        || !binding_bytes(&o.bindings).is_ok_and(|n| n <= l.max_binding_bytes)
    {
        return false;
    }
    if !o.bindings.iter().enumerate().all(|(i, b)| {
        b.reference.stream == *id
            && b.reference.index == i
            && o.bindings[..i].iter().all(|p| p.address != b.address)
    }) {
        return false;
    }
    let frame_valid = |p: &ResourceRegistryProjectionV1| {
        p.rows.len() <= l.max_rows
            && projection_bytes(p).is_ok_and(|n| n <= l.max_projection_bytes)
            && frame_bytes(p).is_ok_and(|n| n <= l.max_reply_bytes)
            && p.rows.iter().enumerate().all(|(i, r)| {
                o.bindings
                    .iter()
                    .any(|b| b.reference == r.reference && b.address.table == r.table)
                    && p.rows[..i].iter().all(|old| old.reference != r.reference)
                    && (r.descriptor.is_none()
                        || match r.table {
                            ResourceCatalogTableV1::Read => o.scope.read_descriptors,
                            ResourceCatalogTableV1::Write => o.scope.write_descriptors,
                        })
                    && r.descriptor.as_ref().is_none_or(|d| {
                        matches!(
                            (r.table, d),
                            (
                                ResourceCatalogTableV1::Read,
                                ResourceCatalogCapabilitiesV1::StrictUtf8OneBom { .. }
                                    | ResourceCatalogCapabilitiesV1::AnnotationsV1 { .. }
                            ) | (
                                ResourceCatalogTableV1::Write,
                                ResourceCatalogCapabilitiesV1::Write { .. }
                            )
                        )
                    })
            })
    };
    match o.phase {
        Phase::Gap => return o.baseline.is_none() && o.queue.is_empty(),
        Phase::Stopped => {
            return o.baseline.is_none() && o.queue.is_empty() && o.bindings.is_empty();
        }
        Phase::Active => {
            if !o.baseline.as_ref().is_some_and(|p| frame_valid(p)) {
                return false;
            }
        }
    }
    let mut before = None;
    for q in &o.queue {
        if q.event.before.stream != *id
            || q.event.after.stream != *id
            || q.event.before.serial.checked_add(1) != Some(q.event.after.serial)
            || before.is_some_and(|b| b != &q.event.before)
            || !frame_valid(&q.event.projection)
        {
            return false;
        }
        before = Some(&q.event.after);
    }
    o.queue.back().is_none_or(|q| {
        q.event.after == o.generation
            && o.baseline
                .as_ref()
                .is_some_and(|p| Arc::ptr_eq(p, &q.event.projection))
    })
}
pub(super) fn snapshot(
    root: &Root,
    s: &ResourceRegistryObservationScopeV1,
    f: &TrustedResourceRegistrySnapshotFactsV1<'_>,
    l: ResourceRegistryObservationLimitsV1,
    native: ResourceCatalogLimitsV1,
) -> Result<Vec<SnapshotRow>> {
    if root.rows.len() > l.max_rows || f.rows.len() > l.max_rows {
        return Err(ResourceError::Limit);
    }
    if f.rows.len() != root.rows.len() {
        return Err(ResourceError::Malformed);
    }
    if rows_bytes(&root.rows)? > l.max_root_bytes {
        return Err(ResourceError::Limit);
    }
    if [
        f.capture_policy_current,
        f.visibility_complete,
        f.order_attested,
    ]
    .into_iter()
    .any(|v| v != Some(true))
    {
        return Err(ResourceError::Unavailable);
    }
    let visible = f
        .rows
        .iter()
        .filter(|f| f.captured_visible && f.current_visible)
        .count();
    for (i, row) in f.rows.iter().enumerate() {
        if !row.address.valid(native)
            || (!row.captured_visible && row.captured_descriptor)
            || (!row.current_visible && row.current_descriptor)
            || f.rows[..i].iter().any(|old| old.address == row.address)
            || root
                .rows
                .iter()
                .filter(|r| r.address == *row.address)
                .count()
                != 1
        {
            return Err(ResourceError::Malformed);
        }
        let member = row.captured_visible && row.current_visible;
        if member {
            if row.visible_order.is_none_or(|rank| rank >= visible)
                || f.rows[..i]
                    .iter()
                    .any(|old| old.visible_order == row.visible_order)
            {
                return Err(ResourceError::Malformed);
            }
        } else if row.visible_order.is_some() {
            return Err(ResourceError::Malformed);
        }
    }
    // Preflight the entire capture before copying any descriptor or native key.
    let mut bytes = add(size_of::<Packet>(), scope_bytes(s, l)?)?;
    for fact in f
        .rows
        .iter()
        .filter(|f| f.captured_visible && f.current_visible)
    {
        let row = root
            .rows
            .iter()
            .find(|r| r.address == *fact.address)
            .ok_or(ResourceError::Faulted)?;
        let cap = match row.address.table {
            ResourceCatalogTableV1::Read => s.read_descriptors,
            ResourceCatalogTableV1::Write => s.write_descriptors,
        };
        bytes = add(
            add(
                add(bytes, size_of::<SnapshotRow>())?,
                row.address.payload()?,
            )?,
            if cap && fact.captured_descriptor && fact.current_descriptor {
                descriptor_bytes(&row.configuration.capabilities)?
            } else {
                0
            },
        )?;
    }
    if bytes > l.max_packet_bytes {
        return Err(ResourceError::Limit);
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(visible)
        .map_err(|_| ResourceError::Capacity)?;
    for rank in 0..visible {
        let fact = f
            .rows
            .iter()
            .find(|f| f.visible_order == Some(rank))
            .ok_or(ResourceError::Malformed)?;
        let row = root
            .rows
            .iter()
            .find(|r| r.address == *fact.address)
            .ok_or(ResourceError::Faulted)?;
        let cap = match row.address.table {
            ResourceCatalogTableV1::Read => s.read_descriptors,
            ResourceCatalogTableV1::Write => s.write_descriptors,
        };
        rows.push(SnapshotRow {
            address: row.address.copy()?,
            descriptor: if cap && fact.captured_descriptor && fact.current_descriptor {
                Some(descriptor_copy(&row.configuration.capabilities)?)
            } else {
                None
            },
            identity_known: (row.address.table == ResourceCatalogTableV1::Write
                && cap
                && fact.captured_descriptor
                && fact.current_descriptor)
                .then_some(row.configuration.identity_known),
        });
    }
    Ok(rows)
}
pub(super) fn matches(rows: &[SnapshotRow], o: &Observer) -> bool {
    let Some(p) = &o.baseline else {
        return false;
    };
    rows.len() == p.rows.len()
        && rows.iter().zip(&p.rows).all(|(row, old)| {
            o.bindings
                .iter()
                .any(|b| b.address == row.address && b.reference == old.reference)
                && row.descriptor == old.descriptor
                && row.identity_known == old.identity_known
                && row.address.table == old.table
        })
}
pub(super) struct Stage {
    pub bindings: Vec<Binding>,
    pub projection: Arc<ResourceRegistryProjectionV1>,
    pub added: usize,
}
pub(super) fn stage_admission(
    rows: &[SnapshotRow],
    old: &[Binding],
    l: ResourceRegistryObservationLimitsV1,
) -> Result<usize> {
    let added = rows
        .iter()
        .filter(|r| !old.iter().any(|b| b.address == r.address))
        .count();
    let count = old.len().checked_add(added).ok_or(ResourceError::Limit)?;
    if count > l.max_bindings {
        return Err(ResourceError::Limit);
    }
    let mut bytes = binding_bytes(old)?;
    let mut projection = size_of::<ResourceRegistryProjectionV1>();
    for row in rows {
        if !old.iter().any(|b| b.address == row.address) {
            bytes = add(add(bytes, size_of::<Binding>())?, row.address.payload()?)?;
        }
        projection = add(
            add(projection, size_of::<ResourceRegistryObservationRowV1>())?,
            row.descriptor.as_ref().map_or(Ok(0), descriptor_bytes)?,
        )?;
    }
    if bytes > l.max_binding_bytes
        || projection > l.max_projection_bytes
        || add(projection, size_of::<ResourceRegistryDeliveryV1>())? > l.max_reply_bytes
    {
        return Err(ResourceError::Limit);
    }

    Ok(added)
}
pub(super) fn stage(
    id: &ResourceRegistryStreamIdV1,
    rows: &[SnapshotRow],
    old: &[Binding],
    l: ResourceRegistryObservationLimitsV1,
) -> Result<Stage> {
    let added = stage_admission(rows, old, l)?;
    let count = old.len().checked_add(added).ok_or(ResourceError::Limit)?;
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(count)
        .map_err(|_| ResourceError::Capacity)?;
    for b in old {
        bindings.push(Binding {
            address: b.address.copy()?,
            reference: b.reference.clone(),
        });
    }
    let mut out = Vec::new();
    out.try_reserve_exact(rows.len())
        .map_err(|_| ResourceError::Capacity)?;
    for row in rows {
        let reference = if let Some(b) = bindings.iter().find(|b| b.address == row.address) {
            b.reference.clone()
        } else {
            let reference = ResourceRegistrySlotReferenceV1 {
                stream: id.clone(),
                index: bindings.len(),
            };
            bindings.push(Binding {
                address: row.address.copy()?,
                reference: reference.clone(),
            });
            reference
        };
        out.push(ResourceRegistryObservationRowV1 {
            reference,
            table: row.address.table,
            descriptor: row.descriptor.as_ref().map(descriptor_copy).transpose()?,
            identity_known: row.identity_known,
        });
    }
    Ok(Stage {
        bindings,
        projection: Arc::new(ResourceRegistryProjectionV1 { rows: out }),
        added,
    })
}
