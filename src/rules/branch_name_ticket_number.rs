use core::ops::Range;

use crate::branch::Branch;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref BRANCH_WITH_TICKET_NUMBER: Regex = {
        let mut tempregex = RegexBuilder::new(r"^(\w+[-_/\.])?\d{2,}([-_/\.]\w+)?([-_/\.]\w+)?");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

pub struct BranchNameTicketNumber {}

impl BranchNameTicketNumber {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Branch> for BranchNameTicketNumber {
    fn validate(&self, branch: &Branch) -> Option<Vec<Issue>> {
        let name = &branch.name;
        if let Some(captures) = BRANCH_WITH_TICKET_NUMBER.captures(name) {
            let valid = match (captures.get(1), captures.get(2), captures.get(3)) {
                (None, None, _) => false,
                (Some(_prefix), None, _) => false,
                (None, Some(_suffix), None) => false,
                (None, Some(_suffix), Some(_suffix_more)) => true,
                (Some(_prefix), Some(_suffix), _) => true,
            };
            if !valid {
                let context = vec![Context::branch_removal_suggestion(
                    name.to_string(),
                    Range {
                        start: 0,
                        end: name.len(),
                    },
                    "Remove the ticket number from the branch name or expand the branch name with more details".to_string(),
                )];
                return Some(vec![Issue::error(
                    Rule::BranchNameTicketNumber,
                    "A ticket number was detected in the branch name".to_string(),
                    Position::Branch { column: 1 },
                    context,
                )]);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(branch: &Branch) -> Option<Vec<Issue>> {
        BranchNameTicketNumber::new().validate(branch)
    }

    fn assert_valid(name: &str) {
        assert_eq!(validate(&branch(name)), None);
    }

    fn assert_invalid(name: &str) {
        assert!(validate(&branch(name)).is_some());
    }

    #[test]
    fn valid_names() {
        let names = vec![
            "123-fix-bug",
            "123_fix-bug",
            "123/fix-bug",
            "123-add-feature",
            "fix-123-bug",
            "fix_123-bug",
            "fix/123-bug",
            "feature-123-cool",
            "add-feature-123",
            "add-feature-123-cool",
            "ruby-3",
            "elixir-1.13.2-ci",
            "erlang-20.2",
            "fix-bug",
        ];
        for name in names {
            assert_valid(name);
        }
    }

    #[test]
    fn invalid_names() {
        let names = vec![
            "123",
            "123-FIX",
            "123-Fix",
            "123-fix",
            "123_fix",
            "123/fix",
            "123-feature",
            "FIX-123",
            "Fix-123",
            "fix-123",
            "fix_123",
            "fix/123",
            "feature-123",
            "JIRA-123",
        ];
        for name in names {
            assert_invalid(name);
        }
    }

    #[test]
    fn with_ticket_number() {
        let issue = first_issue(validate(&branch("fix-123")));
        assert_eq!(
            issue.message,
            "A ticket number was detected in the branch name"
        );
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_contains_issue_output(
            &issue,
            "| fix-123\n\
             | ------- Remove the ticket number from the branch name or expand the branch name with more details"
        );
    }
}
