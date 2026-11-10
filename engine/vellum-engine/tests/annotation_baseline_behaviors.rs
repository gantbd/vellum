//! Scripted privileged issuer tests; real codec/filesystem coverage is separate.
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::Limits;
#[path = "annotation_baseline_behaviors/root_admission.rs"]
mod root_admission;
struct Fixture {
    owner: SourceCoordinator,
    provider: ProviderPort,
    encoder: AnnotationEncoderPort,
    source: DocumentVersion,
    recorder: Recorder,
}
impl Fixture {
    fn new() -> Self {
        Self::with_capacity(65536)
    }
    fn with_capacity(future_baseline_bytes: usize) -> Self {
        let mut recorder = Recorder::disabled();
        let mut owner = SourceCoordinator::configured(CoordinatorConfig {
            limits: CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 8192,
                source: Limits {
                    max_source_bytes: 4096,
                    max_edits: 16,
                    max_edit_payload_bytes: 8192,
                },
            },
            invocation: None,
            history: None,
            search: None,
            annotations: Some(AnnotationLimits {
                max_issued: 16,
                max_records: 16,
                max_body_bytes: 1024,
                max_retained_body_bytes: 8192,
                max_mapping_steps: 8192,
                max_live_snapshots: 16,
                max_snapshot_bytes: 16384,
            }),
            resources: Some(ResourceLimits {
                max_providers: 4,
                max_live_tasks: 16,
                max_issued_tasks: 32,
                max_queued_per_destination: 4,
                max_outstanding: 4,
                max_blob_bytes: 4096,
                max_key_bytes: 64,
                max_validator_bytes: 64,
                max_encoders: 4,
                max_live_artifacts: 16,
                max_issued_artifacts: 64,
                max_held_artifact_bytes: 32768,
                max_future_baseline_bytes: future_baseline_bytes,
                max_live_reads: 8,
                max_issued_reads: 32,
                max_read_chunks: 16,
                max_held_read_bytes: 1048576,
                max_restorers: 4,
                max_restoration_bytes: 8192,
            }),
        });
        let provider = owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: true,
                },
                None,
                &mut recorder,
            )
            .unwrap();
        let source = owner
            .load_resource(
                provider.read_result(b"source", b"s0", b"text").unwrap(),
                &mut recorder,
            )
            .unwrap();
        let encoder = owner.register_annotation_encoder(&mut recorder).unwrap();
        Self {
            owner,
            provider,
            encoder,
            source,
            recorder,
        }
    }
    fn bind(&mut self) {
        let plan = self
            .owner
            .prepare_annotation_binding(
                &self.source.document,
                self.provider
                    .read_result(b"comments", b"a0", b"{}")
                    .unwrap(),
                &mut self.recorder,
            )
            .unwrap();
        let proof = self
            .provider
            .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
            .unwrap();
        self.owner
            .activate_annotation_binding(plan, Some(proof), &mut self.recorder)
            .unwrap();
    }
    fn pair(&mut self) -> Option<OrdinaryPairEvidence> {
        let check = self
            .owner
            .ordinary_pair_check(&self.source.document, &mut self.recorder)
            .unwrap()
            .unwrap();
        Some(
            self.provider
                .attest_ordinary_pair(&check, ResourceRelation::Distinct)
                .unwrap(),
        )
    }
    fn encoded(&mut self, bytes: &[u8]) -> EncodedAnnotations {
        let set = self
            .owner
            .annotation_set_version(&self.source.document, &mut self.recorder)
            .unwrap();
        let source = self
            .owner
            .read(&self.source.document, &mut self.recorder)
            .unwrap()
            .version;
        let snapshot = self
            .owner
            .capture_annotations(&source, set, &mut self.recorder)
            .unwrap();
        self.encoder.seal(snapshot, bytes.to_vec()).unwrap()
    }
    fn retained(&mut self, bytes: &[u8]) -> ResourceArtifactId {
        let encoded = self.encoded(bytes);
        self.owner
            .retain_annotation_artifact(encoded, &mut self.recorder)
            .unwrap()
    }
    fn queue(&mut self, artifact: &ResourceArtifactId, associated: bool) -> SaveId {
        let read = self
            .provider
            .read_result(b"comments", b"a0", b"{}")
            .unwrap();
        let req = SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Synchronized,
        };
        if associated {
            let pair = self.pair();
            self.owner
                .queue_bound_annotation_export(artifact, read, req, pair, &mut self.recorder)
                .unwrap()
        } else {
            self.owner
                .queue_annotation_export(artifact, read, req, &mut self.recorder)
                .unwrap()
        }
    }
    fn written(&mut self, task: &SaveId, token: Option<&[u8]>) -> SaveDispatch {
        let read = self
            .provider
            .read_result(b"comments", b"a0", b"{}")
            .unwrap();
        let dispatch = if self
            .owner
            .save_outcome(task, &mut self.recorder)
            .unwrap()
            .purpose
            == SavePurpose::OrdinaryAnnotations
        {
            let pair = self.pair().unwrap();
            self.owner
                .dispatch_save_with_pair(task, read, pair, &mut self.recorder)
        } else {
            self.owner.dispatch_save(task, read, &mut self.recorder)
        }
        .unwrap();
        for fact in [
            self.provider
                .result(
                    &dispatch,
                    Application::Written,
                    dispatch.bytes().len(),
                    Synchronization::Succeeded,
                    token,
                )
                .unwrap(),
            self.provider.no_future_effects(&dispatch).unwrap(),
            self.provider.quiescence(&dispatch).unwrap(),
        ] {
            self.owner
                .record_save_evidence(fact, &mut self.recorder)
                .unwrap();
        }
        dispatch
    }
    fn publish(&mut self, task: &SaveId, budget: usize) -> Result<bool, ResourceError> {
        let pair = self.pair();
        self.owner
            .publish_annotation_baseline(task, pair, budget, &mut self.recorder)
    }
    fn fact(&mut self) -> Option<AnnotationBaselineFact> {
        let pair = self.pair();
        self.owner
            .annotation_baseline(&self.source.document, pair, &mut self.recorder)
            .unwrap()
    }
    fn compare(
        &mut self,
        current: &EncodedAnnotations,
        budget: usize,
    ) -> Result<ContentComparison, ResourceError> {
        let pair = self.pair();
        self.owner.compare_annotation_baseline(
            &self.source.document,
            current,
            pair,
            budget,
            &mut self.recorder,
        )
    }
    fn ordinary(&mut self, bytes: &[u8]) -> Result<SaveId, ResourceError> {
        let current = self.encoded(bytes);
        let pair = self.pair();
        self.owner.queue_annotation_save(
            current,
            self.provider
                .read_result(b"comments", b"a0", b"{}")
                .unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            pair,
            &mut self.recorder,
        )
    }
}

