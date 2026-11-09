use super::*;
use crate::capture::InertValueV1;
use crate::coordinator::observation_control::request_v3 as envelope;
use crate::coordinator::publication_stream::tests::{facts, limits, native, owner};
use crate::instrumentation::NativeTimingProfileV1;

#[test]
fn decoded_revocation_typed_and_dynamic_routes_share_native_authority_and_absorbing_result() {
    for typed in [false, true] {
        for capacity in [0, 1, 64] {
            let (mut o, mut setup, scope) = owner(limits());
            let n = native(&scope);
            let stream = o
                .open_source_publication_stream_v1(&scope, &facts(&scope, &n), &mut setup)
                .unwrap()
                .stream;
            let target = ObservationStopTargetV3::Existing(
                ObservationStopTargetV2::SourcePublication(&stream),
            );
            let control = SourceReadContextControlV1 {
                scope: &scope,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            let port = || {
                ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                    stream: &stream,
                    control: &control,
                })
            };
            let mut authority = TrustedObservationRevocationV1 {
                original_target: target,
                revocation_allowed: None,
            };
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            let debt = o.retention_usage();
            envelope(
                target,
                typed,
                "revoke-observation-grant",
                "revoke-observation-grant",
                |e| {
                    assert_eq!(
                        normalize_observation_revocation_v1(e, &mut setup)
                            .unwrap()
                            .target(),
                        target
                    );
                    assert_eq!(
                        o.revoke_decoded_observation_context_v1(e, port(), &authority, &mut r),
                        Err(ObservationRevocationIngressErrorV1::Execution(
                            ObservationRevocationErrorV1::Unavailable
                        ))
                    );
                    assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 0);
                    authority.revocation_allowed = Some(true);
                    for _ in 0..2 {
                        o.revoke_decoded_observation_context_v1(e, port(), &authority, &mut r)
                            .unwrap();
                    }
                },
            );
            assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 1);
            assert_eq!(o.retention_usage(), debt);
            if capacity == 64 {
                let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(records.iter().any(|e| e.boundary
                    == Boundary::ObservationRevocationArgumentNormalization
                    && e.service_time.is_some()));
                assert!(
                    records
                        .iter()
                        .any(|e| e.boundary == Boundary::ObservationContextRevocation
                            && e.disposition == Disposition::Completed
                            && e.service_time.is_some())
                );
            }
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
            if capacity == 0 {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}

#[test]
fn closed_revocation_ingress_rejects_mutated_fields_without_native_control() {
    use ObservationRevocationEnvelopeValueV1 as V;
    for typed in [false, true] {
        let (mut o, mut setup, scope) = owner(limits());
        let n = native(&scope);
        let stream = o
            .open_source_publication_stream_v1(&scope, &facts(&scope, &n), &mut setup)
            .unwrap()
            .stream;
        let target =
            ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(&stream));
        let authority = TrustedObservationRevocationV1 {
            original_target: target,
            revocation_allowed: Some(true),
        };
        let control = SourceReadContextControlV1 {
            scope: &scope,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        let before = o.source_publication_usage_v1().unwrap();
        let mut refused = |e: &V<'_, '_>| {
            let mut r = Recorder::try_with_capacity(64)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            assert_eq!(
                o.revoke_decoded_observation_context_v1(
                    e,
                    ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                        stream: &stream,
                        control: &control
                    }),
                    &authority,
                    &mut r
                ),
                Err(ObservationRevocationIngressErrorV1::InvalidRequest)
            );
            assert_eq!(o.source_publication_usage_v1().unwrap(), before);
            let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(
                !records
                    .iter()
                    .any(|e| e.boundary == Boundary::ObservationStopFacadeV3)
            );
            assert!(
                records
                    .iter()
                    .any(|e| e.boundary == Boundary::ObservationContextRevocation
                        && e.service_time.is_none())
            );
        };
        for (variant, operation) in [
            ("stop-observation", "stop-observation"),
            ("revoke-observation-grant", "stop-observation"),
            ("unknown", "revoke-observation-grant"),
        ] {
            envelope(target, typed, variant, operation, &mut refused);
        }
        envelope(
            target,
            typed,
            "revoke-observation-grant",
            "revoke-observation-grant",
            |e| {
                let V::Object(outer) = e else { panic!() };
                let V::Object(header) = outer[1].1 else {
                    panic!()
                };
                let V::Object(args) = header[3].1 else {
                    panic!()
                };
                for replacement in [
                    V::Data(InertValueV1::Null),
                    V::Data(InertValueV1::Text("forged")),
                ] {
                    refused(&replacement);
                }
                for level in 0..3 {
                    let original = match level {
                        0 => outer,
                        1 => header,
                        _ => args,
                    };
                    for index in 0..original.len() {
                        let mut rows = original.to_vec();
                        rows[index].1 = V::Data(InertValueV1::Null);
                        let mut h = header.to_vec();
                        let mut out = outer.to_vec();
                        match level {
                            0 => refused(&V::Object(&rows)),
                            1 => {
                                out[1].1 = V::Object(&rows);
                                refused(&V::Object(&out));
                            }
                            _ => {
                                h[3].1 = V::Object(&rows);
                                out[1].1 = V::Object(&h);
                                refused(&V::Object(&out));
                            }
                        }
                    }
                    for change in 0..3 {
                        let mut rows = original.to_vec();
                        match change {
                            0 => {
                                rows.pop();
                            }
                            1 => rows.push(rows[0]),
                            _ => rows.push(("extra", V::Data(InertValueV1::Null))),
                        }
                        let mut h = header.to_vec();
                        let mut out = outer.to_vec();
                        match level {
                            0 => refused(&V::Object(&rows)),
                            1 => {
                                out[1].1 = V::Object(&rows);
                                refused(&V::Object(&out));
                            }
                            _ => {
                                h[3].1 = V::Object(&rows);
                                out[1].1 = V::Object(&h);
                                refused(&V::Object(&out));
                            }
                        }
                    }
                }
            },
        );
    }
}

