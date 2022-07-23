use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const ZERO_WIDTH_JOINER: &str = "\u{200d}";
const VARIATION_SELECTOR_16: &str = "\u{fe0f}";
const SKIN_TONES: [&str; 5] = [
    "\u{1f3fb}", // Light Skin Tone
    "\u{1f3fc}", // Medium-Light Skin Tone
    "\u{1f3fd}", // Medium Skin Tone
    "\u{1f3fe}", // Medium-Dark Skin Tone
    "\u{1f3ff}", // Dark Skin Tone
];

lazy_static! {
    static ref OTHER_PUNCTUATION: Vec<char> = vec!['…', '⋯',];
}

pub fn is_punctuation(character: char) -> bool {
    character.is_ascii_punctuation() || OTHER_PUNCTUATION.contains(&character)
}

// Return String display width as rendered in a monospace font according to the Unicode
// specification.
//
// This may return some odd results at times where some symbols are counted as more character width
// than they actually are.
//
// This function has exceptions for skin tones and other emoji modifiers to determine a more
// accurate display with.
pub fn display_width(string: &str) -> usize {
    // String expressed as a vec of Unicode characters. Characters with accents and emoji may
    // be multiple characters combined.
    let unicode_chars = string.graphemes(true);
    let mut width = 0;
    for c in unicode_chars {
        width += display_width_char(c);
    }
    width
}

/// Calculate the render width of a single Unicode character. Unicode characters may consist of
/// multiple String characters, which is why the function argument takes a string.
fn display_width_char(string: &str) -> usize {
    // Characters that are used as modifiers on emoji. By themselves they have no width.
    if string == ZERO_WIDTH_JOINER || string == VARIATION_SELECTOR_16 {
        return 0;
    }
    // Emoji that are representations of combined emoji. They are normally calculated as the
    // combined width of the emoji, rather than the actual display width. This check fixes that and
    // returns a width of 2 instead.
    if string.contains(ZERO_WIDTH_JOINER) {
        return 2;
    }
    // Any character with a skin tone is most likely an emoji.
    // Normally it would be counted as as four or more characters, but these emoji should be
    // rendered as having a width of two.
    for skin_tone in SKIN_TONES {
        if string.contains(skin_tone) {
            return 2;
        }
    }

    match string {
        "\t" => {
            // unicode-width returns 0 for tab width, which is not how it's rendered.
            // I choose 4 columns as that's what most applications render a tab as.
            4
        }
        _ => UnicodeWidthStr::width(string),
    }
}

#[derive(Debug, PartialEq)]
pub struct MarkerStats {
    pub bytes_index: usize, // Zero index of marked width in bytes of the line
    pub char_count: usize,  // Character count of marked width of the line
}

// max_width: max display width
pub fn line_length_stats(line: &str, max_width: usize) -> (usize, MarkerStats) {
    // String expressed as a vec of Unicode characters. Characters with accents and emoji may
    // be multiple characters combined.
    let unicode_chars = line.graphemes(true);
    // Track where the display width is more than 50 characters expressed in bytes
    let mut bytes_index = 0;
    // Track which character (not column) the subject becomes too long
    let mut char_count = 0;
    // The total display width of the subject.
    let mut width = 0;
    for c in unicode_chars {
        width += display_width_char(c);
        if width <= max_width {
            char_count += 1;
            bytes_index += c.len();
        }
    }
    (
        width,
        MarkerStats {
            bytes_index,
            char_count,
        },
    )
}

/// Given a String and a bytes starting index, it will return the character count.
///
/// # Examples
///
/// ```
/// let s = "Hello world";
///          ^ zero bytes index
///               ^ starting bytes index mapped to character count
///                ^ this character is the target
/// assert_eq!(s.len(), 12);
/// assert_eq!(character_count_for_bytes_index(s, 6), 7);
/// ```
pub fn character_count_for_bytes_index(string: &str, bytes_index: usize) -> usize {
    match &string.get(0..bytes_index) {
        Some(sub_string) => {
            // Plus 1, because we fetch everything leading up to the character byte we want to
            // return as a column, and then increment it by one.
            sub_string.graphemes(true).count() + 1
        }
        None => {
            error!(
                "character_count_for_bytes_index: Unable to determine substring length.\n\
                Please report this error: https://github.com/tombruijn/lintje/issues\n\
                String: {:?}\nbytes_index: {:?}",
                string, bytes_index
            );
            // Fall back on the bytes_index. It's better than nothing, but won't produce any useful
            // results unless it's all ASCII characters.
            bytes_index
        }
    }
}

pub fn pluralize(label: &str, count: usize) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{}{}", label, plural)
}

#[cfg(test)]
pub mod tests {
    use super::{character_count_for_bytes_index, display_width, line_length_stats, MarkerStats};

