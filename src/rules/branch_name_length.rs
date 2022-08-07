use core::ops::Range;

use crate::branch::Branch;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::display_width;

pub struct BranchNameLength {}

impl BranchNameLength {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Branch> for BranchNameLength {
    fn validate(&self, branch: &Branch) -> Option<Vec<Issue>> {
        let name = &branch.name;
        let width = display_width(name);
        if width < 4 {
            let context = vec![Context::branch_error(
                name.to_string(),
                Range {
                    start: 0,
                    end: name.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            Some(vec![Issue::error(
                Rule::BranchNameLength,
                format!("Branch name of {} characters is too short", width),
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
        BranchNameLength::new().validate(&branch)
    }

    fn assert_valid(name: &str) {
        assert_eq!(validate(&branch(name)), None)
    }

    fn assert_invalid(name: &str) {
        assert!(validate(&branch(name)).is_some())
    }

    #[test]
    fn valid_names() {
        let names = vec![
            "abcd".to_string(),
            "-_/!".to_string(),
            "a".repeat(5),
            "a".repeat(50),
            "あ".repeat(4),
            "✨".repeat(4),
        ];
        for name in names {
            assert_valid(&name);
        }
    }

    #[test]
    fn invalid_names() {
        let names = vec!["", "a", "ab", "abc"];
        for name in names {
            assert_invalid(name);
        }
    }

    #[test]
    fn too_short_name() {
        let issue = first_issue(validate(&branch("abc")));
        assert_eq!(issue.message, "Branch name of 3 characters is too short");
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_contains_issue_output(
            &issue,
            "| abc\n\
             | ^^^ Describe the change in more detail",
        );
    }
}
