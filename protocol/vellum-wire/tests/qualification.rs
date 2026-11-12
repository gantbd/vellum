//! Opt-in, deterministic decoder mutation qualification.
#![forbid(unsafe_code)]

use std::{env, panic::catch_unwind, sync::Arc};
use vellum_protocol::{
    ClientId, CommandV1, MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1, OutcomeV1, RefusalV1, ReplyV1,
    RequestId, RequestV1, SnapshotV1, VersionWitness, WorkspaceChangeCursorV1,
    WorkspaceChangeGapV1, WorkspaceChangesV1, WorkspaceIdV1, WorkspaceInventoryCursorV1,
};
use vellum_wire::{
    WireLimits, WireReplyV1, WireRequestV1, decode_reply_v1, decode_request_v1, encode_reply_v1,
    encode_request_v1,
};

const DEFAULT_SEED: u64 = 0x5645_4c4c_554d_0001;
const DEFAULT_CASES: usize = 4_096;

#[derive(Clone, Copy)]
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

fn limits() -> WireLimits {
    WireLimits {
        max_frame_bytes: 256,
        max_text_bytes: 64,
        max_fields: 16,
    }
}

fn request_corpus() -> Vec<Vec<u8>> {
    let client = ClientId::new(7).unwrap();
    let request = |id, command| {
        WireRequestV1::Execute(RequestV1 {
            client,
            request: RequestId::new(id).unwrap(),
            command,
        })
    };
    let workspace = WorkspaceIdV1::new(11).unwrap();
    [
        WireRequestV1::Connect,
        WireRequestV1::Disconnect(client),
        request(1, CommandV1::Snapshot),
        request(
            2,
            CommandV1::Replace {
                expected: VersionWitness::new(3).unwrap(),
                start: 0,
                end: 1,
                inserted: "e\u{301}".into(),
            },
        ),
        request(
            3,
            CommandV1::WorkspaceInventoryPage {
                cursor: None,
                page_size: MAX_WORKSPACE_INVENTORY_PAGE_SIZE_V1,
            },
        ),
        request(
            4,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(WorkspaceInventoryCursorV1::new(workspace, 5, 2).unwrap()),
                page_size: 1,
            },
        ),
        request(
            5,
            CommandV1::WorkspaceChanges {
                after: Some(WorkspaceChangeCursorV1::new(workspace, 3, 2).unwrap()),
                page_size: 1,
            },
        ),
    ]
    .iter()
    .map(|request| encode_request_v1(request, limits()).unwrap())
    .collect()
}

fn reply_corpus() -> Vec<Vec<u8>> {
    let client = ClientId::new(7).unwrap();
    [
        WireReplyV1::Connected(client),
        WireReplyV1::ConnectRefused(RefusalV1::ClientCapacity),
        WireReplyV1::Executed(ReplyV1::Refused(RefusalV1::StaleInventoryCursor)),
        WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(
            WorkspaceChangesV1::Gap(
                WorkspaceChangeGapV1::new(WorkspaceIdV1::new(11).unwrap(), 3, 1, 3, 4).unwrap(),
            ),
        ))),
        WireReplyV1::Executed(ReplyV1::Accepted(OutcomeV1::Snapshot(SnapshotV1 {
            witness: VersionWitness::new(3).unwrap(),
            revision: 8,
            text: Arc::from("a\0\u{10ffff}"),
        }))),
        WireReplyV1::Disconnected,
    ]
    .iter()
    .map(|reply| encode_reply_v1(reply, limits()).unwrap())
    .collect()
}

fn mutate(generator: &mut Generator, source: &[u8]) -> Vec<u8> {
    let mut candidate = source.to_vec();
    match generator.index(5) {
        0 if !candidate.is_empty() => {
            let index = generator.index(candidate.len());
            candidate[index] ^= 1 << generator.index(8);
        }
        1 => candidate.truncate(generator.index(candidate.len() + 1)),
        2 if candidate.len() < limits().max_frame_bytes => {
            let index = generator.index(candidate.len() + 1);
            candidate.insert(index, generator.next().to_le_bytes()[0]);
        }
        3 if !candidate.is_empty() => {
            let index = generator.index(candidate.len());
            candidate[index] = generator.next().to_le_bytes()[0];
        }
        _ if candidate.len() < limits().max_frame_bytes => {
            candidate.push(generator.next().to_le_bytes()[0]);
        }
        _ => {
            candidate.pop();
        }
    };
    candidate
}

#[test]
#[ignore = "opt-in deterministic qualification; run make check-fuzz-soak"]
fn deterministic_wire_mutation_is_panic_free_repeatable_and_canonical() {
    let seed = setting("VELLUM_QUALIFICATION_SEED", DEFAULT_SEED);
    let cases = usize::try_from(setting("VELLUM_WIRE_CASES", DEFAULT_CASES as u64)).unwrap();
    assert!(seed != 0, "qualification seed must be nonzero");
    assert!(
        (1..=1_000_000).contains(&cases),
        "wire case budget is invalid"
    );

    let requests = request_corpus();
    let replies = reply_corpus();
    let mut generator = Generator(seed);
    for case in 0..cases {
        let request_side = generator.next() & 1 == 0;
        let corpus = if request_side { &requests } else { &replies };
        let corpus_index = generator.index(corpus.len());
        let candidate = mutate(&mut generator, &corpus[corpus_index]);
        if request_side {
            let first = catch_unwind(|| decode_request_v1(&candidate, limits()))
                .unwrap_or_else(|_| panic!("request decoder panicked at case {case}"));
            let second = decode_request_v1(&candidate, limits());
            assert_eq!(
                first, second,
                "request decode was not repeatable at case {case}"
            );
            if let Ok(decoded) = first {
                assert_eq!(
                    encode_request_v1(&decoded, limits()).unwrap(),
                    candidate,
                    "request decoder accepted a noncanonical frame at case {case}"
                );
            }
        } else {
            let first = catch_unwind(|| decode_reply_v1(&candidate, limits()))
                .unwrap_or_else(|_| panic!("reply decoder panicked at case {case}"));
            let second = decode_reply_v1(&candidate, limits());
            assert_eq!(
                first, second,
                "reply decode was not repeatable at case {case}"
            );
            if let Ok(decoded) = first {
                assert_eq!(
                    encode_reply_v1(&decoded, limits()).unwrap(),
                    candidate,
                    "reply decoder accepted a noncanonical frame at case {case}"
                );
            }
        }
    }
}
