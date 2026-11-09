//! Shape/request checks; fresh identity registration remains native authority.
use super::*;
pub(super) fn valid(
    reply: &ReadAcquisitionFacadeReplyV1,
    port: ReadAcquisitionFacadePortV1<'_>,
) -> bool {
    use ReadAcquisitionFacadePortV1 as P;
    use ReadAcquisitionFacadeReplyV1 as R;
    match (port, reply) {
        (P::SourceCurrent { .. }, R::Source(_)) => true,
        (P::SourceRetained { original, .. }, R::Source(id)) => id != original,
        (P::AnnotationCurrent { classes, .. }, R::Annotation(v)) => v.classes == classes,
        (
            P::AnnotationRetained {
                classes, original, ..
            },
            R::Annotation(v),
        ) => v.classes == classes && &v.context != original,
        (P::VersionSetCurrent { participants, .. }, R::VersionSet(v)) => roster(v, participants),
        (
            P::VersionSetRetained {
                participants,
                original,
                ..
            },
            R::VersionSet(v),
        ) => &v.context != original && roster(v, participants),
        _ => false,
    }
}
fn roster(v: &VersionSetAcquisitionV1, participants: &[VersionSetParticipantV1<'_>]) -> bool {
    !participants.is_empty()
        && v.participants.len() == participants.len()
        && v.participants
            .iter()
            .zip(participants)
            .enumerate()
            .all(|(i, (out, requested))| {
                out.document == requested.scope.document
                    && out.data == requested.data
                    && v.participants[..i]
                        .iter()
                        .all(|prior| prior.snapshot != out.snapshot)
            })
}
#[cfg(test)]
pub(super) fn corrupt(
    reply: &mut ReadAcquisitionFacadeReplyV1,
    port: ReadAcquisitionFacadePortV1<'_>,
) {
    use ReadAcquisitionFacadeReplyV1 as R;
    match reply {
        R::Source(id) => {
            if let ReadAcquisitionFacadePortV1::SourceRetained { original, .. } = port {
                *id = original.clone();
            } else {
                panic!("injected unrepresentable source acquisition result");
            }
        }
        R::Annotation(v) => {
            v.classes = if v.classes == AnnotationProjectionClassesV1::Source {
                AnnotationProjectionClassesV1::Annotations
            } else {
                AnnotationProjectionClassesV1::Source
            }
        }
        R::VersionSet(v) => {
            v.participants.pop();
        }
    }
}
