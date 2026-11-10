use super::*;
use crate::coordinator::source_query::result_contract::tests::{owner, source_facts};
use crate::instrumentation::NativeTimingProfileV1;
#[test]
fn shared_stop_preserves_read_owner_guards_funding_and_editing_faults() {
    for capacity in [0, 1, 128] {
        for timing in [
            NativeTimingProfileV1::Off,
            NativeTimingProfileV1::ProcessMonotonicV1,
        ] {
            for kind in 0..4 {
                for allowed in [None, Some(false), Some(true)] {
                    for faulted in [false, true] {
                        let mut f = owner();
                        f.r = Recorder::try_with_capacity(capacity)
                            .unwrap()
                            .with_native_timing(timing);
                        let control = SourceReadContextControlV1 {
                            scope: &f.s,
                            owner_control_allowed: allowed,
                            retention_released: None,
                            delivery_drained: Some(false),
                        };
                        let controls = [control];
                        let port = match kind {
                            0 => ReadObservationStopPortV1::Source {
                                context: &f.source,
                                control: &controls[0],
                            },
                            1 => ReadObservationStopPortV1::Annotation {
                                context: &f.combined,
                                control: &controls[0],
                            },
                            2 => ReadObservationStopPortV1::VersionSet {
                                target: VersionSetControlTargetV1::Context(&f.set.context),
                                controls: &controls,
                            },
                            _ => ReadObservationStopPortV1::VersionSet {
                                target: VersionSetControlTargetV1::Member(
                                    &f.set.participants[0].snapshot,
                                ),
                                controls: &controls,
                            },
                        };
                        let funding = f.o.retention_usage();
                        let generation = f.o.generation();
                        f.o.faulted = faulted;
                        let result = f.o.stop_observation_facade_v1(
                            ObservationStopPortV1::Read(port),
                            &mut f.r,
                        );
                        assert_eq!(result.is_ok(), allowed == Some(true));
                        assert_eq!(f.o.is_faulted(), faulted);
                        assert_eq!(f.o.generation(), generation);
                        assert_eq!(f.o.retention_usage(), funding);
                        if !faulted && kind == 0 {
                            assert_eq!(
                                f.o.query_source_read_context(
                                    &f.source,
                                    &f.s,
                                    SourceContextQueryV1::Line { index: 0 },
                                    &source_facts(&f.s),
                                    &mut f.r
                                )
                                .is_some(),
                                allowed != Some(true)
                            );
                        }
                        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                        assert!(events.len() <= capacity);
                        if capacity == 128 {
                            assert!(events.iter().any(|e| e.boundary
                                == Boundary::ObservationStopFacade
                                && e.service_time.is_some()
                                    == (timing == NativeTimingProfileV1::ProcessMonotonicV1)
                                && e.disposition
                                    == if allowed == Some(true) {
                                        Disposition::Completed
                                    } else {
                                        Disposition::Refused
                                    }));
                        }
                    }
                }
            }
        }
    }
}
