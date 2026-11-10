//! Ordinary private-plan integrity; current versions and policy remain staging's job.
use super::*;

impl AnnotationPlan {
    pub(in crate::coordinator) fn matches_mutation(&self, mutation: &AnnotationMutation) -> bool {
        let plan = self;
        match (&plan.wish, mutation) {
            (Wish::Create { body, target }, AnnotationMutation::Create { body: b, target: t }) => {
                body.as_ref() == b && target == t && plan.id.document == t.source.document
            }
            (
                Wish::Body { expected, body },
                AnnotationMutation::UpdateBody {
                    annotation,
                    expected_content,
                    body: b,
                },
            ) => plan.id == *annotation && expected == expected_content && body.as_ref() == b,
            (
                Wish::Target { expected, target },
                AnnotationMutation::Reattach {
                    annotation,
                    expected_target,
                    target: t,
                },
            ) => plan.id == *annotation && expected == expected_target && target == t,
            (
                Wish::Delete { content, target },
                AnnotationMutation::Delete {
                    annotation,
                    expected_content,
                    expected_target,
                },
            ) => plan.id == *annotation && content == expected_content && target == expected_target,
            _ => false,
        }
    }
}
impl PreparedTransaction {
    pub(in crate::coordinator) fn standalone_annotation_matches_mutation(
        &self,
        mutation: &AnnotationMutation,
    ) -> bool {
        self.standalone_annotation_seal()
            .is_some_and(|plan| plan.matches_mutation(mutation))
    }
    /// This closed standalone profile has no source/view/selection transform.
    pub(in crate::coordinator) fn standalone_annotation_seal(&self) -> Option<AnnotationPlan> {
        let Self {
            configured_required,
            configured_line,
            reads_required: _,
            declared_reads: _,
            scoped,
            scopes,
            derived_edit,
            derived_kind,
            view_dependencies,
            namespace: _,
            candidates,
            read_dependencies,
            history,
            search_dependency,
            annotations,
        } = self;
        if *configured_required
            || configured_line.is_some()
            || *scoped
            || scopes.is_some()
            || !derived_edit.is_empty()
            || derived_kind.is_some()
            || !view_dependencies.is_empty()
            || !candidates.is_empty()
            || !read_dependencies.is_empty()
            || !matches!(history, history::PreparedHistory::Ordinary)
            || search_dependency.is_some()
            || annotations.len() != 1
        {
            return None;
        }
        Some(annotations[0].clone())
    }
    pub(in crate::coordinator) fn standalone_annotation_bytes(&self) -> Option<usize> {
        let seal = self.standalone_annotation_seal()?;
        let body = match &seal.wish {
            Wish::Create { body, .. } | Wish::Body { body, .. } => body.len(),
            Wish::Target { .. } | Wish::Delete { .. } => 0,
            _ => return None,
        };
        std::mem::size_of::<Self>()
            .checked_add(std::mem::size_of::<AnnotationPlan>())?
            .checked_add(body)?
            .checked_add(
                self.declared_reads
                    .as_ref()
                    .map_or(Some(0), |r| r.logical_bytes())?,
            )
    }
}

