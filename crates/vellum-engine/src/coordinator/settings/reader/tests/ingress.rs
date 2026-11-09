use super::*;
use crate::capture::{CaptureLimitsV1, InertValueV1 as D};
use crate::coordinator::settings::reader::ingress::Probe;
use SettingsReadEnvelopeValueV1 as V;
pub(in crate::coordinator) fn input_limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 128,
        max_depth: 8,
    }
}
pub(in crate::coordinator) fn request<'h, T>(
    context: &'h SettingsReadContextIdV1,
    target: &'h SettingsReadTargetRefV1,
    typed: bool,
    operation: &str,
    key: &str,
    consume: impl FnOnce(V<'_, 'h>) -> T,
) -> T {
    let a = [
        ("contract", V::Data(D::Text("configuration-query/1"))),
        ("operation", V::Data(D::Text(operation))),
        ("context", V::Context(context)),
        ("target", V::Target(target)),
        ("profile", V::Data(D::Text(SETTINGS_READ_PROFILE_V1))),
        ("key", V::Data(D::Text(key))),
    ];
    let h = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(D::Number(6.0))
            } else {
                V::Data(D::Text("inspect-context"))
            },
        ),
        ("variant", V::Data(D::Text(operation))),
        ("version", V::Data(D::Number(1.0))),
        (
            "arguments",
            V::Object(&a[..if operation == "capabilities" { 5 } else { 6 }]),
        ),
    ];
    let e = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&h)),
    ];
    consume(V::Object(&e))
}
fn setup() -> (SourceCoordinator, SettingsTargetId, Recorder) {
    let l = limits();
    setup_with_parent(
        l,
        l.max_pool_bytes
            + settings_read_decode_classification_bytes_v1()
            + 7 * input_limits().max_bytes,
    )
}
#[test]
fn both_routes_deliver_all_three_owned_products_without_leaking_temporary_funding() {
    for typed in [false, true] {
        let (mut o, t, mut r) = setup();
        let s = scope();
        let p = cut(&mut o, &t, &s, &mut r);
        let a = o
            .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .unwrap();
        for operation in [
            "effective-configuration",
            "permitted-origin",
            "capabilities",
        ] {
            let p = cut(&mut o, &t, &s, &mut r);
            let before = (
                o.retention_usage(),
                o.settings_reader_usage_v1(),
                o.generation(),
            );
            request(&a.context, &a.target, typed, operation, "flag", |e| {
                let normalized =
                    normalize_settings_read_envelope_v1(&e, &s, input_limits(), &mut r).unwrap();
                assert_eq!(normalized.is_typed_route(), typed);
                assert!(!format!("{normalized:?}").contains("flag"));
                let out = o
                    .query_decoded_settings_read_v1(&e, input_limits(), &s, &p, &facts(), &mut r)
                    .unwrap();
                match (operation, out) {
                    (
                        "effective-configuration",
                        SettingsReadReplyV1::Value {
                            context,
                            target,
                            key,
                            value,
                        },
                    ) => {
                        assert_eq!(value.read().boolean(), Some(false));
                        assert_eq!(context, a.context);
                        assert_eq!(target, a.target);
                        assert_eq!(key, "flag");
                    }
                    (
                        "permitted-origin",
                        SettingsReadReplyV1::Origins {
                            context,
                            target,
                            key,
                            origins: SettingsReadOriginsV1::Whole(ids),
                        },
                    ) => {
                        assert_eq!(ids.len(), 1);
                        assert_eq!(context, a.context);
                        assert_eq!(target, a.target);
                        assert_eq!(key, "flag");
                    }
                    (
                        "capabilities",
                        SettingsReadReplyV1::Capabilities {
                            context,
                            target,
                            capabilities,
                        },
                    ) => {
                        assert_eq!(capabilities.len(), 1);
                        assert_eq!(context, a.context);
                        assert_eq!(target, a.target);
                    }
                    _ => panic!("wrong product"),
                }
            });
            assert_eq!(o.retention_usage(), before.0);
            assert_eq!(o.generation(), before.2);
            assert_eq!(
                o.settings_reader_usage_v1().issued_lifetime,
                before.1.issued_lifetime
            );
            assert_eq!(o.settings_reader_usage_v1().packets + 1, before.1.packets);
            assert_eq!(
                o.settings_read_decoder_usage_v1(),
                SettingsReadDecoderUsageV1::default()
            );
        }
    }
}
#[test]
fn malformed_envelopes_roles_and_profiles_refuse_before_any_loss_or_cut_consumption() {
    let (mut o, t, mut r) = setup();
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let p = cut(&mut o, &t, &s, &mut r);
    let mut loss = facts();
    loss.continuous = Some(false);
    for typed in [false, true] {
        for variant in 0..15 {
            let mut arows = vec![
                ("contract", V::Data(D::Text("configuration-query/1"))),
                ("operation", V::Data(D::Text("effective-configuration"))),
                ("context", V::Context(&a.context)),
                ("target", V::Target(&a.target)),
                ("profile", V::Data(D::Text(SETTINGS_READ_PROFILE_V1))),
                ("key", V::Data(D::Text("flag"))),
            ];
            match variant {
                0 => arows.push(("extra", V::Data(D::Null))),
                1 => {
                    arows.pop();
                }
                2 => arows[2].1 = V::Target(&a.target),
                3 => arows[3].1 = V::Context(&a.context),
                4 => arows[2].1 = V::Data(D::Object(&[])),
                5 => arows[3].1 = V::Data(D::Null),
                6 => arows[0].1 = V::Data(D::Text("another-contract")),
                7 => arows[1].1 = V::Data(D::Text("capabilities")),
                8 => arows[4].1 = V::Data(D::Text("unknown-profile")),
                9 => arows[5].1 = V::Data(D::Text("")),
                10 => arows[5].1 = V::Data(D::Boolean(true)),
                11 => arows[1].0 = "contract",
                _ => {}
            }
            let mut h = [
                (
                    if typed { "family" } else { "name" },
                    if typed {
                        V::Data(D::Number(6.0))
                    } else {
                        V::Data(D::Text("inspect-context"))
                    },
                ),
                ("variant", V::Data(D::Text("effective-configuration"))),
                ("version", V::Data(D::Number(1.0))),
                ("arguments", V::Object(&arows)),
            ];
            if variant == 12 {
                h[0].1 = V::Data(D::Text("6"));
            }
            if variant == 13 {
                h[2].1 = V::Data(D::Number(f64::NAN));
            }
            if variant == 14 {
                h[1].1 = V::Data(D::Text("source"));
            }
            let e = [
                (
                    "route",
                    V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
                ),
                ("request", V::Object(&h)),
            ];
            let before = (o.retention_usage(), o.settings_reader_usage_v1());
            assert!(
                o.query_decoded_settings_read_v1(
                    &V::Object(&e),
                    input_limits(),
                    &s,
                    &p,
                    &loss,
                    &mut r
                )
                .is_err(),
                "variant {variant}"
            );
            assert_eq!((o.retention_usage(), o.settings_reader_usage_v1()), before);
            assert!(!o.settings_read_decoder_usage_v1().faulted);
        }
    }
    request(
        &a.context,
        &a.target,
        true,
        "effective-configuration",
        "unknown-key",
        |e| {
            assert!(
                o.query_decoded_settings_read_v1(&e, input_limits(), &s, &p, &loss, &mut r)
                    .is_err()
            );
        },
    );
    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 1);
}
#[test]
fn decoder_workspace_uses_exact_actual_parent_headroom_and_refunds_every_refusal() {
    let l = limits();
    let b = input_limits();
    let workspace = settings_read_decode_classification_bytes_v1() + 7 * b.max_bytes;
    for delta in [-1isize, 0, 1] {
        let (mut o, t, mut r) = setup_with_parent(
            l,
            l.max_pool_bytes + workspace.checked_add_signed(delta).unwrap(),
        );
        let s = scope();
        let p = cut(&mut o, &t, &s, &mut r);
        let a = o
            .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .unwrap();
        let p = cut(&mut o, &t, &s, &mut r);
        let before = o.retention_usage();
        request(&a.context, &a.target, true, "capabilities", "", |e| {
            assert_eq!(
                o.query_decoded_settings_read_v1(&e, b, &s, &p, &facts(), &mut r)
                    .is_ok(),
                delta >= 0
            );
        });
        assert_eq!(o.retention_usage(), before);
        assert!(!o.settings_read_decoder_usage_v1().busy);
    }
    for b in [
        CaptureLimitsV1 { max_bytes: 0, ..b },
        CaptureLimitsV1 { max_nodes: 0, ..b },
        CaptureLimitsV1 { max_depth: 0, ..b },
        CaptureLimitsV1 {
            max_bytes: usize::MAX,
            ..b
        },
    ] {
        let (mut o, t, mut r) = setup();
        let s = scope();
        let p = cut(&mut o, &t, &s, &mut r);
        let a = o
            .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .unwrap();
        let p = cut(&mut o, &t, &s, &mut r);
        let before = (o.retention_usage(), o.settings_reader_usage_v1());
        request(&a.context, &a.target, false, "capabilities", "", |e| {
            assert!(
                o.query_decoded_settings_read_v1(&e, b, &s, &p, &facts(), &mut r)
                    .is_err()
            )
        });
        assert_eq!((o.retention_usage(), o.settings_reader_usage_v1()), before);
    }
}
#[test]
fn decoder_faults_are_pool_local_absorbing_and_cleanup_survives_all_three_checkpoints() {
    for point in 0..3 {
        for panic in [false, true] {
            let (mut o, t, mut r) = setup();
            let s = scope();
            let p = cut(&mut o, &t, &s, &mut r);
            let a = o
                .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
                .unwrap();
            let p = cut(&mut o, &t, &s, &mut r);
            o.settings_reader_mut().unwrap().decode_probe = Some(Probe { point, panic });
            let before = (o.retention_usage(), o.generation());
            request(&a.context, &a.target, true, "capabilities", "", |e| {
                assert_eq!(
                    o.query_decoded_settings_read_v1(&e, input_limits(), &s, &p, &facts(), &mut r)
                        .is_ok(),
                    !panic
                );
            });
            assert_eq!((o.retention_usage(), o.generation()), before);
            assert!(!o.faulted);
            assert_eq!(
                o.settings_read_decoder_usage_v1(),
                SettingsReadDecoderUsageV1 {
                    busy: false,
                    declared_workspace_bytes: 0,
                    faulted: panic
                }
            );
            if panic {
                assert!(prepare(&mut o, &t, &s, true, true, true, &mut r).is_err());
                assert!(
                    o.acquire_settings_read_v1(&p, &s, &facts(), &mut r)
                        .is_err()
                );
                assert!(
                    o.query_settings_read_v1(
                        &a.context,
                        &a.target,
                        &s,
                        SettingsReadOperationV1::Capabilities,
                        &p,
                        &facts(),
                        &mut r
                    )
                    .is_err()
                );
                assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
                o.discard_settings_read_cut_v1(&p, &control(&s), &mut r)
                    .unwrap();
            } else {
                assert_eq!(
                    o.settings_reader_mut().unwrap().decode_probe_report,
                    Some((true, true, true))
                );
            }
            o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
                .unwrap();
            o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
                .unwrap();
            o.retire_settings_reader_v1(&mut r).unwrap();
            assert_eq!(o.retention_usage().reserved, 0);
            assert!(!o.faulted);
        }
    }
}

