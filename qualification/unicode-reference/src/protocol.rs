use crate::{MAX_OUTPUT_BYTES, ReferenceError, ReferenceRow};
use base64::Engine;
use serde::Deserialize;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::{BoundaryProfileV1, BoundaryTableV1, ByteRange};
pub(super) fn encode(texts: &[&str]) -> Result<Vec<u8>, ReferenceError> {
    if texts.is_empty() || texts.len() > 4096 {
        return Err(ReferenceError::InputLimit);
    }
    let mut total = 0usize;
    for text in texts {
        let size = base64::encoded_len(text.len(), true).ok_or(ReferenceError::InputLimit)?;
        if size > 8192 || text.chars().take(2049).count() > 2048 {
            return Err(ReferenceError::InputLimit);
        }
        total = total
            .checked_add(size + 1)
            .ok_or(ReferenceError::InputLimit)?;
    }
    let mut input = String::new();
    input
        .try_reserve_exact(total)
        .map_err(|_| ReferenceError::Capacity)?;
    for text in texts {
        base64::engine::general_purpose::STANDARD.encode_string(text.as_bytes(), &mut input);
        input.push('\n');
    }
    Ok(input.into_bytes())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    icu: String,
    unicode: String,
    uax29: u32,
}
type Row = (Vec<usize>, Vec<usize>, Vec<[usize; 2]>, Vec<usize>);
fn grid(text: &str, values: &[usize]) -> bool {
    values.len() <= text.chars().count() + 1
        && values.first() == Some(&0)
        && values.last() == Some(&text.len())
        && values.iter().all(|p| text.is_char_boundary(*p))
        && values.windows(2).all(|p| p[0] < p[1])
}
pub(super) fn decode(texts: &[&str], bytes: &[u8]) -> Result<Vec<ReferenceRow>, ReferenceError> {
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(ReferenceError::OutputLimit);
    }
    let output = std::str::from_utf8(bytes).map_err(|_| ReferenceError::Protocol)?;
    let mut lines = output.lines();
    let header = lines.next().ok_or(ReferenceError::Protocol)?;
    if header.len() > 256 {
        return Err(ReferenceError::Protocol);
    }
    let version: Version = serde_json::from_str(header).map_err(|_| ReferenceError::Protocol)?;
    if version.icu != "79.1" || version.unicode != "18.0.0" || version.uax29 != 49 {
        return Err(ReferenceError::Protocol);
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(texts.len())
        .map_err(|_| ReferenceError::Capacity)?;
    for text in texts {
        let line = lines.next().ok_or(ReferenceError::Protocol)?;
        if line.len() > 262144 {
            return Err(ReferenceError::OutputLimit);
        }
        let (graphemes, words, significant, dictionary): Row =
            serde_json::from_str(line).map_err(|_| ReferenceError::Protocol)?;
        if [&graphemes, &words, &dictionary]
            .iter()
            .any(|g| !grid(text, g))
            || significant.len() > words.len().saturating_sub(1)
        {
            return Err(ReferenceError::Protocol);
        }
        let mut previous = 0;
        for [start, end] in &significant {
            let at = words
                .binary_search(start)
                .map_err(|_| ReferenceError::Protocol)?;
            if start >= end || *start < previous || words.get(at + 1) != Some(end) {
                return Err(ReferenceError::Protocol);
            }
            previous = *end;
        }
        rows.push(ReferenceRow {
            table: BoundaryTableV1 {
                profile: BoundaryProfileV1::Unicode18Uax29Revision49,
                graphemes: graphemes.into_iter().map(BytePosition).collect(),
                words: words.into_iter().map(BytePosition).collect(),
                significant_words: significant
                    .into_iter()
                    .map(|[start, end]| ByteRange { start, end })
                    .collect(),
            },
            dictionary: dictionary.into_iter().map(BytePosition).collect(),
        });
    }
    if lines.next().is_some() {
        return Err(ReferenceError::Protocol);
    }
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "{\"icu\":\"79.1\",\"unicode\":\"18.0.0\",\"uax29\":49}";
    const ROW: &str = "[[0,1],[0,1],[[0,1]],[0,1]]";
    #[test]
    fn exact_version_and_source_safe_rows_are_required_not_just_a_profile_label() {
        let result = decode(&["a"], format!("{HEADER}\n{ROW}\n").as_bytes()).unwrap();
        assert_eq!(result[0].table.graphemes.len(), 2);
        for header in [
            "{\"icu\":\"78.1\",\"unicode\":\"17.0.0\",\"uax29\":47}",
            "{\"icu\":\"79.1\",\"unicode\":\"18.0.0\",\"uax29\":49,\"extra\":true}",
            "{\"icu\":\"79.1\",\"icu\":\"79.1\",\"unicode\":\"18.0.0\",\"uax29\":49}",
        ] {
            assert_eq!(
                decode(&["a"], format!("{header}\n{ROW}\n").as_bytes()).err(),
                Some(ReferenceError::Protocol)
            );
        }
    }
    #[test]
    fn cardinality_unknown_shapes_negative_fractional_and_surrogate_style_offsets_refuse() {
        for row in [
            "[[0,1],[0,1],[],[0,1],[]]",
            "[[-1,1],[0,1],[],[0,1]]",
            "[[0,1.0],[0,1],[],[0,1]]",
            "[[0,2],[0,1],[],[0,1]]",
            "[[0,0,1],[0,1],[],[0,1]]",
        ] {
            assert_eq!(
                decode(&["a"], format!("{HEADER}\n{row}\n").as_bytes()).err(),
                Some(ReferenceError::Protocol)
            );
        }
        assert_eq!(
            decode(&["a", "a"], format!("{HEADER}\n{ROW}\n").as_bytes()).err(),
            Some(ReferenceError::Protocol)
        );
        assert_eq!(
            decode(
                &["é"],
                format!("{HEADER}\n[[0,1,2],[0,2],[],[0,2]]\n").as_bytes()
            )
            .err(),
            Some(ReferenceError::Protocol)
        );
    }
    #[test]
    fn dictionary_grid_is_separate_and_raw_word_grids_are_not_intersected() {
        let row = "[[0,3],[0,1,3],[],[0,3]]";
        let result = decode(&["a\u{301}"], format!("{HEADER}\n{row}\n").as_bytes()).unwrap();
        assert_eq!(result[0].table.words.len(), 3);
        assert_eq!(result[0].dictionary.len(), 2);
    }
    #[test]
    fn input_bounds_precede_encoding_and_literal_feff_is_not_file_decoded() {
        assert_eq!(encode(&[]).err(), Some(ReferenceError::InputLimit));
        assert_eq!(
            encode(&vec![""; 4097]).err(),
            Some(ReferenceError::InputLimit)
        );
        assert_eq!(
            encode(&[&"x".repeat(2049)]).err(),
            Some(ReferenceError::InputLimit)
        );
        assert_eq!(encode(&["\u{feff}"]).unwrap(), b"77u/\n");
        assert_eq!(encode(&[""]).unwrap(), b"\n");
    }
}
