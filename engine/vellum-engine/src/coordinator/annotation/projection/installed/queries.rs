//! Atomic F03 disclosure and opaque selector registration under the actual owner.
use super::*;
#[cfg(test)]
pub(super) mod tests;
macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            owner: Arc<()>,
            slot: usize,
        }
        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
            }
        }
        impl Eq for $name {}
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}
identity!(AnnotationRecordId);
identity!(AnnotationContinuationId);
#[derive(Clone)]
pub enum AnnotationQueryV1 {
    Enumerate {
        limit: usize,
        continuation: Option<AnnotationContinuationId>,
    },
    Inspect {
        annotation: AnnotationRecordId,
    },
    TargetLocation {
        annotation: AnnotationRecordId,
    },
}
pub struct AnnotationContentV1 {
    pub body: String,
    pub version: u64,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AnnotationLocationV1 {
    pub placement: Placement,
    pub source_revision: Option<u64>,
    pub target_generation: u64,
}
pub struct AnnotationRecordV1 {
    pub reference: AnnotationRecordId,
    pub content: Option<AnnotationContentV1>,
    pub location: Option<AnnotationLocationV1>,
}
pub enum AnnotationQueryReplyV1 {
    Page {
        context: AnnotationReadContextId,
        records: Vec<AnnotationRecordV1>,
        continuation: Option<AnnotationContinuationId>,
    },
    Record {
        context: AnnotationReadContextId,
        record: AnnotationRecordV1,
    },
    Location {
        context: AnnotationReadContextId,
        reference: AnnotationRecordId,
        location: AnnotationLocationV1,
    },
}
macro_rules! redacted {
    ($name:ident) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($name), "(<redacted>)"))
            }
        }
    };
}
redacted!(AnnotationQueryV1);
redacted!(AnnotationQueryReplyV1);
redacted!(AnnotationContentV1);
redacted!(AnnotationLocationV1);
redacted!(AnnotationRecordV1);
use super::super::reader as kernel;
pub(super) use kernel::Selector;
use kernel::{Resolved, content_bytes, location};
impl ContextPool {
    fn selector_slot(&self, key: Selector) -> Option<usize> {
        self.selectors.iter().position(|v| v.as_ref() == Some(&key))
    }
}
impl kernel::result::Record for AnnotationRecordV1 {
    type Id = AnnotationRecordId;
    fn id(&self) -> &Self::Id {
        &self.reference
    }
    fn content(&self) -> Option<&AnnotationContentV1> {
        self.content.as_ref()
    }
    fn location(&self) -> Option<AnnotationLocationV1> {
        self.location
    }
}
fn row_bytes(row: &Row) -> Result<usize, Error> {
    std::mem::size_of::<AnnotationRecordV1>()
        .checked_add(content_bytes(row))
        .ok_or(Error::Limit)
}
fn disclose(row: &Row, reference: AnnotationRecordId) -> Result<AnnotationRecordV1, Error> {
    Ok(AnnotationRecordV1 {
        reference,
        content: kernel::content(row)?,
        location: location(row),
    })
}
impl SourceCoordinator {
    fn resolve_annotation_query(
        &self,
        id: &AnnotationReadContextId,
        q: &AnnotationQueryV1,
        ctx: &Context,
    ) -> Result<Resolved, Error> {
        let request = match q {
            AnnotationQueryV1::Enumerate {
                limit,
                continuation,
            } => kernel::Request::Enumerate {
                limit: *limit,
                continuation: match continuation {
                    Some(c) => {
                        if !Arc::ptr_eq(&c.owner, &self.namespace) {
                            return Err(Error::Unavailable);
                        }
                        Some(c.slot)
                    }
                    None => None,
                },
            },
            AnnotationQueryV1::Inspect { annotation }
            | AnnotationQueryV1::TargetLocation { annotation } => {
                if !Arc::ptr_eq(&annotation.owner, &self.namespace) {
                    return Err(Error::Unavailable);
                }
                match q {
                    AnnotationQueryV1::Inspect { .. } => kernel::Request::Inspect {
                        slot: annotation.slot,
                    },
                    _ => kernel::Request::TargetLocation {
                        slot: annotation.slot,
                    },
                }
            }
        };
        let p = self.annotation_context_pool()?;
        kernel::resolve(
            &ctx.value.rows,
            &p.selectors,
            id.slot,
            request,
            p.limits.max_page_rows,
        )
    }
    pub fn query_annotation_context(
        &mut self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
        q: &AnnotationQueryV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<AnnotationQueryReplyV1> {
        self.query_annotation_context_result(id, s, q, f, r).ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn query_annotation_context_result(
        &mut self,
        id: &AnnotationReadContextId,
        s: &SourceReadScopeV1,
        q: &AnnotationQueryV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<AnnotationQueryReplyV1, Error> {
        let result = (|| {
            let p = self.annotation_context_pool()?;
            if !valid_scope(s, p) || id != f.context || s != f.scope {
                return Err(Error::Unavailable);
            }
            if scope_bytes(s)?
                .checked_add(std::mem::size_of::<AnnotationReadContextId>())
                .and_then(|n| n.checked_add(std::mem::size_of::<AnnotationQueryV1>()))
                .is_none_or(|n| n > p.limits.max_request_bytes)
            {
                return Err(Error::Limit);
            }
            let ctx = self.annotation_context_row(id, s)?;
            if ctx.stopped || !ctx.value.classes.includes_annotations() {
                return Err(Error::Unavailable);
            }
            let resolved = self.resolve_annotation_query(id, q, ctx)?;
            self.check_annotation_context(id, s, f, r)?;
            let p = self.annotation_context_pool()?;
            let ctx = self.annotation_context_row(id, s)?;
            let mut pending = Vec::new();
            let mut reply_bytes = std::mem::size_of::<AnnotationQueryReplyV1>();
            match &resolved {
                Resolved::Page {
                    start,
                    end,
                    next,
                    limit,
                } => {
                    pending
                        .try_reserve_exact(end - start + usize::from(next.is_some()))
                        .map_err(|_| Error::Capacity)?;
                    for index in *start..*end {
                        let row = &ctx.value.rows[index];
                        let bytes = row_bytes(row)?;
                        if bytes > p.limits.max_row_bytes {
                            return Err(Error::Limit);
                        }
                        reply_bytes = reply_bytes.checked_add(bytes).ok_or(Error::Limit)?;
                        let key = Selector::Record {
                            context: id.slot,
                            index,
                        };
                        if p.selector_slot(key).is_none() {
                            pending.push(key);
                        }
                    }
                    if let Some(next) = next {
                        let key = Selector::Continuation {
                            context: id.slot,
                            limit: *limit,
                            next: *next,
                        };
                        if p.selector_slot(key).is_none() {
                            pending.push(key);
                        }
                    }
                }
                Resolved::Record { index, .. } => {
                    let row = &ctx.value.rows[*index];
                    if row_bytes(row)? > p.limits.max_row_bytes {
                        return Err(Error::Limit);
                    }
                    reply_bytes = reply_bytes
                        .checked_add(content_bytes(row))
                        .ok_or(Error::Limit)?;
                }
                Resolved::Location { index, .. } => {
                    if location(&ctx.value.rows[*index]).is_none() {
                        return Err(Error::Unavailable);
                    }
                }
            }
            if reply_bytes > p.limits.max_reply_bytes {
                return Err(Error::Limit);
            }
            if p.issued()?
                .checked_add(pending.len())
                .is_none_or(|n| n > p.limits.max_issued)
                || p.selector_count()
                    .checked_add(pending.len())
                    .is_none_or(|n| n > p.limits.max_selectors)
            {
                return Err(Error::Limit);
            }
            if !pending.is_empty()
                && (std::mem::size_of::<AnnotationRecordId>() > p.limits.max_selector_bytes
                    || std::mem::size_of::<AnnotationContinuationId>()
                        > p.limits.max_selector_bytes)
            {
                return Err(Error::Limit);
            }
            if p.selectors.capacity() < p.limits.max_issued {
                return Err(Error::Faulted);
            }
            let lookup = |key| {
                p.selector_slot(key)
                    .or_else(|| {
                        pending
                            .iter()
                            .position(|v| *v == key)
                            .and_then(|offset| p.selectors.len().checked_add(offset))
                    })
                    .ok_or(Error::Faulted)
            };
            let reference = |slot| AnnotationRecordId {
                owner: self.namespace.clone(),
                slot,
            };
            #[allow(unused_mut)]
            let mut reply = match resolved {
                Resolved::Page {
                    start,
                    end,
                    next,
                    limit,
                } => {
                    let mut records = Vec::new();
                    records
                        .try_reserve_exact(end - start)
                        .map_err(|_| Error::Capacity)?;
                    for index in start..end {
                        records.push(disclose(
                            &ctx.value.rows[index],
                            reference(lookup(Selector::Record {
                                context: id.slot,
                                index,
                            })?),
                        )?);
                    }
                    let continuation = match next {
                        Some(next) => Some(AnnotationContinuationId {
                            owner: self.namespace.clone(),
                            slot: lookup(Selector::Continuation {
                                context: id.slot,
                                limit,
                                next,
                            })?,
                        }),
                        None => None,
                    };
                    AnnotationQueryReplyV1::Page {
                        context: id.clone(),
                        records,
                        continuation,
                    }
                }
                Resolved::Record { index, slot } => AnnotationQueryReplyV1::Record {
                    context: id.clone(),
                    record: disclose(&ctx.value.rows[index], reference(slot))?,
                },
                Resolved::Location { index, slot } => AnnotationQueryReplyV1::Location {
                    context: id.clone(),
                    reference: reference(slot),
                    location: location(&ctx.value.rows[index]).ok_or(Error::Faulted)?,
                },
            };
            #[cfg(test)]
            if let Some(fault) = self.annotation_read_result_fault {
                use crate::coordinator::source_query::result_contract::AnnotationResultFault as F;
                let (snapshot, record, continuation) = match &mut reply {
                    AnnotationQueryReplyV1::Page {
                        context,
                        records,
                        continuation,
                    } => {
                        if matches!(fault, F::Duplicate) && records.len() > 1 {
                            records[1].reference = records[0].reference.clone();
                        }
                        if matches!(fault, F::Truncate) {
                            records.pop();
                        }
                        (context, records.first_mut(), continuation.as_mut())
                    }
                    AnnotationQueryReplyV1::Record { context, record } => {
                        (context, Some(record), None)
                    }
                    AnnotationQueryReplyV1::Location {
                        context,
                        reference,
                        location,
                    } => {
                        if matches!(fault, F::RecordAlias) {
                            reference.slot = usize::MAX;
                        }
                        if matches!(fault, F::Location) {
                            location.target_generation ^= 1;
                        }
                        (context, None, None)
                    }
                };
                if matches!(fault, F::Snapshot) {
                    snapshot.slot = usize::MAX;
                }
                if let Some(c) = continuation
                    && matches!(fault, F::Continuation)
                {
                    c.slot = usize::MAX;
                }
                if let Some(record) = record {
                    match fault {
                        F::RecordAlias => record.reference.slot = usize::MAX,
                        F::MissingContent => record.content = None,
                        F::Version => {
                            if let Some(content) = &mut record.content {
                                content.version ^= 1;
                            }
                        }
                        F::Location => {
                            if let Some(location) = &mut record.location {
                                location.target_generation ^= 1;
                            }
                        }
                        _ => (),
                    }
                }
            }
            let staged = |slot: usize| {
                if slot < p.selectors.len() {
                    p.selectors.get(slot).copied().flatten()
                } else {
                    slot.checked_sub(p.selectors.len())
                        .and_then(|n| pending.get(n).copied())
                }
            };
            let (bound, view) = match &reply {
                AnnotationQueryReplyV1::Page {
                    context,
                    records,
                    continuation,
                } => (
                    context == id,
                    kernel::result::View::Page(
                        records.as_slice(),
                        continuation.as_ref().map(|c| {
                            if Arc::ptr_eq(&c.owner, &self.namespace) {
                                staged(c.slot)
                            } else {
                                None
                            }
                        }),
                    ),
                ),
                AnnotationQueryReplyV1::Record { context, record } => {
                    (context == id, kernel::result::View::Record(record))
                }
                AnnotationQueryReplyV1::Location {
                    context,
                    reference,
                    location,
                } => (
                    context == id,
                    kernel::result::View::Location(reference, location),
                ),
            };
            let accepted = kernel::result::valid(
                resolved,
                &ctx.value.rows,
                id.slot,
                bound,
                view,
                |reference| {
                    if Arc::ptr_eq(&reference.owner, &self.namespace) {
                        staged(reference.slot).map(|key| (reference.slot, key))
                    } else {
                        None
                    }
                },
            );
            if !self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeAnnotationResultInvariant,
                Boundary::AnnotationReadResultValidation,
                r,
            ) {
                return Err(Error::Faulted);
            }
            if !pending.is_empty() {
                // Reply ownership and the full selector candidate set are ready.
                // There is no allocation/callback after this commit guard.
                self.faulted = true;
                #[cfg(test)]
                assert!(
                    !self.annotation_selector_registration_fault,
                    "injected annotation selector staging fault"
                );
                let InstalledState::Active(p) = &mut self.annotation_contexts else {
                    return Err(Error::Faulted);
                };
                for selector in pending {
                    p.selectors.push(Some(selector));
                }
                self.faulted = false;
                self.finish_annotation_preparation(
                    Boundary::AnnotationSelectorRegistration,
                    &Ok::<_, Error>(()),
                    r,
                );
            }
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextQuery, &result, r);
        result
    }
}
