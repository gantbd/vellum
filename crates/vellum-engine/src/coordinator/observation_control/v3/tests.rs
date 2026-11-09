use super::*;
pub(in crate::coordinator) fn request<'h, T>(
    target: ObservationStopTargetV3<'h>,
    typed: bool,
    variant: &str,
    operation: &str,
    call: impl FnOnce(&ObservationStopEnvelopeValueV3<'_, 'h>) -> T,
) -> T {
    use ObservationStopEnvelopeValueV3 as V;
    let args = [
        ("contract", V::Data(InertValueV1::Text("read-context/1"))),
        ("operation", V::Data(InertValueV1::Text(operation))),
        ("context", V::Context(target)),
    ];
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(InertValueV1::Number(18.0))
            } else {
                V::Data(InertValueV1::Text("control-observation"))
            },
        ),
        ("variant", V::Data(InertValueV1::Text(variant))),
        ("version", V::Data(InertValueV1::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        (
            "route",
            V::Data(InertValueV1::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ];
    call(&V::Object(&outer))
}

#[test]
fn stop_v3_preserves_existing_owner_routing_and_instrumentation() {
    use crate::coordinator::publication_stream::tests::{facts, limits, native, owner};
    use crate::instrumentation::NativeTimingProfileV1;
    for typed in [false, true] {
        for capacity in [0, 1, 32] {
            let (mut o, mut setup, s) = owner(limits());
            let n = native(&s);
            let opened = o
                .open_source_publication_stream_v1(&s, &facts(&s, &n), &mut setup)
                .unwrap();
            let debt = o.retention_usage();
            o.faulted = true;
            let control = SourceReadContextControlV1 {
                scope: &s,
                owner_control_allowed: Some(true),
                retention_released: None,
                delivery_drained: None,
            };
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            request(
                ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(
                    &opened.stream,
                )),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v3(
                        DecodedObservationStopRequestV3 {
                            envelope,
                            native: ObservationStopPortV3::Existing(
                                ObservationStopPortV2::SourcePublication {
                                    stream: &opened.stream,
                                    control: &control,
                                },
                            ),
                        },
                        &mut r,
                    )
                },
            )
            .unwrap();
            assert_eq!(o.retention_usage(), debt);
            assert!(o.faulted);
            assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 1);
            if capacity == 32 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::ObservationStopFacadeV3
                            && e.service_time.is_some())
                );
                assert!(events.iter().any(|e| e.boundary
                    == Boundary::ObservationStopArgumentNormalizationV3
                    && e.service_time.is_some()));
            }
        }
    }
}
#[test]
fn stop_v3_closed_grammar_rejects_unknown_revoke_and_extra_fields() {
    use crate::coordinator::publication_stream::tests::{facts, limits, native, owner};
    let (mut o, mut r, s) = owner(limits());
    let n = native(&s);
    let opened = o
        .open_source_publication_stream_v1(&s, &facts(&s, &n), &mut r)
        .unwrap();
    for typed in [false, true] {
        let target = ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(
            &opened.stream,
        ));
        request(
            target,
            typed,
            "revoke-observation-grant",
            "revoke-observation-grant",
            |e| assert!(normalize_observation_stop_envelope_v3(e, &mut r).is_err()),
        );
        request(target, typed, "stop-observation", "stop-observation", |e| {
            let ObservationStopEnvelopeValueV3::Object(outer) = e else {
                panic!()
            };
            let mut extra = outer.to_vec();
            extra.push((
                "surprise",
                ObservationStopEnvelopeValueV3::Data(InertValueV1::Null),
            ));
            assert!(
                normalize_observation_stop_envelope_v3(
                    &ObservationStopEnvelopeValueV3::Object(&extra),
                    &mut r
                )
                .is_err()
            );
        });
    }
    assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 0);
}
