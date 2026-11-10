//! Borrowed native F03 result checks, before selector publication. No payload copy.
use super::*;
pub(in crate::coordinator) trait Record {
    type Id: Eq;
    fn id(&self) -> &Self::Id;
    fn content(&self) -> Option<&AnnotationContentV1>;
    fn location(&self) -> Option<AnnotationLocationV1>;
}
pub(in crate::coordinator) enum View<'a, T: Record> {
    Page(&'a [T], Option<Option<Selector>>),
    Record(&'a T),
    Location(&'a T::Id, &'a AnnotationLocationV1),
}
fn facets<T: Record>(record: &T, row: &Row) -> bool {
    // Native shape/reference check, not a second byte-content oracle.
    let content = match (record.content(), &row.body) {
        (None, None) => true,
        (Some(v), Some((body, version))) => v.version == *version && v.body.len() == body.len(),
        _ => false,
    };
    content && record.location() == location(row)
}
pub(in crate::coordinator) fn valid<T: Record>(
    resolved: Resolved,
    rows: &[Row],
    context: usize,
    bound: bool,
    view: View<'_, T>,
    binding: impl Fn(&T::Id) -> Option<(usize, Selector)>,
) -> bool {
    if !bound {
        return false;
    }
    match (resolved, view) {
        (
            Resolved::Page {
                start,
                end,
                next,
                limit,
            },
            View::Page(records, continuation),
        ) => {
            let Some(expected) = rows.get(start..end) else {
                return false;
            };
            if limit == 0
                || records.len() != expected.len()
                || records.len() > limit
                || next != (end < rows.len()).then_some(end)
                || (next.is_some() && records.len() != limit)
                || continuation
                    != next.map(|next| {
                        Some(Selector::Continuation {
                            context,
                            limit,
                            next,
                        })
                    })
            {
                return false;
            }
            records.iter().enumerate().all(|(offset, record)| {
                binding(record.id()).map(|v| v.1)
                    == Some(Selector::Record {
                        context,
                        index: start + offset,
                    })
                    && facets(record, &expected[offset])
                    && records[..offset]
                        .iter()
                        .all(|prior| prior.id() != record.id())
            })
        }
        (Resolved::Record { index, slot }, View::Record(record)) => {
            rows.get(index).is_some_and(|row| facets(record, row))
                && binding(record.id()) == Some((slot, Selector::Record { context, index }))
        }
        (Resolved::Location { index, slot }, View::Location(id, value)) => {
            rows.get(index)
                .is_some_and(|row| location(row).as_ref() == Some(value))
                && binding(id) == Some((slot, Selector::Record { context, index }))
        }
        _ => false,
    }
}
