use super::*;
pub(super) fn descriptor(
    c: &ResourceCatalogConfigurationV1,
) -> Result<ResourceCatalogReadDescriptorV1> {
    Ok(match &c.capabilities {
        ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
            atomic_read,
            insertion_line_break,
        } => ResourceCatalogReadDescriptorV1::StrictUtf8OneBom {
            atomic_read: *atomic_read,
            insertion_line_break: *insertion_line_break,
        },
        ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read } => {
            ResourceCatalogReadDescriptorV1::AnnotationsV1 {
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
        } => ResourceCatalogReadDescriptorV1::Write {
            procedure: text(procedure)?,
            identity_known: c.identity_known,
            logical_continuity: *logical_continuity,
            synchronization_supported: *synchronization_supported,
            conditional_write: *conditional_write,
            causal_receipts: *causal_receipts,
            non_reuse_tokens: *non_reuse_tokens,
            exclusive_create: *exclusive_create,
        },
    })
}
pub(super) fn descriptor_bytes(d: &ResourceCatalogReadDescriptorV1) -> Result<usize> {
    add(
        size_of::<ResourceCatalogReadDescriptorV1>(),
        match d {
            ResourceCatalogReadDescriptorV1::Write { procedure, .. } => procedure.len(),
            _ => 0,
        },
    )
}
pub(super) fn descriptor_copy(
    d: &ResourceCatalogReadDescriptorV1,
) -> Result<ResourceCatalogReadDescriptorV1> {
    Ok(match d {
        ResourceCatalogReadDescriptorV1::StrictUtf8OneBom {
            atomic_read,
            insertion_line_break,
        } => ResourceCatalogReadDescriptorV1::StrictUtf8OneBom {
            atomic_read: *atomic_read,
            insertion_line_break: *insertion_line_break,
        },
        ResourceCatalogReadDescriptorV1::AnnotationsV1 { atomic_read } => {
            ResourceCatalogReadDescriptorV1::AnnotationsV1 {
                atomic_read: *atomic_read,
            }
        }
        ResourceCatalogReadDescriptorV1::Write {
            procedure,
            identity_known,
            logical_continuity,
            synchronization_supported,
            conditional_write,
            causal_receipts,
            non_reuse_tokens,
            exclusive_create,
        } => ResourceCatalogReadDescriptorV1::Write {
            procedure: text(procedure)?,
            identity_known: *identity_known,
            logical_continuity: *logical_continuity,
            synchronization_supported: *synchronization_supported,
            conditional_write: *conditional_write,
            causal_receipts: *causal_receipts,
            non_reuse_tokens: *non_reuse_tokens,
            exclusive_create: *exclusive_create,
        },
    })
}
fn row_bytes(row: &Snapshot) -> Result<usize> {
    add(
        add(size_of::<Snapshot>(), row.address.payload()?)?,
        row.descriptor.as_ref().map_or(Ok(0), descriptor_bytes)?,
    )
}
pub(super) fn bytes(rows: &[Snapshot]) -> Result<usize> {
    rows.iter()
        .try_fold(size_of::<Vec<Snapshot>>(), |n, row| add(n, row_bytes(row)?))
}
pub(super) fn project(
    root: &Root,
    facts: &[ResourceCatalogVisibilityV1<'_>],
    l: ResourceCatalogReadLimitsV1,
) -> Result<Vec<Snapshot>> {
    if root.rows.len() > l.max_rows || facts.len() != root.rows.len() {
        return Err(ResourceError::Limit);
    }
    for (i, f) in facts.iter().enumerate() {
        if facts[..i].iter().any(|old| old.address == f.address)
            || !root.rows.iter().any(|row| row.address == *f.address)
            || !f.visible && (f.descriptor || f.order.is_some())
            || f.visible && f.order.is_none()
        {
            return Err(ResourceError::Unavailable);
        }
    }
    let count = facts.iter().filter(|f| f.visible).count();
    if add(
        size_of::<Vec<Snapshot>>(),
        count
            .checked_mul(size_of::<Snapshot>())
            .ok_or(ResourceError::Limit)?,
    )? > l.max_capture_bytes
    {
        return Err(ResourceError::Limit);
    }
    let mut out = Vec::new();
    reserve(&mut out, count)?;
    let mut charged = size_of::<Vec<Snapshot>>();
    for order in 0..count {
        let mut found = None;
        for f in facts {
            if f.visible && f.order == Some(order) {
                if found.is_some() {
                    return Err(ResourceError::Unavailable);
                }
                found = Some(f);
            }
        }
        let f = found.ok_or(ResourceError::Unavailable)?;
        let row = root
            .rows
            .iter()
            .find(|row| row.address == *f.address)
            .ok_or(ResourceError::Faulted)?;
        let descriptor_size = if f.descriptor {
            add(
                size_of::<ResourceCatalogReadDescriptorV1>(),
                match &row.configuration.capabilities {
                    ResourceCatalogCapabilitiesV1::Write { procedure, .. } => procedure.len(),
                    _ => 0,
                },
            )?
        } else {
            0
        };
        let size = add(
            add(size_of::<Snapshot>(), row.address.payload()?)?,
            descriptor_size,
        )?;
        charged = add(charged, size)?;
        if size > l.max_row_bytes || charged > l.max_capture_bytes {
            return Err(ResourceError::Limit);
        }
        let snapshot = Snapshot {
            address: row.address.copy()?,
            descriptor: if f.descriptor {
                Some(descriptor(&row.configuration)?)
            } else {
                None
            },
        };
        out.push(snapshot);
    }
    Ok(out)
}
pub(super) fn matches_ceiling(current: &[Snapshot], captured: &[Snapshot]) -> bool {
    captured.iter().all(|old| {
        current
            .iter()
            .find(|row| row.address == old.address)
            .is_some_and(|row| old.descriptor.is_none() || old.descriptor == row.descriptor)
    })
}
