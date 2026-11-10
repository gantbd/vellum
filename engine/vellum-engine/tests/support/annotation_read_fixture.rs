use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::*;
use vellum_engine::source::Limits;
pub(crate) struct Fixture {
    pub(crate) owner: SourceCoordinator,
    pub(crate) port: ProviderPort,
    pub(crate) doc: DocumentVersion,
    pub(crate) recorder: Recorder,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        Self::with_outstanding(8)
    }
    pub(crate) fn with_outstanding(max_outstanding: usize) -> Self {
        Self::with_profiles(max_outstanding, None)
    }
    #[allow(dead_code)] // Optional profile for consumers that exercise invocations.
    pub(crate) fn with_invocations() -> Self {
        Self::with_profiles(
            8,
            Some(InvocationLimits {
                max_lifetime_issued: 16,
                max_retained: 16,
                max_intent_bytes: 8192,
            }),
        )
    }
    fn with_profiles(max_outstanding: usize, invocation: Option<InvocationLimits>) -> Self {
        let mut recorder = Recorder::disabled();
        let mut owner = SourceCoordinator::configured(CoordinatorConfig {
            limits: CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 4096,
                source: Limits {
                    max_source_bytes: 4096,
                    max_edits: 8,
                    max_edit_payload_bytes: 4096,
                },
            },
            invocation,
            history: Some(HistoryLimits {
                max_retained_groups: 16,
                max_journal_bytes: 65536,
                max_pins: 8,
            }),
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
                max_live_tasks: 8,
                max_issued_tasks: 32,
                max_queued_per_destination: 8,
                max_outstanding,
                max_blob_bytes: 64,
                max_key_bytes: 64,
                max_validator_bytes: 64,
                max_encoders: 4,
                max_live_artifacts: 8,
                max_issued_artifacts: 32,
                max_held_artifact_bytes: 4096,
                max_future_baseline_bytes: 4096,
                max_live_reads: 4,
                max_issued_reads: 8,
                max_read_chunks: 8,
                max_held_read_bytes: 65536,
                max_restorers: 4,
                max_restoration_bytes: 8192,
            }),
        });
        let port = owner
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut recorder,
            )
            .unwrap();
        let doc = owner
            .load_resource(
                port.read_result(b"source", b"s0", b"text").unwrap(),
                &mut recorder,
            )
            .unwrap();
        let plan = owner
            .prepare_annotation_binding(
                &doc.document,
                port.read_result(b"comments", b"a0", b"{}").unwrap(),
                &mut recorder,
            )
            .unwrap();
        let proof = port
            .attest_ordinary_pair(plan.pair_check(), ResourceRelation::Distinct)
            .unwrap();
        owner
            .activate_annotation_binding(plan, Some(proof), &mut recorder)
            .unwrap();
        Self {
            owner,
            port,
            doc,
            recorder,
        }
    }
    pub(crate) fn pair(&mut self) -> Option<OrdinaryPairEvidence> {
        let check = self
            .owner
            .ordinary_pair_check(&self.doc.document, &mut self.recorder)
            .unwrap()
            .unwrap();
        Some(
            self.port
                .attest_ordinary_pair(&check, ResourceRelation::Distinct)
                .unwrap(),
        )
    }
    pub(crate) fn admit(&mut self) -> AnnotationReadId {
        let pair = self.pair();
        self.owner
            .admit_annotation_read(
                &self.doc,
                0,
                self.port.annotation_read_target(b"comments").unwrap(),
                ReadStrength::CapturedRead,
                pair,
                &mut self.recorder,
            )
            .unwrap()
    }
    pub(crate) fn submit(&mut self, id: &AnnotationReadId) -> AnnotationReadDispatch {
        let pair = self.pair();
        self.owner
            .submit_annotation_read(id, pair, &mut self.recorder)
            .unwrap()
    }
    pub(crate) fn capture(
        &mut self,
        id: &AnnotationReadId,
    ) -> Result<CapturedAnnotationRead, ResourceError> {
        let pair = self.pair();
        self.owner
            .annotation_read_capture(id, pair, &mut self.recorder)
    }
    pub(crate) fn record(
        &mut self,
        evidence: AnnotationReadEvidence,
    ) -> Result<EvidenceDisposition, ResourceError> {
        self.owner
            .record_annotation_read(evidence, &mut self.recorder)
    }
}
