//! Explicit candidate gate, not a replacement for the engine's reference test.
//! No source text is printed. The inputs are the pinned public Unicode corpus.
use sha2::{Digest, Sha256};
use std::{error::Error, path::Path};

pub struct Case {
    line: usize,
    text: String,
    expected: Vec<usize>,
}

fn parse(source: &str) -> Result<Vec<Case>, Box<dyn Error>> {
    let mut cases = Vec::new();
    for (line, row) in source.lines().enumerate() {
        let data = row.split('#').next().unwrap_or("").trim();
        if data.is_empty() {
            continue;
        }
        let mut text = String::new();
        let mut expected = Vec::new();
        for token in data.split_whitespace() {
            match token {
                "÷" => expected.push(text.len()),
                "×" => {}
                hex => text.push(
                    char::from_u32(u32::from_str_radix(hex, 16)?)
                        .ok_or("corpus contains a non-scalar value")?,
                ),
            }
        }
        if expected.first() != Some(&0)
            || expected.last() != Some(&text.len())
            || expected.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err("invalid corpus boundary grid".into());
        }
        cases.push(Case {
            line: line + 1,
            text,
            expected,
        });
    }
    Ok(cases)
}

fn corpus(name: &str, pin: &str, count: usize) -> Result<Vec<Case>, Box<dyn Error>> {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../formal")
            .join(name),
    )?;
    if format!("{:x}", Sha256::digest(&bytes)) != pin {
        return Err("Unicode corpus pin mismatch".into());
    }
    let cases = parse(std::str::from_utf8(&bytes)?)?;
    if cases.len() != count {
        return Err("Unicode corpus count mismatch".into());
    }
    Ok(cases)
}

pub fn check(
    name: &str,
    cases: &[Case],
    segment: impl Fn(&str) -> Result<Vec<usize>, Box<dyn Error>>,
) -> Result<usize, Box<dyn Error>> {
    let mut failed = 0;
    for case in cases {
        if segment(&case.text)? != case.expected {
            failed += 1;
            if failed <= 8 {
                eprintln!("{name}: mismatch at public corpus line {}", case.line);
            }
        }
    }
    println!(
        "{name}: cases={}, passed={}, mismatches={failed}",
        cases.len(),
        cases.len() - failed
    );
    Ok(failed)
}

pub fn corpora() -> Result<(Vec<Case>, Vec<Case>), Box<dyn Error>> {
    let graphemes = corpus(
        "unicode18-grapheme-test.txt",
        "b0cf047ee94485bbdc846de2b902f5f8a815f6b674f9d04223cddadd91c9df31",
        853,
    )?;
    let words = corpus(
        "unicode18-word-test.txt",
        "3dd70c071781276067c680d87303f60434adce7b067bd063194af374edad86a5",
        1944,
    )?;
    Ok((graphemes, words))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_parser_uses_utf8_bytes_and_preserves_literal_bom_and_crlf() {
        let cases = parse("÷ FEFF ÷ 0065 × 0301 ÷ 000D × 000A ÷ # inert comment").unwrap();
        assert_eq!(cases[0].text, "\u{feff}e\u{301}\r\n");
        assert_eq!(cases[0].expected, [0, 3, 6, 8]);
    }

    #[test]
    fn malformed_grid_or_non_scalar_input_fails_instead_of_skipping_a_case() {
        for source in ["× 0061 ÷", "÷ 0061 ×", "÷ D800 ÷", "÷ ÷", "÷ xyz ÷"] {
            assert!(parse(source).is_err());
        }
        assert!(parse("# comment only").unwrap().is_empty());
    }
}
