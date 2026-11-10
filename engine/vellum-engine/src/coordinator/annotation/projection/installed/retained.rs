//! Bounded owned narrowing of an original available installed projection.
use super::*;
#[cfg(test)]
mod tests;

impl Projection {
    pub(in crate::coordinator::annotation::projection) fn narrowed_bytes(
        &self,
        classes: AnnotationProjectionClassesV1,
    ) -> Result<usize, Error> {
        let mut n = std::mem::size_of::<Self>();
        if classes.includes_source() {
            n = n
                .checked_add(self.source.as_ref().ok_or(Error::Faulted)?.0.text().len())
                .ok_or(Error::Limit)?;
        }
        if classes.includes_annotations() {
            for row in &self.rows {
                n = n
                    .checked_add(std::mem::size_of::<Row>())
                    .and_then(|n| n.checked_add(row.body.as_ref().map_or(0, |v| v.0.len())))
                    .ok_or(Error::Limit)?;
            }
        }
        Ok(n)
    }
    pub(in crate::coordinator::annotation::projection) fn copy_narrowed(
        &self,
        classes: AnnotationProjectionClassesV1,
    ) -> Result<Self, Error> {
        let source = if classes.includes_source() {
            let (s, v, p) = self.source.as_ref().ok_or(Error::Faulted)?;
            Some((s.copy_owned().map_err(|_| Error::Capacity)?, v.clone(), *p))
        } else {
            None
        };
        let mut rows = Vec::new();
        if classes.includes_annotations() {
            rows.try_reserve_exact(self.rows.len())
                .map_err(|_| Error::Capacity)?;
            for row in &self.rows {
                rows.push(Row {
                    id: row.id.clone(),
                    body: match &row.body {
                        Some((body, version)) => Some((owned(body)?, *version)),
                        None => None,
                    },
                    location: row.location,
                });
            }
        }
        Ok(Self {
            classes,
            source,
            rows,
        })
    }
}
impl SourceCoordinator {
    /// Copy only the original admitted cut; current root content is never a fallback.
    /// `preparation_ready` is an independently attributed exact-true staging gate.
    #[allow(clippy::too_many_arguments)]
    pub fn capture_retained_annotation_context(
        &mut self,
        original: &AnnotationReadContextId,
        classes: AnnotationProjectionClassesV1,
        s: &SourceReadScopeV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        preparation_ready: Option<bool>,
        r: &mut Recorder,
    ) -> Option<AnnotationContextAcquisitionV1> {
        self.capture_retained_annotation_context_result(
            original,
            classes,
            s,
            f,
            preparation_ready,
            r,
        )
        .ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn capture_retained_annotation_context_result(
        &mut self,
        original: &AnnotationReadContextId,
        classes: AnnotationProjectionClassesV1,
        s: &SourceReadScopeV1,
        f: &TrustedAnnotationContextFactsV1<'_>,
        preparation_ready: Option<bool>,
        r: &mut Recorder,
    ) -> Result<AnnotationContextAcquisitionV1, Error> {
        let result = (|| {
            let p = self.annotation_context_pool()?;
            if !valid_scope(s, p) || f.scope != s || f.context != original {
                return Err(Error::Unavailable);
            }
            let row = self.annotation_context_row(original, s)?;
            if !row.value.valid(s, &self.namespace) {
                return Err(Error::Faulted);
            }
            if !classes.subset_of(row.value.classes) {
                return Err(Error::InvalidRequest);
            }
            let scope = scope_bytes(s)?;
            let request = scope
                .checked_add(std::mem::size_of::<AnnotationReadContextId>())
                .and_then(|n| n.checked_add(std::mem::size_of::<AnnotationProjectionClassesV1>()))
                .and_then(|n| n.checked_add(std::mem::size_of::<Option<bool>>()))
                .ok_or(Error::Limit)?;
            if request > p.limits.max_request_bytes {
                return Err(Error::Limit);
            }
            // Original whole-domain loss precedes new admission/capacity pressure.
            self.check_annotation_context(original, s, f, r)?;
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            if preparation_ready != Some(true) {
                return Err(Error::Unavailable);
            }
            let p = self.annotation_context_pool()?;
            let row = self.annotation_context_row(original, s)?;
            let rows = if classes.includes_annotations() {
                row.value.rows.len()
            } else {
                0
            };
            if p.issued()? >= p.limits.max_issued
                || p.selector_count() >= p.limits.max_selectors
                || p.rows.iter().flatten().count() >= p.limits.max_contexts
                || rows > p.limits.max_rows
                || row.value.narrowed_bytes(classes)? > p.limits.max_capture_bytes
                || scope > p.limits.max_context_bytes
                || std::mem::size_of::<AnnotationContextAcquisitionV1>() > p.limits.max_reply_bytes
                || std::mem::size_of::<AnnotationReadContextId>() > p.limits.max_selector_bytes
                || settings::basis::labels(&s.context)
                    .into_iter()
                    .chain([s.policy.as_str()])
                    .any(|v| v.len() > p.limits.max_label_bytes)
            {
                return Err(Error::Limit);
            }
            if p.rows.capacity() < p.limits.max_issued {
                return Err(Error::Faulted);
            }
            let value = row.value.copy_narrowed(classes)?;
            let scope = SourceReadScopeV1 {
                document: s.document.clone(),
                context: settings::basis::copy_context(&s.context).map_err(|_| Error::Capacity)?,
                policy: owned(&s.policy)?,
            };
            let expected_slot = p.rows.len();
            let expected_classes = classes;
            #[allow(unused_mut)]
            let mut reply = AnnotationContextAcquisitionV1 {
                context: AnnotationReadContextId {
                    owner: self.namespace.clone(),
                    slot: p.rows.len(),
                },
                classes,
            };
            #[cfg(test)]
            {
                use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
                match self.acquisition_result_fault {
                    Some(F::Context) => reply.context.slot = usize::MAX,
                    Some(F::Classes) => {
                        reply.classes = if expected_classes == AnnotationProjectionClassesV1::Source
                        {
                            AnnotationProjectionClassesV1::Annotations
                        } else {
                            AnnotationProjectionClassesV1::Source
                        }
                    }
                    Some(F::RetainedParent) => {
                        reply.context = original.clone();
                    }
                    _ => (),
                }
            }
            let accepted = Arc::ptr_eq(&reply.context.owner, &self.namespace)
                && reply.context.slot == expected_slot
                && reply.classes == expected_classes
                && reply.classes == value.classes
                && reply.context != *original;
            if !self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeReadAcquisitionResultInvariant,
                Boundary::ReadAcquisitionResultValidation,
                r,
            ) {
                return Err(Error::Faulted);
            }
            // All fallible stages finish before this local nonallocating installation.
            self.faulted = true;
            #[cfg(test)]
            assert!(
                !self.annotation_context_install_fault,
                "injected retained context staging fault"
            );
            let InstalledState::Active(p) = &mut self.annotation_contexts else {
                return Err(Error::Faulted);
            };
            p.rows.push(Some(Context {
                scope,
                value,
                stopped: false,
                holds: 0,
            }));
            self.faulted = false;
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::AnnotationContextRetainedCapture, &result, r);
        result
    }
}
