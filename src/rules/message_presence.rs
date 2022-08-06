use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::display_width;

pub struct MessagePresence {}

impl MessagePresence {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessagePresence {
    fn dependent_rules(&self) -> Option<Vec<Rule>> {
        None
    }

    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let message_without_line_breaks = &commit
            .message
            .trim()
            .lines()
            .filter(|l| !l.is_empty())
            .collect::<Vec<&str>>()
            .join("");
        let width = display_width(message_without_line_breaks);
        if width == 0 {
            let context = vec![
                Context::subject(commit.subject.to_string()),
                Context::message_line(2, "".to_string()),
                Context::message_line_error(
                    3,
                    "".to_string(),
                    Range { start: 0, end: 1 },
                    "Add a message that describes the change and why it was made".to_string(),
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
            let message = commit.message.trim_end();
            let line_length = message.lines().count();
            for (line_number, line) in message.lines().enumerate() {
                if line_number == 0 && line.is_empty() {
                    // Skip first line only if it's empty. It should be empty, but if not, print
                    // it. See also the MessageEmptyFirstLine rule.
                    continue;
                }

                let human_line_number = line_number + 2;
                // Account for zero index array to find last line
                if line_number + 1 == line_length {
                    // Only show error message on last line
                    context.push(Context::message_line_error(
                        human_line_number,
                        line.to_string(),
                        Range {
                            start: 0,
                            end: line.len(),
                        },
                        "Add more detail about the change and why it was made".to_string(),
                    ));
                } else if line.trim().is_empty() {
                    // Do not show an error message for lines that are empty, because they don't
                    // count towards the message body count.
                    context.push(Context::message_line(human_line_number, line.to_string()));
                } else {
                    // Do not show an error message on lines that are not the last line to avoid
                    // repeating the same error message for every line in the message body.
                    context.push(Context::message_line_error(
                        human_line_number,
                        line.to_string(),
                        Range {
                            start: 0,
                            end: line.len(),
                        },
                        "".to_string(),
                    ));
                }
            }
            let line_number_of_start_of_issue = if commit.message.starts_with('\n') {
                3
            } else {
                // The first line is not empty like it should be.
                // This is also handled by the MessageEmptyFirstLine rule, but line number two
                // needs to be pointed at as the start of the issue in this rule because of it.
                2
            };
            return Some(vec![Issue::error(
                Rule::MessagePresence,
                "The message body is too short".to_string(),
                Position::MessageLine {
                    line: line_number_of_start_of_issue,
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
        assert_contains_issue_output(
            &issue,
            "1 | Subject\n\
             2 | \n\
             3 | \n\
               | ^ Add a message that describes the change and why it was made",
        );
    }

    #[test]
    fn with_only_line_numbers() {
        // More than 10 characters in line numbers, would be valid if line breaks aren't ignored
        let commit = commit("Subject", &"\n".repeat(11));
        let issues = validate(&commit);
        assert!(issues.is_some());
    }

    #[test]
    fn with_short_message() {
        let short = commit("Subject", "\nShort.");
        let issue = first_issue(validate(&short));
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(3, 1));
        assert_contains_issue_output(
            &issue,
            "3 | Short.\n\
               | ^^^^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_very_short_message() {
        let very_short = commit("Subject".to_string(), "WIP".to_string());
        let issue = first_issue(validate(&very_short));
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(2, 1));
        assert_contains_issue_output(
            &issue,
            "2 | WIP\n\
               | ^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_very_short_multi_line_message() {
        let very_short = commit("Subject".to_string(), "\n.\n.\n\nShort.\n".to_string());
        let issues = validate(&very_short);
        let issue = first_issue(issues);
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(3, 1));
        assert_contains_issue_output(
            &issue,
            "3 | .\n\
               | ^\n\
             4 | .\n\
               | ^\n\
             5 | \n\
             6 | Short.\n\
               | ^^^^^^ Add more detail about the change and why it was made",
        );
    }
}
