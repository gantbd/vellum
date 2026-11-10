//! Opt-in release experiment: first receipt closure, not repeat-stop latency.
//! Setup, oracle, diagnostic drain and owner drop are outside each sample.
use super::*;
use std::time::Instant;

macro_rules! cohort {
    ($label:literal, $setup:ident, $role:ident, $control:ident) => {{
        for revoke in [false, true] {
            for typed in [false, true] {
                for mode in ["off", "capture", "timed", "saturated"] {
                    let mut samples = Vec::with_capacity(128);
                    let mut losses = 0u64;
                    for iteration in 0..144 {
                        let (mut o, mut setup, s, command, one, two) = $setup();
                        let control = $control {
                            scope: &s,
                            owner_control_allowed: Some(true),
                            retention_released: None,
                            delivery_drained: None,
                        };
                        let target = ObservationStopTargetV4::$role {
                            command: &command,
                            context: &one,
                        };
                        let authority = TrustedObservationRevocationV2 {
                            original_target: target,
                            revocation_allowed: Some(true),
                        };
                        let mut recorder = match mode {
                            "off" => Recorder::disabled(),
                            "capture" => Recorder::try_with_capacity(32).unwrap(),
                            "timed" => Recorder::try_with_capacity(32)
                                .unwrap()
                                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                            "saturated" => {
                                let mut r = Recorder::try_with_capacity(1).unwrap();
                                r.record(Observation {
                                    boundary: Boundary::Query,
                                    disposition: Disposition::Completed,
                                    service_time: None,
                                });
                                r
                            }
                            _ => unreachable!(),
                        };
                        let debt = o.retention_usage();
                        let operation = if revoke {
                            "revoke-observation-grant"
                        } else {
                            "stop-observation"
                        };
                        let elapsed = request(target, typed, operation, |envelope| {
                            let port = ObservationStopPortV4::$role {
                                command: &command,
                                context: &one,
                                control: &control,
                            };
                            let start = Instant::now();
                            if revoke {
                                o.revoke_decoded_observation_context_v2(
                                    envelope,
                                    port,
                                    &authority,
                                    &mut recorder,
                                )
                                .unwrap();
                            } else {
                                o.stop_decoded_observation_facade_v4(
                                    DecodedObservationStopRequestV4 {
                                        envelope,
                                        native: port,
                                    },
                                    &mut recorder,
                                )
                                .unwrap();
                            }
                            start.elapsed().as_nanos()
                        });
                        // First closure is real; sibling and retained debt survive.
                        assert_eq!(o.retention_usage(), debt);
                        verify_family!($role, o, one, two, command, s, setup);
                        let mut native_completions = 0;
                        while let Some(event) = recorder.pop_front() {
                            if event.boundary == Boundary::ObservationStopFacadeV4 {
                                assert_eq!(event.disposition, Disposition::Completed);
                                assert_eq!(event.service_time.is_some(), mode == "timed");
                                native_completions += 1;
                            }
                        }
                        assert_eq!(
                            native_completions,
                            usize::from(matches!(mode, "capture" | "timed"))
                        );
                        if mode == "saturated" {
                            assert!(recorder.dropped() > 0);
                        }
                        if mode == "off" {
                            assert_eq!(recorder.dropped(), 0);
                        }
                        if iteration >= 16 {
                            samples.push(elapsed);
                            losses += recorder.dropped();
                        }
                    }
                    samples.sort_unstable();
                    println!(
                        "{},{},{},{},128,{},{},{},{},{},{}",
                        $label,
                        if revoke { "revoke" } else { "stop" },
                        if typed { "typed" } else { "dynamic" },
                        mode,
                        samples[0],
                        samples[63],
                        samples[121],
                        samples[126],
                        samples[127],
                        losses
                    );
                }
            }
        }
    }};
}
macro_rules! verify_family {
    (SourceReceipt, $o:ident, $one:ident, $two:ident, $c:ident, $s:ident, $r:ident) => {
        assert_eq!($o.source_publication_receipt_usage_v1().unwrap().stopped, 1);
        assert!(
            $o.source_publication_receipt_basis_v1(&$one, &$c, &$s, &mut $r)
                .is_err()
        );
        assert!(
            $o.source_publication_receipt_basis_v1(&$two, &$c, &$s, &mut $r)
                .is_ok()
        );
    };
    (FacetReceipt, $o:ident, $one:ident, $two:ident, $c:ident, $s:ident, $r:ident) => {
        assert_eq!($o.facet_publication_receipt_usage_v1().unwrap().stopped, 1);
        assert!(
            $o.facet_publication_receipt_basis_v1(&$one, &$c, &$s, &mut $r)
                .is_err()
        );
        assert!(
            $o.facet_publication_receipt_basis_v1(&$two, &$c, &$s, &mut $r)
                .is_ok()
        );
    };
    (CollectionReceipt, $o:ident, $one:ident, $two:ident, $c:ident, $s:ident, $r:ident) => {
        assert_eq!(
            $o.collection_publication_receipt_usage_v1()
                .unwrap()
                .stopped,
            1
        );
        assert!(
            $o.collection_publication_receipt_basis_v1(&$one, &$c, &$s, &mut $r)
                .is_err()
        );
        assert!(
            $o.collection_publication_receipt_basis_v1(&$two, &$c, &$s, &mut $r)
                .is_ok()
        );
    };
}

#[test]
#[ignore = "explicit release performance experiment; no invented latency SLO"]
fn receipt_control_release_workload() -> Result<(), &'static str> {
    if cfg!(debug_assertions) {
        return Err("run this experiment with --release");
    }
    println!("family,operation,route,capture,samples,min_ns,p50_ns,p95_ns,p99_ns,max_ns,dropped");
    cohort!("source", source, SourceReceipt, SourceReadContextControlV1);
    cohort!("facet", facet, FacetReceipt, FacetPublicationControlV1);
    cohort!(
        "collection",
        collection,
        CollectionReceipt,
        CollectionPublicationControlV1
    );
    Ok(())
}
