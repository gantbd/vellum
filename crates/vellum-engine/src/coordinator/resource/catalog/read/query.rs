use super::*;
impl SourceCoordinator {
    pub fn query_resource_catalog_read_v1(
        &mut self,
        context: &ResourceCatalogReadContextIdV1,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        query: ResourceCatalogReadQueryV1<'_>,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogReadReplyV1> {
        let gate = self.catalog_read_pool().and_then(|p| {
            p.ready()?;
            if p.decoding {
                Err(ResourceError::Unavailable)
            } else {
                Ok(())
            }
        });
        if let Err(error) = gate {
            let result = Err(error);
            record(r, Boundary::ResourceCatalogReadRelease, &result, None);
            return result;
        }
        self.query_resource_catalog_read_owned(context, scope, cut, query, facts, r)
    }
    pub(super) fn query_resource_catalog_read_owned(
        &mut self,
        context: &ResourceCatalogReadContextIdV1,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        query: ResourceCatalogReadQueryV1<'_>,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogReadReplyV1> {
        let timing = r.begin_service_timing();
        let mut invalidation = None;
        let result = (|| {
            self.ensure_operational().map_err(admission)?;
            if !Arc::ptr_eq(&context.owner, &self.namespace) {
                return Err(ResourceError::Unavailable);
            }
            let p = self.catalog_read_pool()?;
            p.ready()?;
            p.scope(scope)?;
            let c = p
                .contexts
                .get(context.slot)
                .and_then(Option::as_ref)
                .ok_or(ResourceError::Unavailable)?;
            if c.scope != *scope || c.stopped {
                return Err(ResourceError::Unavailable);
            }
            if facts.loss() {
                self.catalog_read_pool_mut()?.end(context.slot, true);
                invalidation = Some(crate::instrumentation::Disposition::Refused);
                return Err(ResourceError::Unavailable);
            }
            if !facts.release() {
                return Err(ResourceError::Unavailable);
            }
            match catch_unwind(AssertUnwindSafe(|| {
                self.release_catalog_read(context, scope, cut, query)
            })) {
                Ok(Err(ResourceError::Faulted)) | Err(_) => {
                    self.catalog_read_pool_mut()?.end(context.slot, true);
                    invalidation = Some(crate::instrumentation::Disposition::Unknown);
                    Err(ResourceError::Unavailable)
                }
                Ok(Err(ResourceError::Stale)) => {
                    self.catalog_read_pool_mut()?.end(context.slot, true);
                    invalidation = Some(crate::instrumentation::Disposition::Refused);
                    Err(ResourceError::Unavailable)
                }
                Ok(result) => result,
            }
        })();
        if let Some(disposition) = invalidation {
            r.record(Observation {
                boundary: Boundary::ResourceCatalogReadInvalidation,
                disposition,
                service_time: None,
            });
        }
        record(
            r,
            Boundary::ResourceCatalogReadRelease,
            &result,
            timing.finish(),
        );
        result
    }
    fn release_catalog_read(
        &mut self,
        id: &ResourceCatalogReadContextIdV1,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        query: ResourceCatalogReadQueryV1<'_>,
    ) -> Result<ResourceCatalogReadReplyV1> {
        let packet = self.take_catalog_visibility(cut, scope)?;
        let p = self.catalog_read_pool()?;
        let c = p.contexts[id.slot].as_ref().ok_or(ResourceError::Faulted)?;
        if !projection::matches_ceiling(&packet.rows, &c.rows) {
            return Err(ResourceError::Stale);
        }
        #[cfg(test)]
        self.catalog_read_probe(0);
        let p = self.catalog_read_pool()?;
        let c = p.contexts[id.slot].as_ref().ok_or(ResourceError::Faulted)?;
        let l = p.limits;
        match query {
            ResourceCatalogReadQueryV1::Inspect { entry } => {
                if !Arc::ptr_eq(&entry.owner, &c.marker) || c.entries.get(entry.slot) != Some(&true)
                {
                    return Err(ResourceError::Unavailable);
                }
                let row = c.rows.get(entry.slot).ok_or(ResourceError::Faulted)?;
                let bytes = add(
                    size_of::<ResourceCatalogReadReplyV1>(),
                    row.descriptor
                        .as_ref()
                        .map_or(Ok(0), projection::descriptor_bytes)?,
                )?;
                if bytes > l.max_reply_bytes {
                    return Err(ResourceError::Limit);
                }
                Ok(ResourceCatalogReadReplyV1::Registration {
                    context: id.clone(),
                    entry: entry.clone(),
                    table: row.address.table,
                    descriptor: row
                        .descriptor
                        .as_ref()
                        .map(projection::descriptor_copy)
                        .transpose()?,
                })
            }
            ResourceCatalogReadQueryV1::List {
                limit,
                continuation,
            } => {
                if limit == 0 || limit > l.max_page_rows {
                    return Err(ResourceError::Unavailable);
                }
                let start = match continuation {
                    None => 0,
                    Some(cursor) => {
                        if !Arc::ptr_eq(&cursor.owner, &c.marker) {
                            return Err(ResourceError::Unavailable);
                        }
                        let prior = c
                            .cursors
                            .get(cursor.slot)
                            .ok_or(ResourceError::Unavailable)?;
                        if prior.limit != limit {
                            return Err(ResourceError::Unavailable);
                        }
                        prior.start
                    }
                };
                if start > c.rows.len() {
                    return Err(ResourceError::Faulted);
                }
                let end = start
                    .checked_add(limit)
                    .ok_or(ResourceError::Limit)?
                    .min(c.rows.len());
                let prior_cursor = (end < c.rows.len())
                    .then(|| {
                        c.cursors
                            .iter()
                            .position(|cursor| cursor.limit == limit && cursor.start == end)
                    })
                    .flatten();
                let new_cursor = end < c.rows.len() && prior_cursor.is_none();
                let new_entries = c.entries[start..end].iter().filter(|v| !**v).count();
                let new = add(new_entries, usize::from(new_cursor))?;
                if new > l.max_issued.saturating_sub(p.issued)
                    || new > l.max_selectors.saturating_sub(p.live_selectors)
                    || new_entries != 0
                        && add(size_of::<ResourceCatalogReadEntryIdV1>(), size_of::<bool>())?
                            > l.max_selector_bytes
                    || new_cursor
                        && add(
                            size_of::<ResourceCatalogReadContinuationIdV1>(),
                            size_of::<Cursor>(),
                        )? > l.max_selector_bytes
                {
                    return Err(ResourceError::Limit);
                }
                let bytes = add(
                    size_of::<ResourceCatalogReadReplyV1>(),
                    (end - start)
                        .checked_mul(size_of::<ResourceCatalogReadPageEntryV1>())
                        .ok_or(ResourceError::Limit)?,
                )?;
                if bytes > l.max_reply_bytes {
                    return Err(ResourceError::Limit);
                }
                let mut entries = Vec::new();
                reserve(&mut entries, end - start)?;
                for at in start..end {
                    entries.push(ResourceCatalogReadPageEntryV1 {
                        reference: ResourceCatalogReadEntryIdV1 {
                            owner: c.marker.clone(),
                            slot: at,
                        },
                        table: c.rows[at].address.table,
                    });
                }
                let continuation = if end < c.rows.len() {
                    Some(ResourceCatalogReadContinuationIdV1 {
                        owner: c.marker.clone(),
                        slot: prior_cursor.unwrap_or(c.cursors.len()),
                    })
                } else {
                    None
                };
                let reply = ResourceCatalogReadReplyV1::Page {
                    context: id.clone(),
                    entries,
                    continuation,
                };
                // Nothing fallible follows publication. Reply/selector capacity
                // was checked jointly; a failed reservation installs no aliases.
                let p = self.catalog_read_pool_mut()?;
                let c = p.contexts[id.slot].as_mut().ok_or(ResourceError::Faulted)?;
                if new_cursor {
                    reserve(&mut c.cursors, 1)?;
                }
                for at in start..end {
                    c.entries[at] = true;
                }
                if new_cursor {
                    c.cursors.push(Cursor { limit, start: end });
                }
                p.issued += new;
                p.live_selectors += new;
                #[cfg(test)]
                self.catalog_read_probe(1);
                Ok(reply)
            }
        }
    }
    #[cfg(test)]
    fn catalog_read_probe(&mut self, point: usize) {
        let p = self.catalog_read_pool_mut().unwrap();
        if p.query_fault == Some(point) {
            p.query_fault = None;
            panic!("injected catalog read interruption");
        }
    }
}
