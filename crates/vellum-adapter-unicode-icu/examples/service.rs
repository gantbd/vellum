//! Warm provider service plus owned-result drop. Not an engine, UI, startup,
//! allocation/RSS, distribution or adopted-product-SLO measurement.
use std::{hint::black_box, time::Instant};
use vellum_adapter_unicode_icu::{Limits, Provider, Timing};
use vellum_engine::instrumentation::Recorder;

fn main() {
    println!("workload,bytes,capture,samples,p50_ns,p95_ns,p99_ns,max_ns,dropped");
    for (name, text) in [
        ("ascii-small", "a word. ".repeat(512)),
        (
            "unicode-prose",
            "é 🇺🇸 ภาษาไทย 日本語 a\u{301}\r\n".repeat(96),
        ),
        ("ascii-limit", "a".repeat(65536)),
    ] {
        for (capture, capacity) in [("off", 0), ("enabled", 1024), ("saturated", 1)] {
            let mut recorder = Recorder::try_with_capacity(capacity).unwrap();
            let mut provider = Provider::new(
                Limits {
                    max_source_bytes: 65536,
                    max_boundaries: 262144,
                    max_words: 65536,
                    max_logical_bytes: 4 * 1024 * 1024,
                },
                Timing::Off,
                &mut recorder,
            )
            .unwrap();
            for _ in 0..16 {
                black_box(provider.segment(&text, &mut recorder).unwrap());
            }
            let mut samples = Vec::with_capacity(256);
            for _ in 0..256 {
                let start = Instant::now();
                black_box(provider.segment(&text, &mut recorder).unwrap());
                samples.push(start.elapsed().as_nanos());
            }
            assert_eq!(provider.usage().completed, 272);
            assert_eq!(provider.usage().refused, 0);
            samples.sort_unstable();
            let percentile = |p: usize| samples[(samples.len() * p).div_ceil(100) - 1];
            println!(
                "{name},{},{capture},{},{},{},{},{},{}",
                text.len(),
                samples.len(),
                percentile(50),
                percentile(95),
                percentile(99),
                samples.last().unwrap(),
                recorder.dropped()
            );
        }
    }
}
