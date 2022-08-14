use crate::issue::Issue;
use crate::rule::Rule;

#[derive(Debug)]
pub struct Branch {
    pub name: String,
    pub issues: Vec<Issue>,
    pub checked_rules: Vec<Rule>,
}

impl Branch {
    pub fn new(name: String) -> Self {
        Self {
            name,
            issues: Vec::<Issue>::new(),
            checked_rules: Vec::<Rule>::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn validate(&mut self) {
        self.validate_rule(Rule::BranchNameLength);
        self.validate_rule(Rule::BranchNameTicketNumber);
        self.validate_rule(Rule::BranchNamePunctuation);
        self.validate_rule(Rule::BranchNameCliche);
    }

    fn validate_rule(&mut self, rule: Rule) {
        match rule.validate_branch(self) {
            Some(mut issues) => {
                self.issues.append(&mut issues);
            }
            None => {
                debug!("No issues found for rule '{}'", rule);
            }
        };
        self.checked_rules.push(rule);
    }
}

impl std::fmt::Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Branch: {}\n\
            Checked rules: {}\n\
            Issues: {}\n",
            self.name,
            self.checked_rules
                .iter()
                .map(|r| format!("{}", r))
                .collect::<Vec<String>>()
                .join(", "),
            self.issues
                .iter()
                .map(|i| format!("{}", i.rule))
                .collect::<Vec<String>>()
                .join(", "),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Branch;

    #[test]
    fn display() {
        let mut branch = Branch::new("branch-name!".to_string());
        branch.validate();
        let display_branch = format!("{}", branch);
        assert_eq!(
            display_branch,
            "Branch: branch-name!\n\
            Checked rules: BranchNameLength, BranchNameTicketNumber, BranchNamePunctuation, BranchNameCliche\n\
            Issues: BranchNamePunctuation\n",
            "{}",
            display_branch
        );
    }
}
