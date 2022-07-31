use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;

pub struct RebaseCommit {}

impl RebaseCommit {
    pub fn new() -> Self {
        Self {}
    }

    pub fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let subject = &commit.subject;
        if subject.starts_with("fixup! ") {
            let context = Context::subject_error(
                subject.to_string(),
                Range { start: 0, end: 6 },
                "Rebase fixup commits before pushing or merging".to_string(),
            );
            return Some(vec![Issue::error(
                Rule::RebaseCommit,
                "A fixup commit was found".to_string(),
                Position::Subject { line: 1, column: 1 },
                vec![context],
            )]);
        } else if subject.starts_with("squash! ") {
            let context = Context::subject_error(
                subject.to_string(),
                Range { start: 0, end: 7 },
                "Rebase squash commits before pushing or merging".to_string(),
            );
            return Some(vec![Issue::error(
                Rule::RebaseCommit,
                "A squash commit was found".to_string(),
                Position::Subject { line: 1, column: 1 },
                vec![context],
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
        RebaseCommit::new().validate(&commit)
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
}
