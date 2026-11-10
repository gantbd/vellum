//! Trusted scripted producers; native JSON/filesystem composition is separate.
use vellum_engine::attachment::{AttachmentTarget, LocationEvidence, Placement};
use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/annotation_read_fixture.rs"]
mod fixture;
use fixture::Fixture;
#[path = "loaded_annotation_baseline_behaviors/root_admission.rs"]
mod root_admission;
struct Loaded {
    f: Fixture,
    encoder: AnnotationEncoderPort,
    read: AnnotationReadId,
    dispatch: AnnotationReadDispatch,
}
impl Loaded {
    fn dispose_and_release_keys(&mut self) -> LifecycleState {
        let source = self
            .f
            .owner
            .read(&self.f.doc.document, &mut self.f.recorder)
            .unwrap()
            .version;
        let set = self
            .f
            .owner
            .annotation_set_version(&source.document, &mut self.f.recorder)
            .unwrap();
        let plan = self
            .f
            .owner
            .prepare_retained_disposal(&source, set, &mut self.f.recorder)
            .unwrap();
        let state = self
            .f
            .owner
            .commit_retained_disposal(plan, &mut self.f.recorder)
            .unwrap();
        self.f
            .owner
            .release_disposed_source_binding(&source.document, state, &mut self.f.recorder)
            .unwrap();
        let state = self
            .f
            .owner
            .lifecycle_state(&source.document, &mut self.f.recorder)
            .unwrap();
        self.f
            .owner
            .release_disposed_annotation_binding(&source.document, state, &mut self.f.recorder)
            .unwrap();
        self.f
            .owner
            .lifecycle_state(&source.document, &mut self.f.recorder)
            .unwrap()
    }
    fn checkpoint(&mut self) -> SaveId {
        self.checkpoint_with_dispatch().0
    }
    fn checkpoint_with_dispatch(&mut self) -> (SaveId, SaveDispatch) {
        let save = self.queue(b"loaded").unwrap();
        let write = self.complete_checkpoint(&save);
        (save, write)
    }
    fn complete_checkpoint(&mut self, save: &SaveId) -> SaveDispatch {
        let pair = self.f.pair().unwrap();
        let write = self
            .f
            .owner
            .dispatch_save_with_pair(
                save,
                self.f
                    .port
                    .read_result(b"comments", b"v0", b"loaded")
                    .unwrap(),
                pair,
                &mut self.f.recorder,
            )
            .unwrap();
        for evidence in [
            self.f
                .port
                .result(
                    &write,
                    Application::Written,
                    6,
                    Synchronization::NotRequested,
                    Some(b"v0"),
                )
                .unwrap(),
            self.f.port.no_future_effects(&write).unwrap(),
            self.f.port.quiescence(&write).unwrap(),
        ] {
            self.f
                .owner
                .record_save_evidence(evidence, &mut self.f.recorder)
                .unwrap();
        }
        let pair = self.f.pair();
        self.f
            .owner
            .install_annotation_save_baseline(save, pair, &mut self.f.recorder)
            .unwrap();
        write
    }
    fn new(linked: bool) -> Self {
        Self::new_with_quiescence(linked, true)
    }
    fn new_with_quiescence(linked: bool, quiescent: bool) -> Self {
        let mut f = Fixture::new();
        let encoder = f
            .owner
            .register_annotation_encoder(&mut f.recorder)
            .unwrap();
        let read = f.admit();
        let dispatch = f.submit(&read);
        for evidence in [
            f.port.annotation_read_start(&dispatch, b"v0").unwrap(),
            f.port
                .annotation_read_chunk(&dispatch, 0, 0, b"loaded")
                .unwrap(),
            f.port.annotation_read_end(&dispatch, b"v0", 1, 6).unwrap(),
        ] {
            f.record(evidence).unwrap();
        }
        if quiescent {
            f.record(f.port.annotation_read_quiescence(&dispatch).unwrap())
                .unwrap();
        }
        let capture = f.capture(&read).unwrap();
        let source = f
            .owner
            .capture_annotations(&f.doc, 0, &mut f.recorder)
            .unwrap();
        let port = if linked {
            f.owner
                .register_annotation_restorer_for_encoding(&encoder.context(), &mut f.recorder)
        } else {
            f.owner.register_annotation_restorer(&mut f.recorder)
        }
        .unwrap();
        let record = RestoredAnnotation {
            identity: port.identity(b"one", 7).unwrap(),
            body: "body".into(),
            content_version: 1,
            target_generation: 1,
            authored: SourceTarget {
                source: f.doc.clone(),
                basis: TargetBasis::Before,
                target: AttachmentTarget::Document,
            },
            placement: Placement::Located {
                target: AttachmentTarget::Document,
                evidence: LocationEvidence::Located,
            },
        };
        let package = port.seal(capture, vec![source], vec![record]).unwrap();
        let pair = f.pair();
        f.owner
            .restore_annotations(package, pair, &mut f.recorder)
            .unwrap();
        Self {
            f,
            encoder,
            read,
            dispatch,
        }
    }
    fn attach(&mut self) {
        assert!(
            self.f
                .owner
                .attach_annotation_read_baselines(&mut self.f.recorder)
                .unwrap()
        );
    }
    fn adopt(&mut self) -> Result<bool, ResourceError> {
        let pair = self.f.pair();
        self.f
            .owner
            .adopt_loaded_annotation_baseline(&self.read, pair, 65536, &mut self.f.recorder)
    }
    fn encoded(&mut self, bytes: &[u8]) -> EncodedAnnotations {
        let source = self
            .f
            .owner
            .read(&self.f.doc.document, &mut self.f.recorder)
            .unwrap()
            .version;
        let set = self
            .f
            .owner
            .annotation_set_version(&source.document, &mut self.f.recorder)
            .unwrap();
        let snapshot = self
            .f
            .owner
            .capture_annotations(&source, set, &mut self.f.recorder)
            .unwrap();
        self.encoder.seal(snapshot, bytes.to_vec()).unwrap()
    }
    fn queue(&mut self, bytes: &[u8]) -> Result<SaveId, ResourceError> {
        let encoded = self.encoded(bytes);
        let pair = self.f.pair();
        self.f.owner.queue_annotation_save(
            encoded,
            self.f
                .port
                .read_result(b"comments", b"v0", b"loaded")
                .unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut self.f.recorder,
        )
    }
    fn compare(&mut self, bytes: &[u8]) -> Result<ContentComparison, ResourceError> {
        let encoded = self.encoded(bytes);
        let pair = self.f.pair();
        self.f.owner.compare_annotation_baseline(
            &self.f.doc.document,
            &encoded,
            pair,
            65536,
            &mut self.f.recorder,
        )
    }
}