#[test]
fn ordinary_admission_atomically_installs_capture_task_and_future_baseline_hold() {
    let mut f = Fixture::new();
    f.bind();
    let before = f.owner.resource_usage();
    let generation = f.owner.generation();
    let task = f.ordinary(b"ordinary").unwrap();
    let after = f.owner.resource_usage();
    assert_eq!(
        after.issued_artifacts_lifetime,
        before.issued_artifacts_lifetime + 1
    );
    assert_eq!(after.issued_lifetime, before.issued_lifetime + 1);
    assert_eq!(after.pending_baseline_publications, 1);
    assert!(after.reserved_baseline_bytes > 8);
    assert_eq!(f.owner.generation(), generation);
    let outcome = f.owner.save_outcome(&task, &mut f.recorder).unwrap();
    assert_eq!(outcome.purpose, SavePurpose::OrdinaryAnnotations);
    assert_eq!(outcome.binding_generation, Some(1));
    assert_eq!(
        outcome.baseline_capacity,
        Some(BaselineCapacityState::Pending)
    );
    let pair = f.pair();
    assert!(matches!(
        f.owner.dispatch_save(
            &task,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    ));
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &task,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair.unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(
            f.provider
                .result(
                    &dispatch,
                    Application::Written,
                    8,
                    Synchronization::Succeeded,
                    Some(b"a1"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert!(f.fact().is_none());
    f.publish(&task, usize::MAX).unwrap();
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 0);
    assert_eq!(f.owner.resource_usage().reserved_baseline_bytes, 0);
    assert_eq!(
        f.owner
            .save_outcome(&task, &mut f.recorder)
            .unwrap()
            .baseline_capacity,
        Some(BaselineCapacityState::Installed)
    );
}

#[test]
fn insufficient_ordinary_capacity_or_a_stale_candidate_spends_no_persistent_identity() {
    let mut f = Fixture::with_capacity(0);
    f.bind();
    let before = f.owner.resource_usage();
    let generation = f.owner.generation();
    assert_eq!(f.ordinary(b"one"), Err(ResourceError::Limit));
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(f.owner.generation(), generation);
    let mut f = Fixture::new();
    f.bind();
    let candidate = f.encoded(b"one");
    f.owner
        .set_read_only(&f.source.document, true, &mut f.recorder)
        .unwrap();
    let pair = f.pair();
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner.queue_annotation_save(
            candidate,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized
            },
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Stale)
    );
    assert_eq!(f.owner.resource_usage(), before);
}

#[test]
fn queued_stop_releases_only_the_capacity_promise_not_issued_artifacts_or_tasks() {
    let mut f = Fixture::new();
    f.bind();
    let task = f.ordinary(b"one").unwrap();
    let before = f.owner.resource_usage();
    f.owner.stop_save(&task, &mut f.recorder).unwrap();
    let after = f.owner.resource_usage();
    assert_eq!(after.pending_baseline_publications, 0);
    assert_eq!(after.reserved_baseline_bytes, 0);
    assert_eq!(after.held_artifact_bytes, before.held_artifact_bytes);
    assert_eq!(
        after.issued_artifacts_lifetime,
        before.issued_artifacts_lifetime
    );
    assert_eq!(after.retained, before.retained);
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&task, &mut f.recorder),
        Ok(BaselineCapacityState::NotWritten)
    );
    f.owner.retire_save(&task, &mut f.recorder).unwrap();
}

