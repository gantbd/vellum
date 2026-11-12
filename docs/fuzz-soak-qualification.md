# Deterministic fuzz and soak qualification

Vellum has an opt-in, offline qualification gate for adversarial decoding and repeated lifecycle work:

```sh
make check-fuzz-soak
```

The gate uses no fuzzing service, clock assertion, random-device input, or new dependency. A fixed xorshift generator drives bounded cases. Accepted mutated wire frames must decode repeatably and re-encode byte-for-byte; rejected frames must produce the same closed error on repetition. The transport phase checks exact outer-frame round trips, every selected truncation, oversize declarations, and one request/one reply dispatch. On macOS, the local adapter profile also exercises exact service replay, changed-input conflict, pagination termination and bounds, stale cursors after removal, idempotent removal, and actual daemon connections containing valid, malformed, oversized, and truncated frames.

The ordinary workspace test gate only compiles these ignored qualification tests. It does not run their loops. The opt-in gate is also available as a manually dispatched GitHub Actions workflow.

## Reproduction and limits

The following decimal environment variables select the seed and budgets:

| Variable | Default | Maximum |
| --- | ---: | ---: |
| `VELLUM_QUALIFICATION_SEED` | `6216458751387172865` | `18446744073709551615` |
| `VELLUM_WIRE_CASES` | `4096` | `1000000` |
| `VELLUM_TRANSPORT_CASES` | `2048` | `1000000` |
| `VELLUM_SERVICE_CYCLES` | `64` | `10000` |
| `VELLUM_DAEMON_CYCLES` | `16` | `1000` |

The seed must be nonzero and every budget must be positive. Failures identify only the phase, case or cycle, and violated property; input frames, source text, filesystem locators, client identifiers, and request identifiers are not emitted.

## Claims and nonclaims

This is deterministic corpus mutation and bounded repetition, not coverage-guided fuzzing, exhaustive state exploration, a concurrency proof, a latency benchmark, or a durability/power-loss test. A passing run establishes the listed properties only for the selected seed, budgets, target, locked dependency closure, and revision. The daemon phase deliberately uses peer EOF to close malformed and truncated exchanges; it makes no scheduling or timeout-duration claim. Linux runs cover wire and transport only because the current filesystem-backed service/daemon composition is macOS-specific.
