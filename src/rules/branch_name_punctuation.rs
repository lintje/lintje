use core::ops::Range;

use crate::branch::Branch;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::{character_count_for_bytes_index, is_punctuation};

pub struct BranchNamePunctuation {}

impl BranchNamePunctuation {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Branch> for BranchNamePunctuation {
    fn validate(&self, branch: &Branch) -> Option<Vec<Issue>> {
        let mut issues = vec![];
        match &branch.name.chars().next() {
            Some(character) => {
                if is_punctuation(*character) {
                    let branch = &branch.name;
                    let context = vec![Context::branch_removal_suggestion(
                        branch.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Remove punctuation from the start of the branch name".to_string(),
                    )];
                    issues.push(Issue::error(
                        Rule::BranchNamePunctuation,
                        "The branch name starts with a punctuation character".to_string(),
                        Position::Branch { column: 1 },
                        context,
                    ));
                }
            }
            None => {
                error!(
                    "BranchNamePunctuation validation failure: No first character found of branch name."
                );
            }
        }

        match &branch.name.chars().last() {
            Some(character) => {
                if is_punctuation(*character) {
                    let branch_length = branch.name.len();
                    let branch = &branch.name;
                    let context = vec![Context::branch_removal_suggestion(
                        branch.to_string(),
                        Range {
                            start: branch_length - character.len_utf8(),
                            end: branch_length,
                        },
                        "Remove punctuation from the end of the branch name".to_string(),
                    )];
                    issues.push(Issue::error(
                        Rule::BranchNamePunctuation,
                        "The branch name ends with a punctuation character".to_string(),
                        Position::Branch {
                            column: character_count_for_bytes_index(
                                branch,
                                branch.len() - character.len_utf8(),
                            ),
                        },
                        context,
                    ));
                }
            }
            None => {
                error!(
                    "BranchNamePunctuation validation failure: No last character found of branch name."
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

    fn validate(branch: &Branch) -> Option<Vec<Issue>> {
        BranchNamePunctuation::new().validate(&branch)
    }

    fn assert_valid(name: &str) {
        assert_eq!(validate(&branch(name)), None)
    }

    fn assert_invalid(name: &str) {
        assert!(validate(&branch(name)).is_some())
    }

    #[test]
    fn valid_names() {
        let names = vec!["fix-test", "fix-あ-test"];
        for name in names {
            assert_valid(&name);
        }
    }

    #[test]
    fn invalid_names() {
        let names = vec![
            "fix.",
            "fix!",
            "fix?",
            "fix:",
            "fix-",
            "fix_",
            "fix/",
            "fix\'",
            "fix\"",
            "fix…",
            "fix⋯",
            ".fix",
            "!fix",
            "?fix",
            ":fix",
            "-fix",
            "_fix",
            "/fix",
            "…fix",
            "⋯fix",
            "[JIRA-123",
            "[bug-fix",
            "(feat-fix",
            "{fix-test",
            "|fix-test",
            "-fix-test",
            "+fix-test",
            "*fix-test",
            "%fix-test",
            "@fix-test",
        ];
        for name in names {
            assert_invalid(name);
        }
    }

    #[test]
    fn punctuation_at_start() {
        let issue = first_issue(validate(&branch("!fix")));
        assert_eq!(
            issue.message,
            "The branch name starts with a punctuation character"
        );
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_contains_issue_output(
            &issue,
            "| !fix\n\
             | - Remove punctuation from the start of the branch name",
        );
    }

    #[test]
    fn punctuation_at_end() {
        let issue = first_issue(validate(&branch("fix!")));
        assert_eq!(
            issue.message,
            "The branch name ends with a punctuation character"
        );
        assert_eq!(issue.position, Position::Branch { column: 4 });
        assert_contains_issue_output(
            &issue,
            "| fix!\n\
             |    - Remove punctuation from the end of the branch name",
        );
    }

    #[test]
    fn punctuation_at_start_and_end() {
        let issues = validate(&branch("!fix!")).expect("No issues found");
        assert_eq!(issues.len(), 2);
    }
}
