use super::*;
fn drain_limits() -> DiagnosticDrainLimitsV1 {
    DiagnosticDrainLimitsV1 {
        max_tasks: 3,
        max_issued: 5,
        max_context_bytes: 2048,
        max_binding_bytes: 4096,
        max_request_bytes: 512,
        max_batch_bytes: 4096,
        max_result_bytes: 8192,
        max_control_reply_bytes: 512,
        max_held_bytes: 131072,
    }
}
fn df(s: &DiagnosticScopeV1) -> TrustedDiagnosticDrainFactsV1<'_> {
    TrustedDiagnosticDrainFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        outside_publication_guard: Some(true),
        drain_allowed: Some(true),
        resources_ready: Some(true),
        bookkeeping_reserved: Some(true),
        completion_reserved: Some(true),
        queue_interpretation_current: Some(true),
        drain_interpretation_current: Some(true),
        scheduler_owned: Some(true),
        result_allowed: Some(true),
        queue_release_allowed: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        reply_ready: Some(true),
    }
}
fn dc(s: &DiagnosticScopeV1) -> DiagnosticDrainControlV1<'_> {
    DiagnosticDrainControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        outside_publication_guard: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        binding_ready: Some(true),
        queue_interpretation_current: Some(true),
        drain_interpretation_current: Some(true),
        control_allowed: Some(true),
        control_ready: Some(true),
        transfer_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
