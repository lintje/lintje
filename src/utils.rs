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

mod test {
    use super::display_width;

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
}
