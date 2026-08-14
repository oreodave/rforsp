//! Character classification the lexer cannot get from the standard library.
//!
//! [`char::is_control`] covers the Unicode `Cc` category, but there is no
//! equivalent for `Cf` - the format characters - and no general-category
//! accessor to derive one from.  The class is therefore carried here as data.

use std::cmp::Ordering;

/// The Unicode `Cf` (format) category, as inclusive ranges sorted by their
/// lower bound and pairwise disjoint - a property the `ranges_are_searchable`
/// test asserts, since the binary search below depends on it.
///
/// Generated from `unicodedata` at Unicode 16.0.0: 170 characters in 21
/// ranges.  The category grows between Unicode releases, so this table is only
/// as current as that version, and regenerating it is the whole maintenance
/// burden of this module.
const FORMAT_RANGES: [(char, char); 21] = [
    ('\u{00ad}', '\u{00ad}'),   // SOFT HYPHEN
    ('\u{0600}', '\u{0605}'), // ARABIC NUMBER SIGN .. ARABIC NUMBER MARK ABOVE
    ('\u{061c}', '\u{061c}'), // ARABIC LETTER MARK
    ('\u{06dd}', '\u{06dd}'), // ARABIC END OF AYAH
    ('\u{070f}', '\u{070f}'), // SYRIAC ABBREVIATION MARK
    ('\u{0890}', '\u{0891}'), // ARABIC POUND MARK ABOVE .. ARABIC PIASTRE MARK ABOVE
    ('\u{08e2}', '\u{08e2}'), // ARABIC DISPUTED END OF AYAH
    ('\u{180e}', '\u{180e}'), // MONGOLIAN VOWEL SEPARATOR
    ('\u{200b}', '\u{200f}'), // ZERO WIDTH SPACE .. RIGHT-TO-LEFT MARK
    ('\u{202a}', '\u{202e}'), // LEFT-TO-RIGHT EMBEDDING .. RIGHT-TO-LEFT OVERRIDE
    ('\u{2060}', '\u{2064}'), // WORD JOINER .. INVISIBLE PLUS
    ('\u{2066}', '\u{206f}'), // LEFT-TO-RIGHT ISOLATE .. NOMINAL DIGIT SHAPES
    ('\u{feff}', '\u{feff}'), // ZERO WIDTH NO-BREAK SPACE (the BOM)
    ('\u{fff9}', '\u{fffb}'), // INTERLINEAR ANNOTATION ANCHOR .. TERMINATOR
    ('\u{110bd}', '\u{110bd}'), // KAITHI NUMBER SIGN
    ('\u{110cd}', '\u{110cd}'), // KAITHI NUMBER SIGN ABOVE
    ('\u{13430}', '\u{1343f}'), // EGYPTIAN HIEROGLYPH VERTICAL JOINER .. END WALLED ENCLOSURE
    ('\u{1bca0}', '\u{1bca3}'), // SHORTHAND FORMAT LETTER OVERLAP .. UP STEP
    ('\u{1d173}', '\u{1d17a}'), // MUSICAL SYMBOL BEGIN BEAM .. END PHRASE
    ('\u{e0001}', '\u{e0001}'), // LANGUAGE TAG
    ('\u{e0020}', '\u{e007f}'), // TAG SPACE .. CANCEL TAG
];

/// Check if a given [`char`] is a Unicode `Cf` format character.
///
/// This is the `Cf` counterpart to [`char::is_control`]'s `Cc`, and the two
/// are used together: both classes render as nothing, so neither may enter a
/// symbol.
pub(super) fn is_format(c: char) -> bool {
    // A character below the first range is the overwhelmingly common case and
    // is worth not paying a search for.
    c >= FORMAT_RANGES[0].0
        && FORMAT_RANGES
            .binary_search_by(|&(low, high)| {
                if c < low {
                    Ordering::Greater
                } else if c > high {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            })
            .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_searchable() {
        // `binary_search_by` is only correct over a table that is sorted and
        // has no overlapping or touching ranges, and the table is maintained
        // by hand-editing generated output, so the property is asserted rather
        // than assumed.
        let successors = FORMAT_RANGES.iter().skip(1);
        for (&(low, high), &(next_low, _)) in
            FORMAT_RANGES.iter().zip(successors)
        {
            assert!(low <= high, "{low:?}..{high:?} is inverted");
            assert!(high < next_low, "{high:?} is not before {next_low:?}");
        }
    }

    #[test]
    fn format_characters() {
        // The BOM, the zero-width set and the bidirectional overrides are the
        // members with consequences.
        for c in ['\u{feff}', '\u{200b}', '\u{200d}', '\u{202e}', '\u{2066}'] {
            assert!(is_format(c), "{c:?} should be a format character");
        }

        // Ordinary symbol material is not, including the boundaries either
        // side of a range and the astral characters between two of them.
        for c in [
            'a',
            '0',
            '∀',
            '\u{0}',
            '\u{ac}',
            '\u{ae}',
            '\u{2065}',
            '\u{2070}',
            '\u{13429}',
            '\u{13440}',
        ] {
            assert!(!is_format(c), "{c:?} should not be a format character");
        }
    }
}
