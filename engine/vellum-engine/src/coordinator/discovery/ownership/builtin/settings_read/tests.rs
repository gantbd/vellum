use super::*;
use crate::coordinator::discovery::ownership::tests::{
    facts as discovery_facts, limits as discovery_limits,
};
use crate::coordinator::settings::reader::tests::ingress::{input_limits, request};
use crate::coordinator::settings::reader::tests::{
    control, cut, facts, limits, scope, setup_with_parent,
};
use crate::instrumentation::NativeTimingProfileV1;
fn visibility() -> NativeBuiltinEntryVisibilityV1<3> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
            order: Some(i),
            enabled: None,
        }),
    }
}
fn prepare(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_settings_read_discovery_cut_v1(s, &visibility(), &discovery_facts(), r)
            .unwrap();
    }
    let sref = o.discovery_store().unwrap();
    let authority = sref.authority.clone();
    let command = command(
        &sref.engine,
        "F07-native-settings-read-v1",
        sref.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |i| {
        o.prepare_discovery_cut_v1(s, i, &discovery_facts(), r)
    })
    .unwrap()
}
#[test]
fn installed_settings_read_routes_join_all_three_products_and_copied_metadata_has_no_handler() {
    for maximum in [0, 1, 256, 4096] {
        assert!(
            workspace_bytes(maximum).unwrap()
                <= super::super::context::workspace_bytes(maximum).unwrap()
        );
    }
    for installed in [false, true] {
        for typed in [false, true] {
            for mode in 0..4 {
                let mut dl = discovery_limits();
                dl.max_context_bytes = 256;
                dl.max_entries = 1;
                dl.max_variants = 3;
                dl.max_row_bytes = 65536;
                dl.max_capture_bytes = 131072;
                dl.max_packet_bytes = 131072;
                dl.max_reply_bytes = 131072;
                dl.max_issued = 128;
                dl.max_selectors = 32;
                dl.metadata.max_bytes = 32768;
                dl.metadata.max_nodes = 2048;
                dl.metadata.max_depth = 12;
                dl.max_held_bytes = dl.declared_bytes().unwrap();
                let rl = limits();
                let temporary =
                    settings_read_decode_classification_bytes_v1() + 7 * input_limits().max_bytes;
                let (mut o, t, mut r) =
                    setup_with_parent(rl, rl.max_pool_bytes + dl.max_held_bytes + temporary);
                let s = scope();
                let ds = DiscoveryScopeV1 {
                    context: SettingsExecutionContextV1 {
                        profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                        ..s.execution.clone()
                    },
                    policy_epoch: 1,
                };
                o.configure_discovery_v1(&ds, dl, &discovery_facts(), &mut r)
                    .unwrap();
                let p = cut(&mut o, &t, &s, &mut r);
                let a = o
                    .acquire_settings_read_v1(&p, &s, &facts(), &mut r)
                    .unwrap();
                let p = prepare(&mut o, &ds, installed, &mut r);
                let dc = o
                    .acquire_discovery_v1(&ds, &p, &discovery_facts(), &mut r)
                    .unwrap()
                    .context;
                let p = prepare(&mut o, &ds, installed, &mut r);
                let DiscoveryReplyV1::Page { mut entries, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &ds,
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                        &discovery_facts(),
                        &mut r,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                let entry = entries.remove(0).reference;
                let p = prepare(&mut o, &ds, installed, &mut r);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &ds,
                        &p,
                        DiscoveryQueryV1::Inspect { entry: &entry },
                        &discovery_facts(),
                        &mut r,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                let variants: Vec<_> = descriptor
                    .variants
                    .into_iter()
                    .map(|v| v.reference)
                    .collect();
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                for (i, operation) in [
                    "effective-configuration",
                    "permitted-origin",
                    "capabilities",
                ]
                .into_iter()
                .enumerate()
                {
                    let p = prepare(&mut o, &ds, installed, &mut r);
                    let current = cut(&mut o, &t, &s, &mut r);
                    let before = (
                        o.retention_usage(),
                        o.generation(),
                        o.settings_reader_usage_v1().issued_lifetime,
                    );
                    let df = discovery_facts();
                    let result = request(&a.context, &a.target, typed, operation, "flag", |e| {
                        o.query_discovered_settings_read_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &ds,
                                cut: &p,
                                entry: &entry,
                                variant: &variants[i],
                                facts: &df,
                            },
                            &e,
                            input_limits(),
                            &s,
                            &current,
                            &facts(),
                            &mut r,
                        )
                    });
                    assert_eq!(result.is_ok(), installed);
                    if let Ok(out) = result {
                        match (i, out) {
                            (
                                0,
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
                                1,
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
                                2,
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
                            _ => panic!("wrong settings product"),
                        }
                    } else {
                        o.discard_settings_read_cut_v1(&current, &control(&s), &mut r)
                            .unwrap();
                        o.retire_discovery_cut_v1(&p, &df, &mut r).unwrap();
                    }
                    assert_eq!(
                        (
                            o.retention_usage(),
                            o.generation(),
                            o.settings_reader_usage_v1().issued_lifetime
                        ),
                        before
                    );
                }
                if installed {
                    // Wrong operation selectors cannot consume a native cut or attribute loss.
                    let p = prepare(&mut o, &ds, true, &mut r);
                    let current = cut(&mut o, &t, &s, &mut r);
                    let mut loss = facts();
                    loss.authorized = Some(false);
                    let df = discovery_facts();
                    let result = request(&a.context, &a.target, typed, "capabilities", "", |e| {
                        o.query_discovered_settings_read_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &ds,
                                cut: &p,
                                entry: &entry,
                                variant: &variants[0],
                                facts: &df,
                            },
                            &e,
                            input_limits(),
                            &s,
                            &current,
                            &loss,
                            &mut r,
                        )
                    });
                    assert!(result.is_err());
                    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
                    o.retire_discovery_cut_v1(&p, &df, &mut r).unwrap();
                    let p = prepare(&mut o, &ds, true, &mut r);
                    let mut denied = df;
                    denied.authorized = Some(false);
                    let result = request(&a.context, &a.target, typed, "capabilities", "", |e| {
                        o.query_discovered_settings_read_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &ds,
                                cut: &p,
                                entry: &entry,
                                variant: &variants[2],
                                facts: &denied,
                            },
                            &e,
                            input_limits(),
                            &s,
                            &current,
                            &loss,
                            &mut r,
                        )
                    });
                    assert!(result.is_err());
                    assert_eq!(o.settings_reader_usage_v1().ended_contexts, 0);
                    // Discovery loss does not revoke separately current native release.
                    request(&a.context, &a.target, typed, "capabilities", "", |e| {
                        o.query_decoded_settings_read_v1(
                            &e,
                            input_limits(),
                            &s,
                            &current,
                            &facts(),
                            &mut r,
                        )
                    })
                    .unwrap();
                    o.retire_discovery_cut_v1(&p, &df, &mut r).unwrap();
                }
                if mode == 3 {
                    assert!(r.dropped() > 0);
                }
                if mode == 1 || mode == 2 {
                    let mut seen = Vec::new();
                    while let Some(record) = r.pop_front() {
                        if [
                            Boundary::SettingsReadArgumentNormalization,
                            Boundary::SettingsReadDecodedDispatch,
                            Boundary::DiscoverySettingsReadDispatch,
                            Boundary::SettingsReadRelease,
                        ]
                        .contains(&record.boundary)
                        {
                            assert_eq!(record.service_time.is_some(), mode == 2);
                            seen.push(record.boundary);
                        }
                    }
                    assert!(seen.contains(&Boundary::DiscoverySettingsReadDispatch));
                    if installed {
                        for boundary in [
                            Boundary::SettingsReadArgumentNormalization,
                            Boundary::SettingsReadDecodedDispatch,
                            Boundary::SettingsReadRelease,
                        ] {
                            assert!(seen.contains(&boundary));
                        }
                    }
                }
                o.stop_settings_read_v1(&a.context, &control(&s), &mut r)
                    .unwrap();
                o.retire_settings_read_v1(&a.context, &control(&s), &mut r)
                    .unwrap();
                o.retire_settings_reader_v1(&mut r).unwrap();
                o.stop_discovery_context_v1(&dc, &ds, &discovery_facts(), &mut r)
                    .unwrap();
                o.retire_discovery_context_v1(&dc, &discovery_facts(), &mut r)
                    .unwrap();
                o.stop_discovery_admission_v1(&discovery_facts(), &mut r)
                    .unwrap();
                o.retire_discovery_registry_v1(&discovery_facts(), &mut r)
                    .unwrap();
                assert_eq!(o.retention_usage().reserved, 0);
            }
        }
    }
}
