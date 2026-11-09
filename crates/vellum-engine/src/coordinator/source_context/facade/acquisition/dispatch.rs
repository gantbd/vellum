//! Only fixed native producers; original opaque fact bindings are never repaired.
use super::*;
use ReadAcquisitionFacadePortV1 as P;
use ReadAcquisitionFacadeReplyV1 as R;
pub(super) fn run(
    owner: &mut SourceCoordinator,
    port: P<'_>,
    requested: &[(SourceReadScopeV1, AnnotationProjectionClassesV1)],
    rows: &facts::FactRows,
    r: &mut Recorder,
) -> Result<R, Error> {
    let (scope, _) = requested.first().ok_or(Error::InvalidRequest)?;
    let (current_scope, flags) = rows.first().ok_or(Error::Unavailable)?;
    match port {
        P::SourceCurrent { .. } | P::SourceRetained { .. } => {
            let facts = TrustedSourceReadFactsV1 {
                scope: current_scope,
                authorized: flags[0],
                scope_allowed: flags[1],
                continuous: flags[2],
                live: flags[3],
                not_stopped: flags[4],
                source_readable: flags[6],
                retained: flags[8],
                resources_ready: flags[9],
                reply_ready: flags[10],
                transfer_ready: flags[11],
            };
            match port {
                P::SourceCurrent { .. } => {
                    owner.capture_source_read_context_result(scope, &facts, r)
                }
                P::SourceRetained { original, .. } => {
                    owner.capture_retained_source_read_context_result(original, scope, &facts, r)
                }
                _ => unreachable!(),
            }
            .map(R::Source)
        }
        P::AnnotationCurrent {
            prepared,
            facts: original,
            ..
        } => {
            let facts = TrustedPreparedAnnotationFactsV1 {
                projection: original.projection,
                scope: current_scope,
                authorized: flags[0],
                scope_allowed: flags[1],
                continuous: flags[2],
                live: flags[3],
                not_stopped: flags[4],
                annotation_readable: flags[5],
                source_readable: flags[6],
                projection_current: flags[7],
                retained: flags[8],
                resources_ready: flags[9],
                reply_ready: flags[10],
                transfer_ready: flags[11],
            };
            owner
                .install_annotation_context_result(prepared, scope, &facts, r)
                .map(R::Annotation)
        }
        P::AnnotationRetained {
            original,
            facts: supplied,
            preparation_ready,
            ..
        } => {
            let facts = TrustedAnnotationContextFactsV1 {
                context: supplied.context,
                scope: current_scope,
                authorized: flags[0],
                scope_allowed: flags[1],
                continuous: flags[2],
                live: flags[3],
                not_stopped: flags[4],
                annotation_readable: flags[5],
                source_readable: flags[6],
                projection_current: flags[7],
                retained: flags[8],
                resources_ready: flags[9],
                reply_ready: flags[10],
                transfer_ready: flags[11],
            };
            owner
                .capture_retained_annotation_context_result(
                    original,
                    requested[0].1,
                    scope,
                    &facts,
                    preparation_ready,
                    r,
                )
                .map(R::Annotation)
        }
        P::VersionSetCurrent { .. } | P::VersionSetRetained { .. } => {
            let mut members = Vec::new();
            members
                .try_reserve_exact(rows.len())
                .map_err(|_| Error::Capacity)?;
            for (scope, flags) in rows {
                members.push(TrustedVersionSetMemberFactsV1 {
                    scope,
                    authorized: flags[0],
                    scope_allowed: flags[1],
                    continuous: flags[2],
                    live: flags[3],
                    not_stopped: flags[4],
                    annotation_readable: flags[5],
                    source_readable: flags[6],
                    projection_current: flags[7],
                    retained: flags[8],
                    resources_ready: flags[9],
                    reply_ready: flags[10],
                    transfer_ready: flags[11],
                });
            }
            let mut participants = Vec::new();
            participants
                .try_reserve_exact(requested.len())
                .map_err(|_| Error::Capacity)?;
            for (scope, data) in requested {
                participants.push(VersionSetParticipantV1 { scope, data: *data });
            }
            match port {
                P::VersionSetCurrent {
                    prepared,
                    facts: supplied,
                    ..
                } => {
                    let facts = TrustedPreparedVersionSetFactsV1 {
                        projection: supplied.projection,
                        members: &members,
                    };
                    owner
                        .install_version_set_result(prepared, &facts, r)
                        .map(R::VersionSet)
                }
                P::VersionSetRetained {
                    original,
                    facts: supplied,
                    preparation_ready,
                    ..
                } => {
                    let facts = TrustedVersionSetContextFactsV1 {
                        context: supplied.context,
                        members: &members,
                    };
                    owner
                        .capture_retained_version_set_result(
                            original,
                            &participants,
                            &facts,
                            preparation_ready,
                            r,
                        )
                        .map(R::VersionSet)
                }
                _ => unreachable!(),
            }
        }
    }
}
