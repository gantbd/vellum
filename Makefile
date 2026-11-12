.PHONY: check check-activation-transaction check-artifact-lifecycle check-boundaries check-cargo-install-qualification check-compatibility check-effect-boundaries check-fuzz-soak check-launchd-distribution check-mcp-extraction check-release-contract check-semantic-provider-qualification check-supply-chain sbom fmt lint test doc perf-build perf perf-run check-perf qualify-unicode qualify-unicode-portable qualify-github-source qualify-semantic-provider

check: check-release-contract check-artifact-lifecycle check-activation-transaction check-cargo-install-qualification check-launchd-distribution check-mcp-extraction check-boundaries check-effect-boundaries check-compatibility check-supply-chain check-semantic-provider-qualification fmt lint test doc

check-release-contract:
	node scripts/check-release-contract.mjs

check-artifact-lifecycle:
	node --check scripts/release-candidate-lib.mjs
	node --check scripts/qualify-release-candidate.mjs
	node scripts/qualify-release-candidate.test.mjs

check-activation-transaction:
	node --check scripts/activation-transaction-lib.mjs
	node --check scripts/activation-transaction-lib.test.mjs
	node scripts/activation-transaction-lib.test.mjs

check-cargo-install-qualification:
	node --check scripts/cargo-install-qualification-lib.mjs
	node --check scripts/qualify-github-source.mjs
	node scripts/qualify-github-source.test.mjs

qualify-github-source:
	node scripts/qualify-github-source.mjs --vellum-ref=HEAD

check-launchd-distribution:
	node --check distribution/launchd/render-plist.mjs
	node distribution/launchd/render-plist.test.mjs

check-mcp-extraction:
	node --check scripts/verify-mcp-extraction.mjs
	node --check scripts/emit-mcp-peer-source.mjs
	node --check scripts/verify-semantic-mcp-extraction.mjs
	node --check scripts/emit-semantic-mcp-peer-source.mjs
	node scripts/verify-mcp-extraction.mjs
	node scripts/verify-semantic-mcp-extraction.mjs

check-boundaries:
	node scripts/check-project-boundaries.mjs

check-effect-boundaries:
	node scripts/check-effect-boundaries.mjs

check-compatibility:
	node scripts/check-compatibility-baseline.mjs
	node scripts/check-compatibility-baseline.test.mjs

check-supply-chain:
	node scripts/check-supply-chain.mjs
	node scripts/check-supply-chain.test.mjs

check-fuzz-soak:
	node scripts/run-fuzz-soak.mjs

sbom:
	@node scripts/check-supply-chain.mjs --emit=-

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings

test:
	cargo test --workspace --all-targets --all-features --locked --offline

doc:
	RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --locked --offline
	cargo test --workspace --doc --locked --offline

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
	cargo test --locked --offline --manifest-path qualification/unicode-reference/Cargo.toml --all-targets --all-features

qualify-unicode-portable:
	cargo run --locked --offline --manifest-path qualification/unicode-reference/Cargo.toml --features native-conformance --example portable_qualification

check-semantic-provider-qualification:
	cargo test --locked --offline --manifest-path qualification/semantic-lsp-provider/Cargo.toml --all-targets --all-features

qualify-semantic-provider:
	$(if $(SEMANTIC_PROVIDER_PROFILE),,$(error Set SEMANTIC_PROVIDER_PROFILE to a checked-in provider profile))
	$(if $(SEMANTIC_PROVIDER_EXECUTABLE),,$(error Set SEMANTIC_PROVIDER_EXECUTABLE to an absolute verified executable path))
	cargo run --locked --offline --manifest-path qualification/semantic-lsp-provider/Cargo.toml --bin qualify-semantic-provider -- --qualify-pinned-provider --profile "$(SEMANTIC_PROVIDER_PROFILE)" --executable "$(SEMANTIC_PROVIDER_EXECUTABLE)"