#[test]
fn exact_context_revocation_requires_separate_authority_and_preserves_siblings_and_debt() {
    for capacity in [0, 1, 64] {
        let (mut o, mut setup, scope) = owner(limits());
        let n = native(&scope);
        let f = facts(&scope, &n);
        let first = o
            .open_source_publication_stream_v1(&scope, &f, &mut setup)
            .unwrap();
        let sibling = o
            .open_source_publication_stream_v1(&scope, &f, &mut setup)
            .unwrap();
        let current = o.read(&scope.document, &mut setup).unwrap().version;
        let edit = o
            .prepare(
                &[DocumentEdits {
                    expected: current,
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "A".into(),
                        inserted: "B".into(),
                    }],
                }],
                &[],
                &mut setup,
            )
            .unwrap();
        o.commit(edit, &mut setup).unwrap();
        o.sample_source_publication_stream_v1(&first.stream, &scope, &f, &mut setup)
            .unwrap();
        assert_eq!(o.source_publication_usage_v1().unwrap().queued, 1);
        let control = SourceReadContextControlV1 {
            scope: &scope,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        let target = ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(
            &first.stream,
        ));
        let other = ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(
            &sibling.stream,
        ));
        let port = || {
            ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                stream: &first.stream,
                control: &control,
            })
        };
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let before = o.source_publication_usage_v1().unwrap();
        let debt = o.retention_usage();
        for allowed in [None, Some(false)] {
            assert_eq!(
                o.revoke_observation_context_v1(
                    target,
                    port(),
                    &TrustedObservationRevocationV1 {
                        original_target: target,
                        revocation_allowed: allowed
                    },
                    &mut r
                ),
                Err(ObservationRevocationErrorV1::Unavailable)
            );
            assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        }
        let foreign = TrustedObservationRevocationV1 {
            original_target: other,
            revocation_allowed: Some(true),
        };
        assert_eq!(
            o.revoke_observation_context_v1(target, port(), &foreign, &mut r),
            Err(ObservationRevocationErrorV1::Unavailable)
        );
        let authority = TrustedObservationRevocationV1 {
            original_target: target,
            revocation_allowed: Some(true),
        };
        assert_eq!(
            o.revoke_observation_context_v1(other, port(), &authority, &mut r),
            Err(ObservationRevocationErrorV1::Unavailable)
        );
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        for _ in 0..2 {
            o.revoke_observation_context_v1(target, port(), &authority, &mut r)
                .unwrap();
        }
        assert!(
            o.release_source_publication_event_v1(&first.stream, &scope, &f, &mut setup)
                .is_err()
        );
        assert!(matches!(
            o.release_source_publication_event_v1(&sibling.stream, &scope, &f, &mut setup),
            Ok(SourcePublicationDeliveryV1::Empty)
        ));
        assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 1);
        assert_eq!(o.retention_usage(), debt);
        o.retire_source_publication_stream_v1(&first.stream, &control, &mut setup)
            .unwrap();
        assert!(
            o.revoke_observation_context_v1(target, port(), &authority, &mut r)
                .is_err()
        );
        let regrant = o
            .open_source_publication_stream_v1(&scope, &f, &mut setup)
            .unwrap();
        assert_ne!(regrant.stream, first.stream);
        assert!(
            o.release_source_publication_event_v1(&first.stream, &scope, &f, &mut setup)
                .is_err()
        );
        if capacity == 64 {
            assert!(
                std::iter::from_fn(|| r.pop_front())
                    .any(|e| e.boundary == Boundary::ObservationContextRevocation
                        && e.disposition == Disposition::Completed
                        && e.service_time.is_some())
            );
        }
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 0 {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn revocation_does_not_bypass_native_scope_control_or_heal_editing_fault() {
    let (mut o, mut r, scope) = owner(limits());
    let n = native(&scope);
    let opened = o
        .open_source_publication_stream_v1(&scope, &facts(&scope, &n), &mut r)
        .unwrap();
    let target = ObservationStopTargetV3::Existing(ObservationStopTargetV2::SourcePublication(
        &opened.stream,
    ));
    let authority = TrustedObservationRevocationV1 {
        original_target: target,
        revocation_allowed: Some(true),
    };
    let mut control = SourceReadContextControlV1 {
        scope: &scope,
        owner_control_allowed: Some(false),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert!(
        o.revoke_observation_context_v1(
            target,
            ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
                stream: &opened.stream,
                control: &control
            }),
            &authority,
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 0);
    control.owner_control_allowed = Some(true);
    o.faulted = true;
    o.revoke_observation_context_v1(
        target,
        ObservationStopPortV3::Existing(ObservationStopPortV2::SourcePublication {
            stream: &opened.stream,
            control: &control,
        }),
        &authority,
        &mut r,
    )
    .unwrap();
    assert!(o.faulted);
}
