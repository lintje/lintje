use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::character_count_for_bytes_index;

use crate::rules::CONTAINS_FIX_TICKET;

lazy_static! {
    // Jira project keys are at least 2 uppercase characters long.
    // AB-123
    // JIRA-123
    static ref SUBJECT_WITH_TICKET: Regex = Regex::new(r"[A-Z]{2,}-\d+").unwrap();
}

pub struct SubjectTicketNumber {}

impl SubjectTicketNumber {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for SubjectTicketNumber {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let mut issues = vec![];
        let subject = &commit.subject.to_string();
        if let Some(captures) = SUBJECT_WITH_TICKET.captures(subject) {
            match captures.get(0) {
                Some(capture) => issues.push(add_subject_ticket_number_error(commit, capture)),
                None => {
                    error!(
                        "SubjectTicketNumber: Unable to fetch ticket number match from subject."
                    );
                }
            };
        }
        if let Some(captures) = CONTAINS_FIX_TICKET.captures(subject) {
            match captures.get(0) {
                Some(capture) => issues.push(add_subject_ticket_number_error(commit, capture)),
                None => {
                    error!("SubjectTicketNumber: Unable to fetch issue number match from subject.");
                }
            };
        }

        if issues.is_empty() {
            None
        } else {
            Some(issues)
        }
    }
}

