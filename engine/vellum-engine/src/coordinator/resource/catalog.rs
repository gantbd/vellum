//! Trusted resource metadata, not provider ports, I/O or binding activation.
use super::*;
mod shutdown_inventory;
use std::mem::size_of;
mod activation;
pub(in crate::coordinator) mod basis;
pub(in crate::coordinator) mod consumer;
pub(in crate::coordinator) mod observation;
pub(in crate::coordinator) mod read;
mod values;
use values::*;
#[cfg(test)]
mod tests;
pub(in crate::coordinator) const MAX_STAMP: u64 = 9_007_199_254_740_991;
type Result<T> = std::result::Result<T, ResourceError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceCatalogLimitsV1 {
    pub max_relations: usize,
    pub max_tag_bytes: usize,
    pub max_request_bytes: usize,
    pub max_result_bytes: usize,
    /// Logical original request, two candidate/result frames and native headers.
    /// Not allocator capacity, caller ingress, RSS or a global retained-work cap.
    pub max_plan_bytes: usize,
}
#[derive(PartialEq, Eq)]
pub struct ResourceCatalogIdentityV1 {
    pub authority: String,
    pub id: String,
    pub incarnation: u64,
}
impl std::fmt::Debug for ResourceCatalogIdentityV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceCatalogIdentityV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceCatalogTableV1 {
    Read,
    Write,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ResourceCatalogAddressV1 {
    pub table: ResourceCatalogTableV1,
    pub provider: ResourceCatalogIdentityV1,
    pub resource: ResourceCatalogIdentityV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogInsertionLineBreakV1 {
    Lf,
    CrLf,
    Cr,
}
#[derive(PartialEq, Eq)]
pub enum ResourceCatalogCapabilitiesV1 {
    StrictUtf8OneBom {
        atomic_read: bool,
        insertion_line_break: CatalogInsertionLineBreakV1,
    },
    AnnotationsV1 {
        atomic_read: bool,
    },
    Write {
        procedure: String,
        logical_continuity: bool,
        synchronization_supported: bool,
        conditional_write: bool,
        causal_receipts: bool,
        non_reuse_tokens: bool,
        exclusive_create: bool,
    },
}
impl std::fmt::Debug for ResourceCatalogCapabilitiesV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceCatalogCapabilitiesV1(<declaration>)")
    }
}
#[derive(PartialEq, Eq)]
pub struct ResourceCatalogConfigurationV1 {
    pub identity_domain: String,
    pub identity_key: String,
    pub identity_known: bool,
    pub locator: String,
    pub capabilities: ResourceCatalogCapabilitiesV1,
}
impl std::fmt::Debug for ResourceCatalogConfigurationV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceCatalogConfigurationV1(<redacted>)")
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct ResourceCatalogRowV1 {
    pub address: ResourceCatalogAddressV1,
    pub generation: u64,
    pub configuration: ResourceCatalogConfigurationV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogExpectationV1 {
    Absent,
    Present { generation: u64 },
}
#[derive(Debug, PartialEq, Eq)]
pub enum ResourceCatalogActionV1 {
    Install(ResourceCatalogConfigurationV1),
    Remove,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ResourceCatalogChangeV1 {
    pub address: ResourceCatalogAddressV1,
    pub expected: CatalogExpectationV1,
    pub action: ResourceCatalogActionV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceCatalogStatusV1 {
    Applied,
    Unchanged,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ResourceCatalogResultV1 {
    pub address: ResourceCatalogAddressV1,
    /// None is absent; Some(0) is a valid initial present generation.
    pub generation: Option<u64>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ResourceCatalogOutcomeV1 {
    pub status: ResourceCatalogStatusV1,
    pub registrations: Vec<ResourceCatalogResultV1>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceCatalogUsageV1 {
    pub read_rows: usize,
    pub write_rows: usize,
    /// Trusted-owner clock, not a recipient-safe correlation identifier.
    pub registration_floor: u64,
    pub root_bytes: usize,
}
#[derive(Clone, Copy, Default)]
pub struct TrustedResourceCatalogFactsV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub registration_allowed: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub registry_coverage_complete: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
}
impl TrustedResourceCatalogFactsV1 {
    fn current(self) -> Result<()> {
        if [
            self.authorized,
            self.scope,
            self.continuous,
            self.live,
            self.not_stopped,
            self.registration_allowed,
            self.identity_allowed,
            self.registry_coverage_complete,
            self.companions_ready,
            self.versions_available,
            self.reply_ready,
            self.resources_ready,
            self.owner_control_allowed,
            self.preparation_reserved,
        ]
        .into_iter()
        .all(|v| v == Some(true))
        {
            Ok(())
        } else {
            Err(ResourceError::Unavailable)
        }
    }
}
#[derive(Debug, Default)]
pub(super) struct State {
    limits: Option<ResourceCatalogLimitsV1>,
    root: Option<Arc<Root>>,
    floor: u64,
    faulted: bool,
}
#[derive(Clone, Debug, Default)]
pub(super) struct RootStamp {
    limits: Option<ResourceCatalogLimitsV1>,
    floor: Option<u64>,
    marker: Option<Arc<()>>,
}
impl PartialEq for RootStamp {
    fn eq(&self, other: &Self) -> bool {
        self.limits == other.limits
            && self.floor == other.floor
            && match (&self.marker, &other.marker) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
}
impl Eq for RootStamp {}
#[derive(Debug, PartialEq, Eq)]
struct Root {
    marker: Arc<()>,
    rows: Vec<ResourceCatalogRowV1>,
}
struct Candidate {
    root: Arc<Root>,
    floor: u64,
    outcome: ResourceCatalogOutcomeV1,
}
pub(in crate::coordinator) struct CatalogStage {
    pub(in crate::coordinator) publication: CatalogPublication,
    pub(in crate::coordinator) outcome: ResourceCatalogOutcomeV1,
}
pub(in crate::coordinator) struct CatalogPublication {
    root: Arc<Root>,
    floor: u64,
    generation: u64,
    changed: bool,
}
impl CatalogPublication {
    pub(in crate::coordinator) fn install(self, owner: &mut SourceCoordinator, r: &mut Recorder) {
        if self.changed {
            owner.resources.catalog.root = Some(self.root);
            owner.resources.catalog.floor = self.floor;
            owner.generation = self.generation;
            r.record(Observation {
                boundary: Boundary::ResourceCatalogPublication,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
    }
}
#[derive(Debug)]
pub(in crate::coordinator) struct CatalogRequestCapture {
    pub(in crate::coordinator) profile: ResourceCatalogLimitsV1,
    pub(in crate::coordinator) changes: Vec<ResourceCatalogChangeV1>,
}
impl CatalogRequestCapture {
    pub(in crate::coordinator) fn bytes(
        changes: &[ResourceCatalogChangeV1],
        profile: ResourceCatalogLimitsV1,
    ) -> Result<usize> {
        add(
            request_bytes(changes, profile)?,
            size_of::<Self>() - size_of::<Vec<ResourceCatalogChangeV1>>(),
        )
    }
    pub(in crate::coordinator) fn copy(
        changes: &[ResourceCatalogChangeV1],
        profile: ResourceCatalogLimitsV1,
        maximum: usize,
    ) -> Result<Self> {
        if Self::bytes(changes, profile)? > maximum {
            return Err(ResourceError::Limit);
        }
        Ok(Self {
            profile,
            changes: copy_request(changes)?,
        })
    }
}
impl ResourceCatalogOutcomeV1 {
    pub(in crate::coordinator) fn native_bytes(&self) -> Result<usize> {
        self.registrations
            .iter()
            .try_fold(size_of::<Self>(), |n, v| {
                add(
                    add(n, size_of::<ResourceCatalogResultV1>())?,
                    v.address.payload()?,
                )
            })
    }
    pub(in crate::coordinator) fn copy_bounded(&self, maximum: usize) -> Result<Self> {
        if self.native_bytes()? > maximum {
            return Err(ResourceError::Limit);
        }
        let mut registrations = Vec::new();
        registrations
            .try_reserve_exact(self.registrations.len())
            .map_err(|_| ResourceError::Capacity)?;
        for row in &self.registrations {
            registrations.push(ResourceCatalogResultV1 {
                address: row.address.copy()?,
                generation: row.generation,
            });
        }
        Ok(Self {
            status: self.status,
            registrations,
        })
    }
}
impl PreparedResourceCatalogV1 {
    pub(in crate::coordinator) fn outcome(&self) -> &ResourceCatalogOutcomeV1 {
        &self.candidate.outcome
    }
}
pub struct PreparedResourceCatalogV1 {
    owner: Arc<()>,
    before: root_identity::NativeRootIdentity,
    profile: ResourceCatalogLimitsV1,
    floor: u64,
    request: Vec<ResourceCatalogChangeV1>,
    candidate: Candidate,
}
impl std::fmt::Debug for PreparedResourceCatalogV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedResourceCatalogV1(<owned certificate>)")
    }
}

impl State {
    fn current(&self) -> Result<(&Arc<Root>, ResourceCatalogLimitsV1)> {
        if self.faulted {
            return Err(ResourceError::Faulted);
        }
        let (Some(root), Some(l)) = (&self.root, self.limits) else {
            return if self.root.is_none() && self.limits.is_none() && self.floor == 0 {
                Err(ResourceError::Unavailable)
            } else {
                Err(ResourceError::Faulted)
            };
        };
        validate_rows(&root.rows, l).map_err(|_| ResourceError::Faulted)?;
        if self.floor > MAX_STAMP || root.rows.iter().any(|r| r.generation > self.floor) {
            return Err(ResourceError::Faulted);
        }
        Ok((root, l))
    }
    pub(super) fn root_bytes(&self) -> Result<usize> {
        match &self.root {
            Some(root) => rows_bytes(&root.rows),
            None => Ok(0),
        }
    }
    pub(super) fn stamp(&self) -> Result<RootStamp> {
        if self.root.is_none() && self.limits.is_none() && self.floor == 0 && !self.faulted {
            return Ok(RootStamp::default());
        }
        let (root, l) = self.current()?;
        Ok(RootStamp {
            limits: Some(l),
            floor: Some(self.floor),
            marker: Some(root.marker.clone()),
        })
    }
}

fn result_bytes(changes: &[ResourceCatalogChangeV1]) -> Result<usize> {
    changes
        .iter()
        .try_fold(size_of::<ResourceCatalogOutcomeV1>(), |n, c| {
            add(
                add(n, size_of::<ResourceCatalogResultV1>())?,
                c.address.payload()?,
            )
        })
}
fn preflight(
    root: &Root,
    changes: &[ResourceCatalogChangeV1],
    l: ResourceCatalogLimitsV1,
) -> Result<()> {
    let request = request_bytes(changes, l)?;
    let result = result_bytes(changes)?;
    if result > l.max_result_bytes {
        return Err(ResourceError::Limit);
    }
    let candidate = changes.iter().try_fold(rows_bytes(&root.rows)?, |n, c| {
        if let ResourceCatalogActionV1::Install(config) = &c.action {
            add(
                add(
                    add(n, size_of::<ResourceCatalogRowV1>())?,
                    c.address.payload()?,
                )?,
                config.payload()?,
            )
        } else {
            Ok(n)
        }
    })?;
    // Both retained prediction and final reconstruction are bounded before any
    // candidate/result allocation. This is logical native staging, not RSS.
    let demand = add(
        add(size_of::<PreparedResourceCatalogV1>(), request)?,
        add(add(candidate, candidate)?, add(result, result)?)?,
    )?;
    if demand > l.max_plan_bytes {
        Err(ResourceError::Limit)
    } else {
        Ok(())
    }
}
fn candidate(
    root: &Arc<Root>,
    floor: u64,
    changes: &[ResourceCatalogChangeV1],
    l: ResourceCatalogLimitsV1,
) -> Result<Candidate> {
    preflight(root, changes, l)?;
    let mut changed = false;
    let mut installs = false;
    for c in changes {
        let old = root.rows.iter().find(|r| r.address == c.address);
        let matches = match (c.expected, old) {
            (CatalogExpectationV1::Absent, None) => true,
            (CatalogExpectationV1::Present { generation }, Some(r)) => generation == r.generation,
            _ => false,
        };
        if !matches {
            return Err(ResourceError::Stale);
        }
        match &c.action {
            ResourceCatalogActionV1::Remove => changed |= old.is_some(),
            ResourceCatalogActionV1::Install(config)
                if old.is_some_and(|r| r.configuration == *config) => {}
            ResourceCatalogActionV1::Install(_) => {
                changed = true;
                installs = true;
            }
        }
    }
    let next_floor = if installs {
        floor
            .checked_add(1)
            .filter(|v| *v <= MAX_STAMP)
            .ok_or(ResourceError::VersionExhausted)?
    } else {
        floor
    };
    let mut rows = Vec::new();
    rows.try_reserve_exact(add(root.rows.len(), changes.len())?)
        .map_err(|_| ResourceError::Capacity)?;
    for row in &root.rows {
        rows.push(row.copy()?);
    }
    let mut registrations = Vec::new();
    registrations
        .try_reserve_exact(changes.len())
        .map_err(|_| ResourceError::Capacity)?;
    for c in changes {
        let pos = rows.iter().position(|r| r.address == c.address);
        let generation = match &c.action {
            ResourceCatalogActionV1::Remove => {
                if let Some(i) = pos {
                    rows.remove(i);
                }
                None
            }
            ResourceCatalogActionV1::Install(config) => {
                if let Some(i) = pos.filter(|i| rows[*i].configuration == *config) {
                    Some(rows[i].generation)
                } else {
                    let row = ResourceCatalogRowV1 {
                        address: c.address.copy()?,
                        generation: next_floor,
                        configuration: config.copy()?,
                    };
                    if let Some(i) = pos {
                        rows[i] = row;
                    } else {
                        rows.push(row);
                    }
                    Some(next_floor)
                }
            }
        };
        registrations.push(ResourceCatalogResultV1 {
            address: c.address.copy()?,
            generation,
        });
    }
    validate_rows(&rows, l)?;
    Ok(Candidate {
        root: if changed {
            Arc::new(Root {
                marker: Arc::new(()),
                rows,
            })
        } else {
            root.clone()
        },
        floor: next_floor,
        outcome: ResourceCatalogOutcomeV1 {
            status: if changed {
                ResourceCatalogStatusV1::Applied
            } else {
                ResourceCatalogStatusV1::Unchanged
            },
            registrations,
        },
    })
}
