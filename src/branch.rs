use crate::rule::{Rule, Violation};
use regex::{Regex, RegexBuilder};

lazy_static! {
    static ref BRANCH_WITH_TICKET_NUMBER: Regex = {
        let mut tempregex = RegexBuilder::new(r"^(\w+[-_/])?\d+([-_/]\w+)?([-_/]\w+)?");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

#[derive(Debug)]
pub struct Branch {
    pub name: String,
    pub violations: Vec<Violation>,
}

impl Branch {
    pub fn new(name: String) -> Self {
        Self {
            name,
            violations: Vec::<Violation>::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn validate(&mut self) {
        self.validate_ticket_number();
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
                self.add_violation(
                    Rule::BranchNameTicketNumber,
                    "Remove the ticket number from the branch name or expand the branch name with more details."
                    .to_string(),
                )
            }
        }
    }

    fn add_violation(&mut self, rule: Rule, message: String) {
        self.violations.push(Violation { rule, message })
    }
}

#[cfg(test)]
mod tests {
    use super::{Branch, Rule, Violation};

    fn validated_branch(name: String) -> Branch {
        let mut branch = Branch::new(name);
        branch.validate();
        branch
    }

    fn assert_branch_valid_for(branch: Branch, rule: &Rule) {
        assert!(
            !has_violation(&branch.violations, rule),
            "Branch was not considered valid: {:?}",
            branch
        );
    }

    fn assert_branch_invalid_for(branch: Branch, rule: &Rule) {
        assert!(
            has_violation(&branch.violations, rule),
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

    fn has_violation(violations: &Vec<Violation>, rule: &Rule) -> bool {
        violations.iter().any(|v| &v.rule == rule)
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
    }
}
