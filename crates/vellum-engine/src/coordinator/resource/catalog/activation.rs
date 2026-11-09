//! Exclusive owner publication. No provider execution or callback can intervene.
use super::*;
impl SourceCoordinator {
    pub(in crate::coordinator) fn resource_catalog_profile(
        &self,
    ) -> Result<ResourceCatalogLimitsV1> {
        self.resources.catalog.current().map(|(_, profile)| profile)
    }
    /// Privileged borrowed metadata only; not recipient disclosure or proof of
    /// an actual provider capability. No caller-supplied row can replace it.
    pub fn resource_catalog_registration_v1(
        &self,
        address: &ResourceCatalogAddressV1,
        r: &mut Recorder,
    ) -> Result<Option<&ResourceCatalogRowV1>> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let (root, l) = self.resources.catalog.current()?;
            if !address.valid(l) {
                return Err(ResourceError::Malformed);
            }
            Ok(root.rows.iter().find(|row| row.address == *address))
        })();
        r.record(Observation {
            boundary: Boundary::ResourceCatalogQuery,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(ResourceError::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
    fn catalog_finish<T>(
        &mut self,
        b: Boundary,
        result: &Result<T>,
        elapsed: Option<std::time::Duration>,
        r: &mut Recorder,
    ) {
        if matches!(result, Err(ResourceError::Faulted)) {
            self.resources.catalog.faulted = true;
        }
        r.record(Observation {
            boundary: b,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(ResourceError::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: elapsed,
        });
    }
    /// Explicit known initial catalogs only. Repeating exact current contents
    /// and profile is a no-op; this never resets the owner's registration clock.
    pub fn configure_resource_catalog_v1(
        &mut self,
        initial: &[ResourceCatalogRowV1],
        limits: ResourceCatalogLimitsV1,
        f: &TrustedResourceCatalogFactsV1,
        r: &mut Recorder,
    ) -> Result<bool> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            f.current()?;
            validate_rows(initial, limits)?;
            if self.resources.catalog.root.is_some() {
                let (root, old) = self.resources.catalog.current()?;
                return if old == limits && root.rows == initial {
                    Ok(false)
                } else {
                    Err(ResourceError::Unavailable)
                };
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if self.resources.catalog.limits.is_some()
                || self.resources.catalog.floor != 0
                || self.resources.catalog.faulted
            {
                return Err(ResourceError::Faulted);
            }
            let bytes = rows_bytes(initial)?;
            if add(bytes, bytes)? > limits.max_plan_bytes {
                return Err(ResourceError::Limit);
            }
            self.check_root_growth(bytes, r).map_err(owner_error)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(initial.len())
                .map_err(|_| ResourceError::Capacity)?;
            for row in initial {
                rows.push(row.copy()?);
            }
            let floor = rows.iter().map(|row| row.generation).max().unwrap_or(0);
            self.resources.catalog = State {
                limits: Some(limits),
                root: Some(Arc::new(Root {
                    marker: Arc::new(()),
                    rows,
                })),
                floor,
                faulted: false,
            };
            Ok(true)
        })();
        self.catalog_finish(
            Boundary::ResourceCatalogConfiguration,
            &result,
            timing.finish(),
            r,
        );
        result
    }
    pub fn resource_catalog_usage_v1(
        &mut self,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogUsageV1> {
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            let (root, _) = self.resources.catalog.current()?;
            Ok(ResourceCatalogUsageV1 {
                read_rows: root
                    .rows
                    .iter()
                    .filter(|row| row.address.table == ResourceCatalogTableV1::Read)
                    .count(),
                write_rows: root
                    .rows
                    .iter()
                    .filter(|row| row.address.table == ResourceCatalogTableV1::Write)
                    .count(),
                registration_floor: self.resources.catalog.floor,
                root_bytes: rows_bytes(&root.rows)?,
            })
        })();
        self.catalog_finish(Boundary::ResourceCatalogQuery, &result, None, r);
        result
    }
    pub fn prepare_resource_catalog_v1(
        &mut self,
        changes: &[ResourceCatalogChangeV1],
        f: &TrustedResourceCatalogFactsV1,
        r: &mut Recorder,
    ) -> Result<PreparedResourceCatalogV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(owner_error)?;
            f.current()?;
            let (root, profile) = self.resources.catalog.current()?;
            let floor = self.resources.catalog.floor;
            let candidate = candidate(root, floor, changes, profile)?;
            self.check_root_replacement(
                rows_bytes(&root.rows)?,
                rows_bytes(&candidate.root.rows)?,
                r,
            )
            .map_err(owner_error)?;
            let before = self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?;
            Ok(PreparedResourceCatalogV1 {
                owner: self.namespace.clone(),
                before,
                profile,
                floor,
                request: copy_request(changes)?,
                candidate,
            })
        })();
        self.catalog_finish(
            Boundary::ResourceCatalogPreparation,
            &result,
            timing.finish(),
            r,
        );
        result
    }
    pub fn commit_resource_catalog_v1(
        &mut self,
        prepared: PreparedResourceCatalogV1,
        f: &TrustedResourceCatalogFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogOutcomeV1> {
        let stage = self.stage_resource_catalog_v1(prepared, f, r)?;
        let outcome = stage.outcome;
        stage.publication.install(self, r);
        Ok(outcome)
    }
    pub(in crate::coordinator) fn stage_resource_catalog_v1(
        &mut self,
        prepared: PreparedResourceCatalogV1,
        f: &TrustedResourceCatalogFactsV1,
        r: &mut Recorder,
    ) -> Result<CatalogStage> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&prepared.owner, &self.namespace) {
                return Err(ResourceError::Unavailable);
            }
            self.ensure_ordinary_admission().map_err(owner_error)?;
            f.current()?;
            let (root, profile) = self.resources.catalog.current()?;
            if self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?
                != prepared.before
            {
                return Err(ResourceError::Stale);
            }
            if prepared.profile != profile || prepared.floor != self.resources.catalog.floor {
                return Err(ResourceError::Faulted);
            }
            let fresh = candidate(
                root,
                self.resources.catalog.floor,
                &prepared.request,
                profile,
            )
            .map_err(|e| {
                // The original root, floor and immutable profile matched. Only
                // actual allocation pressure can newly refuse this reconstruction;
                // changed meaning or an impossible expectation is protocol drift.
                if e == ResourceError::Capacity {
                    e
                } else {
                    ResourceError::Faulted
                }
            })?;
            if fresh.root.rows != prepared.candidate.root.rows
                || fresh.floor != prepared.candidate.floor
                || fresh.outcome != prepared.candidate.outcome
            {
                return Err(ResourceError::Faulted);
            }
            self.check_root_replacement(rows_bytes(&root.rows)?, rows_bytes(&fresh.root.rows)?, r)
                .map_err(owner_error)?;
            let changed = fresh.outcome.status == ResourceCatalogStatusV1::Applied;
            let next_generation = if changed {
                self.generation
                    .checked_add(1)
                    .ok_or(ResourceError::VersionExhausted)?
            } else {
                self.generation
            };
            Ok(CatalogStage {
                publication: CatalogPublication {
                    root: fresh.root,
                    floor: fresh.floor,
                    generation: next_generation,
                    changed,
                },
                outcome: fresh.outcome,
            })
        })();
        self.catalog_finish(
            Boundary::ResourceCatalogValidation,
            &result,
            timing.finish(),
            r,
        );
        result
    }
}
