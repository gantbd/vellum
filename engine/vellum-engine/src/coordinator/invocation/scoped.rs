//! Scoped source work uses the existing one-shot source runner and outcome facet.
use super::*;
impl SourceCoordinator {
    pub fn submit_scoped_source(
        &mut self,
        id: &InvocationId,
        source: &SourceIntent,
        scopes: &[SourceOperationScope],
        budget: SourceScopeBudget,
        r: &mut Recorder,
    ) -> Result<InvocationState, InvocationError> {
        self.guard_shutdown_invocation_admission(id, r)?;
        let repeated = self.invocation_slot(id).is_ok_and(|s| s.intent.is_some());
        let result = (|| {
            if self.faulted {
                return Err(InvocationError::Faulted);
            }
            self.invocation_slot(id)?;
            let bytes = self
                .source_intent_bytes(source)?
                .checked_add(
                    scope::ScopeCapture::retained_bytes(scopes).ok_or(InvocationError::Limit)?,
                )
                .ok_or(InvocationError::Limit)?;
            if bytes > self.invocations.limits.max_intent_bytes {
                return Err(InvocationError::Limit);
            }
            // Ordering only: this comparison never acquires current source state.
            let scope = scope::ScopeCapture::copy(scopes, budget).map_err(|e| match e {
                SourceScopeError::Capacity => InvocationError::Capacity,
                _ => InvocationError::Limit,
            })?;
            let owned = self.normalize_intent(source)?;
            if let Some(original) = &self.invocation_slot(id)?.intent {
                if !matches!(original,BoundIntent::ScopedSource {source,scope: old} if source==&owned && old==&scope)
                {
                    return Err(InvocationError::IntentMismatch);
                }
                return self.invocation_slot(id)?.phase.state();
            }
            let checked = self.check_scoped_source_for_invocation(&owned, &scope, r);
            let prepared = checked
                .and_then(|()| self.prepare(&owned.writes, &owned.reads, r))
                .and_then(|mut p| {
                    p.scoped = true;
                    p.scopes = Some(
                        scope::ScopeCapture::copy(&scope.scopes, scope.budget)
                            .map_err(CoordinatorError::Scope)?,
                    );
                    Ok(p)
                });
            let slot = self.invocations.slots[id.slot]
                .as_mut()
                .expect("validated issued slot");
            let Phase::Issued(reserved) = std::mem::replace(&mut slot.phase, Phase::Running) else {
                unreachable!("unbound slot is issued")
            };
            self.install_invocation_admission(
                id,
                BoundIntent::ScopedSource {
                    source: owned,
                    scope,
                },
                Phase::Pending {
                    prepared: prepared.map_err(NativePreparationError::Coordinator),
                    reserved,
                },
            );
            Ok(InvocationState::Pending)
        })();
        capture(
            r,
            if repeated {
                Boundary::InvocationReplay
            } else {
                Boundary::InvocationAdmission
            },
            &result,
        );
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (
        SourceCoordinator,
        DocumentVersion,
        SourceIntent,
        Vec<SourceOperationScope>,
        SourceScopeBudget,
        Recorder,
    ) {
        let mut o = SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 128,
                source: Limits {
                    max_source_bytes: 128,
                    max_edits: 8,
                    max_edit_payload_bytes: 256,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 8,
                max_retained: 8,
                max_intent_bytes: 8192,
            },
        );
        let mut r = Recorder::disabled();
        let v = o.create(b"abc", &mut r).unwrap();
        let intent = SourceIntent {
            writes: vec![DocumentEdits {
                expected: v.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "A".into(),
                }],
            }],
            reads: vec![],
        };
        let scopes = vec![SourceOperationScope {
            expected: v.clone(),
            replacement_ranges: vec![ByteRange { start: 0, end: 1 }],
            insertion_sites: vec![],
        }];
        (
            o,
            v,
            intent,
            scopes,
            SourceScopeBudget {
                max_scopes: 8,
                max_records: 8,
                max_work: 128,
            },
            r,
        )
    }
    #[test]
    fn scoped_descriptor_includes_all_range_site_and_source_storage_before_binding() {
        let (mut o, _, intent, scopes, b, mut r) = setup();
        let bytes = o.source_intent_bytes(&intent).unwrap()
            + scope::ScopeCapture::retained_bytes(&scopes).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        o.invocations.limits.max_intent_bytes = bytes - 1;
        assert_eq!(
            o.submit_scoped_source(&id, &intent, &scopes, b, &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        o.invocations.limits.max_intent_bytes = bytes;
        o.submit_scoped_source(&id, &intent, &scopes, b, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
    }
    #[test]
    fn lost_widened_or_relabelled_scoped_capture_fences_unknown_without_publication() {
        for mutation in 0..4 {
            let (mut o, v, intent, scopes, b, mut r) = setup();
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_scoped_source(&id, &intent, &scopes, b, &mut r)
                .unwrap();
            let Phase::Pending {
                prepared: Ok(p), ..
            } = &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
            else {
                panic!("not prepared")
            };
            match mutation {
                0 => p.scopes = None,
                1 => p.scopes.as_mut().unwrap().scopes[0].replacement_ranges[0].end = 3,
                2 => p.scoped = false,
                _ => p.scopes.as_mut().unwrap().budget.max_work += 1,
            }
            let g = o.generation();
            assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
            assert_eq!(o.generation(), g);
            assert!(o.is_faulted());
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(
                o.documents[v.document.slot].as_ref().unwrap().source.text(),
                "abc"
            );
        }
    }
    #[test]
    fn direct_scope_capture_loss_or_final_record_limit_refuses_before_root_assignment() {
        for loss in [true, false] {
            let (mut o, v, intent, scopes, b, mut r) = setup();
            let mut p = o
                .prepare_scoped(&intent.writes, &intent.reads, &scopes, b, &mut r)
                .unwrap();
            if loss {
                p.scopes = None;
            } else {
                p.scopes.as_mut().unwrap().budget.max_records = 0;
            }
            let g = o.generation();
            let expected = if loss {
                CoordinatorError::Faulted
            } else {
                CoordinatorError::Scope(SourceScopeError::Limit)
            };
            assert_eq!(o.commit(p, &mut r), Err(expected));
            assert_eq!(o.generation(), g);
            assert_eq!(
                o.documents[v.document.slot].as_ref().unwrap().source.text(),
                "abc"
            );
        }
    }
}
