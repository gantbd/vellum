//! Observer-local identity ledger and atomic full-frame staging.
use super::*;

#[derive(Clone, PartialEq, Eq)]
pub struct CollectionPublicationScopeV1 {
    pub source: SourceReadScopeV1,
    pub ceiling: CollectionFacetCeilingV1,
}
pub struct CollectionPublicationControlV1<'a> {
    pub scope: &'a CollectionPublicationScopeV1,
    pub owner_control_allowed: Option<bool>,
    pub retention_released: Option<bool>,
    pub delivery_drained: Option<bool>,
}
#[derive(Clone, PartialEq, Eq)]
pub struct CollectionAnnotationReferenceV1 {
    stream: CollectionPublicationStreamIdV1,
    index: usize,
}
#[derive(PartialEq, Eq)]
pub struct CollectionAnnotationRowV1 {
    pub reference: CollectionAnnotationReferenceV1,
    pub content: Option<(String, u64)>,
    pub location: Option<AnnotationLocationV1>,
}
#[derive(PartialEq, Eq)]
pub struct CollectionPublicationProjectionV1 {
    document: DocumentId,
    rows: Vec<CollectionAnnotationRowV1>,
}
impl CollectionPublicationProjectionV1 {
    pub fn document(&self) -> &DocumentId {
        &self.document
    }
    pub fn rows(&self) -> &[CollectionAnnotationRowV1] {
        &self.rows
    }
    pub(super) fn bytes(&self) -> Result<usize, Error> {
        self.rows.iter().try_fold(size_of::<Self>(), |n, r| {
            n.checked_add(size_of::<CollectionAnnotationRowV1>())
                .and_then(|n| n.checked_add(r.content.as_ref().map_or(0, |(body, _)| body.len())))
                .ok_or(Error::Limit)
        })
    }
    pub(super) fn valid(&self, scope: &CollectionPublicationScopeV1, bindings: &[Binding]) -> bool {
        self.document == scope.source.document
            && self.rows.len() <= bindings.len()
            && self.rows.iter().enumerate().all(|(i, r)| {
                bindings.iter().any(|b| b.reference == r.reference)
                    && self.rows[..i]
                        .iter()
                        .all(|old| old.reference != r.reference)
                    && (scope.ceiling.body || r.content.is_none())
                    && (scope.ceiling.target || r.location.is_none())
                    && r.content.as_ref().is_none_or(|(_, v)| *v > 0)
                    && r.location.as_ref().is_none_or(|v| match v.placement {
                        crate::attachment::Placement::Located { .. } => v.source_revision.is_some(),
                        crate::attachment::Placement::Detached => v.source_revision.is_none(),
                        crate::attachment::Placement::Unavailable => false,
                    })
            })
    }
}
#[derive(Clone)]
pub(super) struct Binding {
    annotation: AnnotationId,
    reference: CollectionAnnotationReferenceV1,
}
pub(super) struct Stage {
    pub bindings: Vec<Binding>,
    pub projection: Arc<CollectionPublicationProjectionV1>,
    pub new_bindings: usize,
}
redacted!(
    CollectionPublicationScopeV1,
    CollectionPublicationControlV1<'_>,
    CollectionAnnotationReferenceV1,
    CollectionAnnotationRowV1
);
pub(super) fn scope_bytes(s: &CollectionPublicationScopeV1) -> Result<usize, Error> {
    source_context::scope_bytes(&s.source)
        .map_err(source_error)?
        .checked_add(size_of::<CollectionPublicationScopeV1>())
        .ok_or(Error::Limit)
}
pub(super) fn copy_scope(
    s: &CollectionPublicationScopeV1,
) -> Result<CollectionPublicationScopeV1, Error> {
    Ok(CollectionPublicationScopeV1 {
        source: SourceReadScopeV1 {
            document: s.source.document.clone(),
            policy: owned(&s.source.policy)?,
            context: settings::basis::copy_context(&s.source.context)
                .map_err(|_| Error::Capacity)?,
        },
        ceiling: s.ceiling,
    })
}
pub(super) fn valid_bindings(
    rows: &[Binding],
    id: &CollectionPublicationStreamIdV1,
    s: &CollectionPublicationScopeV1,
    l: CollectionPublicationLimitsV1,
) -> bool {
    rows.len() <= l.max_bindings
        && rows
            .len()
            .checked_mul(size_of::<Binding>())
            .is_some_and(|n| n <= l.max_binding_bytes)
        && rows.iter().enumerate().all(|(i, b)| {
            b.annotation.document == s.source.document
                && b.reference.stream == *id
                && b.reference.index == i
                && rows[..i].iter().all(|old| old.annotation != b.annotation)
        })
}
pub(super) fn matches(
    c: &CollectionVisibilityCaptureV1,
    p: &CollectionPublicationProjectionV1,
    bindings: &[Binding],
) -> bool {
    c.rows().len() == p.rows.len()
        && c.rows().zip(&p.rows).all(|((id, body, location), row)| {
            bindings
                .iter()
                .any(|b| b.annotation == *id && b.reference == row.reference)
                && body.map(|(_, v)| v) == row.content.as_ref().map(|(_, v)| *v)
                && location == row.location
        })
}
pub(super) fn stage(
    id: &CollectionPublicationStreamIdV1,
    s: &CollectionPublicationScopeV1,
    c: &CollectionVisibilityCaptureV1,
    old: &[Binding],
    l: CollectionPublicationLimitsV1,
) -> Result<Stage, Error> {
    if !valid_bindings(old, id, s, l) {
        return Err(Error::Faulted);
    }
    let mut new_bindings = 0usize;
    let mut bytes = size_of::<CollectionPublicationProjectionV1>();
    for (annotation, body, location) in c.rows() {
        if annotation.document != s.source.document
            || (!s.ceiling.body && body.is_some())
            || (!s.ceiling.target && location.is_some())
        {
            return Err(Error::Faulted);
        }
        if !old.iter().any(|b| b.annotation == *annotation) {
            new_bindings = new_bindings.checked_add(1).ok_or(Error::Limit)?;
        }
        bytes = bytes
            .checked_add(size_of::<CollectionAnnotationRowV1>())
            .and_then(|n| n.checked_add(body.map_or(0, |(b, _)| b.len())))
            .ok_or(Error::Limit)?;
    }
    let count = old.len().checked_add(new_bindings).ok_or(Error::Limit)?;
    if count > l.max_bindings
        || count
            .checked_mul(size_of::<Binding>())
            .is_none_or(|n| n > l.max_binding_bytes)
        || bytes > l.max_projection_bytes
        || bytes
            .checked_add(size_of::<CollectionPublicationDeliveryV1>())
            .is_none_or(|n| n > l.max_reply_bytes)
    {
        return Err(Error::Limit);
    }
    let mut bindings = Vec::new();
    bindings
        .try_reserve_exact(count)
        .map_err(|_| Error::Capacity)?;
    bindings.extend_from_slice(old);
    let mut rows = Vec::new();
    rows.try_reserve_exact(c.rows().len())
        .map_err(|_| Error::Capacity)?;
    for (annotation, body, location) in c.rows() {
        let reference = if let Some(b) = bindings.iter().find(|b| b.annotation == *annotation) {
            b.reference.clone()
        } else {
            let reference = CollectionAnnotationReferenceV1 {
                stream: id.clone(),
                index: bindings.len(),
            };
            bindings.push(Binding {
                annotation: annotation.clone(),
                reference: reference.clone(),
            });
            reference
        };
        rows.push(CollectionAnnotationRowV1 {
            reference,
            content: body.map(|(b, v)| Ok((owned(b)?, v))).transpose()?,
            location,
        });
    }
    let projection = Arc::new(CollectionPublicationProjectionV1 {
        document: s.source.document.clone(),
        rows,
    });
    if !projection.valid(s, &bindings) {
        return Err(Error::Faulted);
    }
    Ok(Stage {
        bindings,
        projection,
        new_bindings,
    })
}
