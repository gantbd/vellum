//! Native named selections only. No collection inference or presentation state.
use super::*;
use crate::selection::{Selection, SelectionSet};

#[derive(Clone, PartialEq, Eq)]
pub struct PublicationAnnotationSelectionV1 {
    pub annotation: AnnotationId,
    /// Host-established recipient reference, not a global annotation ID.
    pub reference: String,
    pub body: bool,
    pub target: bool,
}
#[derive(Clone, PartialEq, Eq)]
pub struct PublicationViewSelectionV1 {
    pub view: LogicalViewId,
    pub reference: String,
}
#[derive(Clone, PartialEq, Eq)]
pub struct FacetPublicationScopeV1 {
    pub source: SourceReadScopeV1,
    pub include_source: bool,
    pub annotations: Vec<PublicationAnnotationSelectionV1>,
    pub views: Vec<PublicationViewSelectionV1>,
}
pub struct FacetPublicationControlV1<'a> {
    pub scope: &'a FacetPublicationScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(PartialEq, Eq)]
pub struct PublicationSourceFacetV1 {
    pub revision: u64,
    pub text: String,
}
#[derive(PartialEq, Eq)]
pub struct PublicationAnnotationFacetV1 {
    pub reference: String,
    /// Exact named membership, not a collection enumeration or deletion cause.
    pub present: bool,
    pub content: Option<(String, u64)>,
    pub location: Option<AnnotationLocationV1>,
}
#[derive(PartialEq, Eq)]
pub struct PublicationViewFacetV1 {
    pub reference: String,
    pub selection_generation: u64,
    pub selections: SelectionSet,
}
impl PublicationViewFacetV1 {
    pub fn profile(&self) -> &'static str {
        crate::selection::POSITIONAL_SELECTION_PROFILE_V1
    }
}
#[derive(PartialEq, Eq)]
pub struct FacetPublicationProjectionV1 {
    document: DocumentId,
    source: Option<PublicationSourceFacetV1>,
    annotations: Vec<PublicationAnnotationFacetV1>,
    views: Vec<PublicationViewFacetV1>,
}
impl FacetPublicationProjectionV1 {
    pub fn document(&self) -> &DocumentId {
        &self.document
    }
    pub fn source(&self) -> Option<&PublicationSourceFacetV1> {
        self.source.as_ref()
    }
    pub fn annotations(&self) -> &[PublicationAnnotationFacetV1] {
        &self.annotations
    }
    pub fn views(&self) -> &[PublicationViewFacetV1] {
        &self.views
    }
    pub(super) fn bytes(&self) -> Result<usize, Error> {
        let mut n = size_of::<Self>();
        if let Some(s) = &self.source {
            n = add(n, size_of::<PublicationSourceFacetV1>())?;
            n = add(n, s.text.len())?;
        }
        for a in &self.annotations {
            n = add(n, size_of::<PublicationAnnotationFacetV1>())?;
            n = add(n, a.reference.len())?;
            if let Some((b, _)) = &a.content {
                n = add(n, b.len())?;
            }
        }
        for v in &self.views {
            n = add(n, size_of::<PublicationViewFacetV1>())?;
            n = add(n, v.reference.len())?;
            n = add(
                n,
                v.selections
                    .selections()
                    .len()
                    .checked_mul(size_of::<Selection>())
                    .ok_or(Error::Limit)?,
            )?;
        }
        Ok(n)
    }
    pub(super) fn valid(&self, s: &FacetPublicationScopeV1, l: FacetPublicationLimitsV1) -> bool {
        self.document == s.source.document
            && self.source.is_some() == s.include_source
            && self.annotations.len() == s.annotations.len()
            && self.views.len() == s.views.len()
            && self
                .annotations
                .iter()
                .zip(&s.annotations)
                .all(|(a, selected)| {
                    a.reference == selected.reference
                        && a.content.is_some() == (a.present && selected.body)
                        && a.location.is_some() == (a.present && selected.target)
                        && a.content.as_ref().is_none_or(|(_, version)| *version > 0)
                })
            && self.views.iter().zip(&s.views).all(|(v, selected)| {
                v.reference == selected.reference
                    && v.selection_generation > 0
                    && v.selections.selections().len() <= l.max_selections
            })
    }
}
redacted!(
    PublicationAnnotationSelectionV1,
    PublicationViewSelectionV1,
    FacetPublicationScopeV1,
    FacetPublicationControlV1<'_>,
    PublicationSourceFacetV1,
    PublicationAnnotationFacetV1,
    PublicationViewFacetV1
);
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}

