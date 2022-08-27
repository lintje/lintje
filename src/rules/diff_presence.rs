use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

pub struct DiffPresence {}

impl DiffPresence {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for DiffPresence {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if commit.has_changes() {
            return None;
        }

        let context_line = "0 files changed, 0 insertions(+), 0 deletions(-)".to_string();
        let context_length = context_line.len();
        let context = Context::diff_error(
            context_line,
            Range {
                start: 0,
                end: context_length,
            },
            "Add changes to the commit or remove the commit".to_string(),
        );
        Some(vec![Issue::error(
            Rule::DiffPresence,
            "No file changes found".to_string(),
            Position::Diff,
            vec![context],
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        DiffPresence::new().validate(commit)
    }

    fn commit_without_file_changes(message: String) -> Commit {
        Commit::new(
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            Some("test@example.com".to_string()),
            "Some subject",
            message,
            "".to_string(),
            vec![],
        )
    }

    #[test]
    fn with_changes() {
        let issues = validate(&commit(
            "Subject".to_string(),
            "\nSome message.".to_string(),
        ));
        assert_eq!(issues, None);
    }

    #[test]
    fn without_changes() {
        let issue = first_issue(validate(&commit_without_file_changes(
            "\nSome Message".to_string(),
        )));
        assert_eq!(issue.message, "No file changes found");
        assert_eq!(issue.position, Position::Diff);
        assert_contains_issue_output(
            &issue,
            "| 0 files changed, 0 insertions(+), 0 deletions(-)\n\
             | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Add changes to the commit or remove the commit"
        );
    }
}
