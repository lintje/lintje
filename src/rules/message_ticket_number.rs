use core::ops::Range;
use regex::{Regex, RegexBuilder};

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

use crate::rules::CONTAINS_FIX_TICKET;

lazy_static! {
    // Match "Part of #123"
    static ref LINK_TO_TICKET: Regex = {
        let mut tempregex = RegexBuilder::new(r"(part of|part of (issue|epic|project)|related):? ([^\s]*[\w\-_/]+)?[#!]{1}\d+");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

pub struct MessageTicketNumber {}

impl MessageTicketNumber {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessageTicketNumber {
    fn dependent_rules(&self) -> Option<Vec<Rule>> {
        None
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
        assert_eq!(
            validate(&commit("Subject", message)),
            None,
            "Message is not valid: {}",
            message
        );
    }

    fn assert_invalid(message: &str) {
        assert!(
            validate(&commit("Subject", message)).is_some(),
            "Message is valid: {}",
            message
        );
    }

    fn assert_message_valid(message: &str) {
        let message = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            message,
            "",
            "Lorem ipsum",
        ]
        .join("\n");
        assert_valid(&message);
    }

    fn assert_message_invalid(message: &str) {
        let message = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            message,
            "",
            "Lorem ipsum",
        ]
        .join("\n");
        assert_invalid(&message);
    }

    #[test]
    fn message_with_fix_issue() {
        assert_message_valid("fix #123");
        assert_message_valid("Fix #123");
        assert_message_valid("Fixes #123");
        assert_message_valid("Fixes: #123");
        assert_message_valid("Fixes org/repo#123");
        assert_message_valid("Fixed org/repo!123");
        assert_message_valid("Fixes https://website.om/org/repo/issues/123");
    }

    #[test]
    fn message_with_ticket_number_part_of() {
        assert_message_valid("part of #123");
        assert_message_valid("Part of #123");
        assert_message_valid("Part of: #123");
        assert_message_valid("related #123");
        assert_message_valid("Related #123");
        assert_message_valid("Related: #123");
    }

    #[test]
    fn message_with_ticket_number_part_of_issue() {
        let types = ["issue", "epic", "project"];
        for reference_type in types {
            assert_message_valid(&format!("part of {}: #123", reference_type));
            assert_message_valid(&format!("Part of {}: #123", reference_type));
            assert_message_valid(&format!("part of {} #123", reference_type));
            assert_message_valid(&format!("Part of {} #123", reference_type));
        }
    }

    #[test]
    fn message_without_fix_issue() {
        assert_message_invalid("Fix /123");
        assert_message_invalid("Fixes repo#123");
        assert_message_invalid("Fixed repo!123");
        assert_message_invalid("Fixes https://website.om/org/repo/issues#123");
        assert_message_invalid("Fixes https://website.om/org/repo/issues!123");
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
        assert_contains_issue_output(
            &issue,
            "5 | Some explanation.\n\
             6 | \n\
             7 | Fixes #123\n\
               | ++++++++++ Consider adding a reference to a ticket or issue",
        );
    }
}
