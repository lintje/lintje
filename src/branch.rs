use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::utils::{character_count_for_bytes_index, display_width, is_punctuation};
use core::ops::Range;
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref BRANCH_WITH_TICKET_NUMBER: Regex = {
        let mut tempregex = RegexBuilder::new(r"^(\w+[-_/])?\d+([-_/]\w+)?([-_/]\w+)?");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
    static ref BRANCH_WITH_CLICHE: Regex = {
        let mut tempregex = RegexBuilder::new(
            r"^(wip|fix(es|ed|ing)?|add(s|ed|ing)?|(updat|chang|remov|delet)(e|es|ed|ing))([-_/]+\w+)?$",
        );
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

#[derive(Debug)]
pub struct Branch {
    pub name: String,
    pub issues: Vec<Issue>,
}

impl Branch {
    pub fn new(name: String) -> Self {
        Self {
            name,
            issues: Vec::<Issue>::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn validate(&mut self) {
        self.validate_length();
        self.validate_ticket_number();
        self.validate_punctuation();
        self.validate_cliche();
    }

    fn validate_length(&mut self) {
        let name = &self.name;
        let width = display_width(name);
        if width < 4 {
            let context = vec![Context::branch_hint(
                name.to_string(),
                Range {
                    start: 0,
                    end: name.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            self.add_issue(
                Rule::BranchNameLength,
                format!("Branch name of {} characters is too short", width),
                Position::Branch { column: 1 },
                context,
            )
        }
    }

    fn validate_ticket_number(&mut self) {
        let name = &self.name;
        if let Some(captures) = BRANCH_WITH_TICKET_NUMBER.captures(name) {
            let valid = match (captures.get(1), captures.get(2), captures.get(3)) {
                (None, None, _) => false,
                (Some(_prefix), None, _) => false,
                (None, Some(_suffix), None) => false,
                (None, Some(_suffix), Some(_suffix_more)) => true,
                (Some(_prefix), Some(_suffix), _) => true,
            };
            if !valid {
                let context = vec![Context::branch_hint(
                    name.to_string(),
                    Range {
                        start: 0,
                        end: name.len(),
                    },
                    "Remove the ticket number from the branch name or expand the branch name with more details".to_string(),
                )];
                self.add_issue(
                    Rule::BranchNameTicketNumber,
                    "A ticket number was detected in the branch name".to_string(),
                    Position::Branch { column: 1 },
                    context,
                )
            }
        }
    }

    fn validate_punctuation(&mut self) {
        match &self.name.chars().next() {
            Some(character) => {
                if is_punctuation(character) {
                    let branch = &self.name;
                    let context = vec![Context::branch_hint(
                        branch.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Remove punctuation from the start of the branch name".to_string(),
                    )];
                    self.add_issue(
                        Rule::BranchNamePunctuation,
                        "The branch name starts with a punctuation character".to_string(),
                        Position::Branch { column: 1 },
                        context,
                    )
                }
            }
            None => {
                error!(
                    "BranchNamePunctuation validation failure: No first character found of branch name."
                )
            }
        }

        match &self.name.chars().last() {
            Some(character) => {
                if is_punctuation(character) {
                    let branch_length = self.name.len();
                    let branch = &self.name;
                    let context = vec![Context::branch_hint(
                        branch.to_string(),
                        Range {
                            start: branch_length - character.len_utf8(),
                            end: branch_length,
                        },
                        "Remove punctuation from the end of the branch name".to_string(),
                    )];
                    self.add_issue(
                        Rule::BranchNamePunctuation,
                        "The branch name ends with a punctuation character".to_string(),
                        Position::Branch {
                            column: character_count_for_bytes_index(
                                &self.name,
                                self.name.len() - character.len_utf8(),
                            ),
                        },
                        context,
                    )
                }
            }
            None => {
                error!(
                    "BranchNamePunctuation validation failure: No last character found of branch name."
                )
            }
        }
    }

    fn validate_cliche(&mut self) {
        let branch = &self.name.to_lowercase();
        if BRANCH_WITH_CLICHE.is_match(branch) {
            let context = vec![Context::branch_hint(
                branch.to_string(),
                Range {
                    start: 0,
                    end: branch.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            self.add_issue(
                Rule::BranchNameCliche,
                "The branch name does not explain the change in much detail".to_string(),
                Position::Branch { column: 1 },
                context,
            )
        }
    }

    fn add_issue(
        &mut self,
        rule: Rule,
        message: String,
        position: Position,
        context: Vec<Context>,
    ) {
        self.issues.push(Issue {
            rule,
            message,
            position,
            context,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::branch::Branch;
    use crate::issue::{Issue, Position};
    use crate::rule::Rule;
    use crate::utils::test::formatted_context;

    fn validated_branch(name: String) -> Branch {
        let mut branch = Branch::new(name);
        branch.validate();
        branch
    }

    fn find_issue(issues: Vec<Issue>, rule: &Rule) -> Issue {
        let mut issues = issues.into_iter().filter(|v| &v.rule == rule);
        let issue = match issues.next() {
            Some(issue) => issue,
            None => panic!("No issue of the {} rule found", rule),
        };
        if issues.next().is_some() {
            panic!("More than one issue of the {} rule found", rule)
        }
        issue
    }

    fn assert_branch_valid_for(branch: Branch, rule: &Rule) {
        assert!(
            !has_issue(&branch.issues, rule),
            "Branch was not considered valid: {:?}",
            branch
        );
    }

    fn assert_branch_invalid_for(branch: Branch, rule: &Rule) {
        assert!(
            has_issue(&branch.issues, rule),
            "Branch was not considered invalid: {:?}",
            branch
        );
    }

    fn assert_branch_name_as_valid<S: AsRef<str>>(name: S, rule: &Rule) {
        let branch = validated_branch(name.as_ref().to_string());
        assert_branch_valid_for(branch, rule);
    }

    fn assert_branch_name_as_invalid<S: AsRef<str>>(name: S, rule: &Rule) {
        let branch = validated_branch(name.as_ref().to_string());
        assert_branch_invalid_for(branch, rule);
    }

    fn assert_branch_names_as_valid<S: AsRef<str>>(names: Vec<S>, rule: &Rule) {
        for name in names {
            assert_branch_name_as_valid(name, rule)
        }
    }

    fn assert_branch_names_as_invalid<S: AsRef<str>>(names: Vec<S>, rule: &Rule) {
        for name in names {
            assert_branch_name_as_invalid(name, rule)
        }
    }

    fn has_issue(issues: &[Issue], rule: &Rule) -> bool {
        issues.iter().any(|v| &v.rule == rule)
    }

    #[test]
    fn test_validate_name_length() {
        let valid_names = vec![
            "abcd".to_string(),
            "-_/!".to_string(),
            "a".repeat(5),
            "a".repeat(50),
            "あ".repeat(4),
            "✨".repeat(4),
        ];
        assert_branch_names_as_valid(valid_names, &Rule::BranchNameLength);

        let invalid_names = vec!["", "a", "ab", "abc"];
        assert_branch_names_as_invalid(invalid_names, &Rule::BranchNameLength);

        let branch = validated_branch("abc".to_string());
        let issue = find_issue(branch.issues, &Rule::BranchNameLength);
        assert_eq!(issue.message, "Branch name of 3 characters is too short");
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_eq!(
            formatted_context(&issue),
            "|\n\
             | abc\n\
             | ^^^ Describe the change in more detail\n"
        );
    }

    #[test]
    fn test_branch_ticket_number() {
        let valid_names = vec![
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
            "fix-bug",
        ];
        assert_branch_names_as_valid(valid_names, &Rule::BranchNameTicketNumber);

        let invalid_names = vec![
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
        assert_branch_names_as_invalid(invalid_names, &Rule::BranchNameTicketNumber);

        let branch = validated_branch("fix-123".to_string());
        let issue = find_issue(branch.issues, &Rule::BranchNameTicketNumber);
        assert_eq!(
            issue.message,
            "A ticket number was detected in the branch name"
        );
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_eq!(
            formatted_context(&issue),
            "|\n\
             | fix-123\n\
             | ^^^^^^^ Remove the ticket number from the branch name or expand the branch name with more details\n"
        );
    }

    #[test]
    fn test_validate_punctuation() {
        let subjects = vec!["fix-test", "fix-あ-test"];
        assert_branch_names_as_valid(subjects, &Rule::BranchNamePunctuation);

        let invalid_subjects = vec![
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
        assert_branch_names_as_invalid(invalid_subjects, &Rule::BranchNamePunctuation);

        let punctuation_start = validated_branch("!fix".to_string());
        let issue = find_issue(punctuation_start.issues, &Rule::BranchNamePunctuation);
        assert_eq!(
            issue.message,
            "The branch name starts with a punctuation character"
        );
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_eq!(
            formatted_context(&issue),
            "|\n\
             | !fix\n\
             | ^ Remove punctuation from the start of the branch name\n"
        );

        let punctuation_end = validated_branch("fix!".to_string());
        let issue = find_issue(punctuation_end.issues, &Rule::BranchNamePunctuation);
        assert_eq!(
            issue.message,
            "The branch name ends with a punctuation character"
        );
        assert_eq!(issue.position, Position::Branch { column: 4 });
        assert_eq!(
            formatted_context(&issue),
            "|\n\
             | fix!\n\
             |    ^ Remove punctuation from the end of the branch name\n"
        );
    }

    #[test]
    fn test_validate_cliche() {
        let subjects = vec!["add-email-validation", "fix-brittle-test"];
        assert_branch_names_as_valid(subjects, &Rule::BranchNameCliche);

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
            invalid_subjects.push(format!("{}-test", uppercase_word));
            invalid_subjects.push(format!("{}-issue", capitalized_word));
            invalid_subjects.push(format!("{}-bug", word));
            invalid_subjects.push(format!("{}-readme", word));
            invalid_subjects.push(format!("{}-something", word));
            invalid_subjects.push(format!("{}_something", word));
            invalid_subjects.push(format!("{}/something", word));
        }
        for subject in invalid_subjects {
            assert_branch_name_as_invalid(subject.as_str(), &Rule::BranchNameCliche);
        }

        let branch = validated_branch("fix-bug".to_string());
        let issue = find_issue(branch.issues, &Rule::BranchNameCliche);
        assert_eq!(
            issue.message,
            "The branch name does not explain the change in much detail"
        );
        assert_eq!(issue.position, Position::Branch { column: 1 });
        assert_eq!(
            formatted_context(&issue),
            "|\n\
             | fix-bug\n\
             | ^^^^^^^ Describe the change in more detail\n"
        );
    }
}
