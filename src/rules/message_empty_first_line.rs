use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

pub struct MessageEmptyFirstLine {}

impl MessageEmptyFirstLine {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessageEmptyFirstLine {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if let Some(line) = commit.message.lines().next() {
            if !line.is_empty() {
                let context = vec![
                    Context::subject(commit.subject.to_string()),
                    Context::message_line_addition(
                        2,
                        "".to_string(),
                        Range { start: 0, end: 3 },
                        "Add an empty line below the subject line".to_string(),
                    ),
                    Context::message_line(3, line.to_string()),
                ];
                return Some(vec![Issue::error(
                    Rule::MessageEmptyFirstLine,
                    "No empty line found below the subject".to_string(),
                    Position::MessageLine { line: 2, column: 1 },
                    context,
                )]);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MessageEmptyFirstLine::new().validate(commit)
    }

    #[test]
    fn with_empty_line() {
        let with_empty_line = commit(
            "Subject".to_string(),
            "\nEmpty line after subject.".to_string(),
        );
        assert_eq!(validate(&with_empty_line), None);
    }

    #[test]
    fn without_empty_line() {
        let without_empty_line = commit("Subject", "No empty line after subject");
        let issue = first_issue(validate(&without_empty_line));
        assert_eq!(issue.message, "No empty line found below the subject");
        assert_eq!(issue.position, message_position(2, 1));
        assert_contains_issue_output(
            &issue,
            "1 | Subject\n\
             2 |\n\
               | +++ Add an empty line below the subject line\n\
             3 | No empty line after subject\n",
        );
    }
}
