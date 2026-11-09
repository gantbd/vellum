//! Fallible flat copies and checked native value costs for the closed catalog.
use super::*;
pub(super) fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or(ResourceError::Limit)
}
fn text(s: &str) -> Result<String> {
    let mut out = String::new();
    out.try_reserve_exact(s.len())
        .map_err(|_| ResourceError::Capacity)?;
    out.push_str(s);
    Ok(out)
}
fn label(s: &str, l: ResourceCatalogLimitsV1) -> bool {
    !s.is_empty() && s.len() <= l.max_tag_bytes
}
impl ResourceCatalogIdentityV1 {
    fn valid(&self, l: ResourceCatalogLimitsV1) -> bool {
        label(&self.authority, l) && label(&self.id, l) && self.incarnation <= MAX_STAMP
    }
    fn copy(&self) -> Result<Self> {
        Ok(Self {
            authority: text(&self.authority)?,
            id: text(&self.id)?,
            incarnation: self.incarnation,
        })
    }
    fn payload(&self) -> Result<usize> {
        add(self.authority.len(), self.id.len())
    }
}
impl ResourceCatalogAddressV1 {
    pub(super) fn valid(&self, l: ResourceCatalogLimitsV1) -> bool {
        self.provider.valid(l) && self.resource.valid(l)
    }
    pub(in crate::coordinator) fn copy(&self) -> Result<Self> {
        Ok(Self {
            table: self.table,
            provider: self.provider.copy()?,
            resource: self.resource.copy()?,
        })
    }
    pub(in crate::coordinator) fn payload(&self) -> Result<usize> {
        add(self.provider.payload()?, self.resource.payload()?)
    }
}
impl ResourceCatalogConfigurationV1 {
    pub(in crate::coordinator) fn valid(
        &self,
        table: ResourceCatalogTableV1,
        l: ResourceCatalogLimitsV1,
    ) -> bool {
        label(&self.identity_domain, l)
            && label(&self.identity_key, l)
            && label(&self.locator, l)
            && match (&self.capabilities, table) {
                (
                    ResourceCatalogCapabilitiesV1::StrictUtf8OneBom { .. }
                    | ResourceCatalogCapabilitiesV1::AnnotationsV1 { .. },
                    ResourceCatalogTableV1::Read,
                ) => true,
                (
                    ResourceCatalogCapabilitiesV1::Write { procedure, .. },
                    ResourceCatalogTableV1::Write,
                ) => label(procedure, l),
                _ => false,
            }
    }
    pub(in crate::coordinator) fn copy(&self) -> Result<Self> {
        let capabilities = match &self.capabilities {
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
        };
        Ok(Self {
            identity_domain: text(&self.identity_domain)?,
            identity_key: text(&self.identity_key)?,
            identity_known: self.identity_known,
            locator: text(&self.locator)?,
            capabilities,
        })
    }
    pub(in crate::coordinator) fn payload(&self) -> Result<usize> {
        [
            self.identity_domain.len(),
            self.identity_key.len(),
            self.locator.len(),
            match &self.capabilities {
                ResourceCatalogCapabilitiesV1::Write { procedure, .. } => procedure.len(),
                _ => 0,
            },
        ]
        .into_iter()
        .try_fold(0, add)
    }
}
impl ResourceCatalogRowV1 {
    pub(super) fn payload(&self) -> Result<usize> {
        add(self.address.payload()?, self.configuration.payload()?)
    }
    pub(super) fn copy(&self) -> Result<Self> {
        Ok(Self {
            address: self.address.copy()?,
            generation: self.generation,
            configuration: self.configuration.copy()?,
        })
    }
}
pub(super) fn rows_bytes(rows: &[ResourceCatalogRowV1]) -> Result<usize> {
    rows.iter().try_fold(size_of::<Root>(), |n, r| {
        add(add(n, size_of::<ResourceCatalogRowV1>())?, r.payload()?)
    })
}
pub(super) fn validate_rows(
    rows: &[ResourceCatalogRowV1],
    l: ResourceCatalogLimitsV1,
) -> Result<()> {
    if rows.len() > l.max_relations.checked_mul(2).ok_or(ResourceError::Limit)? {
        return Err(ResourceError::Limit);
    }
    for table in [ResourceCatalogTableV1::Read, ResourceCatalogTableV1::Write] {
        if rows.iter().filter(|r| r.address.table == table).count() > l.max_relations {
            return Err(ResourceError::Limit);
        }
    }
    for (i, r) in rows.iter().enumerate() {
        if !r.address.valid(l)
            || r.generation > MAX_STAMP
            || !r.configuration.valid(r.address.table, l)
            || rows[..i].iter().any(|old| old.address == r.address)
        {
            return Err(ResourceError::Malformed);
        }
    }
    Ok(())
}
pub(super) fn request_bytes(
    changes: &[ResourceCatalogChangeV1],
    l: ResourceCatalogLimitsV1,
) -> Result<usize> {
    if changes.len() > l.max_relations {
        return Err(ResourceError::Limit);
    }
    let mut size = size_of::<Vec<ResourceCatalogChangeV1>>();
    for (i, c) in changes.iter().enumerate() {
        if !c.address.valid(l)
            || matches!(c.expected,CatalogExpectationV1::Present {generation} if generation > MAX_STAMP)
            || changes[..i].iter().any(|old| old.address == c.address)
        {
            return Err(ResourceError::Malformed);
        }
        size = add(
            add(size, size_of::<ResourceCatalogChangeV1>())?,
            c.address.payload()?,
        )?;
        if let ResourceCatalogActionV1::Install(config) = &c.action {
            if !config.valid(c.address.table, l) {
                return Err(ResourceError::Malformed);
            }
            size = add(size, config.payload()?)?;
        }
    }
    if size > l.max_request_bytes {
        Err(ResourceError::Limit)
    } else {
        Ok(size)
    }
}
pub(super) fn copy_request(
    changes: &[ResourceCatalogChangeV1],
) -> Result<Vec<ResourceCatalogChangeV1>> {
    let mut out = Vec::new();
    out.try_reserve_exact(changes.len())
        .map_err(|_| ResourceError::Capacity)?;
    for c in changes {
        out.push(ResourceCatalogChangeV1 {
            address: c.address.copy()?,
            expected: c.expected,
            action: match &c.action {
                ResourceCatalogActionV1::Install(v) => ResourceCatalogActionV1::Install(v.copy()?),
                ResourceCatalogActionV1::Remove => ResourceCatalogActionV1::Remove,
            },
        });
    }
    Ok(out)
}
