//! Explicit stable candidate gate, not a production provider switch.
#[path = "support/corpus.rs"]
mod corpus;
use icu_segmenter::{GraphemeClusterSegmenter, WordSegmenter};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let (graphemes, words) = corpus::corpora()?;
    let grapheme = GraphemeClusterSegmenter::new();
    // No dictionary/LSTM, locale override or hand-patched boundaries.
    let word = WordSegmenter::new_for_non_complex_scripts(Default::default());
    let failures = corpus::check("icu4x-2.4.0/stable-grapheme", &graphemes, |text| {
        Ok(grapheme.segment_str(text).collect())
    })? + corpus::check("icu4x-2.4.0/stable-word-no-complex", &words, |text| {
        Ok(word.segment_str(text).collect())
    })?;
    if failures != 0 {
        return Err(
            "candidate failed Unicode 18 default-boundary qualification; UI unchanged".into(),
        );
    }
    println!(
        "Candidate corpus gate passed; engine integration, significant-word classification, resource/performance and rendered geometry remain separate obligations."
    );
    Ok(())
}
