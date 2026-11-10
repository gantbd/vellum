use super::*;
use vellum_engine::coordinator::{InvocationMeasurementLimitsV2, InvocationResponsivenessV1};
use vellum_engine::instrumentation::NativeTimingProfileV1;

#[test]
fn given_opt_in_ages_with_off_or_saturated_diagnostics_then_replay_stop_and_publication_stay_identical()
 {
    let mut signatures = vec![];
    for capacity in [0, 1, 128] {
        for profile in 0..4 {
            let mut r = Recorder::try_with_capacity(capacity).unwrap();
            let (mut o, d) = invocation_owner(&mut r);
            match profile {
                0 => {}
                1 => enable_invocation(&mut o, &mut r),
                _ => {
                    let timing = if profile == 2 {
                        NativeTimingProfileV1::Off
                    } else {
                        NativeTimingProfileV1::ProcessMonotonicV1
                    };
                    o.configure_invocation_measurements_v2(
                        InvocationMeasurementLimitsV2 {
                            values: ValueLimitsV1 {
                                max_integer: 100,
                                max_seconds: 0.0,
                            },
                            max_pool_bytes: InvocationMeasurementLimitsV2::required_pool_bytes(
                                16, timing,
                            )
                            .unwrap(),
                            timing,
                        },
                        Some(true),
                        &mut r,
                    )
                    .unwrap();
                }
            }
            let q = source_intent(&o, &d, "A", "CONFIDENTIAL", &mut r);
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit(&id, &q, &mut r).unwrap();
            o.submit(&id, &q, &mut r).unwrap();
            assert_eq!(o.invocation_usage().pending, 1);
            if profile == 3 {
                let InvocationResponsivenessV1::Observed(age) =
                    o.invocation_responsiveness_v1(&mut r).unwrap().unwrap()
                else {
                    panic!("observed cohort");
                };
                assert_eq!((age.admissions, age.pending, age.terminal), (1, 1, 0));
                assert_eq!(age.pending_ages_unknown, 0);
            }
            let stopped = o.issue_invocation(&mut r).unwrap();
            o.submit(&stopped, &q, &mut r).unwrap();
            o.stop_invocation(&stopped, &mut r).unwrap();
            o.run(&stopped, &mut r).unwrap();
            o.run(&id, &mut r).unwrap();
            o.run(&id, &mut r).unwrap();
            let age = o.invocation_responsiveness_v1(&mut r).unwrap();
            match profile {
                0 | 1 => assert_eq!(age, None),
                2 => assert_eq!(age, Some(InvocationResponsivenessV1::Off)),
                _ => {
                    let Some(InvocationResponsivenessV1::Observed(age)) = age else {
                        panic!();
                    };
                    assert_eq!((age.admissions, age.pending, age.terminal), (2, 0, 2));
                    assert_eq!(age.terminal_durations_unknown, 0);
                    assert!(age.max_known_terminal_duration.is_some());
                    assert!(age.population_complete);
                    assert!(!format!("{age:?}").contains("CONFIDENTIAL"));
                }
            }
            if profile != 0 {
                assert_eq!(
                    invocation_totals(o.invocation_measurements_v1(&mut r).unwrap().unwrap()),
                    [1, 0, 0, 1, 0, 0, 2]
                );
            }
            let read = o.read(&d, &mut r).unwrap();
            signatures.push((
                o.generation(),
                read.version.revision,
                read.source.text().to_owned(),
            ));
            assert_eq!(o.invocation_usage().pending, 0);
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
        }
    }
    assert!(signatures.windows(2).all(|p| p[0] == p[1]));
}
