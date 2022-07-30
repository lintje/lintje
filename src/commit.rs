use crate::issue::Issue;
use crate::rule::{rule_by_name, Rule};

#[derive(Debug)]
pub struct Commit {
    pub long_sha: Option<String>,
    pub short_sha: Option<String>,
    pub email: Option<String>,
    pub subject: String,
    pub message: String,
    pub file_changes: Vec<String>,
    pub issues: Vec<Issue>,
    pub ignored: bool,
    pub ignored_rules: Vec<Rule>,
}

impl Commit {
    pub fn new(
        long_sha: Option<String>,
        email: Option<String>,
        subject: &str,
        message: String,
        file_changes: Vec<String>,
    ) -> Self {
        // Get first 7 characters of the commit SHA to get the short SHA.
        let short_sha = match &long_sha {
            Some(long) => match long.get(0..7) {
                Some(sha) => Some(sha.to_string()),
                None => {
                    debug!("Could not determine abbreviated SHA from SHA");
                    None
                }
            },
            None => None,
        };
        let ignored_rules = Self::find_ignored_rules(&message);
        Self {
            long_sha,
            short_sha,
            email,
            subject: subject.trim_end().to_string(),
            message,
            file_changes,
            ignored: false,
            ignored_rules,
            issues: Vec::<Issue>::new(),
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.file_changes.is_empty()
    }

    fn find_ignored_rules(message: &str) -> Vec<Rule> {
        let disable_prefix = "lintje:disable ";
        let mut ignored = vec![];
        for line in message.lines() {
            if let Some(name) = line.strip_prefix(disable_prefix) {
                match rule_by_name(name) {
                    Some(rule) => ignored.push(rule),
                    None => warn!("Attempted to ignore unknown rule: {}", name),
                }
            }
        }
        ignored
    }

    fn rule_ignored(&self, rule: &Rule) -> bool {
        self.ignored_rules.contains(rule)
    }

    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn validate(&mut self) {
        self.validate_rule(&Rule::MergeCommit);
        self.validate_rule(&Rule::NeedsRebase);

        // If a commit has a MergeCommit or NeedsRebase issue, other rules are skipped,
        // because the commit itself will need to be rebased into other commits. So the format
        // of the commit won't matter.
        if !self.has_issue(&Rule::MergeCommit) && !self.has_issue(&Rule::NeedsRebase) {
            self.validate_rule(&Rule::SubjectCliche);
            self.validate_rule(&Rule::SubjectLength);
            self.validate_rule(&Rule::SubjectMood);
            self.validate_rule(&Rule::SubjectWhitespace);
            self.validate_rule(&Rule::SubjectPrefix);
            self.validate_rule(&Rule::SubjectCapitalization);
            self.validate_rule(&Rule::SubjectBuildTag);
            self.validate_rule(&Rule::SubjectPunctuation);
            self.validate_rule(&Rule::SubjectTicketNumber);
            self.validate_rule(&Rule::MessageTicketNumber);
            self.validate_rule(&Rule::MessageEmptyFirstLine);
            self.validate_rule(&Rule::MessagePresence);
            self.validate_rule(&Rule::MessageLineLength);
        }
        self.validate_rule(&Rule::DiffPresence);
    }

    fn validate_rule(&mut self, rule: &Rule) {
        if self.rule_ignored(rule) {
            return;
        }

        match rule.validate_commit(self) {
            Some(mut issues) => {
                self.issues.append(&mut issues);
            }
            None => {
                debug!(
                    "No issues found for commit '{}' in rule '{}'",
                    self.long_sha.as_ref().unwrap_or(&"".to_string()),
                    rule
                );
            }
        }
    }

    pub fn has_issue(&self, rule: &Rule) -> bool {
        self.issues.iter().any(|issue| &issue.rule == rule)
    }
}

#[cfg(test)]
mod tests {
    use crate::rule::Rule;
    use crate::test::*;

    #[test]
    fn test_create_short_sha() {
        let long_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        let with_long_sha = commit_with_sha(long_sha, "Subject".to_string(), "Message".to_string());
        assert_eq!(
            with_long_sha.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(with_long_sha.short_sha, Some("aaaaaaa".to_string()));

        let long_sha = Some("a".to_string());
        let without_long_sha =
            commit_with_sha(long_sha, "Subject".to_string(), "Message".to_string());
        assert_eq!(without_long_sha.long_sha, Some("a".to_string()));
        assert_eq!(without_long_sha.short_sha, None);
    }

    #[test]
    fn is_valid() {
        let mut commit = commit("".to_string(), "Intentionally invalid commit".to_string());
        assert!(commit.is_valid());
        commit.validate();
        assert!(!commit.is_valid());
    }

    #[test]
    fn ignored_rule() {
        let mut ignored_rule = commit(
            "".to_string(),
            "...\n\
            lintje:disable SubjectLength\n\
            lintje:disable MessageEmptyFirstLine"
                .to_string(),
        );
        assert_eq!(
            ignored_rule.ignored_rules,
            vec![Rule::SubjectLength, Rule::MessageEmptyFirstLine]
        );

        ignored_rule.validate();
        assert!(!ignored_rule.has_issue(&Rule::SubjectLength));
        assert!(!ignored_rule.has_issue(&Rule::MessageEmptyFirstLine));
    }
}
