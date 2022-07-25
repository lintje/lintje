use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use core::ops::Range;
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
        self.validate_rule(&Rule::BranchNameLength);
        self.validate_rule(&Rule::BranchNameTicketNumber);
        self.validate_rule(&Rule::BranchNamePunctuation);
        self.validate_cliche();
    }

    fn validate_rule(&mut self, rule: &Rule) {
        match rule.validate_branch(self) {
            Some(mut issues) => {
                self.issues.append(&mut issues);
            }
            None => {
                debug!("No issues found for rule '{}'", rule);
            }
        }
    }

    fn validate_cliche(&mut self) {
        let branch = &self.name.to_lowercase();
        if BRANCH_WITH_CLICHE.is_match(branch) {
            let context = vec![Context::branch_error(
                branch.to_string(),
                Range {
                    start: 0,
                    end: branch.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            self.add_error(
                Rule::BranchNameCliche,
                "The branch name does not explain the change in much detail".to_string(),
                1,
                context,
            );
        }
    }

    fn add_error(&mut self, rule: Rule, message: String, column: usize, context: Vec<Context>) {
        self.issues.push(Issue::error(
            rule,
            message,
            Position::Branch { column },
            context,
        ));
    }
}

#[cfg(test)]
mod tests {
    use crate::branch::Branch;
    use crate::issue::{Issue, Position};
    use crate::rule::Rule;
    use crate::test::formatted_context;

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

    fn has_issue(issues: &[Issue], rule: &Rule) -> bool {
        issues.iter().any(|v| &v.rule == rule)
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