impl SourceCoordinator {
    pub(in crate::coordinator) fn validate_ordinary_annotation_plan(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        self.validate_ordinary_annotation_plan_mode(prepared, false, r)
    }
    pub(in crate::coordinator) fn validate_advisory_annotation_plan(
        &self,
        prepared: &PreparedTransaction,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        self.validate_ordinary_annotation_plan_mode(prepared, true, r)
    }
    fn validate_ordinary_annotation_plan_mode(
        &self,
        prepared: &PreparedTransaction,
        advisory: bool,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        // Reversal has a separate original-group/complete-inverse validator.
        if prepared.annotations.is_empty()
            || !matches!(prepared.history, history::PreparedHistory::Ordinary)
        {
            return Ok(());
        }
        let timing = r.begin_service_timing();
        let result = (|| {
            let limits = self.annotations.limits.ok_or(CoordinatorError::Faulted)?;
            if prepared.annotations.len() > self.limits.max_participants
                || self.annotations.records.len() != self.annotations.issued_documents.len()
                || self.annotations.records.len() != self.annotations.retired.len()
                || self.annotations.records.len() > limits.max_issued
            {
                return Err(CoordinatorError::Faulted);
            }
            let mut fields = Vec::new();
            let multiple = prepared.annotations.len() > 1;
            if multiple {
                fields
                    .try_reserve_exact(prepared.annotations.len())
                    .map_err(|_| CoordinatorError::Capacity)?;
            }
            let mut private_claims = 0usize;
            for plan in &prepared.annotations {
                let private_create = advisory && matches!(plan.wish, Wish::Create { .. });
                let valid_slot = if private_create {
                    let expected = self
                        .annotations
                        .records
                        .len()
                        .checked_add(private_claims)
                        .ok_or(CoordinatorError::Faulted)?;
                    private_claims += 1;
                    plan.id.slot == expected && expected < limits.max_issued
                } else {
                    self.annotations.issued_documents.get(plan.id.slot) == Some(&plan.id.document)
                        && self
                            .annotations
                            .records
                            .get(plan.id.slot)
                            .is_some_and(|record| {
                                record
                                    .as_ref()
                                    .is_none_or(|record| record.value.id == plan.id)
                            })
                };
                if !Arc::ptr_eq(&plan.id.namespace, &self.namespace)
                    || !Arc::ptr_eq(&plan.id.document.namespace, &self.namespace)
                    || !valid_slot
                {
                    return Err(CoordinatorError::Faulted);
                }
                let field = match &plan.wish {
                    Wish::Create { body, target } => {
                        if body.len() > limits.max_body_bytes
                            || target.source.document != plan.id.document
                        {
                            return Err(CoordinatorError::Faulted);
                        }
                        4u8
                    }
                    Wish::Body { expected, body } => {
                        if *expected == 0 || body.len() > limits.max_body_bytes {
                            return Err(CoordinatorError::Faulted);
                        }
                        1
                    }
                    Wish::Target { expected, target } => {
                        if *expected == 0 || target.source.document != plan.id.document {
                            return Err(CoordinatorError::Faulted);
                        }
                        2
                    }
                    Wish::Delete { content, target } => {
                        if *content == 0 || *target == 0 {
                            return Err(CoordinatorError::Faulted);
                        }
                        8
                    }
                    Wish::RestoreBody { .. }
                    | Wish::RestoreTarget { .. }
                    | Wish::RestoreMembership { .. } => return Err(CoordinatorError::Faulted),
                };
                if multiple {
                    fields.push((plan.id.slot, field));
                }
            }
            // Bounded scalar headers only, no body/target copies or lifetime-row
            // scan. Sorting avoids adding another quadratic participant walk.
            fields.sort_unstable_by_key(|(slot, _)| *slot);
            let mut previous = None;
            let mut mask = 0;
            for (slot, field) in fields {
                if previous == Some(slot) {
                    if mask & field != 0 || mask | field != 3 {
                        return Err(CoordinatorError::Faulted);
                    }
                    mask |= field;
                } else {
                    previous = Some(slot);
                    mask = field;
                }
            }
            Ok(())
        })();
        observe_service(
            r,
            Boundary::AnnotationPlanValidation,
            &result,
            timing.finish(),
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standalone_receipt_correlation_matches_all_original_mutation_fields() {
        let mut r = Recorder::disabled();
        let (mut o, source, id) = super::super::tests::setup(&mut r);
        let target = SourceTarget {
            source: source.clone(),
            target: AttachmentTarget::ContentRange { start: 1, end: 2 },
            basis: TargetBasis::Before,
        };
        let mutations = [
            AnnotationMutation::Create {
                target: target.clone(),
                body: "PRIVATE_NEW".into(),
            },
            AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "PRIVATE_NEW".into(),
            },
            AnnotationMutation::Reattach {
                annotation: id.clone(),
                expected_target: 1,
                target,
            },
            AnnotationMutation::Delete {
                annotation: id,
                expected_content: 1,
                expected_target: 1,
            },
        ];
        for mutation in &mutations {
            let p = o
                .prepare_compound(&[], &[], std::slice::from_ref(mutation), &mut r)
                .unwrap();
            assert!(p.standalone_annotation_matches_mutation(mutation));
            let mut changed = mutation.clone();
            match &mut changed {
                AnnotationMutation::Create { body, .. } => body.push('!'),
                AnnotationMutation::UpdateBody {
                    expected_content, ..
                }
                | AnnotationMutation::Delete {
                    expected_content, ..
                } => *expected_content += 1,
                AnnotationMutation::Reattach {
                    expected_target, ..
                } => *expected_target += 1,
            }
            assert!(!p.standalone_annotation_matches_mutation(&changed));
            for other in &mutations {
                if std::mem::discriminant(mutation) != std::mem::discriminant(other) {
                    assert!(!p.standalone_annotation_matches_mutation(other));
                }
            }
        }
    }
    fn create(source: &DocumentVersion) -> AnnotationMutation {
        AnnotationMutation::Create {
            body: "PRIVATE_NEW".into(),
            target: SourceTarget {
                source: source.clone(),
                target: AttachmentTarget::ContentRange { start: 1, end: 2 },
                basis: TargetBasis::Before,
            },
        }
    }
    #[test]
    fn private_create_cannot_install_a_foreign_or_unissued_annotation_identity() {
        for corruption in 0..3 {
            let mut r = Recorder::disabled();
            let (mut o, source, original) = super::super::tests::setup(&mut r);
            let mut p = o
                .prepare_compound(&[], &[], &[create(&source)], &mut r)
                .unwrap();
            match corruption {
                0 => p.annotations[0].id.namespace = Arc::new(()),
                1 => p.annotations[0].id.slot = o.annotations.records.len(),
                _ => {
                    let other = o.create(b"abc", &mut r).unwrap();
                    p.annotations[0].id.document = other.document.clone();
                    let Wish::Create { target, .. } = &mut p.annotations[0].wish else {
                        unreachable!()
                    };
                    target.source = other;
                }
            }
            let generation = o.generation();
            let usage = o.annotation_usage();
            assert_eq!(o.commit(p, &mut r).err(), Some(CoordinatorError::Faulted));
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation);
            assert_eq!(
                o.annotation_usage().retained_records,
                usage.retained_records
            );
            assert_eq!(
                &*o.annotations.records[original.slot]
                    .as_ref()
                    .unwrap()
                    .value
                    .body,
                "PRIVATE_OLD"
            );
        }
    }
    #[test]
    fn private_ordinary_plan_cannot_repeat_a_field_even_when_both_edits_are_noops() {
        let mut r = Recorder::disabled();
        let (mut o, _, id) = super::super::tests::setup(&mut r);
        let mut p = o
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: id,
                    expected_content: 1,
                    body: "PRIVATE_OLD".into(),
                }],
                &mut r,
            )
            .unwrap();
        p.annotations.push(p.annotations[0].clone());
        let generation = o.generation();
        assert_eq!(o.commit(p, &mut r).err(), Some(CoordinatorError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
    }

    #[test]
    fn body_and_target_components_remain_order_independent_and_keep_separate_versions() {
        for reverse in [false, true] {
            let mut r = Recorder::disabled();
            let (mut o, source, id) = super::super::tests::setup(&mut r);
            let mut mutations = [
                AnnotationMutation::UpdateBody {
                    annotation: id.clone(),
                    expected_content: 1,
                    body: "PRIVATE_NEW".into(),
                },
                AnnotationMutation::Reattach {
                    annotation: id.clone(),
                    expected_target: 1,
                    target: SourceTarget {
                        source,
                        target: AttachmentTarget::ContentRange { start: 1, end: 2 },
                        basis: TargetBasis::Before,
                    },
                },
            ];
            if reverse {
                mutations.reverse();
            }
            let p = o.prepare_compound(&[], &[], &mutations, &mut r).unwrap();
            assert!(o.commit(p, &mut r).unwrap().operation.is_some());
            let value = o.annotation(&id, &mut r).unwrap();
            assert_eq!(value.body, "PRIVATE_NEW");
            assert_eq!(value.content_version, 2);
            assert_eq!(value.target_generation, 2);
        }
    }

    #[test]
    fn conflicting_private_field_sets_and_restore_vocabulary_are_never_ordinary_edits() {
        for case in 0..7 {
            let mut r = Recorder::disabled();
            let (mut o, source, id) = super::super::tests::setup(&mut r);
            let mut p = o
                .prepare_compound(
                    &[],
                    &[],
                    &[AnnotationMutation::UpdateBody {
                        annotation: id.clone(),
                        expected_content: 1,
                        body: "PRIVATE_OLD".into(),
                    }],
                    &mut r,
                )
                .unwrap();
            let plan = p.annotations[0].clone();
            let target = Wish::Target {
                expected: 1,
                target: SourceTarget {
                    source: source.clone(),
                    target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                    basis: TargetBasis::Before,
                },
            };
            match case {
                0 => {
                    p.annotations[0].wish = target.clone();
                    p.annotations.push(AnnotationPlan {
                        wish: target,
                        ..plan
                    });
                }
                1 => p.annotations.push(AnnotationPlan {
                    wish: Wish::Delete {
                        content: 1,
                        target: 1,
                    },
                    ..plan
                }),
                2 => {
                    p.annotations.push(AnnotationPlan {
                        wish: target,
                        ..plan.clone()
                    });
                    p.annotations.push(plan);
                }
                3 => {
                    p.annotations[0].wish = Wish::RestoreBody {
                        expected: 1,
                        body: Arc::from("PRIVATE_NEW"),
                    }
                }
                4 => {
                    p.annotations[0].wish = Wish::RestoreTarget {
                        expected: 1,
                        state: o.annotations.records[id.slot]
                            .as_ref()
                            .unwrap()
                            .value
                            .target
                            .clone(),
                        source,
                        linked: false,
                    }
                }
                5 => {
                    p.annotations[0].wish = Wish::RestoreMembership {
                        content: 1,
                        target: 1,
                        present: true,
                        desired: None,
                        source,
                        linked: false,
                    }
                }
                _ => {
                    p.annotations[0].wish = Wish::Body {
                        expected: 0,
                        body: Arc::from("PRIVATE_NEW"),
                    }
                }
            }
            let generation = o.generation();
            assert_eq!(o.commit(p, &mut r).err(), Some(CoordinatorError::Faulted));
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation);
            assert_eq!(
                &*o.annotations.records[id.slot].as_ref().unwrap().value.body,
                "PRIVATE_OLD"
            );
        }
    }

    #[test]
    fn legitimate_later_permission_and_version_changes_remain_known_refusals() {
        for denied in [false, true] {
            let mut r = Recorder::disabled();
            let (mut o, source, id) = super::super::tests::setup(&mut r);
            let mutation = AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "PRIVATE_NEW".into(),
            };
            let p = o
                .prepare_compound(&[], &[], std::slice::from_ref(&mutation), &mut r)
                .unwrap();
            let expected = if denied {
                o.set_annotations_read_only(&source.document, true, &mut r)
                    .unwrap();
                CoordinatorError::Annotation(AnnotationError::Permission)
            } else {
                let later = o.prepare_compound(&[], &[], &[mutation], &mut r).unwrap();
                o.commit(later, &mut r).unwrap();
                CoordinatorError::Stale
            };
            let generation = o.generation();
            assert_eq!(o.commit(p, &mut r).err(), Some(expected));
            assert!(!o.is_faulted());
            assert_eq!(o.generation(), generation);
        }
    }

    #[test]
    fn actual_plan_validation_is_timed_content_free_and_capture_independent() {
        use crate::instrumentation::NativeTimingProfileV1;
        for mode in 0..4 {
            for corrupt in [false, true] {
                let mut setup = Recorder::disabled();
                let (mut o, source, _) = super::super::tests::setup(&mut setup);
                let mut p = o
                    .prepare_compound(&[], &[], &[create(&source)], &mut setup)
                    .unwrap();
                if corrupt {
                    p.annotations[0].id.namespace = Arc::new(());
                }
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Control,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                let generation = o.generation();
                let result = o.commit(p, &mut r);
                assert_eq!(result.is_ok(), !corrupt);
                assert_eq!(o.is_faulted(), corrupt);
                assert_eq!(o.generation(), generation + u64::from(!corrupt));
                let mut found = 0;
                while let Some(event) = r.pop_front() {
                    let debug = format!("{event:?}");
                    assert!(!debug.contains("PRIVATE_"));
                    if event.boundary == Boundary::AnnotationPlanValidation {
                        found += 1;
                        assert_eq!(
                            event.disposition,
                            if corrupt {
                                Disposition::Unknown
                            } else {
                                Disposition::Completed
                            }
                        );
                        assert_eq!(event.service_time.is_some(), mode == 2);
                    }
                }
                assert_eq!(found, usize::from(mode == 1 || mode == 2));
                if mode == 3 {
                    assert!(r.dropped() > 0);
                }
            }
        }
    }
}
