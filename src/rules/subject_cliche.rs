use core::ops::Range;
use regex::{Regex, RegexBuilder};

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;

lazy_static! {
    static ref SUBJECT_WITH_CLICHE: Regex = {
        let mut tempregex = RegexBuilder::new(
            r"^(fix(es|ed|ing)?|add(s|ed|ing)?|(updat|chang|remov|delet)(e|es|ed|ing))(\s+\w+)?$",
        );
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

pub struct SubjectCliche {}

impl SubjectCliche {
    pub fn new() -> Self {
        Self {}
    }

    pub fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let subject = &commit.subject.to_lowercase();
        let wip_commit = subject.starts_with("wip ") || subject == &"wip".to_string();
        if wip_commit || SUBJECT_WITH_CLICHE.is_match(subject) {
            let context = vec![Context::subject_error(
                commit.subject.to_string(),
                Range {
                    start: 0,
                    end: commit.subject.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            Some(vec![Issue::error(
                Rule::SubjectCliche,
                "The subject does not explain the change in much detail".to_string(),
                Position::Subject { line: 1, column: 1 },
                context,
            )])
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
        SubjectCliche::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    fn assert_subject_as_invalid(subject: &str) {
        assert!(validate(&commit(subject, "")).is_some());
    }

    #[test]
    fn with_valid_subjects() {
        let subjects = vec![
            "I am not a cliche",
            "Fix user bug",
            "Fix test for some feature",
            "Fix bug for some feature",
            "Fixes bug for some feature",
            "Fixed bug for some feature",
            "Fixing bug for some feature",
        ];
        for subject in subjects {
            assert_subject_as_valid(subject);
        }
    }

    #[test]
    fn wip_commit() {
        let issue = first_issue(validate(&commit("WIP", "")));
        assert_eq!(
            issue.message,
            "The subject does not explain the change in much detail"
        );
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | WIP\n\
               | ^^^ Describe the change in more detail",
        );
    }

    #[test]
    fn cliche_subjects() {
        let prefixes = vec![
            "wip", "fix", "fixes", "fixed", "fixing", "add", "adds", "added", "adding", "update",
            "updates", "updated", "updating", "change", "changes", "changed", "changing", "remove",
            "removes", "removed", "removing", "delete", "deletes", "deleted", "deleting",
        ];
        let mut invalid_subjects = vec![];
        for word in prefixes.iter() {
            let uppercase_word = word.to_uppercase();
            let mut chars = word.chars();
            let capitalized_word = match chars.next() {
                None => panic!("Could not capitalize word: {}", word),
                Some(letter) => letter.to_uppercase().collect::<String>() + chars.as_str(),
            };

            invalid_subjects.push(uppercase_word.to_string());
            invalid_subjects.push(capitalized_word.to_string());
            invalid_subjects.push(word.to_string());
            invalid_subjects.push(format!("{} test", uppercase_word));
            invalid_subjects.push(format!("{} issue", capitalized_word));
            invalid_subjects.push(format!("{} bug", word));
            invalid_subjects.push(format!("{} readme", word));
            invalid_subjects.push(format!("{} something", word));
        }
        for subject in invalid_subjects {
            assert_subject_as_invalid(subject.as_str());
        }
    }

    #[test]
    fn cliche_subject_detail() {
        let issue = first_issue(validate(&commit("Fixed bug", "")));
        assert_eq!(issue.position, subject_position(1));
        assert_contains_issue_output(
            &issue,
            "1 | Fixed bug\n\
               | ^^^^^^^^^ Describe the change in more detail",
        );
    }
}
