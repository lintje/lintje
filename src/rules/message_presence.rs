use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::{Rule, RuleValidation};
use crate::utils::display_width;

pub struct MessagePresence {}

impl RuleValidation for MessagePresence {
    fn new() -> Self {
        Self {}
    }

    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let message = &commit.message.trim();
        let width = display_width(message);
        if width == 0 {
            let context = vec![
                Context::subject(commit.subject.to_string()),
                Context::message_line(2, "".to_string()),
                Context::message_line_error(
                    3,
                    "".to_string(),
                    Range { start: 0, end: 1 },
                    "Add a message body with context about the change and why it was made"
                        .to_string(),
                ),
            ];
            return Some(vec![Issue::error(
                Rule::MessagePresence,
                "No message body was found".to_string(),
                Position::MessageLine { line: 3, column: 1 },
                context,
            )]);
        } else if width < 10 {
            let mut context = vec![];
            let line_count = commit.message.lines().count();
            let line_number = line_count + 1;
            if let Some(line) = commit.message.lines().last() {
                context.push(Context::message_line_error(
                    line_number,
                    line.to_string(),
                    Range {
                        start: 0,
                        end: line.len(),
                    },
                    "Add a longer message with context about the change and why it was made"
                        .to_string(),
                ));
            }
            return Some(vec![Issue::error(
                Rule::MessagePresence,
                "The message body is too short".to_string(),
                Position::MessageLine {
                    line: line_number,
                    column: 1,
                },
                context,
            )]);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MessagePresence::new().validate(&commit)
    }

    #[test]
    fn with_message() {
        let with_message = commit("Subject".to_string(), "Hello I am a message.".to_string());
        assert_eq!(validate(&with_message), None);
    }

    #[test]
    fn without_message() {
        let without_message = commit("Subject", "");
        let issue = first_issue(validate(&without_message));
        assert_eq!(issue.message, "No message body was found");
        assert_eq!(issue.position, message_position(3, 1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | Subject\n\
                   2 | \n\
                   3 | \n\
             \x20\x20| ^ Add a message body with context about the change and why it was made\n"
        );
    }

    #[test]
    fn with_short_message() {
        let short = commit("Subject", "\nShort.");
        let issue = first_issue(validate(&short));
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(3, 1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   3 | Short.\n\
             \x20\x20| ^^^^^^ Add a longer message with context about the change and why it was made\n"
        );
    }

    #[test]
    fn with_very_short_message() {
        let very_short = commit("Subject".to_string(), "...".to_string());
        let issue = first_issue(validate(&very_short));
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(2, 1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   2 | ...\n\
             \x20\x20| ^^^ Add a longer message with context about the change and why it was made\n"
        );
    }

    #[test]
    fn with_very_short_multi_line_message() {
        let very_short = commit("Subject".to_string(), ".\n.\nShort.\n".to_string());
        let issues = validate(&very_short);
        let issue = first_issue(issues);
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(4, 1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   4 | Short.\n\
             \x20\x20| ^^^^^^ Add a longer message with context about the change and why it was made\n"
        );
    }
}
