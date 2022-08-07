use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::rules::CONTAINS_FIX_TICKET;
use crate::utils::display_width;

lazy_static! {
    static ref CO_AUTHOR_REFERENCE: Regex =
        Regex::new(r"(?im)^co-authored-by: [\w\s\-]+\s+<[^\s]+[@]+[^\s]+>").unwrap();
}

pub struct MessagePresence {}

impl MessagePresence {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessagePresence {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let message_without_line_breaks = &commit
            .message
            .trim()
            .lines()
            .filter(|l| !l.is_empty())
            .collect::<Vec<&str>>()
            .join("");
        let mut width = display_width(message_without_line_breaks);
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
        }

        // The message body is only ticket numbers. Return those issues. No need to check for the
        // message length after this.
        let issues = issues_for_lines_with_only_ticket_numbers(&commit.message);
        if issues.is_some() {
            return issues;
        }

        // Do not count ticket references towards message body length/width
        width -= ticket_number_reference_length(&commit.message);
        // Do not count co-authored-by references towards message body length/width
        width -= co_author_references_length(&commit.message);

        if width < 10 {
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
                    context.push(Context::message_line_error_without_message(
                        human_line_number,
                        line.to_string(),
                        Range {
                            start: 0,
                            end: line.len(),
                        },
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

fn issues_for_lines_with_only_ticket_numbers(message: &str) -> Option<Vec<Issue>> {
    let mut context = vec![];
    let mut ticket_starting_line_number = None;
    let lines = message.lines();
    for (line_number, line) in lines.enumerate() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }

        if let Some(capture) = scan_for_ticket_number(line) {
            let line_label = line_number + 2;
            let capture_str = capture.as_str();
            let capture_len = capture_str.len();
            if trimmed_line.len() == capture_len {
                if ticket_starting_line_number.is_none() {
                    // The line number to start showing the context from. Empty lines at the start
                    // are ignored and not shown.
                    ticket_starting_line_number = Some(line_label);
                }

                context.push(Context::message_line_error(
                    line_label,
                    capture_str.to_string(),
                    // Rebuild the range because the capture range uses indexes based on the whole
                    // message string and not only the line on which the ticket number was found.
                    Range {
                        start: 0,
                        end: capture_len,
                    },
                    "Add more detail about the change and why it was made".to_string(),
                ));
            } else {
                // The message is not only line numbers, some kind of description is probably
                // present. Skip the rest of this check.
                return None;
            }
        } else {
            // No ticket number found on line, skip the rest of this check. All lines need to
            // match.
            return None;
        }
    }
    if context.is_empty() {
        None
    } else {
        Some(vec![Issue::error(
            Rule::MessagePresence,
            "The message body is only a reference to a ticket number".to_string(),
            Position::MessageLine {
                line: ticket_starting_line_number.unwrap_or(2),
                column: 1,
            },
            context,
        )])
    }
}

// Helper function to cleanly return ticket number captures without having to do the debug logging
// in the loop.
fn scan_for_ticket_number(message: &str) -> Option<regex::Match> {
    if let Some(captures) = CONTAINS_FIX_TICKET.captures(message) {
        match captures.get(0) {
            Some(capture) => return Some(capture),
            None => {
                error!("MessagePresence: Unable to fetch ticket number match from message.");
            }
        }
    }
    None
}

// Return the length of all ticket number references from the message body.
fn ticket_number_reference_length(message: &str) -> usize {
    let mut length = 0;
    let lines = message.lines();
    for line in lines {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }

        if let Some(capture) = scan_for_ticket_number(line) {
            let capture_width = display_width(capture.as_str());
            length += capture_width;
        }
    }
    length
}

// Return the length of all co author lines from the message body.
fn co_author_references_length(message: &str) -> usize {
    let mut length = 0;
    for capture in CO_AUTHOR_REFERENCE.find_iter(message) {
        println!("! {:?}", capture);
        let capture_width = display_width(capture.as_str());
        length += capture_width;
    }
    length
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

    #[test]
    fn with_only_ticket_number() {
        // Message is more than 10 chars
        let ticket_only = commit("Subject".to_string(), "\nCloses #123\n".to_string());
        let issue = first_issue(validate(&ticket_only));
        assert_eq!(
            issue.message,
            "The message body is only a reference to a ticket number"
        );
        assert_eq!(issue.position, message_position(3, 1));
        assert_contains_issue_output(
            &issue,
            "3 | Closes #123\n\
               | ^^^^^^^^^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_multiple_ticket_numbers() {
        // Message is more than 10 chars
        let tickets_only = commit(
            "Subject".to_string(),
            "\nImplements #123\nCloses #234\n".to_string(),
        );
        let issue = first_issue(validate(&tickets_only));
        assert_eq!(
            issue.message,
            "The message body is only a reference to a ticket number"
        );
        assert_eq!(issue.position, message_position(3, 1));
        assert_contains_issue_output(
            &issue,
            "3 | Implements #123\n\
               | ^^^^^^^^^^^^^^^\n\
             4 | Closes #234\n\
               | ^^^^^^^^^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_only_ticket_number_on_second_line() {
        let ticket_only = commit("Subject".to_string(), "Closes #123\n".to_string());
        let issue = first_issue(validate(&ticket_only));
        assert_eq!(
            issue.message,
            "The message body is only a reference to a ticket number"
        );
        assert_eq!(issue.position, message_position(2, 1));
        assert_contains_issue_output(
            &issue,
            "2 | Closes #123\n\
               | ^^^^^^^^^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_only_ticket_number_on_forth_line() {
        let ticket_only = commit("Subject".to_string(), "\n\nCloses #123\n".to_string());
        let issue = first_issue(validate(&ticket_only));
        assert_eq!(
            issue.message,
            "The message body is only a reference to a ticket number"
        );
        assert_eq!(issue.position, message_position(4, 1));
        // It shouldn't show line 3 here because it's empty
        assert_contains_issue_output(
            &issue,
            "4 | Closes #123\n\
               | ^^^^^^^^^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_message_and_ticket_number() {
        // The message contains more than just a ticket reference
        let commit = commit(
            "Subject".to_string(),
            "\nThis commit fixes a bug and it also closes #123\n".to_string(),
        );
        let issues = validate(&commit);
        assert_eq!(issues, None);
    }

    #[test]
    fn with_ticket_number_and_short_message() {
        // Ignore the ticket number reference as a count towards the body and only count the
        // remaining text, which is 9 characters, which is too short.
        let message = commit(
            "Subject".to_string(),
            // Message is 9 characters, not including the ticket references
            "\nFixes #1234\nShortmsg closes #123\n".to_string(),
        );
        let issue = first_issue(validate(&message));
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(3, 1));
        assert_contains_issue_output(
            &issue,
            "3 | Fixes #1234\n\
               | ^^^^^^^^^^^\n\
             4 | Shortmsg closes #123\n\
               | ^^^^^^^^^^^^^^^^^^^^ Add more detail about the change and why it was made",
        );
    }

    #[test]
    fn with_co_author_and_short_message() {
        // Ignore the co author line as a count towards the body and only count the remaining text,
        // which is 9 characters, which is too short.
        let message = commit(
            "Subject".to_string(),
            // Message is 9 characters, not including the co author line
            "\nShort msg\nCo-authored-by: Tom de Bruijn <email@domain.com>\nCo-authored-by: Some-other Namé <email@domain.com>\n".to_string(),
        );
        let issue = first_issue(validate(&message));
        assert_eq!(issue.message, "The message body is too short");
        assert_eq!(issue.position, message_position(3, 1));
        assert_contains_issue_output(
            &issue,
            "3 | Short msg\n\
               | ^^^^^^^^^\n\
             4 | Co-authored-by: Tom de Bruijn <email@domain.com>\n\
               | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n\
             5 | Co-authored-by: Some-other Namé <email@domain.com>\n\
               | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Add more detail about the change and why it was made",
        );
    }
}
