use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::{character_count_for_bytes_index, is_punctuation};

lazy_static! {
    // Regex to match emoji, but not all emoji. Emoji using ASCII codepoints like the emojis for
    // the numbers 0-9, and symbols like * and # are not included. Otherwise it would also catches
    // plain numbers 0-9 and those symbols, even when they are not emoji.
    // This regex matches all emoji but subtracts any object with ASCII codepoints.
    // For more information, see:
    // https://github.com/BurntSushi/ripgrep/discussions/1623#discussioncomment-28827
    static ref SUBJECT_STARTS_WITH_EMOJI: Regex = Regex::new(r"^[\p{Emoji}--\p{Ascii}]").unwrap();
}

pub struct SubjectPunctuation {}

impl SubjectPunctuation {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for SubjectPunctuation {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if commit.subject.chars().count() == 0 && commit.has_issue(&Rule::SubjectLength) {
            return None;
        }

        let mut issues = vec![];

        if let Some(emoji) = SUBJECT_STARTS_WITH_EMOJI.find(&commit.subject) {
            let context = vec![Context::subject_removal_suggestion(
                commit.subject.to_string(),
                emoji.range(),
                "Remove emoji from the start of the subject".to_string(),
            )];
            issues.push(Issue::error(
                Rule::SubjectPunctuation,
                "The subject starts with an emoji".to_string(),
                Position::Subject { line: 1, column: 1 },
                context,
            ));
        }

        match commit.subject.chars().next() {
            Some(character) => {
                if is_punctuation(character) {
                    let context = vec![Context::subject_removal_suggestion(
                        commit.subject.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Remove punctuation from the start of the subject".to_string(),
                    )];
                    issues.push(Issue::error(
                        Rule::SubjectPunctuation,
                        format!(
                            "The subject starts with a punctuation character: `{}`",
                            character
                        ),
                        Position::Subject { line: 1, column: 1 },
                        context,
                    ));
                }
            }
            None => {
                error!(
                    "SubjectPunctuation validation failure: No first character found of subject."
                );
            }
        }

        match commit.subject.chars().last() {
            Some(character) => {
                if is_punctuation(character) {
                    let subject_length = commit.subject.len();
                    let context = Context::subject_removal_suggestion(
                        commit.subject.to_string(),
                        Range {
                            start: subject_length - character.len_utf8(),
                            end: subject_length,
                        },
                        "Remove punctuation from the end of the subject".to_string(),
                    );
                    issues.push(Issue::error(
                        Rule::SubjectPunctuation,
                        format!(
                            "The subject ends with a punctuation character: `{}`",
                            character
                        ),
                        Position::Subject {
                            line: 1,
                            column: character_count_for_bytes_index(
                                &commit.subject,
                                subject_length - character.len_utf8(),
                            ),
                        },
                        vec![context],
                    ));
                }
            }
            None => {
                error!(
                    "SubjectPunctuation validation failure: No last character found of subject."
                );
            }
        }

        if issues.is_empty() {
            None
        } else {
            Some(issues)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        SubjectPunctuation::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    fn assert_subject_as_invalid(subject: &str) {
        assert!(validate(&commit(subject, "")).is_some());
    }

    #[test]
    fn valid_subjects() {
        let subjects = vec![
            "Fix test",
            "あ commit",
            "123 digits",
            "0 digit",
            // These should not be allowed, but won't match using the Emoji -- ASCII regex matcher.
            // See the comment for SUBJECT_STARTS_WITH_EMOJI for more information.
            "0️⃣ emoji",
            "﹟emoji",
            "＊emoji",
        ];
        for subject in subjects {
            assert_subject_as_valid(subject);
        }
    }

    #[test]
    fn invalid_subjects() {
        let subjects = vec![
            "Fix test.",
            "Fix test!",
            "Fix test?",
            "Fix test:",
            "Fix test\'",
            "Fix test\"",
            "Fix test…",
            "Fix test⋯",
            ".Fix test",
            "!Fix test",
            "?Fix test",
            ":Fix test",
            "…Fix test",
            "⋯Fix test",
            "📺Fix test",
            "👍Fix test",
            "👍🏻Fix test",
            "[JIRA-123] Fix test",
            "[Bug] Fix test",
            "[chore] Fix test",
            "[feat] Fix test",
            "(feat) Fix test",
            "{fix} Fix test",
            "|fix| Fix test",
            "-fix- Fix test",
            "+fix+ Fix test",
            "*fix* Fix test",
            "%fix% Fix test",
            "@fix Fix test",
        ];
        for subject in subjects {
            assert_subject_as_invalid(subject);
        }
    }

    #[test]
    fn punctuation_at_start() {
        let issue = first_issue(validate(&commit(".Fix test", "")));
        assert_eq!(
            issue.message,
            "The subject starts with a punctuation character: `.`"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | .Fix test\n\
               | - Remove punctuation from the start of the subject",
        );
    }

    #[test]
    fn punctuation_at_end() {
        let issue = first_issue(validate(&commit("Fix test…", "")));
        assert_eq!(
            issue.message,
            "The subject ends with a punctuation character: `…`"
        );
        assert_eq!(issue.position, subject_position(9));
        assert_contains_issue_output(
            &issue,
            "1 | Fix test…\n\
               |         - Remove punctuation from the end of the subject",
        );
    }

    #[test]
    fn emoji_at_start() {
        let issue = first_issue(validate(&commit("👍 Fix test", "")));
        assert_eq!(issue.message, "The subject starts with an emoji");
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | 👍 Fix test\n\
               | -- Remove emoji from the start of the subject",
        );
    }

    #[test]
    fn multiple_issues() {
        let issues = validate(&commit(".Fix test.", "")).expect("No issues");
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn skipped_length() {
        let mut empty_commit = commit("", "");
        empty_commit.issues.push(Issue::error(
            Rule::SubjectLength,
            "some message".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![],
        ));
        // Already a empty SubjectLength issue, so it's skipped
        assert!(empty_commit.has_issue(&Rule::SubjectLength));
        assert!(!empty_commit.has_issue(&Rule::SubjectPunctuation));
    }
}
