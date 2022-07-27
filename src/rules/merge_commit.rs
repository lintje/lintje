use core::ops::Range;

use crate::commit::Commit;
use crate::git::SUBJECT_WITH_MERGE_REMOTE_BRANCH;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;

pub struct MergeCommit {}

impl MergeCommit {
    pub fn new() -> Self {
        Self {}
    }

    // Note: Some merge commits are ignored in git.rs and won't be validated here, because they are
    // Pull/Merge Requests, which are valid.
    pub fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let subject = &commit.subject;
        if !SUBJECT_WITH_MERGE_REMOTE_BRANCH.is_match(subject) {
            return None;
        }

        let subject_length = subject.len();
        let context = Context::subject_error(
            subject.to_string(),
            Range { start: 0, end: subject_length },
            "Rebase on the remote branch, rather than merging the remote branch into the local branch".to_string(),
        );
        Some(vec![Issue::error(
            Rule::MergeCommit,
            "A remote merge commit was found".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![context],
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MergeCommit::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    #[test]
    fn with_valid_subjects() {
        // Not a merge commit
        assert_subject_as_valid("I am not a merge commit");
        // Pull Request merge commit is valid
        assert_subject_as_valid("Merge pull request #123 from repo");
        // Merge into the project's defaultBranch branch
        assert_subject_as_valid("Merge branch 'develop'");
        // Merge a local branch into another local branch
        assert_subject_as_valid("Merge branch 'develop' into feature-branch");
    }

    #[test]
    fn with_remote_branch_merge_commit() {
        // Merge a remote branch into a local branch
        let issue = first_issue(validate(&commit(
            "Merge branch 'develop' of github.com/org/repo into develop",
            "",
        )));
        assert_eq!(issue.message, "A remote merge commit was found");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | Merge branch 'develop' of github.com/org/repo into develop\n\
             | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rebase on the remote branch, rather than merging the remote branch into the local branch"
        );
    }
}