pub(super) fn scope_bytes(
    s: &FacetPublicationScopeV1,
    l: FacetPublicationLimitsV1,
) -> Result<usize, Error> {
    if s.annotations.len() > l.max_annotations || s.views.len() > l.max_views {
        return Err(Error::Limit);
    }
    let mut n = add(
        size_of::<FacetPublicationScopeV1>(),
        source_context::scope_bytes(&s.source).map_err(source_error)?,
    )?;
    for (i, a) in s.annotations.iter().enumerate() {
        if a.reference.is_empty()
            || a.annotation.document != s.source.document
            || s.annotations[..i]
                .iter()
                .any(|p| p.annotation == a.annotation || p.reference == a.reference)
        {
            return Err(Error::Unavailable);
        }
        n = add(
            add(n, size_of::<PublicationAnnotationSelectionV1>())?,
            a.reference.len(),
        )?;
    }
    for (i, v) in s.views.iter().enumerate() {
        if v.reference.is_empty()
            || s.views[..i]
                .iter()
                .any(|p| p.view == v.view || p.reference == v.reference)
        {
            return Err(Error::Unavailable);
        }
        n = add(
            add(n, size_of::<PublicationViewSelectionV1>())?,
            v.reference.len(),
        )?;
    }
    if !s.include_source && s.annotations.is_empty() && s.views.is_empty() {
        return Err(Error::Unavailable);
    }
    if n > l.max_context_bytes {
        return Err(Error::Limit);
    }
    Ok(n)
}
pub(super) fn copy_scope(
    s: &FacetPublicationScopeV1,
    l: FacetPublicationLimitsV1,
) -> Result<FacetPublicationScopeV1, Error> {
    scope_bytes(s, l)?;
    let mut annotations = Vec::new();
    annotations
        .try_reserve_exact(s.annotations.len())
        .map_err(|_| Error::Capacity)?;
    for a in &s.annotations {
        annotations.push(PublicationAnnotationSelectionV1 {
            annotation: a.annotation.clone(),
            reference: owned(&a.reference)?,
            body: a.body,
            target: a.target,
        });
    }
    let mut views = Vec::new();
    views
        .try_reserve_exact(s.views.len())
        .map_err(|_| Error::Capacity)?;
    for v in &s.views {
        views.push(PublicationViewSelectionV1 {
            view: v.view.clone(),
            reference: owned(&v.reference)?,
        });
    }
    Ok(FacetPublicationScopeV1 {
        source: SourceReadScopeV1 {
            document: s.source.document.clone(),
            policy: owned(&s.source.policy)?,
            context: settings::basis::copy_context(&s.source.context)
                .map_err(|_| Error::Capacity)?,
        },
        include_source: s.include_source,
        annotations,
        views,
    })
}
/// Missing/misbound facts are unknown, not evidence of a grant withdrawal.
pub(super) fn facet_rights(
    s: &FacetPublicationScopeV1,
    f: &TrustedFacetPublicationFactsV1<'_>,
) -> Option<bool> {
    if s.annotations.len() != f.annotation_grants.len()
        || s.views.len() != f.view_grants.len()
        || !s
            .annotations
            .iter()
            .zip(f.annotation_grants)
            .all(|(a, g)| a.annotation == g.annotation)
        || !s.views.iter().zip(f.view_grants).all(|(v, g)| v.view == *g)
    {
        return None;
    }
    Some(
        s.annotations
            .iter()
            .zip(f.annotation_grants)
            .all(|(a, g)| g.visible && (!a.body || g.body) && (!a.target || g.target)),
    )
}
impl SourceCoordinator {
    /// Compare permitted borrowed state before allocating any replacement.
    pub(super) fn facet_publication_matches(
        &self,
        s: &FacetPublicationScopeV1,
        p: &FacetPublicationProjectionV1,
    ) -> Result<bool, Error> {
        let d = self
            .document(&s.source.document)
            .map_err(|_| Error::Unavailable)?;
        if s.include_source
            && p.source
                .as_ref()
                .is_none_or(|old| old.revision != d.revision)
        {
            return Ok(false);
        }
        for (selected, old) in s.annotations.iter().zip(&p.annotations) {
            let r = self
                .publication_annotation(&selected.annotation)
                .map_err(|_| Error::Unavailable)?;
            if r.is_some() != old.present {
                return Ok(false);
            }
            if let Some(r) = r {
                if selected.body
                    && old
                        .content
                        .as_ref()
                        .is_none_or(|(_, version)| *version != r.content_version)
                {
                    return Ok(false);
                }
                if selected.target
                    && old.location
                        != Some(AnnotationLocationV1 {
                            placement: r.placement,
                            source_revision: r.placement_revision,
                            target_generation: r.target_generation,
                        })
                {
                    return Ok(false);
                }
            }
        }
        for (selected, old) in s.views.iter().zip(&p.views) {
            let (source, stamp, selections) = self
                .publication_view(&selected.view)
                .map_err(|_| Error::Unavailable)?;
            if source.is_none_or(|v| v.document != s.source.document) {
                return Err(Error::Unavailable);
            }
            if old.selection_generation != stamp.selection_generation
                || old.selections != *selections
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn facet_publication_projection(
        &self,
        s: &FacetPublicationScopeV1,
        l: FacetPublicationLimitsV1,
    ) -> Result<Arc<FacetPublicationProjectionV1>, Error> {
        scope_bytes(s, l)?;
        let d = self
            .document(&s.source.document)
            .map_err(|_| Error::Unavailable)?;
        // Preflight every payload/selection before allocating the replacement.
        let mut bytes = size_of::<FacetPublicationProjectionV1>();
        if s.include_source {
            bytes = add(
                add(bytes, size_of::<PublicationSourceFacetV1>())?,
                d.source.text().len(),
            )?;
        }
        for a in &s.annotations {
            let record = self.publication_annotation(&a.annotation).map_err(|e| {
                if e == CoordinatorError::Faulted {
                    Error::Faulted
                } else {
                    Error::Unavailable
                }
            })?;
            bytes = add(
                add(bytes, size_of::<PublicationAnnotationFacetV1>())?,
                a.reference.len(),
            )?;
            if a.body
                && let Some(r) = record
            {
                bytes = add(bytes, r.body.len())?;
            }
        }
        for v in &s.views {
            let (version, _, selections) = self
                .publication_view(&v.view)
                .map_err(|_| Error::Unavailable)?;
            if version
                != Some(&DocumentVersion {
                    document: s.source.document.clone(),
                    revision: d.revision,
                })
            {
                return Err(Error::Unavailable);
            }
            if selections.selections().len() > l.max_selections {
                return Err(Error::Limit);
            }
            bytes = add(
                add(bytes, size_of::<PublicationViewFacetV1>())?,
                v.reference.len(),
            )?;
            bytes = add(
                bytes,
                selections
                    .selections()
                    .len()
                    .checked_mul(size_of::<Selection>())
                    .ok_or(Error::Limit)?,
            )?;
        }
        if bytes > l.max_projection_bytes
            || add(bytes, size_of::<FacetPublicationDeliveryV1>())? > l.max_reply_bytes
        {
            return Err(Error::Limit);
        }
        let source = if s.include_source {
            Some(PublicationSourceFacetV1 {
                revision: d.revision,
                text: owned(d.source.text())?,
            })
        } else {
            None
        };
        let mut annotations = Vec::new();
        annotations
            .try_reserve_exact(s.annotations.len())
            .map_err(|_| Error::Capacity)?;
        for a in &s.annotations {
            let r = self
                .publication_annotation(&a.annotation)
                .map_err(|_| Error::Faulted)?;
            let content = if a.body {
                r.as_ref()
                    .map(|r| Ok((owned(r.body)?, r.content_version)))
                    .transpose()?
            } else {
                None
            };
            let location = if a.target {
                r.as_ref().map(|r| AnnotationLocationV1 {
                    placement: r.placement,
                    source_revision: r.placement_revision,
                    target_generation: r.target_generation,
                })
            } else {
                None
            };
            annotations.push(PublicationAnnotationFacetV1 {
                reference: owned(&a.reference)?,
                present: r.is_some(),
                content,
                location,
            });
        }
        let mut views = Vec::new();
        views
            .try_reserve_exact(s.views.len())
            .map_err(|_| Error::Capacity)?;
        for v in &s.views {
            let (_, stamp, selections) =
                self.publication_view(&v.view).map_err(|_| Error::Faulted)?;
            views.push(PublicationViewFacetV1 {
                reference: owned(&v.reference)?,
                selection_generation: stamp.selection_generation,
                selections: selections.copy().map_err(|_| Error::Capacity)?,
            });
        }
        Ok(Arc::new(FacetPublicationProjectionV1 {
            document: s.source.document.clone(),
            source,
            annotations,
            views,
        }))
    }
}
