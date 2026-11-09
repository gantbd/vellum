//! Source-admitted member products, always gated by the complete owning set.
use super::*;
use crate::coordinates::{CoordinateBudget, CoordinateError, SourceCoordinates};
use crate::coordinator::source_context::Resolved;
#[cfg(test)]
mod tests;
pub struct VersionSetSourceReplyV1 {
    pub snapshot: VersionSetMemberId,
    pub content: SourceContextContentV1,
}
impl std::fmt::Debug for VersionSetSourceReplyV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VersionSetSourceReplyV1(<redacted>)")
    }
}
fn copy_error(e: SourceQueryError) -> Error {
    match e {
        SourceQueryError::Coordinate(CoordinateError::Capacity) => Error::Capacity,
        _ => Error::Faulted,
    }
}
impl SourceCoordinator {
    pub(super) fn version_read_member<'a>(
        &'a self,
        id: &VersionSetMemberId,
        f: &TrustedVersionSetContextFactsV1<'_>,
    ) -> Result<(&'a Context, &'a Member), Error> {
        let parent = self.version_parent_slot(VersionSetControlTargetV1::Member(id))?;
        if !Arc::ptr_eq(&f.context.owner, &self.namespace) || f.context.slot != parent {
            return Err(Error::Unavailable);
        }
        let p = self.version_context_pool()?;
        let row = p.rows[parent].as_ref().ok_or(Error::Faulted)?;
        if row.stopped
            || f.members.len() != row.value.members.len()
            || f.members
                .iter()
                .zip(&row.value.members)
                .any(|(f, m)| f.scope != &m.scope)
        {
            return Err(Error::Unavailable);
        }
        let alias = p.aliases[id.slot].as_ref().ok_or(Error::Faulted)?;
        let member = row.value.members.get(alias.index).ok_or(Error::Faulted)?;
        Ok((row, member))
    }
    pub fn query_version_set_source(
        &mut self,
        id: &VersionSetMemberId,
        q: SourceContextQueryV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Option<VersionSetSourceReplyV1> {
        self.query_version_set_source_result(id, q, f, r).ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn query_version_set_source_result(
        &mut self,
        id: &VersionSetMemberId,
        q: SourceContextQueryV1,
        f: &TrustedVersionSetContextFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<VersionSetSourceReplyV1, Error> {
        let result = (|| {
            let (row, member) = self.version_read_member(id, f)?;
            // Reject out-of-class and malformed packets before accepting loss.
            let (source, _, profile) = member.value.source.as_ref().ok_or(Error::Unavailable)?;
            let inner_request = scope_bytes(&member.scope)?
                .checked_add(std::mem::size_of::<VersionSetMemberId>())
                .and_then(|n| n.checked_add(std::mem::size_of::<SourceContextQueryV1>()))
                .ok_or(Error::Limit)?;
            let outer_request = row
                .value
                .context_facts_request_bytes()?
                .checked_add(std::mem::size_of::<VersionSetMemberId>())
                .and_then(|n| n.checked_add(std::mem::size_of::<SourceContextQueryV1>()))
                .ok_or(Error::Limit)?;
            if inner_request > profile.max_request_bytes
                || outer_request > self.version_context_pool()?.limits.max_request_bytes
            {
                return Err(Error::Limit);
            }
            let resolved = Resolved::resolve(source, q)?;
            self.check_version_context(VersionSetControlTargetV1::Member(id), f, r)?;
            let (_, member) = self.version_read_member(id, f)?;
            let (source, version, profile) = member.value.source.as_ref().ok_or(Error::Faulted)?;
            let payload = resolved.payload_bytes();
            let outer_limit = self.version_context_pool()?.limits.max_reply_bytes;
            if std::mem::size_of::<SourceContextContentV1>()
                .checked_add(payload)
                .is_none_or(|n| n > profile.max_reply_bytes)
                || std::mem::size_of::<VersionSetSourceReplyV1>()
                    .checked_add(payload)
                    .is_none_or(|n| n > outer_limit)
            {
                return Err(Error::Limit);
            }
            let content = match resolved {
                Resolved::Range(a, b) => SourceContextContentV1::Range(
                    SourceRangeReply::from_source(source, version, a, b).map_err(copy_error)?,
                ),
                Resolved::Line(index, span) => {
                    SourceCoordinates::new(source)
                        .line_span(
                            index,
                            CoordinateBudget {
                                max_scan_bytes: profile.max_scan_bytes,
                            },
                        )
                        .map_err(|e| match e {
                            CoordinateError::Limit => Error::Limit,
                            CoordinateError::Capacity => Error::Capacity,
                            _ => Error::Faulted,
                        })?;
                    SourceContextContentV1::Line(
                        SourceLineReply::from_source(source, version, index, span)
                            .map_err(copy_error)?,
                    )
                }
            };
            let version = version.clone();
            #[allow(unused_mut)]
            let mut reply = VersionSetSourceReplyV1 {
                snapshot: id.clone(),
                content,
            };
            #[cfg(test)]
            {
                crate::coordinator::source_query::result_contract::corrupt(
                    &mut reply.content,
                    self.source_read_result_fault,
                );
                if matches!(
                    self.source_read_result_fault,
                    Some(
                        crate::coordinator::source_query::result_contract::SourceResultFault::Alias
                    )
                ) {
                    reply.snapshot.slot = usize::MAX;
                }
            }
            self.guard_source_result(resolved, &version, &reply.content, reply.snapshot == *id, r)?;
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetSourceQuery, &result, r);
        result
    }
}