    #[test]
    fn test_character_index_for_bytes() {
        let s = "Hello world!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Héllo world!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Hellあ world!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Hellö̲ world!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Hello wｏrld!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Hell😀 world!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Hell👩‍🔬 world!";
        assert_eq!(character_count_for_bytes_index(s, s.len()), 13);

        let s = "Hello world!"; // Target char `w`
        assert_eq!(character_count_for_bytes_index(s, 6), 7);

        let s = "Hello 😀 world!"; // Target char `w`. This emoji is 4 bytes
        assert_eq!(character_count_for_bytes_index(s, 11), 9);

        let s = "Hello 👩‍🔬 world!"; // Target char `w`. This emoji is 11 bytes
        assert_eq!(character_count_for_bytes_index(s, 18), 9);
    }

    fn assert_width(string: &str, width: usize) {
        assert_eq!(
            display_width(string),
            width,
            "String `{}` is not {} width",
            string,
            width
        );
    }

    #[test]
    fn test_display_width() {
        assert_width("a", 1);
        assert_width("\t", 4);
        assert_width("…", 1);

        assert_width("é", 1);
        assert_width("ö", 1);
        assert_width("ø", 1);
        assert_width("a̐", 1);
        assert_width("é", 1);
        assert_width("ö̲", 1);

        assert_width("ぁ", 2);
        assert_width("あ", 2);

        // Zero width characters
        assert_width("\u{200d}", 0);
        assert_width("\u{fe0f}", 0);

        // Some of these characters don't match the width one would expect. Most of these are
        // display as 2 width in my editor and terminal, but unicode-width returns as the width
        // according to the Unicode specification, which may sometimes be different than the actual
        // display width.
        //
        // Some of these the assertions below do not return the width according to unicode-width.
        // The `display_width` function will check for things like skin tones and other emoji
        // modifiers to return a differen display width.
        assert_width("0️⃣", 1);
        assert_width("1️⃣", 1);
        assert_width("#️⃣", 1);
        assert_width("﹟", 2);
        assert_width("＃", 2);
        assert_width("*️⃣", 1);
        assert_width("＊", 2);
        assert_width("❗️", 2);
        assert_width("☁️", 1);
        assert_width("❤️", 1);
        assert_width("☂️", 1);
        assert_width("✏️", 1);
        assert_width("✂️", 1);
        assert_width("☎️", 1);
        assert_width("✈️", 1);
        assert_width("👁", 1); // Eye without variable selector 16
        assert_width("👁️", 1); // Eye + variable selector 16 `\u{fe0f}`
        assert_width("👁️‍🗨️", 2);
        assert_width("🚀", 2);

        // Skin tones
        assert_width("👩", 2);
        assert_width("👩🏻", 2);
        assert_width("👩🏼", 2);
        assert_width("👩🏽", 2);
        assert_width("👩🏾", 2);
        assert_width("👩🏿", 2);

        // Other variations
        assert_width("👩‍🔬", 2);
        assert_width("🧘🏽‍♀️", 2);
        assert_width("👨🏻‍❤️‍👨🏿", 2);
        assert_width("🧑‍🦲", 2);
        assert_width("👨🏿‍🦲", 2);

        // Strings with multiple characters
        assert_width("abc", 3);
        assert_width(&"a".repeat(50), 50);
        assert_width("!*_-=+|[]`'.,<>():;!@#$%^&{}10/", 31);
        assert_width("I am a string with multiple 😁🚀あ", 34);
        assert_width("👩‍🔬👩‍🔬", 4);
    }

    #[test]
    fn test_line_stats() {
        // 6 width, including the space
        assert_eq!(
            line_length_stats("Lorem ipsum", 6),
            (
                11,
                MarkerStats {
                    bytes_index: 6,
                    char_count: 6,
                }
            )
        );
    }

    #[test]
    fn test_line_stats_unicode() {
        // 3 width, including the accent
        let (width, line_stats) = line_length_stats("Aaa̐Bb", 3);
        assert_eq!("a̐".chars().count(), 2);
        assert_eq!(width, 5);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 5,
                char_count: 3,
            }
        );
        // 4 width, including the Hiragana
        let (width, line_stats) = line_length_stats("AaあBb", 4);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 5,
                char_count: 3,
            }
        );
    }

    #[test]
    fn test_line_stats_emoji() {
        // 2 width, before the emoji
        let (width, line_stats) = line_length_stats("Aa😀Bb", 2);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is in the middle of the emoji, so it will return the position before the emoji
        let (width, line_stats) = line_length_stats("Aa😀Bb", 3);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is after the emoji
        let (width, line_stats) = line_length_stats("Aa😀Bb", 4);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 6,
                char_count: 3,
            }
        );
    }

    #[test]
    fn test_line_stats_multi_char_emoji() {
        // Multi character emoji test
        // Just before the emoji
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 2);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is in the middle of the emoji, so it will return the position before the emoji
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 3);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is after the emoji
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 4);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 13,
                char_count: 3,
            }
        );
        // Max width is after the `B` character
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 5);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 14,
                char_count: 4,
            }
        );
        // Max width is the full string
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 6);
        assert_eq!(width, 6);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 15,
                char_count: 5,
            }
        );
    }
}
