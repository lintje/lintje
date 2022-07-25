use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;

pub struct SubjectWhitespace {}

impl SubjectWhitespace {
    pub fn new() -> Self {
        Self {}
    }

    pub fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if commit.subject.chars().count() == 0 && commit.has_issue(&Rule::SubjectLength) {
            return None;
        }

        match commit.subject.chars().next() {
            Some(character) => {
                if character.is_whitespace() {
                    let context = vec![Context::subject_error(
                        commit.subject.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Remove the leading whitespace from the subject".to_string(),
                    )];
                    Some(vec![Issue::error(
                        Rule::SubjectWhitespace,
                        "The subject starts with a whitespace character such as a space or a tab"
                            .to_string(),
                        Position::Subject { line: 1, column: 1 },
                        context,
                    )])
                } else {
                    None
                }
            }
            None => {
                error!(
                    "SubjectWhitespace validation failure: No first character found of subject."
                );
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
        SubjectWhitespace::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    #[test]
    fn skipped_when_empty() {
        // Rule is ignored because the subject is empty, a SubjectLength issue
        let mut empty_commit = commit("", "");
        empty_commit.issues.push(Issue::error(
            Rule::SubjectLength,
            "some message".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![],
        ));
        let issues = validate(&empty_commit);
        assert_eq!(issues, None);
        assert!(empty_commit.has_issue(&Rule::SubjectLength));
    }

    #[test]
    fn without_whitespace() {
        assert_subject_as_valid("Fix test");
    }

    #[test]
    fn with_withspace_at_start() {
        let issue = first_issue(validate(&commit(" Fix test", "")));
        assert_eq!(
            issue.message,
            "The subject starts with a whitespace character such as a space or a tab"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 |  Fix test\n\
             \x20\x20| ^ Remove the leading whitespace from the subject\n"
        );
    }

    #[test]
    fn with_withspace2_at_start() {
        let issue = first_issue(validate(&commit("\x20Fix test", "")));
        assert_eq!(
            issue.message,
            "The subject starts with a whitespace character such as a space or a tab"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | \x20Fix test\n\
             \x20\x20| ^ Remove the leading whitespace from the subject\n"
        );
    }

    #[test]
    fn with_tab_at_start() {
        let issue = first_issue(validate(&commit("\tFix test", "")));
        assert_eq!(
            issue.message,
            "The subject starts with a whitespace character such as a space or a tab"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 |     Fix test\n\
             \x20\x20| ^^^^ Remove the leading whitespace from the subject\n"
        );
    }
}