#[test]
fn an_unadopted_restoration_does_not_need_a_new_writer_attachment_just_to_retire() {
    let mut l = Loaded::new(false);
    let state = l.dispose_and_release_keys();
    l.f.owner
        .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
        .unwrap();
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    f.owner
        .retire_disposed_annotation_read(&read, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
    assert!(
        !f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
}

#[test]
fn released_root_does_not_invent_read_quiescence_from_eof() {
    let mut l = Loaded::new_with_quiescence(true, false);
    l.attach();
    l.adopt().unwrap();
    let state = l.dispose_and_release_keys();
    l.f.owner
        .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
        .unwrap();
    let quiescence = l.f.port.annotation_read_quiescence(&l.dispatch).unwrap();
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    assert_eq!(
        f.owner
            .retire_disposed_annotation_read(&read, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert!(
        !f.owner
            .annotation_read_status(&read, &mut f.recorder)
            .unwrap()
            .quiescent
    );
    f.owner
        .record_annotation_read(quiescence, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_disposed_annotation_read(&read, &mut f.recorder)
        .unwrap();
}

#[test]
fn claimed_writer_delivery_stays_pinned_after_independent_read_retirement() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let (save, write) = l.checkpoint_with_dispatch();
    let state = l.dispose_and_release_keys();
    l.f.owner
        .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
        .unwrap();
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    f.owner
        .retire_disposed_annotation_read(&read, &mut f.recorder)
        .unwrap();
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner.retire_save(&save, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(f.owner.resource_usage(), before);
    drop(write);
    f.owner.retire_save(&save, &mut f.recorder).unwrap();
}

#[test]
fn a_late_writer_contradiction_fences_provenance_release_after_key_release() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let (save, write) = l.checkpoint_with_dispatch();
    let state = l.dispose_and_release_keys();
    l.f.owner
        .record_save_evidence(
            l.f.port
                .result(
                    &write,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut l.f.recorder,
        )
        .unwrap();
    let before = l.f.owner.resource_usage();
    let generation = l.f.owner.generation();
    assert_eq!(
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder),
        Err(ResourceError::Contested)
    );
    assert_eq!(l.f.owner.resource_usage(), before);
    assert_eq!(l.f.owner.generation(), generation);
    assert_eq!(
        l.f.owner.retire_save(&save, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn obsolete_read_conflict_and_optional_instrumentation_do_not_change_disposed_cleanup() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(128), Some(1)] {
        let mut l = Loaded::new(true);
        l.attach();
        l.adopt().unwrap();
        let state = l.dispose_and_release_keys();
        l.f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        l.f.record(
            l.f.port
                .annotation_read_start(&l.dispatch, b"late")
                .unwrap(),
        )
        .unwrap();
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
            .unwrap();
        let Loaded {
            mut f,
            read,
            dispatch,
            ..
        } = l;
        drop(dispatch);
        let generation = f.owner.generation();
        f.owner
            .retire_disposed_annotation_read(&read, &mut f.recorder)
            .unwrap();
        f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
        let mut observations = Vec::new();
        while let Some(observation) = f.recorder.pop_front() {
            observations.push(observation);
        }
        if capacity == Some(128) {
            for boundary in [
                Boundary::DisposedProvenanceRelease,
                Boundary::DisposedReadRetirement,
                Boundary::AnnotationReadDetach,
            ] {
                assert!(
                    observations
                        .iter()
                        .any(|observation| observation.boundary == boundary)
                );
            }
        }
        assert!(
            observations
                .iter()
                .all(|observation| observation.service_time.is_none())
        );
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn explicit_disposed_provenance_release_allows_read_cleanup_without_a_save_checkpoint() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let state = l.dispose_and_release_keys();
    assert_eq!(
        l.f.owner
            .retire_disposed_annotation_read(&l.read, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    assert!(
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
            .unwrap()
    );
    let current =
        l.f.owner
            .lifecycle_state(&l.f.doc.document, &mut l.f.recorder)
            .unwrap();
    assert_eq!(current.phase, LifecyclePhase::ProvenanceReleased);
    let generation = l.f.owner.generation();
    assert!(
        !l.f.owner
            .release_disposed_provenance(&l.f.doc.document, current, &mut l.f.recorder)
            .unwrap()
    );
    assert_eq!(l.f.owner.generation(), generation);
    // Root release is not delivery drain or erasure of already-disclosed data.
    assert_eq!(
        l.f.owner
            .retire_disposed_annotation_read(&l.read, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    let records = f.owner.annotation_usage();
    f.owner
        .retire_disposed_annotation_read(&read, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.annotation_usage().body_bytes, records.body_bytes);
    assert_eq!(
        f.owner.annotation_usage().retained_records,
        records.retained_records
    );
    assert_eq!(f.owner.generation(), generation);
    assert!(
        f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
    assert_eq!(
        f.owner
            .annotation_restoration_outcome(&read, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn disposed_read_and_claimed_writer_retire_in_either_order_without_refunding_origins() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for (read_first, capacity) in [
        (false, None),
        (true, None),
        (false, Some(128)),
        (true, Some(128)),
        (false, Some(1)),
        (true, Some(1)),
    ] {
        let mut l = Loaded::new(true);
        l.attach();
        l.adopt().unwrap();
        let save = l.checkpoint();
        let state = l.dispose_and_release_keys();
        assert_eq!(
            l.f.owner.retire_save(&save, &mut l.f.recorder),
            Err(ResourceError::Retained)
        );
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
            .unwrap();
        let artifact =
            l.f.owner
                .save_outcome(&save, &mut l.f.recorder)
                .unwrap()
                .artifact;
        let Loaded {
            mut f,
            read,
            dispatch,
            ..
        } = l;
        drop(dispatch);
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = f.owner.resource_usage();
        let generation = f.owner.generation();
        if read_first {
            f.owner
                .retire_disposed_annotation_read(&read, &mut f.recorder)
                .unwrap();
        }
        f.owner.retire_save(&save, &mut f.recorder).unwrap();
        assert_eq!(
            f.owner
                .resource_artifact_bytes(&artifact, 0, 0, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        if !read_first {
            f.owner
                .retire_disposed_annotation_read(&read, &mut f.recorder)
                .unwrap();
        }
        f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap();
        let after = f.owner.resource_usage();
        assert_eq!(after.issued_reads_lifetime, before.issued_reads_lifetime);
        assert_eq!(after.issued_lifetime, before.issued_lifetime);
        assert_eq!(
            after.issued_artifacts_lifetime,
            before.issued_artifacts_lifetime
        );
        assert_eq!(after.retained_reads, 0);
        assert_eq!(after.retained, 0);
        assert_eq!(after.held_read_bytes, 0);
        assert_eq!(after.held_artifact_bytes, 0);
        assert_eq!(f.owner.generation(), generation);
        let mut observations = Vec::new();
        while let Some(observation) = f.recorder.pop_front() {
            observations.push(observation);
        }
        if capacity == Some(128) {
            assert!(
                observations
                    .iter()
                    .any(|observation| observation.boundary == Boundary::DisposedWriteRetirement)
            );
            assert!(
                observations
                    .iter()
                    .any(|observation| observation.boundary == Boundary::DisposedReadRetirement)
            );
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn another_retained_writer_task_keeps_a_claimed_artifact_alive_until_its_own_retirement() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let save = l.checkpoint();
    let artifact =
        l.f.owner
            .save_outcome(&save, &mut l.f.recorder)
            .unwrap()
            .artifact;
    let export =
        l.f.owner
            .queue_annotation_export(
                &artifact,
                l.f.port.read_result(b"export", b"e0", b"").unwrap(),
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Ordinary,
                },
                &mut l.f.recorder,
            )
            .unwrap();
    l.f.owner.stop_save(&export, &mut l.f.recorder).unwrap();
    let state = l.dispose_and_release_keys();
    l.f.owner
        .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
        .unwrap();
    let before = l.f.owner.resource_usage();
    assert_eq!(
        l.f.owner.retire_save(&save, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(l.f.owner.resource_usage(), before);
    l.f.owner.retire_save(&export, &mut l.f.recorder).unwrap();
    l.f.owner.retire_save(&save, &mut l.f.recorder).unwrap();
    assert_eq!(
        l.f.owner
            .resource_artifact_bytes(&artifact, 0, 0, &mut l.f.recorder),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn retained_history_and_active_bindings_are_not_overridden_by_provenance_release() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let state =
        l.f.owner
            .lifecycle_state(&l.f.doc.document, &mut l.f.recorder)
            .unwrap();
    assert_eq!(
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder),
        Err(ResourceError::Unsupported)
    );
    let restored =
        l.f.owner
            .annotation_restoration_outcome(&l.read, &mut l.f.recorder)
            .unwrap();
    let prepared =
        l.f.owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: restored.annotations[0].annotation.clone(),
                    expected_content: 1,
                    body: "changed".into(),
                }],
                &mut l.f.recorder,
            )
            .unwrap();
    let edit = l.f.owner.commit(prepared, &mut l.f.recorder).unwrap();
    let state = l.dispose_and_release_keys();
    let generation = l.f.owner.generation();
    let usage = l.f.owner.resource_usage();
    assert_eq!(
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(l.f.owner.generation(), generation);
    assert_eq!(l.f.owner.resource_usage(), usage);
    l.f.owner
        .retire_history_group(&edit.history_group.unwrap(), &mut l.f.recorder)
        .unwrap();
    assert!(
        l.f.owner
            .release_disposed_provenance(&l.f.doc.document, state, &mut l.f.recorder)
            .unwrap()
    );
}

#[test]
fn a_current_acknowledged_checkpoint_allows_atomic_read_discharge_then_detach() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let save = l.checkpoint();
    assert_eq!(
        l.f.owner
            .retire_superseded_annotation_read(&l.read, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    let generation = f.owner.generation();
    let before = f.owner.resource_usage();
    let outcome = f.owner.save_outcome(&save, &mut f.recorder).unwrap();
    f.owner
        .retire_superseded_annotation_read(&read, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(f.owner.resource_usage().retained_reads, 0);
    assert_eq!(
        f.owner.resource_usage().issued_reads_lifetime,
        before.issued_reads_lifetime
    );
    assert_eq!(
        f.owner
            .annotation_restoration_outcome(&read, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(
        f.owner.save_outcome(&save, &mut f.recorder).unwrap(),
        outcome
    );
    assert!(
        f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert!(
        !f.owner
            .detach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
    let pair = f.pair();
    assert!(matches!(
        f.owner
            .annotation_baseline_evidence(&f.doc.document, pair, &mut f.recorder)
            .unwrap(),
        Some(AnnotationBaselineEvidence::Written(_))
    ));
    assert!(
        f.owner
            .attach_annotation_read_baselines(&mut f.recorder)
            .unwrap()
    );
    assert_eq!(
        f.owner
            .adopt_loaded_annotation_baseline(&read, None, 65536, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn original_loaded_slots_and_live_queued_consumers_block_discharge() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    assert_eq!(
        l.f.owner
            .detach_annotation_read_baselines(&mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        l.f.owner
            .retire_superseded_annotation_read(&l.read, &mut l.f.recorder),
        Err(ResourceError::Retained)
    );
    let first = l.queue(b"loaded").unwrap();
    let queued = l.queue(b"next").unwrap();
    drop(l.complete_checkpoint(&first));
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    assert_eq!(
        f.owner
            .retire_superseded_annotation_read(&read, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner.stop_save(&queued, &mut f.recorder).unwrap();
    f.owner
        .retire_superseded_annotation_read(&read, &mut f.recorder)
        .unwrap();
}

#[test]
fn dirty_state_needs_a_fresh_checkpoint_and_retirement_preserves_authored_history() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    l.checkpoint();
    let restored =
        l.f.owner
            .annotation_restoration_outcome(&l.read, &mut l.f.recorder)
            .unwrap();
    let annotation = restored.annotations[0].annotation.clone();
    let edit =
        l.f.owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: annotation.clone(),
                    expected_content: 1,
                    body: "new body".into(),
                }],
                &mut l.f.recorder,
            )
            .unwrap();
    l.f.owner.commit(edit, &mut l.f.recorder).unwrap();
    assert_eq!(
        l.f.owner
            .retire_superseded_annotation_read(&l.read, &mut l.f.recorder),
        Err(ResourceError::Stale)
    );
    l.checkpoint();
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    let history = f.owner.history_usage();
    let generation = f.owner.generation();
    f.owner
        .retire_superseded_annotation_read(&read, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.history_usage(), history);
    assert_eq!(f.owner.generation(), generation);
    let actual = f.owner.annotation(&annotation, &mut f.recorder).unwrap();
    assert_eq!(actual.body, "new body");
    assert_eq!(actual.content_version, 2);
    assert_eq!(actual.restoration_identity.unwrap().incarnation(), 7);
}

#[test]
fn superseded_read_contradiction_does_not_prevent_independent_checkpoint_discharge() {
    use vellum_engine::instrumentation::{Boundary, Recorder};
    for capacity in [None, Some(128), Some(1)] {
        let mut l = Loaded::new(true);
        l.attach();
        l.adopt().unwrap();
        l.checkpoint();
        l.f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        l.f.record(
            l.f.port
                .annotation_read_start(&l.dispatch, b"late")
                .unwrap(),
        )
        .unwrap();
        let Loaded {
            mut f,
            read,
            dispatch,
            ..
        } = l;
        drop(dispatch);
        let generation = f.owner.generation();
        f.owner
            .retire_superseded_annotation_read(&read, &mut f.recorder)
            .unwrap();
        assert!(
            f.owner
                .detach_annotation_read_baselines(&mut f.recorder)
                .unwrap()
        );
        assert_eq!(f.owner.generation(), generation);
        assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
        let mut boundaries = Vec::new();
        while let Some(observation) = f.recorder.pop_front() {
            assert_eq!(observation.service_time, None);
            boundaries.push(observation.boundary);
        }
        if capacity == Some(128) {
            assert!(boundaries.contains(&Boundary::AnnotationReadDischarge));
            assert!(boundaries.contains(&Boundary::AnnotationReadDetach));
        }
        if capacity == Some(1) {
            assert!(f.recorder.dropped() > 0);
        }
    }
}

#[test]
fn current_checkpoint_contradiction_refuses_cleanup_without_releasing_charges() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let (_, write) = l.checkpoint_with_dispatch();
    l.f.owner
        .record_save_evidence(
            l.f.port
                .result(
                    &write,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None,
                )
                .unwrap(),
            &mut l.f.recorder,
        )
        .unwrap();
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    let usage = f.owner.resource_usage();
    let generation = f.owner.generation();
    assert_eq!(
        f.owner
            .retire_superseded_annotation_read(&read, &mut f.recorder),
        Err(ResourceError::Contested)
    );
    assert_eq!(f.owner.resource_usage(), usage);
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner.detach_annotation_read_baselines(&mut f.recorder),
        Err(ResourceError::Retained)
    );
}

#[test]
fn acknowledgment_without_effect_disposition_and_quiescence_cannot_discharge_a_read() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let save = l.queue(b"loaded").unwrap();
    let pair = l.f.pair().unwrap();
    let write =
        l.f.owner
            .dispatch_save_with_pair(
                &save,
                l.f.port.read_result(b"comments", b"v0", b"loaded").unwrap(),
                pair,
                &mut l.f.recorder,
            )
            .unwrap();
    l.f.owner
        .record_save_evidence(
            l.f.port
                .result(
                    &write,
                    Application::Written,
                    6,
                    Synchronization::NotRequested,
                    Some(b"v0"),
                )
                .unwrap(),
            &mut l.f.recorder,
        )
        .unwrap();
    let pair = l.f.pair();
    l.f.owner
        .install_annotation_save_baseline(&save, pair, &mut l.f.recorder)
        .unwrap();
    let Loaded {
        mut f,
        read,
        dispatch,
        ..
    } = l;
    drop(dispatch);
    let usage = f.owner.resource_usage();
    assert_eq!(
        f.owner
            .retire_superseded_annotation_read(&read, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(f.owner.resource_usage(), usage);
    f.owner
        .record_save_evidence(f.port.no_future_effects(&write).unwrap(), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .retire_superseded_annotation_read(&read, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner
        .record_save_evidence(f.port.quiescence(&write).unwrap(), &mut f.recorder)
        .unwrap();
    f.owner
        .retire_superseded_annotation_read(&read, &mut f.recorder)
        .unwrap();
}
#[test]
fn adoption_is_explicit_read_evidence_not_a_write_acknowledgment_and_repetition_is_current() {
    let mut l = Loaded::new(true);
    let generation = l.f.owner.generation();
    assert_eq!(l.adopt(), Err(ResourceError::MissingEvidence));
    l.attach();
    assert!(
        !l.f.owner
            .attach_annotation_read_baselines(&mut l.f.recorder)
            .unwrap()
    );
    assert!(l.adopt().unwrap());
    assert_eq!(l.f.owner.generation(), generation + 1);
    let pair = l.f.pair();
    let fact =
        l.f.owner
            .annotation_baseline_evidence(&l.f.doc.document, pair, &mut l.f.recorder)
            .unwrap()
            .unwrap();
    let AnnotationBaselineEvidence::Loaded(fact) = fact else {
        panic!("not a write")
    };
    assert_eq!(fact.origin, l.read);
    assert_eq!(fact.source, l.f.doc);
    assert_eq!(fact.set_version, 1);
    assert_eq!(fact.coherence, ReadStrength::CapturedRead);
    assert_eq!(fact.encoding_context, l.encoder.context());
    let pair = l.f.pair();
    assert_eq!(
        l.f.owner
            .annotation_baseline(&l.f.doc.document, pair, &mut l.f.recorder),
        Err(ResourceError::Unsupported)
    );
    let generation = l.f.owner.generation();
    assert!(!l.adopt().unwrap());
    assert_eq!(l.f.owner.generation(), generation);
    assert!(
        l.f.owner
            .annotation_restoration_outcome(&l.read, &mut l.f.recorder)
            .unwrap()
            .loaded_baseline_adopted
    );
    assert_eq!(l.compare(b"loaded"), Ok(ContentComparison::Unchanged));
    assert_eq!(
        l.compare(b"canonical output"),
        Ok(ContentComparison::Modified)
    );
}
#[test]
fn an_unlinked_restorer_cannot_retrofit_an_encoding_context_at_adoption() {
    let mut l = Loaded::new(false);
    l.attach();
    assert_eq!(l.adopt(), Err(ResourceError::MissingEvidence));
    assert!(
        !l.f.owner
            .annotation_restoration_outcome(&l.read, &mut l.f.recorder)
            .unwrap()
            .loaded_baseline_adopted
    );
}
#[test]
fn delayed_adoption_names_original_restoration_and_does_not_mark_later_edits_saved() {
    let mut l = Loaded::new(true);
    l.attach();
    let restored =
        l.f.owner
            .annotation_restoration_outcome(&l.read, &mut l.f.recorder)
            .unwrap();
    let edit =
        l.f.owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: restored.annotations[0].annotation.clone(),
                    expected_content: 1,
                    body: "later".into(),
                }],
                &mut l.f.recorder,
            )
            .unwrap();
    l.f.owner.commit(edit, &mut l.f.recorder).unwrap();
    l.f.owner
        .stop_annotation_read(&l.read, &mut l.f.recorder)
        .unwrap();
    l.adopt().unwrap();
    assert_eq!(l.compare(b"later"), Ok(ContentComparison::Modified));
    assert_eq!(
        l.f.owner
            .annotation(&restored.annotations[0].annotation, &mut l.f.recorder)
            .unwrap()
            .body,
        "later"
    );
}
#[test]
fn a_late_read_conflict_fences_loaded_admission_without_consuming_a_new_write_identity() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    l.f.record(
        l.f.port
            .annotation_read_start(&l.dispatch, b"conflict")
            .unwrap(),
    )
    .unwrap();
    let before = l.f.owner.resource_usage();
    assert_eq!(l.queue(b"new"), Err(ResourceError::Contested));
    assert_eq!(l.f.owner.resource_usage(), before);
    assert_eq!(l.compare(b"loaded"), Err(ResourceError::Contested));
}

#[test]
fn queued_saves_keep_the_original_loaded_producer_after_a_written_baseline_replaces_it() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    let first = l.queue(b"loaded").unwrap();
    let second = l.queue(b"second").unwrap();
    let pair = l.f.pair().unwrap();
    let write =
        l.f.owner
            .dispatch_save_with_pair(
                &first,
                l.f.port.read_result(b"comments", b"v0", b"loaded").unwrap(),
                pair,
                &mut l.f.recorder,
            )
            .unwrap();
    for evidence in [
        l.f.port
            .result(
                &write,
                Application::Written,
                6,
                Synchronization::NotRequested,
                Some(b"v0"),
            )
            .unwrap(),
        l.f.port.no_future_effects(&write).unwrap(),
        l.f.port.quiescence(&write).unwrap(),
    ] {
        l.f.owner
            .record_save_evidence(evidence, &mut l.f.recorder)
            .unwrap();
    }
    let pair = l.f.pair();
    l.f.owner
        .install_annotation_save_baseline(&first, pair, &mut l.f.recorder)
        .unwrap();
    l.f.record(
        l.f.port
            .annotation_read_start(&l.dispatch, b"contradiction")
            .unwrap(),
    )
    .unwrap();
    let pair = l.f.pair().unwrap();
    assert!(matches!(
        l.f.owner.dispatch_save_with_pair(
            &second,
            l.f.port.read_result(b"comments", b"v0", b"loaded").unwrap(),
            pair,
            &mut l.f.recorder
        ),
        Err(ResourceError::Contested)
    ));
    let pair = l.f.pair();
    assert!(matches!(
        l.f.owner
            .annotation_baseline_evidence(&l.f.doc.document, pair, &mut l.f.recorder)
            .unwrap(),
        Some(AnnotationBaselineEvidence::Written(_))
    ));
}

#[test]
fn loaded_admission_cannot_refresh_a_changed_external_condition_or_swap_encoder() {
    let mut l = Loaded::new(true);
    l.attach();
    l.adopt().unwrap();
    for (version, bytes) in [(b"v1".as_slice(), b"loaded".as_slice()), (b"v0", b"other")] {
        let encoded = l.encoded(b"new");
        let pair = l.f.pair();
        let usage = l.f.owner.resource_usage();
        assert_eq!(
            l.f.owner.queue_annotation_save(
                encoded,
                l.f.port.read_result(b"comments", version, bytes).unwrap(),
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Ordinary
                },
                pair,
                &mut l.f.recorder
            ),
            Err(ResourceError::Conflict)
        );
        assert_eq!(l.f.owner.resource_usage(), usage);
    }
    let other =
        l.f.owner
            .register_annotation_encoder(&mut l.f.recorder)
            .unwrap();
    let snapshot =
        l.f.owner
            .capture_annotations(&l.f.doc, 1, &mut l.f.recorder)
            .unwrap();
    let encoded = other.seal(snapshot, b"new".to_vec()).unwrap();
    let pair = l.f.pair();
    assert_eq!(
        l.f.owner.queue_annotation_save(
            encoded,
            l.f.port.read_result(b"comments", b"v0", b"loaded").unwrap(),
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary
            },
            pair,
            &mut l.f.recorder
        ),
        Err(ResourceError::Stale)
    );
}

#[test]
fn adoption_budget_refusal_is_atomic_and_a_queued_writer_blocks_initial_adoption() {
    let mut l = Loaded::new(true);
    l.attach();
    let pair = l.f.pair();
    let usage = l.f.owner.resource_usage();
    let generation = l.f.owner.generation();
    assert_eq!(
        l.f.owner
            .adopt_loaded_annotation_baseline(&l.read, pair, 0, &mut l.f.recorder),
        Err(ResourceError::Limit)
    );
    assert_eq!(l.f.owner.resource_usage(), usage);
    assert_eq!(l.f.owner.generation(), generation);
    let save = l.queue(b"new").unwrap();
    assert_eq!(l.adopt(), Err(ResourceError::Blocked));
    l.f.owner.stop_save(&save, &mut l.f.recorder).unwrap();
    assert!(l.adopt().unwrap());
}
