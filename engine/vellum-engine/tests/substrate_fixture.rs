mod support;

use std::time::Duration;
use support::{ScriptedSubstrate, SubstrateFixture, WriteObservation};
use vellum_engine::instrumentation::{Boundary, Disposition, Observation, Recorder};

fn fixture() -> ScriptedSubstrate {
    ScriptedSubstrate {
        bytes: b"English\r\nDeutsch: \xc3\xbc\0\xff".to_vec(),
        fail_read: false,
        next_write: WriteObservation::Completed,
        write_attempts: 0,
    }
}

#[test]
fn fixture_preserves_arbitrary_source_bytes() {
    let source = fixture();
    let capture = source.read().unwrap();
    let mut destination = fixture();
    destination.bytes.clear();
    assert_eq!(destination.write(&capture), WriteObservation::Completed);
    assert_eq!(destination.read().unwrap(), source.bytes);
}

#[test]
fn fixture_injects_read_failure_without_a_write() {
    let mut substrate = fixture();
    substrate.fail_read = true;
    assert_eq!(substrate.read(), Err(()));
    assert_eq!(substrate.write_attempts, 0);
}

#[test]
fn optional_telemetry_does_not_change_scripted_write_result() {
    for outcome in [
        WriteObservation::Completed,
        WriteObservation::Failed,
        WriteObservation::Unknown,
    ] {
        for capacity in [0, 1] {
            let mut substrate = fixture();
            substrate.next_write = outcome;
            let mut recorder = Recorder::try_with_capacity(capacity).unwrap();
            // Saturate capture before the operation. Recording must not control it.
            recorder.record(Observation {
                boundary: Boundary::Admission,
                disposition: Disposition::Completed,
                service_time: Some(Duration::ZERO),
            });
            let result = substrate.write(b"replacement");
            let disposition = match result {
                WriteObservation::Completed => Disposition::Completed,
                WriteObservation::Failed => Disposition::Failed,
                WriteObservation::Unknown => Disposition::Unknown,
            };
            recorder.record(Observation {
                boundary: Boundary::SubstrateWrite,
                disposition,
                service_time: Some(Duration::ZERO),
            });
            assert_eq!(result, outcome);
            assert_eq!(substrate.write_attempts, 1);
        }
    }
}
