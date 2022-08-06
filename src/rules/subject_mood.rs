use core::ops::Range;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

const MOOD_WORDS: [&str; 41] = [
    "fixed",
    "fixes",
    "fixing",
    "solved",
    "solves",
    "solving",
    "resolved",
    "resolves",
    "resolving",
    "closed",
    "closes",
    "closing",
    "added",
    "adding",
    "updated",
    "updates",
    "updating",
    "removed",
    "removes",
    "removing",
    "deleted",
    "deletes",
    "deleting",
    "changed",
    "changes",
    "changing",
    "moved",
    "moves",
    "moving",
    "refactored",
    "refactors",
    "refactoring",
    "checked",
    "checks",
    "checking",
    "adjusted",
    "adjusts",
    "adjusting",
    "tests",
    "tested",
    "testing",
];

pub struct SubjectMood {}

impl SubjectMood {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for SubjectMood {
    fn dependent_rules(&self) -> Option<Vec<Rule>> {
        None
    }

    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        match commit.subject.split(' ').next() {
            Some(raw_word) => {
                let word = raw_word.to_lowercase();
                if MOOD_WORDS.contains(&word.as_str()) {
                    let context = vec![Context::subject_error(
                        commit.subject.to_string(),
                        Range {
                            start: 0,
                            end: word.len(),
                        },
                        "Use the imperative mood for the subject".to_string(),
                    )];
                    Some(vec![Issue::error(
                        Rule::SubjectMood,
                        "The subject does not use the imperative grammatical mood".to_string(),
                        Position::Subject { line: 1, column: 1 },
                        context,
                    )])
                } else {
                    None
                }
            }
            None => {
                error!("SubjectMood validation failure: No first word found of commit subject.");
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
        SubjectMood::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    fn assert_subject_as_invalid(subject: &str) {
        assert!(validate(&commit(subject, "")).is_some());
    }

    #[test]
    fn with_valid_subjects() {
        assert_subject_as_valid("Fix test");
        assert_subject_as_valid("Fix tests");
    }

    #[test]
    fn with_flagged_mood_words() {
        let mut invalid_subjects = vec![];
        for word in MOOD_WORDS.iter() {
            invalid_subjects.push(format!("{} test", word));
            let mut chars = word.chars();
            let capitalized_word = match chars.next() {
                None => panic!("Could not capitalize word: {}", word),
                Some(letter) => letter.to_uppercase().collect::<String>() + chars.as_str(),
            };
            invalid_subjects.push(format!("{} test", capitalized_word));
        }
        for subject in invalid_subjects {
            assert_subject_as_invalid(subject.as_str());
        }
    }

    #[test]
    fn with_cliche_subject() {
        let issue = first_issue(validate(&commit("Fixing bug", "")));
        assert_eq!(
            issue.message,
            "The subject does not use the imperative grammatical mood"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | Fixing bug\n\
               | ^^^^^^ Use the imperative mood for the subject",
        );
    }
}
