//! Opt-in, deterministic outer-frame and one-exchange qualification.
#![forbid(unsafe_code)]

use std::{env, io::Cursor};
use vellum_protocol::{
    ClientId, RefusalV1, ReplyV1, RequestV1, ServiceAccessErrorV1, ServiceControlErrorV1,
    ServicePort,
};
use vellum_transport::{TransportError, TransportLimits, read_frame, serve_one, write_frame};
use vellum_wire::{WireReplyV1, WireRequestV1, decode_reply_v1, encode_request_v1};

const DEFAULT_SEED: u64 = 0x5645_4c4c_554d_0002;
const DEFAULT_CASES: usize = 2_048;

struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn index(&mut self, upper: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(upper).unwrap()).unwrap()
    }
}

fn setting(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[derive(Default)]
struct CountingService {
    connects: usize,
}

impl ServicePort for CountingService {
    fn connect(&mut self) -> Result<ClientId, ServiceControlErrorV1> {
        self.connects += 1;
        Ok(ClientId::new(u64::try_from(self.connects).unwrap()).unwrap())
    }

    fn execute(&mut self, _: RequestV1) -> Result<ReplyV1, ServiceAccessErrorV1> {
        Ok(ReplyV1::Refused(RefusalV1::Stale))
    }

    fn disconnect(&mut self, _: ClientId) -> Result<(), ServiceControlErrorV1> {
        Ok(())
    }
}

struct Duplex {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl std::io::Read for Duplex {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.input, buffer)
    }
}

impl std::io::Write for Duplex {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
#[ignore = "opt-in deterministic qualification; run make check-fuzz-soak"]
fn deterministic_frame_soak_preserves_bounds_truncation_and_one_exchange() {
    let seed = setting("VELLUM_QUALIFICATION_SEED", DEFAULT_SEED);
    let cases = usize::try_from(setting("VELLUM_TRANSPORT_CASES", DEFAULT_CASES as u64)).unwrap();
    assert!(seed != 0, "qualification seed must be nonzero");
    assert!(
        (1..=1_000_000).contains(&cases),
        "transport case budget is invalid"
    );

    let limits = TransportLimits::reference();
    let bounded_max = limits.max_frame_bytes.min(512);
    let mut generator = Generator(seed);
    let mut service = CountingService::default();
    for case in 0..cases {
        let length = generator.index(bounded_max + 1);
        let mut payload = vec![0_u8; length];
        for byte in &mut payload {
            *byte = generator.next().to_le_bytes()[0];
        }
        let mut framed = Vec::new();
        write_frame(&mut framed, &payload, limits.max_frame_bytes).unwrap();
        assert_eq!(
            read_frame(&mut framed.as_slice(), limits.max_frame_bytes).unwrap(),
            payload,
            "frame round trip changed payload length at case {case}"
        );
        if !framed.is_empty() {
            let end = generator.index(framed.len());
            assert_eq!(
                read_frame(&mut &framed[..end], limits.max_frame_bytes),
                Err(TransportError::Truncated),
                "truncated frame was not closed at case {case}"
            );
        }

        let oversize = u32::try_from(limits.max_frame_bytes + 1)
            .unwrap()
            .to_be_bytes();
        assert_eq!(
            read_frame(&mut oversize.as_slice(), limits.max_frame_bytes),
            Err(TransportError::FrameLimit),
            "oversize declaration was not refused at case {case}"
        );

        let request = encode_request_v1(&WireRequestV1::Connect, limits.wire).unwrap();
        let mut input = Vec::new();
        write_frame(&mut input, &request, limits.max_frame_bytes).unwrap();
        let mut duplex = Duplex {
            input: Cursor::new(input),
            output: Vec::new(),
        };
        let before = service.connects;
        serve_one(&mut service, &mut duplex, limits).unwrap();
        assert_eq!(
            service.connects,
            before + 1,
            "exchange count drift at case {case}"
        );
        let reply = read_frame(&mut duplex.output.as_slice(), limits.max_frame_bytes).unwrap();
        assert_eq!(
            decode_reply_v1(&reply, limits.wire).unwrap(),
            WireReplyV1::Connected(ClientId::new(u64::try_from(before + 1).unwrap()).unwrap()),
            "one-exchange reply drift at case {case}"
        );
    }
}
