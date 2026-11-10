//! Member-scoped F03 identities and atomic handoff under the all-required-peer gate.
use super::super::super::reader as kernel;
use super::*;
#[cfg(test)]
pub(super) mod tests;
use kernel::{Resolved, Selector, content_bytes, location};
opaque!(VersionSetAnnotationRecordId);
opaque!(VersionSetAnnotationContinuationId);
#[derive(Clone)]
pub enum VersionSetAnnotationQueryV1 {
    Enumerate {
        limit: usize,
        continuation: Option<VersionSetAnnotationContinuationId>,
    },
    Inspect {
        annotation: VersionSetAnnotationRecordId,
    },
    TargetLocation {
        annotation: VersionSetAnnotationRecordId,
    },
}
pub struct VersionSetAnnotationRecordV1 {
    pub reference: VersionSetAnnotationRecordId,
    pub content: Option<AnnotationContentV1>,
    pub location: Option<AnnotationLocationV1>,
}
pub enum VersionSetAnnotationReplyV1 {
    Page {
        snapshot: VersionSetMemberId,
        records: Vec<VersionSetAnnotationRecordV1>,
        continuation: Option<VersionSetAnnotationContinuationId>,
    },
    Record {
        snapshot: VersionSetMemberId,
        record: VersionSetAnnotationRecordV1,
    },
    Location {
        snapshot: VersionSetMemberId,
        reference: VersionSetAnnotationRecordId,
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
redacted!(VersionSetAnnotationQueryV1);
redacted!(VersionSetAnnotationRecordV1);
redacted!(VersionSetAnnotationReplyV1);
impl kernel::result::Record for VersionSetAnnotationRecordV1 {
    type Id = VersionSetAnnotationRecordId;
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
    std::mem::size_of::<VersionSetAnnotationRecordV1>()
        .checked_add(content_bytes(row))
        .ok_or(Error::Limit)
}
fn disclose(
    row: &Row,
    reference: VersionSetAnnotationRecordId,
) -> Result<VersionSetAnnotationRecordV1, Error> {
    Ok(VersionSetAnnotationRecordV1 {
        reference,
        content: kernel::content(row)?,
        location: location(row),
    })
}
impl SourceCoordinator {
    fn resolve_version_annotation_query(
        &self,
        id: &VersionSetMemberId,
        q: &VersionSetAnnotationQueryV1,
        ctx: &Member,
    ) -> Result<Resolved, Error> {
        let request = match q {
            VersionSetAnnotationQueryV1::Enumerate {
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
            VersionSetAnnotationQueryV1::Inspect { annotation }
            | VersionSetAnnotationQueryV1::TargetLocation { annotation } => {
                if !Arc::ptr_eq(&annotation.owner, &self.namespace) {
                    return Err(Error::Unavailable);
                }
                match q {
                    VersionSetAnnotationQueryV1::Inspect { .. } => kernel::Request::Inspect {
                        slot: annotation.slot,
                    },
                    _ => kernel::Request::TargetLocation {
                        slot: annotation.slot,
                    },
                }
            }
        };
        let p = self.version_context_pool()?;
        kernel::resolve(
            &ctx.value.rows,
            &p.selectors,
            id.slot,
            request,
            p.limits.max_page_rows,
        )
    }
    pub fn query_version_set_annotations(
        &mut self,
        id: &VersionSetMemberId,
        q: &VersionSetAnnotationQueryV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<VersionSetAnnotationReplyV1> {
        self.query_version_set_annotations_result(id, q, f, r).ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn query_version_set_annotations_result(
        &mut self,
        id: &VersionSetMemberId,
        q: &VersionSetAnnotationQueryV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<VersionSetAnnotationReplyV1, Error> {
        let result = (|| {
            let (parent, ctx) = self.version_read_member(id, f)?;
            if !ctx.value.classes.includes_annotations() {
                return Err(Error::Unavailable);
            }
            let request_limit = self.version_context_pool()?.limits.max_request_bytes;
            if parent
                .value
                .context_facts_request_bytes()?
                .checked_add(std::mem::size_of::<VersionSetMemberId>())
                .and_then(|n| n.checked_add(std::mem::size_of::<VersionSetAnnotationQueryV1>()))
                .is_none_or(|n| n > request_limit)
            {
                return Err(Error::Limit);
            }
            let resolved = self.resolve_version_annotation_query(id, q, ctx)?;
            self.check_version_context(VersionSetControlTargetV1::Member(id), f, r)?;
            let p = self.version_context_pool()?;
            let ctx = self.version_read_member(id, f)?.1;
            let mut pending = Vec::new();
            let mut reply_bytes = std::mem::size_of::<VersionSetAnnotationReplyV1>();
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
                || p.live_selectors()
                    .checked_add(pending.len())
                    .is_none_or(|n| n > p.limits.max_selectors)
            {
                return Err(Error::Limit);
            }
            if !pending.is_empty()
                && (std::mem::size_of::<VersionSetAnnotationRecordId>()
                    > p.limits.max_selector_bytes
                    || std::mem::size_of::<VersionSetAnnotationContinuationId>()
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
            let reference = |slot| VersionSetAnnotationRecordId {
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
                        Some(next) => Some(VersionSetAnnotationContinuationId {
                            owner: self.namespace.clone(),
                            slot: lookup(Selector::Continuation {
                                context: id.slot,
                                limit,
                                next,
                            })?,
                        }),
                        None => None,
                    };
                    VersionSetAnnotationReplyV1::Page {
                        snapshot: id.clone(),
                        records,
                        continuation,
                    }
                }
                Resolved::Record { index, slot } => VersionSetAnnotationReplyV1::Record {
                    snapshot: id.clone(),
                    record: disclose(&ctx.value.rows[index], reference(slot))?,
                },
                Resolved::Location { index, slot } => VersionSetAnnotationReplyV1::Location {
                    snapshot: id.clone(),
                    reference: reference(slot),
                    location: location(&ctx.value.rows[index]).ok_or(Error::Faulted)?,
                },
            };
            #[cfg(test)]
            if let Some(fault) = self.annotation_read_result_fault {
                use crate::coordinator::source_query::result_contract::AnnotationResultFault as F;
                let (snapshot, record, continuation) = match &mut reply {
                    VersionSetAnnotationReplyV1::Page {
                        snapshot,
                        records,
                        continuation,
                    } => {
                        if matches!(fault, F::Duplicate) && records.len() > 1 {
                            records[1].reference = records[0].reference.clone();
                        }
                        if matches!(fault, F::Truncate) {
                            records.pop();
                        }
                        (snapshot, records.first_mut(), continuation.as_mut())
                    }
                    VersionSetAnnotationReplyV1::Record { snapshot, record } => {
                        (snapshot, Some(record), None)
                    }
                    VersionSetAnnotationReplyV1::Location {
                        snapshot,
                        reference,
                        location,
                    } => {
                        if matches!(fault, F::RecordAlias) {
                            reference.slot = usize::MAX;
                        }
                        if matches!(fault, F::Location) {
                            location.target_generation ^= 1;
                        }
                        (snapshot, None, None)
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
                VersionSetAnnotationReplyV1::Page {
                    snapshot,
                    records,
                    continuation,
                } => (
                    snapshot == id,
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
                VersionSetAnnotationReplyV1::Record { snapshot, record } => {
                    (snapshot == id, kernel::result::View::Record(record))
                }
                VersionSetAnnotationReplyV1::Location {
                    snapshot,
                    reference,
                    location,
                } => (
                    snapshot == id,
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
                    !self.version_set_selector_fault,
                    "injected annotation selector staging fault"
                );
                let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
                    return Err(Error::Faulted);
                };
                for selector in pending {
                    p.selectors.push(Some(selector));
                }
                self.faulted = false;
                self.finish_annotation_preparation(
                    Boundary::VersionSetAnnotationSelectorRegistration,
                    &Ok::<_, Error>(()),
                    r,
                );
            }
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetAnnotationQuery, &result, r);
        result
    }
}
