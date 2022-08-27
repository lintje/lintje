use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::line_length_stats;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"https?://\w+").unwrap();
    static ref CODE_BLOCK_LINE_WITH_LANGUAGE: Regex = Regex::new(r"^\s*```\s*([\w]+)?$").unwrap();
    static ref CODE_BLOCK_LINE_END: Regex = Regex::new(r"^\s*```$").unwrap();
}

#[derive(PartialEq)]
enum CodeBlockStyle {
    None,
    Fenced,
    Indenting,
}

pub struct MessageLineLength {}

impl MessageLineLength {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessageLineLength {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let mut code_block_style = CodeBlockStyle::None;
        let mut previous_line_was_empty_line = false;
        let mut issues = vec![];
        for (index, raw_line) in commit.message.lines().enumerate() {
            let line = raw_line.trim_end();
            let (width, line_stats) = line_length_stats(line, 72);
            match code_block_style {
                CodeBlockStyle::Fenced => {
                    if CODE_BLOCK_LINE_END.is_match(line) {
                        code_block_style = CodeBlockStyle::None;
                    }
                }
                CodeBlockStyle::Indenting => {
                    if !line.starts_with("    ") {
                        code_block_style = CodeBlockStyle::None;
                    }
                }
                CodeBlockStyle::None => {
                    if CODE_BLOCK_LINE_WITH_LANGUAGE.is_match(line) {
                        code_block_style = CodeBlockStyle::Fenced;
                    } else if line.starts_with("    ") && previous_line_was_empty_line {
                        code_block_style = CodeBlockStyle::Indenting;
                    }
                }
            }
            if code_block_style != CodeBlockStyle::None {
                // When in a code block, skip line length validation
                continue;
            }
            if width > 72 {
                if URL_REGEX.is_match(line) {
                    continue;
                }
                let line_number = index + 2; // + 1 for subject + 1 for zero index
                let context = Context::message_line_error(
                    line_number,
                    line.to_string(),
                    Range {
                        start: line_stats.bytes_index,
                        end: line.len(),
                    },
                    "Shorten line to maximum 72 characters".to_string(),
                );
                issues.push(Issue::error(
                    Rule::MessageLineLength,
                    format!(
                        "Line {} in the message body is longer than 72 characters",
                        line_number
                    ),
                    Position::MessageLine {
                        line: line_number,
                        column: line_stats.char_count + 1, // + 1 because the next char is the problem
                    },
                    vec![context],
                ));
            }
            previous_line_was_empty_line = line.trim() == "";
        }

        if issues.is_empty() {
            None
        } else {
            Some(issues)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MessageLineLength::new().validate(commit)
    }

    fn assert_valid(message: &str) {
        assert_eq!(validate(&commit("Subject", message)), None);
    }

    fn assert_invalid(message: &str) {
        assert!(validate(&commit("Subject", message)).is_some());
    }

    #[test]
    fn valid_message() {
        let message = ["Hello I am a message.", "Line 2.", &"a".repeat(72)].join("\n");
        assert_valid(&message);
    }

    #[test]
    fn invalid_long_line() {
        let message = ["".to_string(), "a".repeat(72), "a".repeat(73)].join("\n");
        let issue = first_issue(validate(&commit("Subject", &message)));
        assert_eq!(
            issue.message,
            "Line 4 in the message body is longer than 72 characters"
        );
        assert_eq!(issue.position, message_position(4, 73));
        assert_contains_issue_output(
            &issue,
            "4 | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
               |                                                                         ^ Shorten line to maximum 72 characters"
        );
    }

    #[test]
    fn valid_long_line_with_https_url() {
        let message = [
            "This message is accepted.".to_string(),
            "This a long line with a link https://tomdebruijn.com/posts/git-is-about-communication/".to_string()
        ].join("\n");
        assert_valid(&message);
    }

    #[test]
    fn valid_long_line_with_http_url() {
        let message = [
            "This message is accepted.".to_string(),
            "This a long line with a link http://tomdebruijn.com/posts/git-is-about-communication/"
                .to_string(),
        ]
        .join("\n");
        assert_valid(&message);
    }

    #[test]
    fn long_line_with_url_schema_only() {
        let message =
            "This a too long line with only protocols http:// https://, not accepted!!".to_string();
        let issue = first_issue(validate(&commit("Subject", &message)));
        assert_eq!(
            issue.message,
            "Line 2 in the message body is longer than 72 characters"
        );
        assert_eq!(issue.position, message_position(2, 73));
        assert_contains_issue_output(
            &issue,
            "2 | This a too long line with only protocols http:// https://, not accepted!!\n\
               |                                                                         ^ Shorten line to maximum 72 characters"
        );
    }

    #[test]
    fn emoji_line_short_enough() {
        // This emoji display width is 2
        let message = ["✨".repeat(36)].join("\n");
        assert_valid(&message);
    }

    #[test]
    fn emoji_line_too_long() {
        let message = ["✨".repeat(37)].join("\n");
        assert_invalid(&message);
    }

    #[test]
    fn hiragana_line_short_enough() {
        // Hiragana display width is 2
        let message = ["あ".repeat(36)].join("\n");
        assert_valid(&message);
    }

    #[test]
    fn hiragana_line_too_long() {
        let message = ["あ".repeat(37)].join("\n");
        assert_invalid(&message);
    }

    #[test]
    fn valid_fenced_code_blocks() {
        let message = [
            "Beginning of message.",
            "```",
            &"a".repeat(73), // Valid, inside code block
            &"b".repeat(73),
            &"c".repeat(73),
            "```",
            "Normal line",
            "```md",
            "I am markdown",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "Normal line",
            "``` yaml",
            "I am yaml",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "Normal line",
            "```  elixir ",
            "I am elixir",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "",
            "  ```",
            "  I am elixir",
            &"  d".repeat(73), // Valid, inside fenced indented code block
            "  ```",
            "End of message",
        ]
        .join("\n");
        assert_valid(&message);
    }

    #[test]
    fn invalid_long_line_outside_fenced_code_block() {
        let message = [
            "Beginning of message.",
            "```",
            &"a".repeat(73),
            "```",
            &"a".repeat(73), // Long line outside code block is invalid
            "End of message",
        ]
        .join("\n");
        assert_invalid(&message);
    }

    #[test]
    fn invalid_fenced_code_block_language_identifier() {
        let message = [
            "Beginning of message.",
            "``` m d", // Invald language identifier
            &"a".repeat(73),
            "```",
            "End of message",
        ]
        .join("\n");
        assert_invalid(&message);
    }

    #[test]
    fn valid_indented_code_blocks() {
        let message = [
            "Beginning of message.",
            "",
            "    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "",
            "    ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "    ",
            "    ddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "",
            "End of message",
        ]
        .join("\n");
        assert_valid(&message);
    }

    #[test]
    fn invalid_long_ling_outside_indended_code_block() {
        let message = [
            "Beginning of message.",
            "",
            "    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "",
            "    ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "",
            "End of message",
            &"a".repeat(73), // Long line outside code block is invalid
        ]
        .join("\n");
        assert_invalid(&message);
    }
}
