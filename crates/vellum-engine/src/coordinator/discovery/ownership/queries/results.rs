//! Actual native F01 reply guard, before selector publication. This validates
//! typed values and original native roles, not JSON, host truth or wire identity.
use super::*;

fn capture(v: &InertCaptureV1, limits: CaptureLimitsV1) -> bool {
    let u = v.usage();
    // Native captures are immutable and only the bounded copier constructs them.
    // The guard does not reparse metadata, invoke schemas or clone payloads.
    u.nodes > 0
        && u.depth > 0
        && u.nodes <= limits.max_nodes
        && u.depth <= limits.max_depth
        && u.peak_bytes <= limits.max_bytes
        && u.retained_bytes <= u.peak_bytes
}
fn metadata(
    expected: &Option<Arc<InertCaptureV1>>,
    actual: &Option<Arc<InertCaptureV1>>,
    schema: bool,
    limits: CaptureLimitsV1,
) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(a), Some(b)) if Arc::ptr_eq(a, b) && capture(b, limits) => {
            let Value::Object(fields) = b.root() else {
                return false;
            };
            let allowed: &[&str] = if schema {
                arguments::SCHEMA_FACETS
            } else {
                arguments::PRESENTATION_FIELDS
            };
            fields.len() <= allowed.len()
                && (!schema || fields.len() == allowed.len())
                && fields.iter().enumerate().all(|(i, (k, _))| {
                    allowed.contains(&k.as_str()) && fields[..i].iter().all(|(old, _)| old != k)
                })
        }
        _ => false,
    }
}
fn entry(s: &Stage<'_>, v: &DiscoveryEntryIdV1, index: usize) -> bool {
    s.candidate(&v.owner, &v.marker, v.slot) == Some(Kind::Entry(index))
}
fn variant(s: &Stage<'_>, v: &DiscoveryVariantIdV1, ei: usize, vi: usize) -> bool {
    s.candidate(&v.owner, &v.marker, v.slot)
        == Some(Kind::Variant {
            entry: ei,
            index: vi,
        })
}
fn mentioned(reply: &DiscoveryReplyV1, owner: &Arc<()>, marker: &Arc<()>, slot: usize) -> bool {
    let matches =
        |o: &Arc<()>, m: &Arc<()>, i| Arc::ptr_eq(o, owner) && Arc::ptr_eq(m, marker) && i == slot;
    match reply {
        DiscoveryReplyV1::Page {
            entries,
            continuation,
            ..
        } => {
            entries
                .iter()
                .any(|e| matches(&e.reference.owner, &e.reference.marker, e.reference.slot))
                || continuation
                    .as_ref()
                    .is_some_and(|v| matches(&v.owner, &v.marker, v.slot))
        }
        DiscoveryReplyV1::Descriptor { descriptor: d, .. } => {
            matches(&d.reference.owner, &d.reference.marker, d.reference.slot)
                || d.variants
                    .iter()
                    .any(|v| matches(&v.reference.owner, &v.reference.marker, v.reference.slot))
        }
        DiscoveryReplyV1::Advice {
            entry: e,
            variant: v,
            ..
        } => matches(&e.owner, &e.marker, e.slot) || matches(&v.owner, &v.marker, v.slot),
    }
}
fn valid(
    s: &Stage<'_>,
    id: &DiscoveryContextIdV1,
    query: DiscoveryQueryV1<'_>,
    reply: &DiscoveryReplyV1,
) -> Option<()> {
    let c = s.context;
    if c.stopped
        || reply.result_form() != query.operation().result_declaration().form
        || !Arc::ptr_eq(&id.marker, &c.marker)
        || reply_bytes(reply).ok()? > s.limits.max_reply_bytes
        || add(s.live, s.selectors.len()).ok()? > s.limits.max_selectors
        || add(s.issued, s.selectors.len()).ok()? > s.limits.max_issued
    {
        return None;
    }
    for (i, pending) in s.selectors.iter().enumerate() {
        if size_of::<Selector>() > s.limits.max_selector_bytes
            || c.selectors
                .iter()
                .chain(&s.selectors[..i])
                .any(|v| v.kind == pending.kind || Arc::ptr_eq(&v.marker, &pending.marker))
            || !mentioned(
                reply,
                &c.marker,
                &pending.marker,
                add(c.selectors.len(), i).ok()?,
            )
        {
            return None;
        }
    }
    match (query, reply) {
        (
            DiscoveryQueryV1::List {
                limit,
                continuation: request,
            },
            DiscoveryReplyV1::Page {
                context,
                coverage: DiscoveryCoverageV1::CapturedPermitted,
                entries,
                continuation,
            },
        ) => {
            if context != id
                || limit == 0
                || limit > s.limits.max_page_rows
                || (limit as u128) > 9_007_199_254_740_991
                || entries.len() > limit
            {
                return None;
            }
            let start = if let Some(v) = request {
                match s.original(&v.owner, &v.marker, v.slot).ok()? {
                    Kind::Cursor { limit: old, start } if old == limit => start,
                    _ => return None,
                }
            } else {
                0
            };
            let end = add(start, limit).ok()?.min(c.rows.len());
            if entries.len() != end.checked_sub(start)? {
                return None;
            }
            for (i, v) in entries.iter().enumerate() {
                let index = add(start, i).ok()?;
                let row = c.rows.get(index)?;
                if !entry(s, &v.reference, index)
                    || !natural(v.version)
                    || v.version == 0
                    || v.version != row.version
                    || v.kind != row.kind
                {
                    return None;
                }
            }
            match continuation {
                Some(v)
                    if end < c.rows.len()
                        && s.candidate(&v.owner, &v.marker, v.slot)
                            == Some(Kind::Cursor { limit, start: end }) => {}
                None if end == c.rows.len() => {}
                _ => return None,
            }
        }
        (
            DiscoveryQueryV1::Inspect { entry: request },
            DiscoveryReplyV1::Descriptor {
                context,
                descriptor: d,
            },
        ) => {
            if context != id || d.reference != *request || !natural(d.version) || d.version == 0 {
                return None;
            }
            let Kind::Entry(ei) = s
                .original(&request.owner, &request.marker, request.slot)
                .ok()?
            else {
                return None;
            };
            let row = c.rows.get(ei)?;
            if d.version != row.version
                || d.kind != row.kind
                || d.variants.len() != row.variants.len()
                || d.variants.len() > s.limits.max_variants
                || (!row.presentation_visible && d.presentation.is_some())
                || (!row.schema_visible && d.variants.iter().any(|v| v.schema.is_some()))
                || !metadata(&row.presentation, &d.presentation, false, s.limits.metadata)
            {
                return None;
            }
            for (vi, v) in d.variants.iter().enumerate() {
                if !variant(s, &v.reference, ei, vi)
                    || !metadata(&row.variants[vi].schema, &v.schema, true, s.limits.metadata)
                {
                    return None;
                }
            }
        }
        (
            DiscoveryQueryV1::AdvisoryEnablement {
                entry: re,
                variant: rv,
                target: rt,
            },
            DiscoveryReplyV1::Advice {
                context,
                entry: e,
                variant: v,
                target: t,
                enabled: _,
            },
        ) => {
            if context != id || e != re || v != rv || t != rt {
                return None;
            }
            let Kind::Entry(ei) = s.original(&e.owner, &e.marker, e.slot).ok()? else {
                return None;
            };
            let Kind::Variant {
                entry: owner,
                index,
            } = s.original(&v.owner, &v.marker, v.slot).ok()?
            else {
                return None;
            };
            let (target_marker, _) = c.target.as_ref()?;
            if owner != ei
                || c.rows.get(ei)?.variants.get(index).is_none()
                || !Arc::ptr_eq(&t.owner, &id.owner)
                || t.slot != id.slot
                || !Arc::ptr_eq(&t.marker, target_marker)
            {
                return None;
            }
            // enabled is already a closed Boolean. Its policy truth remains the
            // independently attested current native advice path, not shape proof.
        }
        _ => return None,
    }
    Some(())
}
pub(super) fn validate(
    s: &Stage<'_>,
    id: &DiscoveryContextIdV1,
    query: DiscoveryQueryV1<'_>,
    reply: &DiscoveryReplyV1,
) -> Result<(), Error> {
    valid(s, id, query, reply).ok_or(Error::Faulted)
}
