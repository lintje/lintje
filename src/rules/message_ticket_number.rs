use core::ops::Range;
use regex::{Regex, RegexBuilder};

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::{Rule, RuleValidation};

use crate::rules::CONTAINS_FIX_TICKET;

lazy_static! {
    // Match "Part of #123"
    static ref LINK_TO_TICKET: Regex = {
        let mut tempregex = RegexBuilder::new(r"(part of|related):? ([^\s]*[\w\-_/]+)?[#!]{1}\d+");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

pub struct MessageTicketNumber {}

impl RuleValidation for MessageTicketNumber {
    fn new() -> Self {
        Self {}
    }

    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let message = &commit.message.to_string();
        if CONTAINS_FIX_TICKET.captures(message).is_none()
            && LINK_TO_TICKET.captures(message).is_none()
        {
            let line_count = message.lines().count() + 1; // + 1 for subject
            let last_line = if line_count == 1 {
                commit.subject.to_string()
            } else {
                message.lines().last().unwrap_or("").to_string()
            };
            let context = vec![
                Context::message_line(line_count, last_line),
                // Add empty line for spacing
                Context::message_line(line_count + 1, "".to_string()),
                // Suggestion because it indicates a suggested change?
                Context::message_line_addition(
                    line_count + 2,
                    "Fixes #123".to_string(),
                    Range { start: 0, end: 10 },
                    "Consider adding a reference to a ticket or issue".to_string(),
                ),
            ];
            Some(vec![Issue::hint(
                Rule::MessageTicketNumber,
                "The message body does not contain a ticket or issue number".to_string(),
                Position::MessageLine {
                    line: line_count + 2,
                    column: 1,
                },
                context,
            )])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MessageTicketNumber::new().validate(&commit)
    }

    fn assert_valid(message: &str) {
        assert_eq!(validate(&commit("Subject", message)), None);
    }

    #[test]
    fn message_with_ticket_number() {
        let message = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            "Fixes #123",
        ]
        .join("\n");
        assert_valid(&message);
    }

    #[test]
    fn message_with_ticket_number_part_of() {
        let message = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            "Part of #123",
        ]
        .join("\n");
        assert_valid(&message);
    }

    #[test]
    fn message_with_ticket_number_related() {
        let message = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            "Related #123",
        ]
        .join("\n");
        assert_valid(&message);
    }

    #[test]
    fn message_without_ticket_number() {
        let message_without_ticket_number =
            ["", "Beginning of message.", "", "Some explanation."].join("\n");
        let issue = first_issue(validate(&commit(
            "Subject".to_string(),
            message_without_ticket_number,
        )));
        assert_eq!(
            issue.message,
            "The message body does not contain a ticket or issue number"
        );
        assert_eq!(issue.position, message_position(7, 1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   5 | Some explanation.\n\
                   6 | \n\
                   7 | Fixes #123\n\
             \x20\x20| ---------- Consider adding a reference to a ticket or issue\n"
        );
    }
}
