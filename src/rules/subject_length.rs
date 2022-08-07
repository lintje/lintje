use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::line_length_stats;

pub struct SubjectLength {}

impl SubjectLength {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for SubjectLength {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if commit.has_issue(&Rule::SubjectCliche) {
            return None;
        }

        let (width, line_stats) = line_length_stats(&commit.subject, 50);

        match width {
            0 => {
                let context = Context::subject_error(
                    commit.subject.to_string(),
                    Range { start: 0, end: 1 },
                    "Add a subject to describe the change".to_string(),
                );
                Some(vec![Issue::error(
                    Rule::SubjectLength,
                    "The commit has no subject".to_string(),
                    Position::Subject { line: 1, column: 1 },
                    vec![context],
                )])
            }
            x if x > 50 => {
                let total_width_index = commit.subject.len();
                let context = Context::subject_error(
                    commit.subject.to_string(),
                    Range {
                        start: line_stats.bytes_index,
                        end: total_width_index,
                    },
                    "Shorten the subject to a maximum width of 50 characters".to_string(),
                );
                Some(vec![Issue::error(
                    Rule::SubjectLength,
                    format!("The subject of `{}` characters wide is too long", width),
                    Position::Subject {
                        line: 1,
                        column: line_stats.char_count + 1,
                    },
                    vec![context],
                )])
            }
            x if x < 5 => {
                let total_width_index = commit.subject.len();
                let context = Context::subject_error(
                    commit.subject.to_string(),
                    Range {
                        start: 0,
                        end: total_width_index,
                    },
                    "Describe the change in more detail".to_string(),
                );
                Some(vec![Issue::error(
                    Rule::SubjectLength,
                    format!("The subject of `{}` characters wide is too short", width),
                    Position::Subject { line: 1, column: 1 },
                    vec![context],
                )])
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::Rule;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        SubjectLength::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    fn assert_subject_as_invalid(subject: &str) {
        assert!(validate(&commit(subject, "")).is_some());
    }

    #[test]
    fn with_valid_subjects() {
        assert_subject_as_valid("I don't need a rebase");
    }

    #[test]
    fn with_valid_lengths() {
        assert_subject_as_valid(&"a".repeat(5));
        assert_subject_as_valid(&"a".repeat(50));
    }

    #[test]
    fn with_cliche_subject() {
        let mut wip_commit = commit("wip", "");
        wip_commit.issues.push(Issue::error(
            Rule::SubjectCliche,
            "some message".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![],
        ));
        let issues = validate(&wip_commit);
        // Already a SubjectCliche issue, so it's skipped.
        assert_eq!(issues, None);
    }

    #[test]
    fn with_empty_subject() {
        let issue = first_issue(validate(&commit("", "")));
        assert_eq!(issue.message, "The commit has no subject");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | \n\
               | ^ Add a subject to describe the change",
        );
    }

    #[test]
    fn with_short_subject() {
        let issue = first_issue(validate(&commit("a".repeat(4).as_str(), "")));
        assert_eq!(
            issue.message,
            "The subject of `4` characters wide is too short"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | aaaa\n\
               | ^^^^ Describe the change in more detail",
        );
    }

    #[test]
    fn with_too_long_subject() {
        let issue = first_issue(validate(&commit("a".repeat(51).as_str(), "")));
        assert_eq!(
            issue.message,
            "The subject of `51` characters wide is too long"
        );
        assert_eq!(issue.position, subject_position(51));
        assert_contains_issue_output(
            &issue,
            "1 | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
               |                                                   ^ Shorten the subject to a maximum width of 50 characters",
        );
    }

    #[test]
    fn with_short_unicode_subject() {
        // Character is two characters, but is counted as 1 column
        assert_eq!("ö̲".chars().count(), 2);
        let issue = first_issue(validate(&commit("A ö̲", "")));
        assert_eq!(
            issue.message,
            "The subject of `3` characters wide is too short"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | A ö̲\n\
               | ^^^ Describe the change in more detail",
        );
    }

    #[test]
    fn with_short_emoji_subject() {
        let issue = first_issue(validate(&commit("👁️‍🗨️", "")));
        assert_eq!(
            issue.message,
            "The subject of `2` characters wide is too short"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | 👁️‍🗨️\n\
               | ^^ Describe the change in more detail",
        );
    }

    #[test]
    fn with_unicode_subjects() {
        // These emoji display width is 2
        assert_subject_as_valid(&"✨".repeat(25));
        assert_subject_as_invalid(&"✨".repeat(26));
        // Hiragana display width is 2
        assert_subject_as_valid(&"あ".repeat(25));
    }

    #[test]
    fn with_long_unicode_subject() {
        let issue = first_issue(validate(&commit("あ".repeat(26).as_str(), "")));
        assert_eq!(
            issue.message,
            "The subject of `52` characters wide is too long"
        );
        assert_eq!(issue.position, subject_position(26));
        assert_contains_issue_output(
            &issue,
            "1 | ああああああああああああああああああああああああああ\n\
               |                                                   ^^ Shorten the subject to a maximum width of 50 characters",
        );
    }
}
