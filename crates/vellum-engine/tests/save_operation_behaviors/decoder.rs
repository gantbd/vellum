use super::observation::{control, facts, policy};
use super::*;
#[path = "../support/native_operation_decoder.rs"]
pub(super) mod support_decoder;
fn exercise(
    f: &mut Fixture,
    task: NativeOperationTaskV1<'_>,
    id: &NativeOperationObservationIdV1,
    c: &SettingsExecutionContextV1,
    kind: &str,
) {
    f.owner
        .enable_native_operation_decoder_v1(
            id,
            support_decoder::limits(),
            &control(c),
            &mut f.recorder,
        )
        .unwrap();
    let funded = f.owner.retention_usage();
    let work = f.owner.resource_usage();
    let generation = f.owner.generation();
    let mut g = facts(c);
    if kind == "annotation-load" {
        g.source_readable = None;
        g.disposition_readable = None;
    } else {
        g.read_allowed = None;
    }
    for route in ["typed", "dynamic"] {
        let mut after = None;
        for variant in ["status", "retained-outcome", "minimal-control-status"] {
            let result = support_decoder::envelope(
                route,
                variant,
                task,
                id,
                &c.profile,
                kind,
                None,
                1.0,
                |e| {
                    f.owner.query_decoded_native_operation_v1(
                        id,
                        e,
                        &policy(c),
                        &g,
                        None,
                        &mut f.recorder,
                    )
                },
            )
            .unwrap();
            let NativeOperationDecodedReplyV1::Query(q) = result else {
                panic!("query")
            };
            if variant == "status" {
                after = Some(q.observation().clone());
            }
        }
        let NativeOperationDecodedReplyV1::Waiting(wait) = support_decoder::envelope(
            route,
            "bounded-observation-wait",
            task,
            id,
            &c.profile,
            kind,
            after.as_ref(),
            1.0,
            |e| {
                f.owner.query_decoded_native_operation_v1(
                    id,
                    e,
                    &policy(c),
                    &g,
                    Some(NativeOperationWaitClockV1 { epoch: 1, tick: 1 }),
                    &mut f.recorder,
                )
            },
        )
        .unwrap() else {
            panic!("wait")
        };
        f.owner
            .advance_native_operation_observation_wait_v1(
                id,
                &wait,
                NativeOperationWaitEventV1::Stop,
                &policy(c),
                &g,
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .retire_native_operation_observation_wait_v1(id, &wait, &control(c), &mut f.recorder)
            .unwrap();
    }
    assert_eq!(f.owner.retention_usage(), funded);
    assert_eq!(f.owner.resource_usage(), work);
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn native_save_decoders_use_both_routes_and_all_four_forms_without_execution_or_new_workspace() {
    for annotation in [false, true] {
        let (mut f, task, c) = observation::setup(annotation);
        let id = observation::open(&mut f, &task, &c, observation::obs_limits());
        exercise(
            &mut f,
            NativeOperationTaskV1::Save(&task),
            &id,
            &c,
            if annotation {
                "annotation-save"
            } else {
                "source-save"
            },
        );
    }
}
#[test]
fn annotation_decoder_reuses_original_reader_and_never_borrows_source_or_write_disclosure() {
    let (mut f, task, c) = read_observation::setup();
    let id = read_observation::open(&mut f, &task, &c);
    exercise(
        &mut f,
        NativeOperationTaskV1::AnnotationLoad(&task),
        &id,
        &c,
        "annotation-load",
    );
}
#[test]
fn native_decoder_capture_modes_leave_delivery_and_original_work_unchanged() {
    for capacity in [None, Some(0), Some(128)] {
        let (mut f, task, c) = observation::setup(false);
        let id = observation::open(&mut f, &task, &c, observation::obs_limits());
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        exercise(
            &mut f,
            NativeOperationTaskV1::Save(&task),
            &id,
            &c,
            "source-save",
        );
        let mut events = String::new();
        while let Some(event) = f.recorder.pop_front() {
            events.push_str(&format!("{event:?}"));
        }
        assert!(!events.contains("recipient") && !events.contains("destination"));
        if capacity == Some(128) {
            assert!(
                events.contains("NativeOperationDecoderQuery")
                    && events.contains("NativeOperationArgumentNormalization")
            );
        }
    }
}
