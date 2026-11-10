use super::*;
use vellum_engine::resource::{ProviderPort, SaveDispatch, SaveId};

fn fixture() -> (
    SourceCoordinator,
    Recorder,
    ProviderPort,
    vellum_engine::coordinator::DocumentVersion,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let p = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    let d = o
        .load_resource(p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    (o, r, p, d)
}
fn complete(
    o: &mut SourceCoordinator,
    p: &ProviderPort,
    dispatch: SaveDispatch,
    version: &[u8],
    r: &mut Recorder,
) {
    for e in [
        p.result(
            &dispatch,
            Application::Written,
            dispatch.bytes().len(),
            Synchronization::NotRequested,
            Some(version),
        )
        .unwrap(),
        p.no_future_effects(&dispatch).unwrap(),
        p.quiescence(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, r).unwrap();
    }
}
fn launch(
    o: &mut SourceCoordinator,
    p: &ProviderPort,
    id: &SaveId,
    version: &[u8],
    r: &mut Recorder,
) -> SaveDispatch {
    o.dispatch_save(id, p.read_result(b"key", version, b"A").unwrap(), r)
        .unwrap()
}
#[test]
fn queued_original_baseline_origin_survives_a_newer_published_baseline_until_the_consumer_stops() {
    let (mut o, mut r, p, d) = fixture();
    let first = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = launch(&mut o, &p, &first, b"v0", &mut r);
    complete(&mut o, &p, dispatch, b"v1", &mut r);
    o.publish_save_baseline(&first, &mut r).unwrap();
    let second = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let consumer = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = launch(&mut o, &p, &second, b"v1", &mut r);
    complete(&mut o, &p, dispatch, b"v2", &mut r);
    o.publish_save_baseline(&second, &mut r).unwrap();
    let before = o.resource_usage();
    let generation = o.generation();
    assert_eq!(o.retire_save(&first, &mut r), Err(ResourceError::Retained));
    assert_eq!(o.resource_usage(), before);
    assert_eq!(o.generation(), generation);
    o.stop_save(&consumer, &mut r).unwrap();
    o.retire_save(&first, &mut r).unwrap();
    assert_eq!(o.retire_save(&second, &mut r), Err(ResourceError::Retained));
}

#[test]
fn a_loaded_anchor_keeps_intermediate_written_records_even_without_an_exact_origin_reference() {
    let (mut o, mut r, p, d) = fixture();
    let first = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let second = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let consumer = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = launch(&mut o, &p, &first, b"v0", &mut r);
    complete(&mut o, &p, dispatch, b"v1", &mut r);
    o.publish_save_baseline(&first, &mut r).unwrap();
    // This trusted scripted probe attests the original condition again. Pinned
    // best-effort providers do NOT promise non-reused tokens or own continuity.
    let dispatch = launch(&mut o, &p, &second, b"v0", &mut r);
    complete(&mut o, &p, dispatch, b"v2", &mut r);
    o.publish_save_baseline(&second, &mut r).unwrap();
    assert_eq!(o.retire_save(&first, &mut r), Err(ResourceError::Retained));
    o.stop_save(&consumer, &mut r).unwrap();
    o.retire_save(&first, &mut r).unwrap();
    assert_eq!(o.resource_usage().issued_lifetime, 3);
}

#[test]
fn a_newer_queued_anchor_does_not_pin_older_written_records_or_refresh_its_original_condition() {
    let (mut o, mut r, p, d) = fixture();
    let first = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = launch(&mut o, &p, &first, b"v0", &mut r);
    complete(&mut o, &p, dispatch, b"v1", &mut r);
    o.publish_save_baseline(&first, &mut r).unwrap();
    let second = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    let dispatch = launch(&mut o, &p, &second, b"v1", &mut r);
    complete(&mut o, &p, dispatch, b"v2", &mut r);
    o.publish_save_baseline(&second, &mut r).unwrap();
    let consumer = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
    o.retire_save(&first, &mut r).unwrap();
    assert_eq!(o.retire_save(&second, &mut r), Err(ResourceError::Retained));
    let dispatch = launch(&mut o, &p, &consumer, b"v2", &mut r);
    assert_eq!(dispatch.expected_validator(), Some(b"v2".as_slice()));
    assert_eq!(dispatch.expected_bytes(), Some(b"A".as_slice()));
    assert_eq!(dispatch.bytes(), b"A");
}

#[test]
fn producer_retirement_capture_modes_are_bounded_private_and_cannot_discharge_a_queued_consumer() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, mut r, p, d) = fixture();
        let first = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let second = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let consumer = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let dispatch = launch(&mut o, &p, &first, b"v0", &mut r);
        complete(&mut o, &p, dispatch, b"v1", &mut r);
        let dispatch = launch(&mut o, &p, &second, b"v0", &mut r);
        complete(&mut o, &p, dispatch, b"v2", &mut r);
        o.publish_save_baseline(&second, &mut r).unwrap();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        assert_eq!(o.retire_save(&first, &mut r), Err(ResourceError::Retained));
        o.stop_save(&consumer, &mut r).unwrap();
        o.retire_save(&first, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(o.resource_usage().issued_lifetime, 3);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{events:?}").contains("v0"));
        if capacity == Some(128) {
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SaveProducerRetirement
                        && e.disposition == vellum_engine::instrumentation::Disposition::Refused)
            );
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SaveProducerRetirement
                        && e.disposition == vellum_engine::instrumentation::Disposition::Completed)
            );
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
