//! Bounded ordinary requested meaning, with original opaque identities separate.
use super::*;
pub(super) struct Requested {
    pub(super) rows: Vec<(SourceReadScopeV1, AnnotationProjectionClassesV1)>,
    _shadow: InertCaptureV1,
    _envelope: InertCaptureV1,
}
pub(super) fn copy(
    port: ReadAcquisitionFacadePortV1<'_>,
    limits: CaptureLimitsV1,
    r: &mut Recorder,
) -> Result<Requested, Error> {
    let count = port.count();
    let values_count = count.checked_mul(12).ok_or(Error::Limit)?;
    if count == 0
        || values_count
            .checked_add(1)
            .is_none_or(|n| n > limits.max_nodes)
        || values_count
            .checked_mul(size_of::<InertValueV1<'_>>())
            .and_then(|n| n.checked_add(size_of::<Vec<InertValueV1<'_>>>()))
            .is_none_or(|n| n > limits.max_bytes)
    {
        return Err(Error::Limit);
    }
    let mut bytes = size_of::<Requested>()
        .checked_add(
            count
                .checked_mul(
                    size_of::<AnnotationProjectionClassesV1>()
                        + size_of::<VersionSetParticipantV1<'_>>(),
                )
                .ok_or(Error::Limit)?,
        )
        .ok_or(Error::Limit)?;
    for i in 0..count {
        let (scope, _) = port.at(i);
        if (0..i).any(|j| port.at(j).0.document == scope.document) {
            return Err(Error::InvalidRequest);
        }
        bytes = bytes.checked_add(scope_bytes(scope)?).ok_or(Error::Limit)?;
    }
    if bytes > limits.max_bytes {
        return Err(Error::Limit);
    }
    let selection = match port {
        ReadAcquisitionFacadePortV1::SourceCurrent { .. }
        | ReadAcquisitionFacadePortV1::AnnotationCurrent { .. }
        | ReadAcquisitionFacadePortV1::VersionSetCurrent { .. } => "capture-current",
        _ => "retained",
    };
    let fields = [
        ("operation", InertValueV1::Text("snapshot-handoff")),
        ("selection", InertValueV1::Text(selection)),
    ];
    let envelope = frame(&InertValueV1::Object(&fields), limits, r)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(values_count)
        .map_err(|_| Error::Capacity)?;
    for i in 0..count {
        let (scope, classes) = port.at(i);
        values.extend(labels(&scope.context).into_iter().map(|(_, v)| v));
        values.push(InertValueV1::Text(&scope.policy));
        values.push(InertValueV1::Null);
        values.push(InertValueV1::Null);
        values.push(InertValueV1::Boolean(matches!(
            classes,
            AnnotationProjectionClassesV1::Source | AnnotationProjectionClassesV1::Combined
        )));
        values.push(InertValueV1::Boolean(matches!(
            classes,
            AnnotationProjectionClassesV1::Annotations | AnnotationProjectionClassesV1::Combined
        )));
    }
    let shadow = frame(&InertValueV1::Array(&values), limits, r)?;
    drop(values);
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(|_| Error::Capacity)?;
    for i in 0..count {
        let (scope, classes) = port.at(i);
        rows.push((copy_scope(scope)?, classes));
    }
    Ok(Requested {
        rows,
        _shadow: shadow,
        _envelope: envelope,
    })
}