struct F {
    o: SourceCoordinator,
    r: Recorder,
    q: DiagnosticScopeV1,
    g: DiagnosticScopeV1,
}
impl F {
    fn new() -> Self {
        Self::with_headroom(0)
    }
    fn with_headroom(additional: usize) -> Self {
        let mut l = limits();
        l.max_records = 4;
        l.max_queue_bytes = 1024;
        l.max_prepared_issued = 16;
        let (mut o, mut r, q, _) = ready_with_budget(
            l,
            drain_limits()
                .required_hold_bytes()
                .unwrap()
                .checked_add(additional)
                .unwrap(),
        );
        let mut g = q.clone();
        g.context.profile = OWNED_DIAGNOSTIC_DRAIN_PROFILE_V1.into();
        o.open_diagnostic_drain_v1(&g, drain_limits(), &dc(&g), &mut r)
            .unwrap();
        Self { o, r, q, g }
    }
    fn offer(&mut self, milestone: DiagnosticMilestoneV1) {
        let token = self
            .o
            .prepare_diagnostic_marker_v1(&self.q, marker(milestone), &facts(&self.q), &mut self.r)
            .unwrap();
        self.o
            .capture_diagnostic_marker_v1(&self.q, &token, &facts(&self.q), &mut self.r)
            .unwrap();
    }
    fn issue(&mut self) -> DiagnosticDrainIssuedV1 {
        self.o
            .issue_diagnostic_drain_v1(&self.g, &df(&self.g), &mut self.r)
            .unwrap()
    }
    fn submit(
        &mut self,
        task: &DiagnosticDrainIssuedV1,
        limit: usize,
    ) -> Result<DiagnosticDrainPhaseV1, DiagnosticDrainErrorV1> {
        self.o.submit_diagnostic_drain_v1(
            &task.task,
            &task.generation,
            limit,
            &self.g,
            &df(&self.g),
            &mut self.r,
        )
    }
    fn run(
        &mut self,
        task: &DiagnosticDrainIssuedV1,
    ) -> Result<DiagnosticDrainPhaseV1, DiagnosticDrainErrorV1> {
        self.o.run_diagnostic_drain_v1(
            &task.task,
            &self.g,
            &df(&self.g),
            &facts(&self.q),
            &mut self.r,
        )
    }
    fn cleanup(mut self, task: &DiagnosticDrainIssuedV1) {
        self.o
            .retire_diagnostic_drain_task_v1(&task.task, &dc(&self.g), &mut self.r)
            .unwrap();
        self.o
            .stop_diagnostic_drain_v1(&dc(&self.g), &mut self.r)
            .unwrap();
        self.o
            .retire_diagnostic_drain_v1(&dc(&self.g), &mut self.r)
            .unwrap();
        self.o
            .stop_diagnostic_capture_v1(&self.q, false, &control(&self.q), &mut self.r)
            .unwrap();
        self.o
            .retire_diagnostic_capture_v1(&control(&self.q), &mut self.r)
            .unwrap();
        assert_eq!(self.o.retention_usage().reserved, 0);
    }
}
#[test]
fn original_task_selects_service_time_prefix_and_replay_or_result_queries_never_redrain() {
    let mut f = F::new();
    let d = f.o.create(b"prose", &mut f.r).unwrap();
    let root_generation = f.o.generation();
    f.offer(DiagnosticMilestoneV1::Offered);
    f.offer(DiagnosticMilestoneV1::Completed);
    let task = f.issue();
    assert_eq!(f.submit(&task, 1), Ok(DiagnosticDrainPhaseV1::Pending));
    assert_eq!(
        f.submit(&task, 2),
        Err(DiagnosticDrainErrorV1::IntentMismatch)
    );
    f.o.drain_diagnostic_markers_v1(&f.q, &task.generation, 1, &facts(&f.q), &mut f.r)
        .unwrap();
    f.offer(DiagnosticMilestoneV1::Discarded);
    assert_eq!(f.run(&task), Ok(DiagnosticDrainPhaseV1::Terminal));
    for _ in 0..2 {
        assert_eq!(f.submit(&task, 1), Ok(DiagnosticDrainPhaseV1::Terminal));
        let result =
            f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &df(&f.g), &mut f.r)
                .unwrap();
        assert!(result.admitted);
        assert_eq!(result.drain, DiagnosticDrainDispositionV1::Performed);
        let batch = result.batch.unwrap();
        assert_eq!(batch.records, [marker(DiagnosticMilestoneV1::Completed)]);
        assert!(batch.remaining);
        assert_eq!(
            f.o.read_diagnostic_health_v1(&control(&f.q))
                .unwrap()
                .queued_records,
            1
        );
    }
    assert_eq!(f.run(&task), Err(DiagnosticDrainErrorV1::Unavailable));
    f.o.stop_diagnostic_drain_task_v1(&task.task, &dc(&f.g), &mut f.r)
        .unwrap();
    let result =
        f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &df(&f.g), &mut f.r)
            .unwrap();
    assert_eq!(result.drain, DiagnosticDrainDispositionV1::Performed);
    assert!(result.batch.is_none());
    assert_eq!(f.o.generation(), root_generation);
    assert_eq!(
        f.o.read(&d.document, &mut f.r).unwrap().source.text(),
        "prose"
    );
    f.cleanup(&task);
}
#[test]
fn readiness_defers_but_bridge_authority_loss_does_not_stop_independent_capture() {
    let mut f = F::new();
    f.offer(DiagnosticMilestoneV1::Completed);
    let task = f.issue();
    f.submit(&task, 1).unwrap();
    let mut unavailable = df(&f.g);
    unavailable.scheduler_owned = None;
    assert_eq!(
        f.o.run_diagnostic_drain_v1(&task.task, &f.g, &unavailable, &facts(&f.q), &mut f.r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    assert_eq!(
        f.o.query_diagnostic_drain_control_v1(&task.task, &dc(&f.g), &mut f.r)
            .unwrap()
            .phase,
        DiagnosticDrainPhaseV1::Pending
    );
    let mut lost = df(&f.g);
    lost.authorized = Some(false);
    assert_eq!(
        f.o.run_diagnostic_drain_v1(&task.task, &f.g, &lost, &facts(&f.q), &mut f.r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    let terminal =
        f.o.query_diagnostic_drain_control_v1(&task.task, &dc(&f.g), &mut f.r)
            .unwrap();
    assert!(terminal.admitted);
    assert_eq!(
        terminal.drain,
        Some(DiagnosticDrainDispositionV1::NotPerformed)
    );
    let health = f.o.read_diagnostic_health_v1(&control(&f.q)).unwrap();
    assert_eq!(health.mode, DiagnosticModeV1::Enabled);
    assert_eq!(health.queued_records, 1);
    assert!(
        f.o.issue_diagnostic_drain_v1(&f.g, &df(&f.g), &mut f.r)
            .is_err()
    );
    f.cleanup(&task);
}
#[test]
fn current_result_authority_and_queue_generation_do_not_rewrite_performed_knowledge() {
    let mut f = F::new();
    f.offer(DiagnosticMilestoneV1::Completed);
    let task = f.issue();
    f.submit(&task, 1).unwrap();
    f.run(&task).unwrap();
    let mut denied = df(&f.g);
    denied.result_allowed = None;
    assert_eq!(
        f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &denied, &mut f.r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    assert!(
        f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &df(&f.g), &mut f.r)
            .unwrap()
            .batch
            .is_some()
    );
    f.o.stop_diagnostic_capture_v1(&f.q, false, &control(&f.q), &mut f.r)
        .unwrap();
    f.o.reset_diagnostic_capture_v1(&control(&f.q), &mut f.r)
        .unwrap();
    let result =
        f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &df(&f.g), &mut f.r)
            .unwrap();
    assert_eq!(result.drain, DiagnosticDrainDispositionV1::Performed);
    assert!(result.batch.is_none());
    f.cleanup(&task);
}

#[test]
fn pending_first_stop_and_known_holders_block_premature_retirement() {
    let mut f = F::new();
    f.offer(DiagnosticMilestoneV1::Completed);
    let task = f.issue();
    f.submit(&task, 1).unwrap();
    assert_eq!(
        f.o.retire_diagnostic_drain_task_v1(&task.task, &dc(&f.g), &mut f.r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    f.o.stop_diagnostic_drain_task_v1(&task.task, &dc(&f.g), &mut f.r)
        .unwrap();
    let c =
        f.o.query_diagnostic_drain_control_v1(&task.task, &dc(&f.g), &mut f.r)
            .unwrap();
    assert!(c.admitted);
    assert_eq!(c.drain, Some(DiagnosticDrainDispositionV1::NotPerformed));
    assert_eq!(f.run(&task), Err(DiagnosticDrainErrorV1::Unavailable));
    assert_eq!(
        f.o.read_diagnostic_health_v1(&control(&f.q))
            .unwrap()
            .counters
            .drained
            .value,
        0
    );
    f.cleanup(&task);
}

#[test]
fn task_aliases_are_once_only_but_foreign_and_retired_slot_aliases_never_become_new_tasks() {
    let mut f = F::new();
    let mut other = F::new();
    let foreign = other.issue();
    assert_eq!(
        f.submit(&foreign, 1),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    let first = f.issue();
    let alias = first.clone();
    f.o.retire_diagnostic_drain_task_v1(&first.task, &dc(&f.g), &mut f.r)
        .unwrap();
    let mut live = Vec::new();
    for _ in 0..3 {
        live.push(f.issue());
    }
    assert_eq!(
        f.submit(&alias, 1),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    assert_eq!(
        f.o.issue_diagnostic_drain_v1(&f.g, &df(&f.g), &mut f.r),
        Err(DiagnosticDrainErrorV1::Limit)
    );
    for task in live {
        f.o.retire_diagnostic_drain_task_v1(&task.task, &dc(&f.g), &mut f.r)
            .unwrap();
    }
    let last = f.issue();
    assert_eq!(
        f.o.issue_diagnostic_drain_v1(&f.g, &df(&f.g), &mut f.r),
        Err(DiagnosticDrainErrorV1::Limit)
    );
    f.cleanup(&last);
    other.cleanup(&foreign);
}

#[test]
fn service_refusals_are_terminal_not_performed_but_an_empty_accepted_drain_is_performed() {
    for kind in 0..4 {
        let mut f = F::new();
        let task = f.issue();
        f.submit(&task, 1).unwrap();
        let mut current = df(&f.g);
        let mut queue = facts(&f.q);
        match kind {
            0 => current.drain_allowed = Some(false),
            1 => current.queue_release_allowed = None,
            2 => queue.reply_ready = Some(false),
            _ => {}
        }
        assert_eq!(
            f.o.run_diagnostic_drain_v1(&task.task, &f.g, &current, &queue, &mut f.r),
            Ok(DiagnosticDrainPhaseV1::Terminal)
        );
        let result =
            f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &df(&f.g), &mut f.r)
                .unwrap();
        assert_eq!(
            result.drain,
            if kind == 3 {
                DiagnosticDrainDispositionV1::Performed
            } else {
                DiagnosticDrainDispositionV1::NotPerformed
            }
        );
        assert_eq!(result.batch.is_some(), kind == 3);
        f.cleanup(&task);
    }
}

#[test]
fn typed_dynamic_and_installed_requests_share_the_original_task_with_all_capture_modes() {
    use crate::selection_discovery::Route;
    use DiagnosticDrainEnvelopeValueV1 as V;
    use vellum_engine::capture::InertValueV1 as D;
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for installed in [false, true] {
        for dynamic in [false, true] {
            for mode in 0..4 {
                // Explicit test-only headroom for the independent discovery owner.
                let mut f = F::with_headroom(4_194_304);
                f.offer(DiagnosticMilestoneV1::Completed);
                let route = installed.then(|| Route::install_diagnostic_drain(&mut f.o, &mut f.r));
                let task = f.issue();
                f.r = match mode {
                    0 => Recorder::disabled(),
                    1 => Recorder::try_with_capacity(512).unwrap(),
                    2 => Recorder::try_with_capacity(512)
                        .unwrap()
                        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                    _ => Recorder::try_with_capacity(0)
                        .unwrap()
                        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                };
                let args = [
                    ("contract", V::Data(D::Text("host-diagnostic-drain/1"))),
                    ("task", V::Task(&task.task)),
                    ("generation", V::Generation(&task.generation)),
                    ("limit", V::Data(D::Number(1.0))),
                ];
                let header = [
                    (
                        if dynamic { "name" } else { "family" },
                        V::Data(if dynamic {
                            D::Text("request-host-facility")
                        } else {
                            D::Number(25.0)
                        }),
                    ),
                    ("variant", V::Data(D::Text("authorized-diagnostic-drain"))),
                    ("version", V::Data(D::Number(1.0))),
                    ("arguments", V::Object(&args)),
                ];
                let envelope = [
                    (
                        "route",
                        V::Data(D::Text(if dynamic { "dynamic" } else { "typed" })),
                    ),
                    ("request", V::Object(&header)),
                ];
                let n =
                    normalize_diagnostic_drain_envelope_v1(V::Object(&envelope), &mut f.r).unwrap();
                let submit = |f: &mut F| match &route {
                    Some(route) => {
                        route.submit_diagnostic_drain(&mut f.o, &n, &f.g, &df(&f.g), &mut f.r)
                    }
                    None => {
                        f.o.submit_diagnostic_drain_arguments_v1(&n, &f.g, &df(&f.g), &mut f.r)
                            .map_err(DiscoveredDiagnosticDrainErrorV1::Drain)
                    }
                };
                assert_eq!(submit(&mut f), Ok(DiagnosticDrainPhaseV1::Pending));
                assert_eq!(submit(&mut f), Ok(DiagnosticDrainPhaseV1::Pending));
                f.run(&task).unwrap();
                assert_eq!(submit(&mut f), Ok(DiagnosticDrainPhaseV1::Terminal));
                assert_eq!(
                    f.o.query_diagnostic_drain_result_v1(&task.task, &f.g, &df(&f.g), &mut f.r)
                        .unwrap()
                        .batch
                        .unwrap()
                        .records,
                    [marker(DiagnosticMilestoneV1::Completed)]
                );
                if let Some(route) = route {
                    route.retire(&mut f.o, &mut f.r);
                }
                let mut normalization = 0;
                let mut dispatch = 0;
                let mut performed = 0;
                while let Some(record) = f.r.pop_front() {
                    match record.boundary {
                        Boundary::DiagnosticDrainArgumentNormalization => {
                            normalization += 1;
                            assert_eq!(record.service_time.is_some(), mode == 2);
                        }
                        Boundary::DiscoveryDiagnosticDrainDispatch => {
                            dispatch += 1;
                            assert_eq!(record.service_time.is_some(), mode == 2);
                        }
                        Boundary::DiagnosticDrainPerformed => performed += 1,
                        _ => {}
                    }
                }
                if mode == 1 || mode == 2 {
                    assert_eq!(normalization, 1);
                    assert_eq!(dispatch, if installed { 3 } else { 0 });
                    assert_eq!(performed, 1);
                }
                f.cleanup(&task);
            }
        }
    }
}

#[test]
fn malformed_facility_input_cannot_admit_a_task_or_touch_the_queue() {
    use DiagnosticDrainEnvelopeValueV1 as V;
    use vellum_engine::capture::InertValueV1 as D;
    let mut f = F::new();
    f.offer(DiagnosticMilestoneV1::Completed);
    let task = f.issue();
    let args = [
        ("contract", V::Data(D::Text("host-diagnostic-drain/1"))),
        ("task", V::Task(&task.task)),
        ("generation", V::Generation(&task.generation)),
        ("limit", V::Data(D::Number(1.0))),
    ];
    for omitted in 0..4 {
        let fields: Vec<_> = args
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (i != omitted).then_some(*v))
            .collect();
        assert!(normalize_diagnostic_drain_arguments_v1(V::Object(&fields), &mut f.r).is_err());
    }
    for extra in [("callback", V::Data(D::Null)), args[0]] {
        let mut fields = args.to_vec();
        fields.push(extra);
        assert!(normalize_diagnostic_drain_arguments_v1(V::Object(&fields), &mut f.r).is_err());
    }
    for limit in [
        0.0,
        -1.0,
        0.5,
        f64::NAN,
        f64::INFINITY,
        9_007_199_254_740_992.0,
    ] {
        let mut fields = args;
        fields[3].1 = V::Data(D::Number(limit));
        assert!(normalize_diagnostic_drain_arguments_v1(V::Object(&fields), &mut f.r).is_err());
    }
    for variant in ["timer", "task-cancellation", "dialog", "unknown"] {
        let header = [
            ("name", V::Data(D::Text("request-host-facility"))),
            ("variant", V::Data(D::Text(variant))),
            ("version", V::Data(D::Number(1.0))),
            ("arguments", V::Object(&args)),
        ];
        let envelope = [
            ("route", V::Data(D::Text("dynamic"))),
            ("request", V::Object(&header)),
        ];
        assert!(normalize_diagnostic_drain_envelope_v1(V::Object(&envelope), &mut f.r).is_err());
    }
    assert_eq!(
        f.o.query_diagnostic_drain_control_v1(&task.task, &dc(&f.g), &mut f.r)
            .unwrap()
            .phase,
        DiagnosticDrainPhaseV1::Issued
    );
    assert_eq!(
        f.o.read_diagnostic_health_v1(&control(&f.q))
            .unwrap()
            .queued_records,
        1
    );
    f.cleanup(&task);
}

#[test]
fn a_foreign_installed_origin_cannot_apply_supplied_bridge_authority_loss() {
    use crate::selection_discovery::Route;
    use DiagnosticDrainEnvelopeValueV1 as V;
    use vellum_engine::capture::InertValueV1 as D;
    let mut f = F::with_headroom(4_194_304);
    let route = Route::install(&mut f.o, &mut f.r);
    let task = f.issue();
    let args = [
        ("contract", V::Data(D::Text("host-diagnostic-drain/1"))),
        ("task", V::Task(&task.task)),
        ("generation", V::Generation(&task.generation)),
        ("limit", V::Data(D::Number(1.0))),
    ];
    let n = normalize_diagnostic_drain_arguments_v1(V::Object(&args), &mut f.r).unwrap();
    let mut lost = df(&f.g);
    lost.authorized = Some(false);
    assert!(matches!(
        route.submit_diagnostic_drain(&mut f.o, &n, &f.g, &lost, &mut f.r),
        Err(DiscoveredDiagnosticDrainErrorV1::Discovery(_))
    ));
    assert_eq!(f.submit(&task, 1), Ok(DiagnosticDrainPhaseV1::Pending));
    f.o.stop_diagnostic_drain_task_v1(&task.task, &dc(&f.g), &mut f.r)
        .unwrap();
    route.retire(&mut f.o, &mut f.r);
    f.cleanup(&task);
}