#[test]
fn quiescence_alone_does_not_release_a_dispatched_promise_but_safe_nonapplication_does() {
    let mut f = Fixture::new();
    f.bind();
    let task = f.ordinary(b"one").unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &task,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_save_evidence(f.provider.quiescence(&dispatch).unwrap(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&task, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    for fact in [
        f.provider
            .result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::NotRequested,
                None,
            )
            .unwrap(),
        f.provider.no_future_effects(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(fact, &mut f.recorder).unwrap();
    }
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&task, &mut f.recorder),
        Ok(BaselineCapacityState::NotWritten)
    );
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 0);
    assert_eq!(
        f.owner.retire_save(&task, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    f.owner.retire_save(&task, &mut f.recorder).unwrap();
}

#[test]
fn terminal_ineligible_capacity_can_drain_after_admission_stop_without_erasing_late_evidence() {
    let mut f = Fixture::new();
    f.bind();
    let task = f.ordinary(b"one").unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &task,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    for fact in [
        f.provider
            .result(
                &dispatch,
                Application::Written,
                3,
                Synchronization::Failed,
                None,
            )
            .unwrap(),
        f.provider.no_future_effects(&dispatch).unwrap(),
        f.provider.quiescence(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(fact, &mut f.recorder).unwrap();
    }
    f.owner.stop_annotation_save_admission(&mut f.recorder);
    assert_eq!(f.ordinary(b"later"), Err(ResourceError::Unavailable));
    let generation = f.owner.generation();
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&task, &mut f.recorder),
        Ok(BaselineCapacityState::TerminalIneligible)
    );
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 0);
    assert_eq!(f.owner.resource_usage().reserved_baseline_bytes, 0);
    assert_eq!(
        f.owner
            .save_outcome(&task, &mut f.recorder)
            .unwrap()
            .application,
        Application::Written
    );
    assert!(f.fact().is_none());
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner.record_save_evidence(
            f.provider
                .result(
                    &dispatch,
                    Application::Written,
                    3,
                    Synchronization::Succeeded,
                    None
                )
                .unwrap(),
            &mut f.recorder
        ),
        Ok(EvidenceDisposition::Contradiction)
    );
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&task, &mut f.recorder),
        Err(ResourceError::Contested)
    );
    assert_eq!(f.owner.resource_usage().reserved_baseline_bytes, 0);
    assert_eq!(
        f.owner
            .save_outcome(&task, &mut f.recorder)
            .unwrap()
            .baseline_capacity,
        Some(BaselineCapacityState::TerminalIneligible)
    );
}

