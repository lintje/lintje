use core::ops::Range;

use crate::branch::Branch;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref BRANCH_WITH_CLICHE: Regex = {
        let mut tempregex = RegexBuilder::new(
            r"^(wip|fix(es|ed|ing)?|add(s|ed|ing)?|(updat|chang|remov|delet)(e|es|ed|ing))([-_/]+\w+)?$",
        );
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

pub struct BranchNameCliche {}

impl BranchNameCliche {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Branch> for BranchNameCliche {
    fn validate(&self, branch: &Branch) -> Option<Vec<Issue>> {
        let name = &branch.name.to_lowercase();
        if BRANCH_WITH_CLICHE.is_match(name) {
            let context = vec![Context::branch_error(
                name.to_string(),
                Range {
                    start: 0,
                    end: name.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            Some(vec![Issue::error(
                Rule::BranchNameCliche,
                "The branch name does not explain the change in much detail".to_string(),
                Position::Branch { column: 1 },
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

    fn validate(branch: &Branch) -> Option<Vec<Issue>> {
        BranchNameCliche::new().validate(&branch)
    }

    fn assert_valid(name: &str) {
        assert_eq!(validate(&branch(name)), None)
    }

    fn assert_invalid(name: &str) {
        assert!(validate(&branch(name)).is_some())
    }

    #[test]
    fn valid_names() {
        let names = vec!["add-email-validation", "fix-brittle-test"];
        for name in names {
            assert_valid(&name);
        }
    }

    #[test]
    fn invalid_names() {
        let prefixes = vec![
            "wip", "fix", "fixes", "fixed", "fixing", "add", "adds", "added", "adding", "update",
            "updates", "updated", "updating", "change", "changes", "changed", "changing", "remove",
            "removes", "removed", "removing", "delete", "deletes", "deleted", "deleting",
        ];
        let mut names = vec![];
        for word in prefixes.iter() {
            let uppercase_word = word.to_uppercase();
            let mut chars = word.chars();
            let capitalized_word = match chars.next() {
                None => panic!("Could not capitalize word: {}", word),
                Some(letter) => letter.to_uppercase().collect::<String>() + chars.as_str(),
            };

            names.push(uppercase_word.to_string());
            names.push(capitalized_word.to_string());
            names.push(word.to_string());
            names.push(format!("{}-test", uppercase_word));
            names.push(format!("{}-issue", capitalized_word));
            names.push(format!("{}-bug", word));
            names.push(format!("{}-readme", word));
            names.push(format!("{}-something", word));
            names.push(format!("{}_something", word));
            names.push(format!("{}/something", word));
        }
        for name in names {
            assert_invalid(&name);
        }
    }

    #[test]
    fn with_ticket_number() {
        let issue = first_issue(validate(&branch("fix-bug")));
        assert_eq!(
            issue.message,
            "The branch name does not explain the change in much detail"
        );
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_contains_issue_output(
            &issue,
            "| fix-bug\n\
             | ^^^^^^^ Describe the change in more detail",
        );
    }
}
