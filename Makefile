.PHONY: check check-unicode check-unicode-native check-unicode-icu4x check-unicode-icu4c fmt test lint doc perf-build perf perf-run check-perf app-macos

check: fmt lint test doc

fmt:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --all-features --offline -- -D warnings

test:
	cargo test --workspace --all-targets --offline

doc:
	cargo test --workspace --doc --offline

# Local reference bundle; select ICU4C 79.1 with PKG_CONFIG_PATH. Native dylibs
# remain host prerequisites, not vendored portable/notarized distribution.
app-macos:
	cargo build --locked --release --offline -p vellum-iced
	mkdir -p target/Vellum.app/Contents/MacOS
	cp crates/vellum-iced/resources/Info.plist target/Vellum.app/Contents/Info.plist
	cp target/release/vellum-iced target/Vellum.app/Contents/MacOS/Vellum
	/usr/bin/codesign --force --sign - target/Vellum.app

# Explicit, non-skippable reference gate; requires VELLUM_ICU4J_JAR and Java.
check-unicode:
	cargo test -p vellum-unicode-reference --features reference-conformance --test reference_conformance --offline -- --nocapture

# Actual native adapter -> exact source capture/evidence -> engine commands.
# Same pinned Java oracle as check-unicode; selected ICU4C must be 79.1.
check-unicode-native:
	cargo test --release --offline -p vellum-unicode-reference --features native-conformance --test reference_conformance -- --nocapture

# Candidate checks never change the UI's selected provider. ICU4X stable
# currently fails the pinned grapheme corpus; that is a refusal, not a skip.
check-unicode-icu4x:
	cargo run --release --offline -p vellum-unicode-reference --features icu4x-conformance --example icu4x_qualification

# Select the actual host installation explicitly (e.g. PKG_CONFIG_PATH).
check-unicode-icu4c:
	test "$$(pkg-config --modversion icu-i18n)" = "79.1"
	cargo run --release --offline -p vellum-unicode-reference --features icu4c-conformance --example icu4c_qualification

# Separate compilation from repeatable warm-process measurements. Override
# PERF_BIN when using a non-default Cargo target directory. No product SLO is
# silently selected; report mode and explicit budget enforcement are distinct.
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
