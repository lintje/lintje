use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

lazy_static! {
    static ref OTHER_PUNCTUATION: Vec<char> = vec!['…', '⋯',];
}

pub fn is_punctuation(character: &char) -> bool {
    character.is_ascii_punctuation() || OTHER_PUNCTUATION.contains(&character)
}

// Return String display width as rendered in a monospace font according to the Unicode
// specification.
//
// This may return some odd results at times where some symbols are counted as more character width
// than they actually are.
pub fn display_width(string: &str) -> usize {
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
    for c in unicode_chars.into_iter() {
        width += display_width(c);
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

/// Indent all lines of a string by a number of spaces
pub fn indent_string(string: String, level: usize) -> String {
    let mut result = "".to_string();
    for line in string.lines() {
        if line.is_empty() {
            result.push('\n')
        } else {
            result.push_str(&format!("{}{}\n", " ".repeat(level), line))
        }
    }
    result
}

#[cfg(test)]
mod test {
    use super::{
        character_count_for_bytes_index, display_width, indent_string, line_length_stats,
        MarkerStats,
    };

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
        assert_width("abc", 3);
        assert_width(&"a".repeat(50), 50);
        assert_width("!*_-=+|[]`'.,<>():;!@#$%^&{}10/", 31);
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

        // Some of these characters don't match the width one would expect. Most of these are
        // rendered as 2 width in my editor and terminal, but this is what unicode-width returns as
        // the width according to the Unicode specification.
        // These checks are mostly here for a reference to improve the calculated display width
        // better in the future.
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
        assert_width("👁", 1);
        assert_width("👁️", 1); // Eye + variable selector 16 `\u{fe0f}`
        assert_width("👁️‍🗨️", 2);
        assert_width("👩‍🔬", 4);
        assert_width("👨‍🦰", 4);
        assert_width("🧔🏿", 4);
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
        assert_eq!(width, 8); // TODO: Should be 6, but unicode-rs returns 4 width for the emoji
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is in the middle of the emoji, so it will return the position before the emoji
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 3);
        assert_eq!(width, 8); // TODO: Should be 6, but unicode-rs returns 4 width for the emoji
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is after the emoji
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 4);
        // TODO: Should be 6, but unicode-width returns 4 width for the emoji
        assert_eq!(width, 8);
        // TODO: Is inaccurate because the emoji is considered 4 characters wide
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 2,
                char_count: 2,
            }
        );
        // Max width is after the emoji
        let (width, line_stats) = line_length_stats("Aa👩‍🔬Bb", 6);
        // TODO: Should be 6, but unicode-width returns 4 width for the emoji
        assert_eq!(width, 8);
        assert_eq!(
            line_stats,
            MarkerStats {
                bytes_index: 13,
                char_count: 3,
            }
        );
    }

    #[test]
    fn test_indent_string() {
        assert_eq!(
            indent_string("line 1\nline 2\nline 3".to_string(), 1),
            " line 1\n line 2\n line 3\n"
        );
        assert_eq!(
            indent_string("line 1\n\nline 2".to_string(), 1),
            " line 1\n\n line 2\n"
        );
        assert_eq!(
            indent_string("line 1\n\nline 2".to_string(), 6),
            "      line 1\n\n      line 2\n"
        );
    }
}