fn add_subject_ticket_number_error(commit: &Commit, capture: regex::Match) -> Issue {
    let subject = commit.subject.to_string();
    let line_count = commit.message.lines().count();
    let base_line_count = if line_count == 0 { 3 } else { line_count + 2 };
    let context = vec![
        Context::subject_removal_suggestion(
            subject,
            capture.range(),
            "Remove the ticket number from the subject".to_string(),
        ),
        Context::gap(),
        Context::message_line(base_line_count, "".to_string()),
        Context::message_line_addition(
            base_line_count + 1,
            capture.as_str().to_string(),
            Range {
                start: 0,
                end: capture.range().len(),
            },
            "Move the ticket number to the message body".to_string(),
        ),
    ];
    Issue::error(
        Rule::SubjectTicketNumber,
        "The subject contains a ticket number".to_string(),
        Position::Subject {
            line: 1,
            column: character_count_for_bytes_index(&commit.subject, capture.start()),
        },
        context,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        SubjectTicketNumber::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(
            validate(&commit(subject, "")),
            None,
            "Subject not valid: {}",
            subject
        );
    }

    fn assert_subject_as_invalid(subject: &str) {
        let issues = validate(&commit(subject, ""));
        assert!(issues.is_some(), "No issues found for: {:?}", subject);
    }

    #[test]
    fn valid_subjects() {
        let subjects = vec![
            "This is a normal commit",
            "Fix #", // Not really good subjects, but won't fail on this rule
            "Fix ##123",
            "Fix #a123",
            "Fix !",
            "Fix !!123",
            "Fix !a123",
            "Fix /123",                                   // No org/repo format
            "Fix repo/123",                               // Missing org
            "Fix repo#123",                               // Missing org
            "Fix repo!123",                               // Missing org
            "Fix https://website.om/org/repo/issues#123", // No full format with only slashes
            "Fix https://website.om/org/repo/issues!123", // No full format with only slashes
            "Change A-1 config",
            "Change A-12 config",
        ];
        for subject in subjects {
            assert_subject_as_valid(subject);
        }
    }

    #[test]
    fn with_ticket_numbers() {
        let ticket_only_subjects = vec![
            "JI-1",
            "JI-12",
            "JI-1234567890",
            "JIR-1",
            "JIR-12",
            "JIR-1234567890",
            "JIRA-12",
            "JIRA-123",
            "JIRA-1234",
            "JIRA-1234567890",
            "Fix JIRA-1234 lorem",
        ];
        for subject in ticket_only_subjects {
            assert_subject_as_invalid(subject);
        }
    }

    #[test]
    fn with_keywords() {
        let invalid_subjects = vec![
            "Fix {}1234",
            "Fixed {}1234",
            "Fixes {}1234",
            "Fixing {}1234",
            "Fix {}1234 lorem",
            "Fix: {}1234 lorem",
            "Fix my-org/repo{}1234 lorem",
            "Commit fixes {}1234",
            "Close {}1234",
            "Closed {}1234",
            "Closes {}1234",
            "Closing {}1234",
            "Close {}1234 lorem",
            "Close: {}1234 lorem",
            "Commit closes {}1234",
            "Resolve {}1234",
            "Resolved {}1234",
            "Resolves {}1234",
            "Resolving {}1234",
            "Resolve {}1234 lorem",
            "Resolve: {}1234 lorem",
            "Commit resolves {}1234",
            "Implement {}1234",
            "Implemented {}1234",
            "Implements {}1234",
            "Implementing {}1234",
            "Implement {}1234 lorem",
            "Implement: {}1234 lorem",
            "Commit implements {}1234",
        ];
        let invalid_issue_subjects: Vec<String> = invalid_subjects
            .iter()
            .map(|s| s.replace("{}", "#"))
            .collect();
        for subject in invalid_issue_subjects {
            assert_subject_as_invalid(subject.as_str());
        }
        let invalid_merge_request_subjects: Vec<String> = invalid_subjects
            .iter()
            .map(|s| s.replace("{}", "!"))
            .collect();
        for subject in invalid_merge_request_subjects {
            assert_subject_as_invalid(subject.as_str());
        }
    }

    #[test]
    fn jira_ticket_number() {
        let issue = first_issue(validate(&commit("Fix JIRA-123 about email validation", "")));
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(5));
        assert_contains_issue_output(
            &issue,
            "1 | Fix JIRA-123 about email validation\n\
               |     -------- Remove the ticket number from the subject\n\
              ~~~\n\
             3 | \n\
             4 | JIRA-123\n\
               | ++++++++ Move the ticket number to the message body",
        );
    }

    #[test]
    fn jira_ticket_number_unicode() {
        let issue = first_issue(validate(&commit(
            "Fix ❤\u{fe0f} JIRA-123 about email validation",
            "",
        )));
        assert_eq!(issue.position, subject_position(7));
        assert_contains_issue_output(
            &issue,
            "1 | Fix ❤️ JIRA-123 about email validation\n\
               |       -------- Remove the ticket number from the subject\n\
              ~~~\n\
             3 | \n\
             4 | JIRA-123\n\
               | ++++++++ Move the ticket number to the message body",
        );
    }

    #[test]
    fn fix_ticket_number() {
        let issue = first_issue(validate(&commit(
            "Email validation: Fixes #123 for good",
            "",
        )));
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(19));
        assert_contains_issue_output(
            &issue,
            "1 | Email validation: Fixes #123 for good\n\
               |                   ---------- Remove the ticket number from the subject\n\
              ~~~\n\
             3 | \n\
             4 | Fixes #123\n\
               | ++++++++++ Move the ticket number to the message body",
        );
    }

    #[test]
    fn fix_ticket_number_unicode() {
        let issue = first_issue(validate(&commit("Email validatiｏn: Fixes #123", "")));
        assert_eq!(issue.position, subject_position(19));
    }

    #[test]
    fn fix_ticket_number_link_shorthand() {
        let issue = first_issue(validate(&commit(
            "Email validation: Closed org/repo#123 for good",
            "",
        )));
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(19));
        assert_contains_issue_output(
            &issue,
            "1 | Email validation: Closed org/repo#123 for good\n\
               |                   ------------------- Remove the ticket number from the subject\n\
              ~~~\n\
             3 | \n\
             4 | Closed org/repo#123\n\
               | +++++++++++++++++++ Move the ticket number to the message body",
        );
    }

    #[test]
    fn fix_ticket_number_link() {
        let issue = first_issue(validate(&commit(
            "Email validation: Closes https://website.com:80/org/repo/issues/123 for good",
            "",
        )));
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(19));
        assert_contains_issue_output(
            &issue,
            "1 | Email validation: Closes https://website.com:80/org/repo/issues/123 for good\n\
               |                   ------------------------------------------------- Remove the ticket number from the subject\n\
              ~~~\n\
             3 | \n\
             4 | Closes https://website.com:80/org/repo/issues/123\n\
               | +++++++++++++++++++++++++++++++++++++++++++++++++ Move the ticket number to the message body",
        );
    }

    #[test]
    fn multiple_issues() {
        let issues = validate(&commit("Fix #123 JIRA-123", "")).expect("No issues");
        assert_eq!(issues.len(), 2);
    }
}
