use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

pub struct RebaseCommit {}

impl RebaseCommit {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for RebaseCommit {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let subject = &commit.subject;
        let fixup_check = validate_prefix("fixup", subject);
        if fixup_check.is_some() {
            return fixup_check;
        }
        let squash_check = validate_prefix("squash", subject);
        if squash_check.is_some() {
            return squash_check;
        }
        let amend_check = validate_prefix("amend", subject);
        if amend_check.is_some() {
            return amend_check;
        }

        None
    }
}

fn validate_prefix(prefix: &str, subject: &str) -> Option<Vec<Issue>> {
    if subject.starts_with(&format!("{}! ", prefix)) {
        let context = Context::subject_error(
            subject.to_string(),
            Range {
                start: 0,
                end: prefix.len() + 1,
            },
            format!("Rebase {} commits before pushing or merging", prefix),
        );
        Some(vec![Issue::error(
            Rule::RebaseCommit,
            format!("A {} commit was found", prefix),
            Position::Subject { line: 1, column: 1 },
            vec![context],
        )])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        RebaseCommit::new().validate(commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    #[test]
    fn with_valid_subjects() {
        assert_subject_as_valid("I don't need a rebase");
    }

    #[test]
    fn with_fixup_commit() {
        let issue = first_issue(validate(&commit("fixup! I need a rebase", "")));
        assert_eq!(issue.message, "A fixup commit was found");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | fixup! I need a rebase\n\
               | ^^^^^^ Rebase fixup commits before pushing or merging",
        );
    }

    #[test]
    fn with_squash_commit() {
        let issue = first_issue(validate(&commit("squash! I need a rebase", "")));
        assert_eq!(issue.message, "A squash commit was found");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | squash! I need a rebase\n\
               | ^^^^^^^ Rebase squash commits before pushing or merging",
        );
    }

    #[test]
    fn with_amend_commit() {
        let issue = first_issue(validate(&commit("amend! I need a rebase", "")));
        assert_eq!(issue.message, "A amend commit was found");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | amend! I need a rebase\n\
               | ^^^^^^ Rebase amend commits before pushing or merging",
        );
    }
}
