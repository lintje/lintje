use crate::config::ValidationContext;
use crate::issue::Issue;
use crate::rule::{rule_by_name, Rule};

#[derive(Debug)]
pub struct Commit {
    pub long_sha: Option<String>,
    pub short_sha: Option<String>,
    pub email: Option<String>,
    pub subject: String,
    pub message: String,
    pub trailers: String,
    pub file_changes: Vec<String>,
    pub issues: Vec<Issue>,
    pub ignored_rules: Vec<Rule>,
    pub checked_rules: Vec<Rule>,
}

impl Commit {
    pub fn new(
        long_sha: Option<String>,
        email: Option<String>,
        subject: &str,
        message: String,
        trailers: String,
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
            trailers,
            file_changes,
            ignored_rules,
            issues: Vec::<Issue>::new(),
            checked_rules: Vec::<Rule>::new(),
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

    pub fn validate(&mut self, context: &ValidationContext) {
        self.validate_rule(Rule::MergeCommit);
        self.validate_rule(Rule::RebaseCommit);

        // If a commit has a MergeCommit or RebaseCommit issue, other rules are skipped,
        // because the commit itself will need to be rebased into other commits. So the format
        // of the commit won't matter.
        if !self.has_issue(&Rule::MergeCommit) && !self.has_issue(&Rule::RebaseCommit) {
            self.validate_rule(Rule::SubjectCliche);
            self.validate_rule(Rule::SubjectLength);
            self.validate_rule(Rule::SubjectMood);
            self.validate_rule(Rule::SubjectWhitespace);
            self.validate_rule(Rule::SubjectPrefix);
            self.validate_rule(Rule::SubjectCapitalization);
            self.validate_rule(Rule::SubjectBuildTag);
            self.validate_rule(Rule::SubjectPunctuation);
            self.validate_rule(Rule::SubjectTicketNumber);
            self.validate_rule(Rule::MessageTicketNumber);
            self.validate_rule(Rule::MessageEmptyFirstLine);
            self.validate_rule(Rule::MessagePresence);
            self.validate_rule(Rule::MessageLineLength);
            self.validate_rule(Rule::MessageTrailerLine);
            self.validate_rule(Rule::MessageSkipBuildTag);
            if context.changesets {
                self.validate_rule(Rule::DiffChangeset);
            }
        }
        self.validate_rule(Rule::DiffPresence);
    }

    fn validate_rule(&mut self, rule: Rule) {
        if !self.rule_ignored(&rule) {
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
        self.checked_rules.push(rule);
    }

    pub fn has_issue(&self, rule: &Rule) -> bool {
        self.issues.iter().any(|issue| &issue.rule == rule)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::ValidationContext;
    use crate::rule::Rule;
    use crate::test::*;

    fn default_context() -> ValidationContext {
        ValidationContext { changesets: false }
    }

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
    fn trims_subject_end() {
        let commit = commit("This is a subject  ".to_string(), "Message".to_string());
        assert_eq!(commit.subject, "This is a subject");
    }

    #[test]
    fn is_valid() {
        let mut commit = commit("".to_string(), "Intentionally invalid commit".to_string());
        assert!(commit.is_valid());
        commit.validate(&default_context());
        assert!(!commit.is_valid());
    }

    #[test]
    fn check_validated_rules_default() {
        let mut commit = commit("".to_string(), "Intentionally invalid commit".to_string());
        commit.validate(&ValidationContext { changesets: false });
        // Test specific order of rules because they may depend on one another
        assert_eq!(
            commit.checked_rules,
            vec![
                Rule::MergeCommit,
                Rule::RebaseCommit,
                Rule::SubjectCliche,
                Rule::SubjectLength,
                Rule::SubjectMood,
                Rule::SubjectWhitespace,
                Rule::SubjectPrefix,
                Rule::SubjectCapitalization,
                Rule::SubjectBuildTag,
                Rule::SubjectPunctuation,
                Rule::SubjectTicketNumber,
                Rule::MessageTicketNumber,
                Rule::MessageEmptyFirstLine,
                Rule::MessagePresence,
                Rule::MessageLineLength,
                Rule::MessageTrailerLine,
                Rule::MessageSkipBuildTag,
                Rule::DiffPresence
            ]
        );
    }

    #[test]
    fn check_validated_rules_merge_commit() {
        let mut commit = commit(
            "Merge branch 'develop' of github.com/org/repo into develop".to_string(),
            "".to_string(),
        );
        commit.validate(&ValidationContext { changesets: false });
        // Test specific order of rules because they may depend on one another.
        // A lot of rules are skipped for these types of commits because they do not apply.
        assert_eq!(
            commit.checked_rules,
            vec![Rule::MergeCommit, Rule::RebaseCommit, Rule::DiffPresence]
        );
    }

    #[test]
    fn check_validated_rules_fixup_commit() {
        let mut commit = commit("fixup! Some commit".to_string(), "".to_string());
        commit.validate(&ValidationContext { changesets: false });
        // Test specific order of rules because they may depend on one another.
        // A lot of rules are skipped for these types of commits because they do not apply.
        assert_eq!(
            commit.checked_rules,
            vec![Rule::MergeCommit, Rule::RebaseCommit, Rule::DiffPresence]
        );
    }

    #[test]
    fn does_not_validate_changeset_rule_when_changeset_mode_is_false() {
        let mut commit = commit("".to_string(), "Intentionally invalid commit".to_string());
        commit.validate(&ValidationContext { changesets: false });
        assert!(!commit.checked_rules.contains(&Rule::DiffChangeset));
    }

    #[test]
    fn validate_changeset_rule_when_changeset_mode_is_true() {
        let mut commit = commit("".to_string(), "Intentionally invalid commit".to_string());
        commit.validate(&ValidationContext { changesets: true });
        assert!(commit.checked_rules.contains(&Rule::DiffChangeset));
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

        ignored_rule.validate(&default_context());
        assert!(!ignored_rule.has_issue(&Rule::SubjectLength));
        assert!(!ignored_rule.has_issue(&Rule::MessageEmptyFirstLine));
    }
}
