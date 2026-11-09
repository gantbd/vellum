//! Pure shared F03 shape and facet kernel. Owners retain authority and registration.
use super::*;
pub(super) mod result;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Selector {
    /// `context` is an owner-local reader key, never a grant or control selector.
    Record { context: usize, index: usize },
    Continuation {
        context: usize,
        limit: usize,
        next: usize,
    },
}
impl Selector {
    pub(super) fn context(self) -> usize {
        match self {
            Self::Record { context, .. } | Self::Continuation { context, .. } => context,
        }
    }
}
pub(super) enum Request {
    Enumerate {
        limit: usize,
        continuation: Option<usize>,
    },
    Inspect {
        slot: usize,
    },
    TargetLocation {
        slot: usize,
    },
}
#[derive(Clone, Copy)]
pub(super) enum Resolved {
    Page {
        start: usize,
        end: usize,
        next: Option<usize>,
        limit: usize,
    },
    Record {
        index: usize,
        slot: usize,
    },
    Location {
        index: usize,
        slot: usize,
    },
}
pub(super) fn resolve(
    rows: &[Row],
    selectors: &[Option<Selector>],
    context: usize,
    q: Request,
    max_page_rows: usize,
) -> Result<Resolved, Error> {
    match q {
        Request::Enumerate {
            limit,
            continuation,
        } => {
            if limit == 0 || limit > max_page_rows {
                return Err(Error::InvalidRequest);
            }
            let start = if let Some(slot) = continuation {
                match selectors.get(slot).and_then(Option::as_ref) {
                    Some(Selector::Continuation {
                        context: bound,
                        limit: bound_limit,
                        next,
                    }) if *bound == context && *bound_limit == limit => {
                        if *next == 0 || *next >= rows.len() {
                            return Err(Error::Faulted);
                        }
                        *next
                    }
                    _ => return Err(Error::Unavailable),
                }
            } else {
                0
            };
            let end = start
                .checked_add(limit)
                .ok_or(Error::Limit)?
                .min(rows.len());
            Ok(Resolved::Page {
                start,
                end,
                next: (end < rows.len()).then_some(end),
                limit,
            })
        }
        Request::Inspect { slot } | Request::TargetLocation { slot } => {
            let index = match selectors.get(slot).and_then(Option::as_ref) {
                Some(Selector::Record {
                    context: bound,
                    index,
                }) if *bound == context => *index,
                _ => return Err(Error::Unavailable),
            };
            if index >= rows.len() {
                return Err(Error::Faulted);
            }
            match q {
                Request::Inspect { .. } => Ok(Resolved::Record { index, slot }),
                Request::TargetLocation { .. } => Ok(Resolved::Location { index, slot }),
                _ => Err(Error::Faulted),
            }
        }
    }
}
pub(super) fn content_bytes(row: &Row) -> usize {
    row.body.as_ref().map_or(0, |v| v.0.len())
}
pub(super) fn location(row: &Row) -> Option<AnnotationLocationV1> {
    row.location
        .as_ref()
        .map(|(placement, revision, generation)| AnnotationLocationV1 {
            placement: *placement,
            source_revision: *revision,
            target_generation: *generation,
        })
}
pub(super) fn content(row: &Row) -> Result<Option<AnnotationContentV1>, Error> {
    match &row.body {
        Some((body, version)) => Ok(Some(AnnotationContentV1 {
            body: owned(body)?,
            version: *version,
        })),
        None => Ok(None),
    }
}
