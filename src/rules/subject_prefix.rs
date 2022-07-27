use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;

lazy_static! {
    static ref SUBJECT_STARTS_WITH_PREFIX: Regex = Regex::new(r"^([\w\(\)/!]+:)\s.*").unwrap();
}

pub struct SubjectPrefix {}

impl SubjectPrefix {
    pub fn new() -> Self {
        Self {}
    }

    pub fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let subject = &commit.subject.to_string();
        if let Some(captures) = SUBJECT_STARTS_WITH_PREFIX.captures(subject) {
            // Get first match from captures, the prefix
            match captures.get(1) {
                Some(capture) => {
                    let context = vec![Context::subject_error(
                        commit.subject.to_string(),
                        capture.range(),
                        "Remove the prefix from the subject".to_string(),
                    )];
                    Some(vec![Issue::error(
                        Rule::SubjectPrefix,
                        format!("Remove the `{}` prefix from the subject", capture.as_str()),
                        Position::Subject { line: 1, column: 1 },
                        context,
                    )])
                }
                None => {
                    error!("SubjectPrefix: Unable to fetch prefix capture from subject.");
                    None
                }
            }
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
        SubjectPrefix::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    fn assert_subject_as_invalid(subject: &str) {
        assert!(validate(&commit(subject, "")).is_some());
    }

    #[test]
    fn valid_subjects() {
        assert_subject_as_valid("This is a commit without prefix");
    }

    #[test]
    fn invalid_subjects() {
        let subjects = vec![
            "fix: bug",
            "fix!: bug",
            "Fix: bug",
            "Fix!: bug",
            "fix(scope): bug",
            "fix(scope)!: bug",
            "Fix(scope123)!: bug",
            "fix(scope/scope): bug",
            "fix(scope/scope)!: bug",
        ];
        for subject in subjects {
            assert_subject_as_invalid(subject);
        }
    }

    #[test]
    fn with_prefix() {
        let issue = first_issue(validate(&commit("Fix: bug", "")));
        assert_eq!(issue.message, "Remove the `Fix:` prefix from the subject");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | Fix: bug\n\
               | ^^^^ Remove the prefix from the subject\n",
        );
    }

    #[test]
    fn test_validate_subject_prefix() {
        let issue = first_issue(validate(&commit("chore(package)!: some package bug", "")));
        assert_eq!(
            issue.message,
            "Remove the `chore(package)!:` prefix from the subject"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | chore(package)!: some package bug\n\
               | ^^^^^^^^^^^^^^^^ Remove the prefix from the subject",
        );
    }
}
