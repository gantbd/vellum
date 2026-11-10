//! Ordinary annotation parents share the source operation namespace and store.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy)]
pub struct ExpectedSaveBindingV1<'a> {
    pub target: &'a SourceBindingTargetV1,
    pub generation: u64,
}
pub struct OwnedAnnotationSaveRequestV1<'a> {
    pub origin: SaveOperationOriginV1<'a>,
    pub source: &'a DocumentVersion,
    pub expected_set: u64,
    pub expected_annotation_binding: u64,
    pub expected_source_binding: Option<ExpectedSaveBindingV1<'a>>,
    pub encoding: &'a AnnotationEncodingContext,
    pub destination: &'a ResourceRead,
    pub requirements: SaveRequirements,
    pub condition: SaveOperationConditionV1,
}
/// Current permission to disclose original annotation task metadata. This packet
/// intentionally has no source-read permission; it grants no payload possession.
pub struct TrustedAnnotationSaveOperationReleaseV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub write_allowed: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub disposition_readable: Option<bool>,
    pub retained: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub resources_ready: Option<bool>,
}
/// Trusted standard-producer/context, native admission and baseline-claim premises.
/// Actual root/pair/encoder/provider/codec/claim bounds are also checked independently.
pub struct TrustedOrdinaryAnnotationSaveAdmissionV1 {
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub annotation_readable: Option<bool>,
    pub complete_disclosure: Option<bool>,
    pub write_allowed: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub writer_coverage_complete: Option<bool>,
    pub provider_available: Option<bool>,
    pub resources_ready: Option<bool>,
    pub launch_allowed: Option<bool>,
    pub destination_allowed: Option<bool>,
    pub destination_context_attested: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub companions_ready: Option<bool>,
    pub versions_available: Option<bool>,
    pub reply_ready: Option<bool>,
    pub aliases_current: Option<bool>,
    pub source_evidence_ready: Option<bool>,
    pub retained: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub ordinary_bindings_current: Option<bool>,
    pub ordinary_identity_current: Option<bool>,
    pub historical_artifact_readable: Option<bool>,
    pub overwrite_destination_allowed: Option<bool>,
}
fn release_current(f: &TrustedAnnotationSaveOperationReleaseV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.write_allowed,
        f.annotation_readable,
        f.disposition_readable,
        f.retained,
        f.reply_ready,
        f.transfer_ready,
        f.resources_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn admission_current(f: &TrustedOrdinaryAnnotationSaveAdmissionV1) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.annotation_readable,
        f.complete_disclosure,
        f.write_allowed,
        f.binding_coverage_complete,
        f.writer_coverage_complete,
        f.provider_available,
        f.resources_ready,
        f.launch_allowed,
        f.destination_allowed,
        f.destination_context_attested,
        f.identity_allowed,
        f.companions_ready,
        f.versions_available,
        f.reply_ready,
        f.aliases_current,
        f.source_evidence_ready,
        f.retained,
        f.transfer_ready,
        f.preparation_reserved,
        f.owner_control_allowed,
        f.ordinary_bindings_current,
        f.ordinary_identity_current,
        f.historical_artifact_readable,
        f.overwrite_destination_allowed,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
fn source_binding(r: &OwnedAnnotationSaveRequestV1<'_>) -> Option<BindingKey> {
    r.expected_source_binding.map(|b| BindingKey {
        identity: b.target.identity.clone(),
        registration: b.target.registration.clone(),
        generation: b.generation,
    })
}
fn matches(
    c: &Capture,
    r: &OwnedAnnotationSaveRequestV1<'_>,
    context: &SettingsExecutionContextV1,
) -> bool {
    c.effect.0.as_ref() == r.origin.effect
        && c.attempt.0.as_ref() == r.origin.attempt
        && c.source == *r.source
        && c.binding == r.expected_annotation_binding
        && c.identity == r.destination.identity
        && Arc::ptr_eq(&c.registration, &r.destination.registration)
        && c.requirements == r.requirements
        && c.context == *context
        && matches!(&c.flavor, Flavor::Annotations { set, encoding, association, before_bytes, before_validator }
            if *set == r.expected_set && encoding == r.encoding && association.source_binding == source_binding(r)
                && *before_bytes == r.destination.bytes && *before_validator == r.destination.validator)
}
impl SourceCoordinator {
    /// A candidate is required only for first admission. Matching lookup does
    /// not consult or install a replacement candidate or technical encoder profile.
    pub fn request_owned_annotation_save_v1(
        &mut self,
        request: &OwnedAnnotationSaveRequestV1<'_>,
        release: &TrustedAnnotationSaveOperationReleaseV1<'_>,
        admission: Option<&TrustedOrdinaryAnnotationSaveAdmissionV1>,
        candidate: Option<EncodedAnnotations>,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1, ResourceError> {
        self.request_owned_annotation_save_catalog(
            (request, None),
            release,
            admission,
            candidate,
            pair,
            r,
        )
    }
    pub(in crate::coordinator) fn request_owned_annotation_save_catalog(
        &mut self,
        input: (
            &OwnedAnnotationSaveRequestV1<'_>,
            Option<&CatalogWriterBindingV1>,
        ),
        release: &TrustedAnnotationSaveOperationReleaseV1<'_>,
        admission: Option<&TrustedOrdinaryAnnotationSaveAdmissionV1>,
        candidate: Option<EncodedAnnotations>,
        pair: Option<OrdinaryPairEvidence>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1, ResourceError> {
        let (request, writer) = input;
        let catalog = writer.map(CatalogWriterBindingV1::retained_capture);
        let timing = r.begin_service_timing();
        let existing = self
            .resources
            .operations
            .rows
            .iter()
            .any(|row| row.operation.0.as_ref() == request.origin.operation);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let limits = self
                .resources
                .operations
                .limits
                .ok_or(ResourceError::Unsupported)?;
            // Pure closed-profile/schema/complete-input bounds precede facts,
            // root reads, candidate consumption or native issuance.
            if request.condition != SaveOperationConditionV1::Pinned
                || request.destination.epoch.is_some()
            {
                return Err(ResourceError::Unsupported);
            }
            for label in [
                request.origin.operation,
                request.origin.effect,
                request.origin.attempt,
            ] {
                if std::str::from_utf8(label).is_err() {
                    return Err(ResourceError::Malformed);
                }
                if label.is_empty() || label.len() > limits.max_label_bytes {
                    return Err(ResourceError::Limit);
                }
            }
            let extra = request
                .destination
                .bytes
                .0
                .len()
                .checked_add(request.destination.validator.0.len())
                .and_then(|n| n.checked_add(request.destination.identity.key.0.len()))
                .and_then(|n| {
                    n.checked_add(
                        request
                            .expected_source_binding
                            .map_or(0, |b| b.target.identity.key.0.len()),
                    )
                })
                .ok_or(ResourceError::Limit)?;
            if settings::basis::labels(release.context)
                .iter()
                .any(|v| v.is_empty() || v.len() > limits.max_label_bytes)
                || capsule_bytes(
                    request.origin.operation.len(),
                    request.origin.effect.len(),
                    request.origin.attempt.len(),
                    request.destination.identity.key.0.len(),
                    release.context,
                )
                .and_then(|n| n.checked_add(extra))
                .and_then(|n| {
                    n.checked_add(
                        catalog
                            .as_ref()
                            .map(|c| c.native_bytes())
                            .transpose()
                            .ok()?
                            .unwrap_or(0),
                    )
                })
                .is_none_or(|n| n > limits.max_capture_bytes)
                || std::mem::size_of::<SaveOperationAdmissionV1>() > limits.max_reply_bytes
            {
                return Err(ResourceError::Limit);
            }
            if !Arc::ptr_eq(&request.source.document.namespace, &self.namespace)
                || !Arc::ptr_eq(&request.destination.registration.owner, &self.namespace)
                || !Arc::ptr_eq(&request.encoding.owner, &self.namespace)
                || request
                    .expected_source_binding
                    .is_some_and(|b| !Arc::ptr_eq(&b.target.registration.owner, &self.namespace))
                || request.expected_annotation_binding == 0
            {
                return Err(ResourceError::Unavailable);
            }
            if !release_current(release) {
                return Err(ResourceError::Unavailable);
            }
            self.validate_save_operations()?;
            if let Some(row) = self
                .resources
                .operations
                .rows
                .iter()
                .find(|row| row.operation.0.as_ref() == request.origin.operation)
            {
                let c = row.capture.as_ref().ok_or(ResourceError::Unavailable)?;
                if !matches(c, request, release.context)
                    || !WriteCatalogCaptureV1::same_intent(c.catalog.as_ref(), catalog.as_ref())
                {
                    return Err(ResourceError::Unavailable);
                }
                return Ok(SaveOperationAdmissionV1 {
                    status: SaveOperationAdmissionStatusV1::ObserveExisting,
                    task: c.task.clone(),
                });
            }
            self.ensure_operational().map_err(owner_error)?;
            if !admission.is_some_and(admission_current) {
                return Err(ResourceError::Unavailable);
            }
            if self.resources.operations.rows.len() >= limits.max_issued {
                return Err(ResourceError::Limit);
            }
            if self.resources.operations.rows.len() >= self.resources.operations.rows.capacity() {
                return Err(ResourceError::Faulted);
            }
            let association = AnnotationAssociation {
                binding: BindingKey {
                    identity: request.destination.identity.clone(),
                    registration: request.destination.registration.clone(),
                    generation: request.expected_annotation_binding,
                },
                source_binding: source_binding(request),
            };
            if self
                .current_annotation_association(&request.source.document)
                .as_ref()
                != Some(&association)
            {
                return Err(ResourceError::Stale);
            }
            let encoded = candidate.ok_or(ResourceError::MissingEvidence)?;
            if encoded.snapshot.source() != request.source
                || encoded.snapshot.set_version() != request.expected_set
                || encoded.registration.context != *request.encoding
            {
                return Err(ResourceError::Stale);
            }
            // Fallible private input copies precede the child. The existing
            // native admission atomically installs artifact/task/baseline claim;
            // no callback, suspension or fallible allocation follows its success.
            let operation = blob(request.origin.operation, limits.max_label_bytes, true)?;
            let effect = blob(request.origin.effect, limits.max_label_bytes, true)?;
            let attempt = blob(request.origin.attempt, limits.max_label_bytes, true)?;
            let context = settings::basis::copy_context(release.context)
                .map_err(|_| ResourceError::Capacity)?;
            let flavor = Flavor::Annotations {
                set: request.expected_set,
                encoding: request.encoding.clone(),
                association,
                before_bytes: request.destination.bytes.clone(),
                before_validator: request.destination.validator.clone(),
            };
            let destination = ResourceRead {
                epoch: None,
                identity: request.destination.identity.clone(),
                registration: request.destination.registration.clone(),
                bytes: request.destination.bytes.clone(),
                validator: request.destination.validator.clone(),
            };
            let task = SaveId {
                owner: self.namespace.clone(),
                slot: self.resources.tasks.len(),
            };
            let parent = Row {
                operation,
                capture: Some(Capture {
                    catalog: catalog.clone(),
                    effect,
                    attempt,
                    source: request.source.clone(),
                    flavor,
                    binding: request.expected_annotation_binding,
                    identity: request.destination.identity.clone(),
                    registration: request.destination.registration.clone(),
                    requirements: request.requirements,
                    context,
                    task: task.clone(),
                }),
            };
            let task = self.queue_annotation_save_with_parent(
                encoded,
                destination,
                request.requirements,
                pair,
                (catalog, Some(parent)),
                r,
            )?;
            Ok(SaveOperationAdmissionV1 {
                status: SaveOperationAdmissionStatusV1::Admitted,
                task,
            })
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                capture_service(
                    r,
                    if existing {
                        Boundary::SaveOperationRecovery
                    } else {
                        Boundary::SaveOperationAdmission
                    },
                    &Err::<SaveOperationAdmissionV1, _>(ResourceError::Faulted),
                    timing.finish(),
                );
                std::panic::resume_unwind(p)
            }
        };
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            if existing {
                Boundary::SaveOperationRecovery
            } else {
                Boundary::SaveOperationAdmission
            },
            &result,
            timing.finish(),
        );
        result
    }
}
