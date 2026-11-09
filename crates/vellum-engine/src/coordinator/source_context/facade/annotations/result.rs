//! Independent northbound shape/request checks; not native alias authentication.
use super::*;
use crate::attachment::{AttachmentTarget, Placement};
trait Record {
    type Id: Eq;
    fn id(&self) -> &Self::Id;
    fn location(&self) -> Option<AnnotationLocationV1>;
}
impl Record for AnnotationRecordV1 {
    type Id = AnnotationRecordId;
    fn id(&self) -> &Self::Id {
        &self.reference
    }
    fn location(&self) -> Option<AnnotationLocationV1> {
        self.location
    }
}
impl Record for VersionSetAnnotationRecordV1 {
    type Id = VersionSetAnnotationRecordId;
    fn id(&self) -> &Self::Id {
        &self.reference
    }
    fn location(&self) -> Option<AnnotationLocationV1> {
        self.location
    }
}

fn location(v: &AnnotationLocationV1) -> bool {
    match v.placement {
        Placement::Unavailable => false,
        Placement::Detached => v.source_revision.is_none(),
        Placement::Located { target, .. } => {
            v.source_revision.is_some()
                && match target {
                    AttachmentTarget::ContentRange { start, end } => start < end,
                    AttachmentTarget::Document | AttachmentTarget::Point { .. } => true,
                }
        }
    }
}
fn record<T: Record>(v: &T) -> bool {
    v.location().as_ref().is_none_or(location)
}
fn page<T: Record>(records: &[T], limit: usize, continuation: bool) -> bool {
    limit > 0
        && records.len() <= limit
        && (!continuation || records.len() == limit)
        && records.iter().enumerate().all(|(i, value)| {
            record(value) && records[..i].iter().all(|prior| prior.id() != value.id())
        })
}
pub(super) fn valid(reply: &AnnotationFacadeReplyV1, port: AnnotationFacadePortV1<'_>) -> bool {
    match (port, reply) {
        (
            AnnotationFacadePortV1::Single {
                context: expected,
                query,
                ..
            },
            AnnotationFacadeReplyV1::Single(reply),
        ) => match (query, reply) {
            (
                AnnotationQueryV1::Enumerate { limit, .. },
                AnnotationQueryReplyV1::Page {
                    context,
                    records,
                    continuation,
                },
            ) => context == expected && page(records, *limit, continuation.is_some()),
            (
                AnnotationQueryV1::Inspect { annotation },
                AnnotationQueryReplyV1::Record {
                    context,
                    record: value,
                },
            ) => context == expected && &value.reference == annotation && record(value),
            (
                AnnotationQueryV1::TargetLocation { annotation },
                AnnotationQueryReplyV1::Location {
                    context,
                    reference,
                    location: value,
                },
            ) => context == expected && reference == annotation && location(value),
            _ => false,
        },

        (
            AnnotationFacadePortV1::VersionSet {
                member: expected,
                query,
                ..
            },
            AnnotationFacadeReplyV1::VersionSet(reply),
        ) => match (query, reply) {
            (
                VersionSetAnnotationQueryV1::Enumerate { limit, .. },
                VersionSetAnnotationReplyV1::Page {
                    snapshot,
                    records,
                    continuation,
                },
            ) => snapshot == expected && page(records, *limit, continuation.is_some()),
            (
                VersionSetAnnotationQueryV1::Inspect { annotation },
                VersionSetAnnotationReplyV1::Record {
                    snapshot,
                    record: value,
                },
            ) => snapshot == expected && &value.reference == annotation && record(value),
            (
                VersionSetAnnotationQueryV1::TargetLocation { annotation },
                VersionSetAnnotationReplyV1::Location {
                    snapshot,
                    reference,
                    location: value,
                },
            ) => snapshot == expected && reference == annotation && location(value),
            _ => false,
        },
        _ => false,
    }
}
#[cfg(test)]
pub(super) fn corrupt(reply: &mut AnnotationFacadeReplyV1) {
    let invalid = AnnotationLocationV1 {
        placement: Placement::Unavailable,
        source_revision: None,
        target_generation: 0,
    };
    match reply {
        AnnotationFacadeReplyV1::Single(reply) => match reply {
            AnnotationQueryReplyV1::Page { records, .. } => {
                if let Some(record) = records.first_mut() {
                    record.location = Some(invalid);
                }
            }
            AnnotationQueryReplyV1::Record { record, .. } => record.location = Some(invalid),
            AnnotationQueryReplyV1::Location { location, .. } => *location = invalid,
        },

        AnnotationFacadeReplyV1::VersionSet(reply) => match reply {
            VersionSetAnnotationReplyV1::Page { records, .. } => {
                if let Some(record) = records.first_mut() {
                    record.location = Some(invalid);
                }
            }
            VersionSetAnnotationReplyV1::Record { record, .. } => record.location = Some(invalid),
            VersionSetAnnotationReplyV1::Location { location, .. } => *location = invalid,
        },
    }
}