#[test]
fn busy_fence_prevents_retiring_an_otherwise_empty_pool_during_decode() {
    let (mut o, t, mut r) = setup();
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
        .unwrap();
    o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
        .unwrap();
    assert_eq!(o.settings_reader_usage_v1().contexts, 0);
    o.settings_reader_mut().unwrap().decode_probe = Some(Probe {
        point: 1,
        panic: false,
    });
    request(&a.context, &a.target, true, "capabilities", "", |e| {
        assert!(
            o.query_decoded_settings_read_v1(&e, input_limits(), &s, &p, &facts(), &mut r)
                .is_err()
        )
    });
    assert_eq!(
        o.settings_reader_mut().unwrap().decode_probe_report,
        Some((true, true, true))
    );
    assert_eq!(
        o.settings_read_decoder_usage_v1(),
        SettingsReadDecoderUsageV1::default()
    );
    o.retire_settings_reader_v1(&mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn normalization_owns_keys_and_enforces_native_byte_node_and_depth_boundaries() {
    let (mut o, t, mut r) = setup();
    let s = scope();
    let p = cut(&mut o, &t, &s, &mut r);
    let a = o
        .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
        .unwrap();
    let mut key = String::from("flag");
    let owned = request(
        &a.context,
        &a.target,
        true,
        "effective-configuration",
        &key,
        |e| normalize_settings_read_envelope_v1(&e, &s, input_limits(), &mut r),
    )
    .unwrap();
    key.clear();
    key.push_str("changed-after-capture");
    assert!(matches!(
        owned.operation(),
        SettingsReadOperationV1::EffectiveConfiguration { key: "flag" }
    ));
    for field in 0..3 {
        let mut low = 0;
        let mut high = match field {
            0 => input_limits().max_bytes,
            1 => input_limits().max_nodes,
            _ => input_limits().max_depth,
        };
        let valid = |bound, r: &mut Recorder| {
            let mut b = input_limits();
            match field {
                0 => b.max_bytes = bound,
                1 => b.max_nodes = bound,
                _ => b.max_depth = bound,
            }
            request(
                &a.context,
                &a.target,
                false,
                "effective-configuration",
                "flag",
                |e| normalize_settings_read_envelope_v1(&e, &s, b, r),
            )
            .is_ok()
        };
        assert!(valid(high, &mut r));
        while low < high {
            let middle = low + (high - low) / 2;
            if valid(middle, &mut r) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(high > 0);
        assert!(!valid(high - 1, &mut r));
        assert!(valid(high, &mut r));
        assert!(valid(high + 1, &mut r));
    }
}

#[test]
fn complete_result_frame_is_bounded_and_reply_refusal_does_not_invalidate_context() {
    let (mut probe, t, mut r) = setup();
    let s = scope();
    let p = cut(&mut probe, &t, &s, &mut r);
    let Some(Some(Row::Cut(c))) = probe.settings_reader_pool().unwrap().rows.get(p.slot) else {
        panic!()
    };
    let full = c.data.logical_bytes + size_of::<SettingsReadReplyV1>() + "flag".len();
    for delta in [-1isize, 0, 1] {
        let mut l = limits();
        l.max_reply_bytes = full.checked_add_signed(delta).unwrap();
        l.max_pool_bytes = l.required_pool_bytes().unwrap();
        let (mut o, t, mut r) = setup_with_parent(
            l,
            l.max_pool_bytes
                + settings_read_decode_classification_bytes_v1()
                + 7 * input_limits().max_bytes,
        );
        let p = cut(&mut o, &t, &s, &mut r);
        let a = o
            .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
            .unwrap();
        let p = cut(&mut o, &t, &s, &mut r);
        let before = o.retention_usage();
        request(
            &a.context,
            &a.target,
            true,
            "effective-configuration",
            "flag",
            |e| {
                assert_eq!(
                    o.query_decoded_settings_read_v1(&e, input_limits(), &s, &p, &facts(), &mut r)
                        .is_ok(),
                    delta >= 0
                )
            },
        );
        assert_eq!(o.retention_usage(), before);
        assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
        let p = cut(&mut o, &t, &s, &mut r);
        assert!(
            o.query_settings_read_v1(
                &a.context,
                &a.target,
                &s,
                SettingsReadOperationV1::Capabilities,
                &p,
                &facts(),
                &mut r
            )
            .is_ok()
        );
    }
}
