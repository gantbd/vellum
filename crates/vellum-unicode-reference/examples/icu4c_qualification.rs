//! Qualify actual native ICU character iteration; never call word iteration
//! default UAX #29 merely because its ICU version matches.
#[path = "support/corpus.rs"]
mod corpus;
use rust_icu_sys::UBreakIteratorType;
use rust_icu_ubrk::UBreakIterator;
use std::error::Error;

fn graphemes(text: &str) -> Result<Vec<usize>, Box<dyn Error>> {
    let iter = UBreakIterator::try_new(UBreakIteratorType::UBRK_CHARACTER, "root", text)?;
    // ICU positions are UTF-16 units, not UTF-8 bytes or scalar indices. Leave
    // surrogate-interior slots absent so a bad native boundary refuses.
    let mut bytes = vec![None; text.encode_utf16().count() + 1];
    let mut units = 0;
    for (at, ch) in text.char_indices() {
        bytes[units] = Some(at);
        units += ch.len_utf16();
    }
    bytes[units] = Some(text.len());
    let mut result = Vec::new();
    for boundary in std::iter::once(iter.first()).chain(iter) {
        let at = usize::try_from(boundary)?;
        result.push(
            *bytes
                .get(at)
                .and_then(Option::as_ref)
                .ok_or("native boundary splits a scalar or exceeds source")?,
        );
    }
    Ok(result)
}

fn main() -> Result<(), Box<dyn Error>> {
    let (cases, _) = corpus::corpora()?;
    let failures = corpus::check("icu4c/character-root", &cases, graphemes)?;
    if failures != 0 {
        return Err("native character iterator failed Unicode 18 corpus; UI unchanged".into());
    }
    println!(
        "Native character corpus passed. Verify actual linked ICU 79.1 library separately; no default-word, production integration or rendered-layout claim."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_utf16_offsets_map_back_to_exact_utf8_boundaries() {
        assert_eq!(graphemes("").unwrap(), [0]);
        assert_eq!(
            graphemes("\u{feff}é🇺🇸e\u{301}\r\n").unwrap(),
            [0, 3, 5, 13, 16, 18]
        );
    }
}
