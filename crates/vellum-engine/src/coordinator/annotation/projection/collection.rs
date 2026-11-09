//! Exact native-root collection visibility preparations, not collection streams.
use super::*;
use std::mem::size_of;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionFacetCeilingV1 {
    pub body: bool,
    pub target: bool,
}
#[derive(Clone, PartialEq, Eq)]
pub struct CollectionVisibilityPreparationIdV1 {
    inner: PreparedAnnotationProjectionId,
}
impl std::fmt::Debug for CollectionVisibilityPreparationIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CollectionVisibilityPreparationIdV1(<opaque>)")
    }
}
pub struct TrustedCollectionVisibilityFactsV1<'a> {
    pub current: &'a TrustedSourceReadFactsV1<'a>,
    pub annotation_readable: Option<bool>,
    /// Independent current attestation for this captured projection, not inferred
    /// from unchanged source or native root identity.
    pub projection_current: Option<bool>,
}
#[derive(Clone, Copy)]
pub(super) struct Binding {
    root: u64,
    ceiling: CollectionFacetCeilingV1,
}
/// Engine-private staged projection. Not a recipient selector or retained result.
pub struct CollectionVisibilityCaptureV1 {
    value: Projection,
}
impl CollectionVisibilityCaptureV1 {
    /// Trusted native staging rows. These native IDs are not recipient collection
    /// references, F03 selectors or grants. A stream must assign its own references.
    pub fn rows(
        &self,
    ) -> impl ExactSizeIterator<
        Item = (
            &AnnotationId,
            Option<(&str, u64)>,
            Option<AnnotationLocationV1>,
        ),
    > {
        self.value.rows.iter().map(|r| {
            (
                &r.id,
                r.body.as_ref().map(|(b, v)| (b.as_str(), *v)),
                r.location
                    .map(
                        |(placement, source_revision, target_generation)| AnnotationLocationV1 {
                            placement,
                            source_revision,
                            target_generation,
                        },
                    ),
            )
        })
    }
}
impl SourceCoordinator {
    pub fn prepare_collection_visibility_v1(
        &mut self,
        scope: &SourceReadScopeV1,
        ceiling: CollectionFacetCeilingV1,
        facts: &TrustedAnnotationCaptureFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionVisibilityPreparationIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission()
                .map_err(source_context::admission_error)?;
            if self.source_facade_busy {
                return Err(Error::Unavailable);
            }
            let p = self.annotation_preparation_pool()?;
            if !valid_scope(scope, p) || scope != facts.scope {
                return Err(Error::Unavailable);
            }
            let l = p.limits;
            // Fixed cardinality and arithmetic checks before caller-sized scans/copies.
            if facts.grants.len() > l.max_rows {
                return Err(Error::Limit);
            }
            let request = scope_bytes(scope)?
                .checked_add(size_of::<CollectionFacetCeilingV1>())
                .and_then(|n| {
                    n.checked_add(
                        facts
                            .grants
                            .len()
                            .checked_mul(size_of::<AnnotationFacetGrantV1>())?,
                    )
                })
                .ok_or(Error::Limit)?;
            if request > l.max_request_bytes
                || size_of::<CollectionVisibilityPreparationIdV1>() > l.max_selector_bytes
            {
                return Err(Error::Limit);
            }
            if p.cuts.len() >= l.max_issued || p.cuts.iter().flatten().count() >= l.max_prepared {
                return Err(Error::Limit);
            }
            self.retention
                .validate_exact(&p.reservation, p.charge)
                .map_err(parent)?;
            let value = self.stage_current_projection(
                AnnotationProjectionClassesV1::Annotations,
                scope,
                facts,
                l,
                Some(ceiling),
                r,
            )?;
            let scope = SourceReadScopeV1 {
                document: scope.document.clone(),
                policy: owned(&scope.policy)?,
                context: settings::basis::copy_context(&scope.context)
                    .map_err(|_| Error::Capacity)?,
            };
            let root = self.generation;
            let State::Active(p) = &mut self.annotation_preparations else {
                unreachable!()
            };
            let inner = PreparedAnnotationProjectionId {
                owner: self.namespace.clone(),
                slot: p.cuts.len(),
            };
            p.cuts.push(Some(Cut {
                scope,
                value,
                invalidated: false,
                collection: Some(Binding { root, ceiling }),
            }));
            Ok(CollectionVisibilityPreparationIdV1 { inner })
        })();
        Self::collection_observe(
            r,
            Boundary::CollectionVisibilityPreparation,
            &result,
            timing.finish(),
        );
        result
    }
    fn collection_visibility_cut_v1(
        &self,
        id: &CollectionVisibilityPreparationIdV1,
        scope: &SourceReadScopeV1,
        ceiling: CollectionFacetCeilingV1,
    ) -> Result<&Cut, Error> {
        let p = self.annotation_preparation_pool()?;
        self.retention
            .validate_exact(&p.reservation, p.charge)
            .map_err(parent)?;
        let cut = self.prepared_annotation_cut_any(&id.inner, scope)?;
        let binding = cut.collection.ok_or(Error::Unavailable)?;
        if cut.invalidated || binding.ceiling != ceiling || binding.root != self.generation {
            return Err(Error::Unavailable);
        }
        if !cut.value.valid(scope, &self.namespace)
            || cut.value.source.is_some()
            || cut.value.rows.iter().any(|r| {
                (!ceiling.body && r.body.is_some()) || (!ceiling.target && r.location.is_some())
            })
            || cut.value.bytes()? > p.limits.max_capture_bytes
        {
            return Err(Error::Faulted);
        }
        Ok(cut)
    }
    pub fn validate_collection_visibility_v1(
        &mut self,
        id: &CollectionVisibilityPreparationIdV1,
        scope: &SourceReadScopeV1,
        ceiling: CollectionFacetCeilingV1,
        facts: &TrustedCollectionVisibilityFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            self.collection_visibility_cut_v1(id, scope, ceiling)?;
            let f = facts.current;
            if f.scope != scope {
                return Err(Error::Unavailable);
            }
            let gates = [
                f.authorized,
                f.scope_allowed,
                f.continuous,
                f.live,
                f.not_stopped,
                f.retained,
                facts.annotation_readable,
                facts.projection_current,
            ];
            if gates.contains(&Some(false)) || self.document(&scope.document).is_err() {
                let State::Active(p) = &mut self.annotation_preparations else {
                    unreachable!()
                };
                p.cuts[id.inner.slot]
                    .as_mut()
                    .ok_or(Error::Unavailable)?
                    .invalidated = true;
                r.record(Observation {
                    boundary: Boundary::CollectionVisibilityInvalidation,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
                return Err(Error::Unavailable);
            }
            if gates.into_iter().any(|v| v != Some(true)) || f.resources_ready != Some(true) {
                return Err(Error::Unavailable);
            }
            self.document(&scope.document)
                .map_err(|_| Error::Unavailable)?;
            Ok(())
        })();
        Self::collection_observe(
            r,
            Boundary::CollectionVisibilityValidation,
            &result,
            timing.finish(),
        );
        result
    }
    /// Trusted native staging only. The receiver must fund any installed frame
    /// and bindings separately; this private capture is not a public read result.
    pub fn consume_collection_visibility_v1(
        &mut self,
        id: &CollectionVisibilityPreparationIdV1,
        scope: &SourceReadScopeV1,
        ceiling: CollectionFacetCeilingV1,
        facts: &TrustedCollectionVisibilityFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionVisibilityCaptureV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_collection_visibility_v1(id, scope, ceiling, facts, r)?;
            if facts.current.reply_ready != Some(true) || facts.current.transfer_ready != Some(true)
            {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.annotation_preparations else {
                unreachable!()
            };
            let cut = p.cuts[id.inner.slot].take().ok_or(Error::Unavailable)?;
            Ok(CollectionVisibilityCaptureV1 { value: cut.value })
        })();
        Self::collection_observe(
            r,
            Boundary::CollectionVisibilityConsumption,
            &result,
            timing.finish(),
        );
        result
    }
    /// Protected abandoned/stale preparation release; not stream revocation.
    pub fn retire_collection_visibility_v1(
        &mut self,
        id: &CollectionVisibilityPreparationIdV1,
        control: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !discharge(control) {
                return Err(Error::Unavailable);
            }
            let p = self.annotation_preparation_pool()?;
            self.retention
                .validate_exact(&p.reservation, p.charge)
                .map_err(parent)?;
            let cut = self.prepared_annotation_cut_any(&id.inner, control.scope)?;
            if cut.collection.is_none() {
                return Err(Error::Unavailable);
            }
            let State::Active(p) = &mut self.annotation_preparations else {
                unreachable!()
            };
            p.cuts[id.inner.slot] = None;
            Ok(())
        })();
        Self::collection_observe(
            r,
            Boundary::CollectionVisibilityRetirement,
            &result,
            timing.finish(),
        );
        result
    }
    fn collection_observe<T>(
        r: &mut Recorder,
        boundary: Boundary,
        result: &Result<T, Error>,
        service_time: Option<std::time::Duration>,
    ) {
        r.record(Observation {
            boundary,
            disposition: match result {
                Ok(_) => Disposition::Completed,
                Err(Error::Faulted) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time,
        });
    }
}
