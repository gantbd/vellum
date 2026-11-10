//! Full official-corpus gate for the production portable provider.
#[path = "support/corpus.rs"]
mod corpus;
use std::error::Error;
use vellum_adapter_unicode_icu::{Limits, Provider, Timing};
use vellum_engine::instrumentation::Recorder;

fn main() -> Result<(), Box<dyn Error>> {
    let (graphemes, words) = corpus::corpora()?;
    let mut provider = Provider::new(
        Limits {
            max_source_bytes: 65536,
            max_boundaries: 262144,
            max_words: 65536,
            max_logical_bytes: 4 * 1024 * 1024,
        },
        Timing::Off,
        &mut Recorder::disabled(),
    )
    .map_err(|_| "production provider limits refused")?;
    let failures = corpus::check("production-portable/grapheme", &graphemes, |text| {
        Ok(provider
            .segment(text, &mut Recorder::disabled())
            .map_err(|_| -> Box<dyn Error> { "production grapheme segmentation refused".into() })?
            .graphemes
            .into_iter()
            .map(|point| point.0)
            .collect())
    })? + corpus::check("production-portable/word", &words, |text| {
        Ok(provider
            .segment(text, &mut Recorder::disabled())
            .map_err(|_| -> Box<dyn Error> { "production word segmentation refused".into() })?
            .words
            .into_iter()
            .map(|point| point.0)
            .collect())
    })?;
    if failures != 0 {
        return Err("production portable provider failed the Unicode 18 corpus".into());
    }
    println!(
        "Production portable provider passed all 853 grapheme and 1,944 word cases from the pinned Unicode 18 corpus."
    );
    Ok(())
}
