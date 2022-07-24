use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::{Rule, RuleValidation};

lazy_static! {
    static ref SUBJECT_STARTS_WITH_PREFIX: Regex = Regex::new(r"^([\w\(\)/!]+:)\s.*").unwrap();
}

pub struct SubjectCapitalization {}

impl RuleValidation for SubjectCapitalization {
    fn new() -> Self {
        Self {}
    }

    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if commit.has_issue(&Rule::SubjectPrefix) {
            return None;
        }
        if commit.subject.chars().count() == 0 && commit.has_issue(&Rule::SubjectLength) {
            return None;
        }

        match commit.subject.chars().next() {
            Some(character) => {
                if character.is_lowercase() {
                    let context = vec![Context::subject_error(
                        commit.subject.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Start the subject with a capital letter".to_string(),
                    )];
                    Some(vec![Issue::error(
                        Rule::SubjectCapitalization,
                        "The subject does not start with a capital letter".to_string(),
                        Position::Subject { line: 1, column: 1 },
                        context,
                    )])
                } else {
                    None
                }
            }
            None => {
                error!("SubjectCapitalization validation failure: No first character found of subject.");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        SubjectCapitalization::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    #[test]
    fn valid_subjects() {
        assert_subject_as_valid("Fix test");
    }

    #[test]
    fn starting_with_lowercase() {
        let issue = first_issue(validate(&commit("fix test", "")));
        assert_eq!(
            issue.message,
            "The subject does not start with a capital letter"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | fix test\n\
             \x20\x20| ^ Start the subject with a capital letter\n"
        );
    }

    #[test]
    fn skipped_length() {
        let mut short_commit = commit("", "");
        short_commit.issues.push(Issue::error(
            Rule::SubjectLength,
            "some message".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![],
        ));
        // Already a SubjectLength issue, so it's skipped
        assert!(short_commit.has_issue(&Rule::SubjectLength));
        assert!(!short_commit.has_issue(&Rule::SubjectCapitalization));
    }

    #[test]
    fn skipped_prefix() {
        let mut short_commit = commit("chore: foo", "");
        short_commit.issues.push(Issue::error(
            Rule::SubjectPrefix,
            "some message".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![],
        ));
        // Already a SubjectLength issue, so it's skipped
        assert!(short_commit.has_issue(&Rule::SubjectPrefix));
        assert!(!short_commit.has_issue(&Rule::SubjectCapitalization));
    }
}
