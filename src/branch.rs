use crate::issue::Issue;
use crate::rule::Rule;

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
        self.validate_rule(&Rule::BranchNameCliche);
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
}