#[test]
fn only_an_actually_adopted_newer_baseline_discharges_a_superseded_ordinary_promise() {
    let mut f = Fixture::new();
    f.bind();
    let first = f.ordinary(b"first").unwrap();
    drop(f.written(&first, None));
    let second = f.ordinary(b"second").unwrap();
    drop(f.written(&second, Some(b"a2")));
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 2);
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&first, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let pair = f.pair();
    f.owner
        .install_annotation_save_baseline(&second, pair, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 1);
    assert_eq!(
        f.owner
            .release_annotation_save_capacity(&first, &mut f.recorder),
        Ok(BaselineCapacityState::Superseded)
    );
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 0);
    f.owner.retire_save(&first, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.retire_save(&second, &mut f.recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn ordinary_launch_rechecks_policy_but_does_not_recapture_later_authorship() {
    use vellum_engine::attachment::AttachmentTarget;
    let mut f = Fixture::new();
    f.bind();
    let task = f.ordinary(b"old").unwrap();
    let authored = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "new comment".into(),
                target: SourceTarget {
                    source: f.source.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
            }],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(authored, &mut f.recorder).unwrap();
    f.owner
        .set_annotations_read_only(&f.source.document, true, &mut f.recorder)
        .unwrap();
    let pair = f.pair().unwrap();
    assert!(matches!(
        f.owner.dispatch_save_with_pair(
            &task,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            pair,
            &mut f.recorder
        ),
        Err(ResourceError::Permission)
    ));
    assert_eq!(f.owner.resource_usage().pending_baseline_publications, 1);
    f.owner
        .set_annotations_read_only(&f.source.document, false, &mut f.recorder)
        .unwrap();
    let dispatch = f.written(&task, None);
    assert_eq!(dispatch.bytes(), b"old");
    drop(dispatch);
    let pair = f.pair();
    f.owner
        .install_annotation_save_baseline(&task, pair, &mut f.recorder)
        .unwrap();
    let current = f.encoded(b"new");
    assert_eq!(f.compare(&current, 65536), Ok(ContentComparison::Modified));
}
#[test]
fn acknowledgment_and_baseline_publication_are_separate_and_only_original_associations_can_publish()
{
    let mut f = Fixture::new();
    let artifact = f.retained(b"original");
    let old_export = f.queue(&artifact, false);
    drop(f.written(&old_export, Some(b"a1")));
    f.bind();
    assert_eq!(f.publish(&old_export, 65536), Err(ResourceError::Unbound));
    assert!(f.fact().is_none());
    let task = f.queue(&artifact, true);
    assert_eq!(
        f.publish(&task, 65536),
        Err(ResourceError::AcknowledgmentUnsatisfied)
    );
    drop(f.written(&task, Some(b"a1")));
    assert!(f.fact().is_none());
    let generation = f.owner.generation();
    assert_eq!(f.publish(&task, 0), Err(ResourceError::Limit));
    assert_eq!(f.owner.generation(), generation);
    assert!(f.publish(&task, 65536).unwrap());
    let fact = f.fact().unwrap();
    assert_eq!(fact.producer, task);
    assert_eq!(fact.source, f.source);
    assert_eq!(fact.encoding_context, f.encoder.context());
    assert_eq!(fact.ordinary_binding_generation, 1);
    assert_eq!(fact.required_acknowledgment, Acknowledgment::Synchronized);
    assert_eq!(fact.qualified_acknowledgment, Acknowledgment::Synchronized);
    assert!(fact.external_version_available);
    assert_eq!(
        f.owner
            .saved_state(&f.source.document, &mut f.recorder)
            .unwrap()
            .content,
        ContentComparison::Unchanged
    );
    let generation = f.owner.generation();
    assert!(!f.publish(&task, 65536).unwrap());
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn comparison_is_exact_context_bound_current_and_does_not_spend_persistent_artifact_capacity() {
    let mut f = Fixture::new();
    f.bind();
    let artifact = f.retained(b"{ \"records\": [] }");
    let task = f.queue(&artifact, true);
    drop(f.written(&task, None));
    f.publish(&task, 65536).unwrap();
    let current = f.encoded(b"{ \"records\": [] }");
    let usage = f.owner.resource_usage();
    let generation = f.owner.generation();
    assert_eq!(f.compare(&current, 33), Err(ResourceError::Limit));
    assert_eq!(f.compare(&current, 34), Ok(ContentComparison::Unchanged));
    let different = f.encoded(b"{\"records\":[]}");
    assert_eq!(
        f.compare(&different, 65536),
        Ok(ContentComparison::Modified)
    );
    assert_eq!(f.owner.resource_usage(), usage);
    assert_eq!(f.owner.generation(), generation);
    let other = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let set = f
        .owner
        .annotation_set_version(&f.source.document, &mut f.recorder)
        .unwrap();
    let snapshot = f
        .owner
        .capture_annotations(&f.source, set, &mut f.recorder)
        .unwrap();
    let foreign_context = other
        .seal(snapshot, b"{ \"records\": [] }".to_vec())
        .unwrap();
    assert_eq!(
        f.compare(&foreign_context, 65536),
        Err(ResourceError::Stale)
    );
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: f.source.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 4,
                    expected: "text".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(edit, &mut f.recorder).unwrap();
    assert_eq!(f.compare(&current, 65536), Err(ResourceError::Stale));
}
#[test]
fn baseline_and_newest_eligible_producer_pin_actual_tasks_and_artifacts() {
    let mut f = Fixture::new();
    f.bind();
    let artifact = f.retained(b"first");
    let first = f.queue(&artifact, true);
    drop(f.written(&first, None));
    assert_eq!(
        f.owner.retire_save(&first, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.publish(&first, 65536).unwrap();
    let newer_artifact = f.retained(b"second");
    let second = f.queue(&newer_artifact, true);
    drop(f.written(&second, Some(b"a2")));
    assert_eq!(f.publish(&first, 65536), Err(ResourceError::Superseded));
    assert_eq!(
        f.owner.retire_save(&first, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner.retire_resource_artifact(&artifact, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.publish(&second, 65536).unwrap();
    f.owner.retire_save(&first, &mut f.recorder).unwrap();
    f.owner
        .retire_resource_artifact(&artifact, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.retire_save(&second, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner
            .retire_resource_artifact(&newer_artifact, &mut f.recorder),
        Err(ResourceError::Retained)
    );
}
#[test]
fn token_enrichment_is_explicit_and_contradiction_fences_without_reverting_authored_state() {
    let mut f = Fixture::new();
    f.bind();
    let artifact = f.retained(b"first");
    let task = f.queue(&artifact, true);
    let dispatch = f.written(&task, None);
    f.publish(&task, 65536).unwrap();
    assert!(!f.fact().unwrap().external_version_available);
    f.owner
        .record_save_evidence(
            f.provider
                .result(
                    &dispatch,
                    Application::Written,
                    5,
                    Synchronization::Succeeded,
                    Some(b"a1"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert!(!f.fact().unwrap().external_version_available);
    assert!(f.publish(&task, 65536).unwrap());
    assert!(f.fact().unwrap().external_version_available);
    let generation = f.owner.generation();
    f.owner
        .record_save_evidence(
            f.provider
                .result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::Succeeded,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.publish(&task, 65536), Err(ResourceError::Contested));
    let current = f.encoded(b"first");
    assert_eq!(f.compare(&current, 65536), Err(ResourceError::Contested));
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner
            .read(&f.source.document, &mut f.recorder)
            .unwrap()
            .version,
        f.source
    );
}

#[test]
fn a_newer_context_wins_by_dispatch_not_arrival_and_unqualified_writes_do_not_erase_history() {
    let mut f = Fixture::new();
    f.bind();
    let first_artifact = f.retained(b"same");
    let first = f.queue(&first_artifact, true);
    let first_dispatch = f.written(&first, None);
    let old_context = f.encoded(b"same");
    let old_encoder = f.encoder.context();
    f.encoder = f
        .owner
        .register_annotation_encoder(&mut f.recorder)
        .unwrap();
    let second_artifact = f.retained(b"same");
    let second = f.queue(&second_artifact, true);
    let dispatch = f
        .owner
        .dispatch_save(
            &second,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    for fact in [
        f.provider
            .result(
                &dispatch,
                Application::Written,
                4,
                Synchronization::Unknown,
                None,
            )
            .unwrap(),
        f.provider.no_future_effects(&dispatch).unwrap(),
        f.provider.quiescence(&dispatch).unwrap(),
    ] {
        f.owner.record_save_evidence(fact, &mut f.recorder).unwrap();
    }
    f.publish(&first, 65536).unwrap();
    assert_eq!(f.fact().unwrap().encoding_context, old_encoder);
    assert_eq!(
        f.publish(&second, 65536),
        Err(ResourceError::AcknowledgmentUnsatisfied)
    );
    f.owner
        .record_save_evidence(
            f.provider
                .result(
                    &dispatch,
                    Application::Written,
                    4,
                    Synchronization::Succeeded,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    // A late token on the older serial cannot make it newest.
    f.owner
        .record_save_evidence(
            f.provider
                .result(
                    &first_dispatch,
                    Application::Written,
                    4,
                    Synchronization::Succeeded,
                    Some(b"late"),
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(f.publish(&first, 65536), Err(ResourceError::Superseded));
    f.publish(&second, 65536).unwrap();
    let fact = f.fact().unwrap();
    assert_eq!(fact.encoding_context, f.encoder.context());
    assert!(
        fact.dispatch_serial
            > f.owner
                .save_outcome(&first, &mut f.recorder)
                .unwrap()
                .dispatch_serial
                .unwrap()
    );
    assert_eq!(f.compare(&old_context, 65536), Err(ResourceError::Stale));
}

#[test]
fn annotation_set_versions_are_not_output_equality_and_old_candidates_do_not_become_current_again()
{
    use vellum_engine::attachment::AttachmentTarget;
    let mut f = Fixture::new();
    f.bind();
    let artifact = f.retained(b"empty");
    let task = f.queue(&artifact, true);
    drop(f.written(&task, None));
    f.publish(&task, 65536).unwrap();
    let old = f.encoded(b"empty");
    let create = f
        .owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "temporary".into(),
                target: SourceTarget {
                    source: f.source.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
            }],
            &mut f.recorder,
        )
        .unwrap();
    let id = create.created_annotations().next().unwrap().clone();
    f.owner.commit(create, &mut f.recorder).unwrap();
    assert_eq!(f.compare(&old, 65536), Err(ResourceError::Stale));
    let record = f.owner.annotation(&id, &mut f.recorder).unwrap();
    let delete = AnnotationMutation::Delete {
        annotation: id.clone(),
        expected_content: record.content_version,
        expected_target: record.target_generation,
    };
    let delete = f
        .owner
        .prepare_compound(&[], &[], &[delete], &mut f.recorder)
        .unwrap();
    f.owner.commit(delete, &mut f.recorder).unwrap();
    let current = f.encoded(b"empty");
    assert_eq!(f.compare(&current, 65536), Ok(ContentComparison::Unchanged));
    assert_eq!(f.compare(&old, 65536), Err(ResourceError::Stale));
    assert_eq!(
        f.owner
            .read(&f.source.document, &mut f.recorder)
            .unwrap()
            .version,
        f.source
    );
}

#[test]
fn original_binding_capture_publication_and_comparison_each_require_current_known_pair() {
    let mut f = Fixture::new();
    f.bind();
    let artifact = f.retained(b"one");
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner.queue_bound_annotation_export(
            &artifact,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized
            },
            None,
            &mut f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    );
    assert_eq!(f.owner.resource_usage(), before);
    let task = f.queue(&artifact, true);
    drop(f.written(&task, None));
    assert_eq!(
        f.owner
            .publish_annotation_baseline(&task, None, 65536, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
    f.publish(&task, 65536).unwrap();
    let current = f.encoded(b"one");
    let check = f
        .owner
        .ordinary_pair_check(&f.source.document, &mut f.recorder)
        .unwrap()
        .unwrap();
    let unknown = Some(
        f.provider
            .attest_ordinary_pair(&check, ResourceRelation::Unknown)
            .unwrap(),
    );
    assert_eq!(
        f.owner.compare_annotation_baseline(
            &f.source.document,
            &current,
            unknown,
            65536,
            &mut f.recorder
        ),
        Err(ResourceError::MissingEvidence)
    );
    assert_eq!(
        f.owner
            .annotation_baseline(&f.source.document, None, &mut f.recorder),
        Err(ResourceError::MissingEvidence)
    );
}

#[test]
fn ordinary_requested_ack_is_not_silently_strengthened_by_a_sync_capable_provider() {
    let mut f = Fixture::new();
    f.bind();
    let artifact = f.retained(b"one");
    let pair = f.pair();
    let task = f
        .owner
        .queue_bound_annotation_export(
            &artifact,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let dispatch = f
        .owner
        .dispatch_save(
            &task,
            f.provider.read_result(b"comments", b"a0", b"{}").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert!(!dispatch.synchronization_required());
    f.owner
        .record_save_evidence(
            f.provider
                .result(
                    &dispatch,
                    Application::Written,
                    3,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.publish(&task, 65536).unwrap();
    let fact = f.fact().unwrap();
    assert_eq!(fact.required_acknowledgment, Acknowledgment::Ordinary);
    assert_eq!(fact.qualified_acknowledgment, Acknowledgment::Ordinary);
}

#[test]
fn a_later_unassociated_export_cannot_supersede_or_release_an_ordinary_associated_producer() {
    let mut f = Fixture::new();
    f.bind();
    let first_artifact = f.retained(b"first");
    let first = f.queue(&first_artifact, true);
    drop(f.written(&first, None));
    let second_artifact = f.retained(b"different external output");
    let second = f.queue(&second_artifact, false);
    drop(f.written(&second, Some(b"a2")));
    assert_eq!(
        f.owner.retire_save(&first, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.publish(&first, 65536).unwrap();
    assert_eq!(f.fact().unwrap().producer, first);
    let current = f.encoded(b"first");
    // Historical equality is not a claim about the now-replaced external file.
    assert_eq!(f.compare(&current, 65536), Ok(ContentComparison::Unchanged));
    assert_eq!(f.publish(&second, 65536), Err(ResourceError::Unbound));
    f.owner.retire_save(&second, &mut f.recorder).unwrap();
    f.owner
        .retire_resource_artifact(&second_artifact, &mut f.recorder)
        .unwrap();
    let usage = f.owner.resource_usage();
    assert_eq!(usage.annotation_baselines, 1);
    assert!(usage.annotation_baseline_logical_bytes > 5);
}
