//! Direct default word rules from UAX #29 revision 49, §4.1.1:
//! <https://www.unicode.org/reports/tr29/tr29-49.html#Default_Word_Boundaries>
//! No editor-language policy, dictionary, normalization or patched source grids.
//! Two monotone scalar iterators, two previous significant classes and RI parity
//! make lookahead/ignored-character handling linear, with constant extra storage.
use super::{Error, push_point};
use icu_properties::{
    CodePointMapData, CodePointSetData,
    props::{ExtendedPictographic, WordBreak as W},
};
use vellum_engine::coordinates::BytePosition;

fn ignored(w: W) -> bool {
    matches!(w, W::Extend | W::Format | W::ZWJ)
}
fn newline(w: W) -> bool {
    matches!(w, W::CR | W::LF | W::Newline)
}
fn ah(w: W) -> bool {
    matches!(w, W::ALetter | W::HebrewLetter)
}
fn mid_letter(w: W) -> bool {
    matches!(w, W::MidLetter | W::MidNumLet | W::SingleQuote)
}
fn mid_number(w: W) -> bool {
    matches!(w, W::MidNum | W::MidNumLet | W::SingleQuote)
}
fn extender_left(w: W) -> bool {
    ah(w) || matches!(w, W::Numeric | W::Katakana | W::ExtendNumLet)
}

fn joined(left: W, right: W, before: W, after: W, ri_odd: bool) -> bool {
    (ah(left) && ah(right)) // WB5
        || (ah(left) && mid_letter(right) && ah(after)) // WB6
        || (ah(before) && mid_letter(left) && ah(right)) // WB7
        || (left == W::HebrewLetter && right == W::SingleQuote) // WB7a
        || (left == W::HebrewLetter && right == W::DoubleQuote && after == W::HebrewLetter) // WB7b
        || (before == W::HebrewLetter && left == W::DoubleQuote && right == W::HebrewLetter) // WB7c
        || (left == W::Numeric && right == W::Numeric) // WB8
        || (ah(left) && right == W::Numeric) // WB9
        || (left == W::Numeric && ah(right)) // WB10
        || (before == W::Numeric && mid_number(left) && right == W::Numeric) // WB11
        || (left == W::Numeric && mid_number(right) && after == W::Numeric) // WB12
        || (left == W::Katakana && right == W::Katakana) // WB13
        || (extender_left(left) && right == W::ExtendNumLet) // WB13a
        || (left == W::ExtendNumLet && (ah(right) || matches!(right, W::Numeric | W::Katakana))) // WB13b
        || (left == W::RegionalIndicator && right == W::RegionalIndicator && ri_odd) // WB15/16
}

pub(super) fn append(text: &str, words: &mut Vec<BytePosition>, limit: usize) -> Result<(), Error> {
    let properties = CodePointMapData::<W>::new();
    let pictographic = CodePointSetData::new::<ExtendedPictographic>();
    let mut next = text
        .char_indices()
        .filter_map(|(byte, ch)| {
            let class = properties.get(ch);
            (!ignored(class)).then_some((byte, class))
        })
        .peekable();
    let mut raw_previous = W::Other;
    let mut left = W::Other;
    let mut before = W::Other;
    let mut ri_odd = false;
    push_point(words, 0, text, limit)?; // WB1; empty input has one boundary.
    for (byte, ch) in text.char_indices() {
        let right = properties.get(ch);
        while next.peek().is_some_and(|&(at, _)| at <= byte) {
            next.next();
        }
        let after = next.peek().map_or(W::Other, |&(_, class)| class);
        if byte > 0 {
            let boundary = if raw_previous == W::CR && right == W::LF {
                false
            }
            // WB3
            else if newline(raw_previous) || newline(right) {
                true
            }
            // WB3a/b
            else if (raw_previous == W::ZWJ && pictographic.contains(ch)) // WB3c
                || (raw_previous == W::WSegSpace && right == W::WSegSpace) // WB3d
                || ignored(right)
            // WB4 (ordered after newline rules)
            {
                false
            } else {
                !joined(left, right, before, after, ri_odd)
            }; // WB5–16, WB999
            if boundary {
                push_point(words, byte, text, limit)?;
            }
        }
        if !ignored(right) {
            ri_odd = if right == W::RegionalIndicator {
                !ri_odd
            } else {
                false
            };
            before = left;
            left = right;
        }
        raw_previous = right;
    }
    if !text.is_empty() {
        push_point(words, text.len(), text, limit)?;
    } // WB2
    Ok(())
}
