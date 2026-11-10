.PHONY: check check-boundaries fmt lint test doc perf-build perf perf-run check-perf qualify-unicode

check: check-boundaries fmt lint test doc

check-boundaries:
	node scripts/check-project-boundaries.mjs

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --all-features --offline -- -D warnings

test:
	cargo test --workspace --all-targets --all-features --offline

doc:
	cargo test --workspace --doc --offline

PERF_EXAMPLE ?= responsiveness
PERF_BIN ?= target/release/examples/$(PERF_EXAMPLE)
PERF_ARGS ?=

perf-build:
	cargo build -p vellum-engine --example $(PERF_EXAMPLE) --release --offline

perf: perf-build
	$(PERF_BIN) $(PERF_ARGS)

perf-run:
	$(PERF_BIN) $(PERF_ARGS)

check-perf:
	$(if $(PERF_MAX_P99_NS),,$(error Set PERF_MAX_P99_NS to an explicit positive total p99 budget in nanoseconds))
	$(MAKE) perf-build
	$(PERF_BIN) $(PERF_ARGS) --max-p99-ns "$(PERF_MAX_P99_NS)"

qualify-unicode:
	cargo test --offline --manifest-path qualification/unicode-reference/Cargo.toml --all-targets --all-features
