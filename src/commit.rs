use regex::Regex;
use std::fmt;

lazy_static! {
    static ref SUBJECT_WITH_TICKET: Regex = Regex::new(r"[A-Z]+-\d+").unwrap();
    // Match all GitHub and GitLab keywords
    static ref SUBJECT_WITH_FIX_TICKET: Regex = Regex::new(r"([fF]ix(es|ed|ing)?|[cC]los(e|es|ed|ing)|[rR]esolv(e|es|ed|ing)|[iI]mplement(s|ed|ing)?):? ([^\s]*[\w\-_/]+)?#\d+").unwrap();
    static ref URL_REGEX: Regex = Regex::new(r"https?://\w+").unwrap();
    static ref MOOD_WORDS: Vec<&'static str> = vec![
        "fixed",
        "fixes",
        "fixing",
        "solved",
        "solves",
        "solving",
        "resolved",
        "resolves",
        "resolving",
        "closed",
        "closes",
        "closing",
        "added",
        "adding",
        "updated",
        "updates",
        "updating",
        "removed",
        "removes",
        "removing",
        "deleted",
        "deletes",
        "deleting",
        "changed",
        "changes",
        "changing",
        "moved",
        "moves",
        "moving",
        "refactored",
        "refactors",
        "refactoring",
        "checked",
        "checks",
        "checking",
        "adjusted",
        "adjusts",
        "adjusting",
    ];
}

#[derive(Debug)]
pub struct Commit {
    pub long_sha: Option<String>,
    pub short_sha: Option<String>,
    pub subject: String,
    pub message: String,
    pub violations: Vec<Violation>,
    pub ignored_rules: Vec<Rule>,
}

impl Commit {
    pub fn new(
        long_sha: Option<String>,
        short_sha: Option<String>,
        subject: String,
        message: String,
    ) -> Self {
        let ignored_rules = Self::find_ignored_rules(&message);
        Self {
            long_sha,
            short_sha,
            subject: subject.trim().to_string(),
            message: message.trim().to_string(),
            ignored_rules,
            violations: Vec::<Violation>::new(),
        }
    }

    pub fn find_ignored_rules(message: &String) -> Vec<Rule> {
        let disable_prefix = "gitlint:disable ";
        let mut ignored = vec![];
        for (_index, line) in message.lines().enumerate() {
            if line.starts_with(disable_prefix) {
                let rule = match &line[disable_prefix.len()..] {
                    "MergeCommit" => Some(Rule::MergeCommit),
                    "NeedsRebase" => Some(Rule::NeedsRebase),
                    "SubjectTooLong" => Some(Rule::SubjectTooLong),
                    "SubjectTooShort" => Some(Rule::SubjectTooShort),
                    "SubjectMood" => Some(Rule::SubjectMood),
                    "SubjectCapitalization" => Some(Rule::SubjectCapitalization),
                    "SubjectPunctuation" => Some(Rule::SubjectPunctuation),
                    "SubjectTicketNumber" => Some(Rule::SubjectTicketNumber),
                    "SubjectCliche" => Some(Rule::SubjectCliche),
                    "MessagePresence" => Some(Rule::MessagePresence),
                    "MessageLineTooLong" => Some(Rule::MessageLineTooLong),
                    unknown => {
                        warn!("Unknown rule disabled: {}", unknown);
                        None
                    }
                };
                match rule {
                    Some(r) => ignored.push(r),
                    None => (),
                }
            }
        }
        ignored
    }

    fn rule_ignored(&self, rule: Rule) -> bool {
        self.ignored_rules.contains(&rule)
    }

    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn validate(&mut self) {
        self.validate_merge_commit();
        self.validate_needs_rebase();
        self.validate_subject_line_length();
        self.validate_subject_mood();
        self.validate_subject_capitalization();
        self.validate_subject_punctuation();
        self.validate_subject_ticket_numbers();
        self.validate_subject_cliches();
        self.validate_message_presence();
        self.validate_message_line_length();
    }

    fn validate_merge_commit(&mut self) {
        if self.rule_ignored(Rule::MergeCommit) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("Merge branch") {
            self.add_violation(Rule::MergeCommit, format!("Commit is a merge commit."))
        }
    }

    fn validate_needs_rebase(&mut self) {
        if self.rule_ignored(Rule::NeedsRebase) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("fixup! ") {
            self.add_violation(Rule::NeedsRebase, format!("Subject is a fixup commit."))
        } else if subject.starts_with("squash! ") {
            self.add_violation(Rule::NeedsRebase, format!("Subject is a squash commit."))
        }
    }

    fn validate_subject_line_length(&mut self) {
        let length = self.subject.len();
        if length > 50 {
            if self.rule_ignored(Rule::SubjectTooLong) {
                return;
            }

            self.add_violation(
                Rule::SubjectTooLong,
                format!("Subject length is too long: {} characters.", length),
            )
        }
        if length < 5 {
            if self.rule_ignored(Rule::SubjectTooShort) {
                return;
            }

            self.add_violation(
                Rule::SubjectTooShort,
                format!("Subject length is too short: {} characters.", length),
            )
        }
    }

    fn validate_subject_mood(&mut self) {
        if self.rule_ignored(Rule::SubjectMood) {
            return;
        }

        match self.subject.split(" ").nth(0) {
            Some(raw_word) => {
                let word = raw_word.to_lowercase();
                if MOOD_WORDS.contains(&word.as_str()) {
                    self.add_violation(
                        Rule::SubjectMood,
                        "Subject is not imperative mood.".to_string(),
                    )
                }
            }
            None => error!("No first word found of subject."),
        }
    }

    fn validate_subject_capitalization(&mut self) {
        if self.rule_ignored(Rule::SubjectCapitalization) {
            return;
        }

        match self.subject.chars().nth(0) {
            Some(character) => {
                if !character.is_uppercase() {
                    self.add_violation(
                        Rule::SubjectCapitalization,
                        "Subject does not start with a capital letter.".to_string(),
                    )
                }
            }
            None => error!("No first character found of subject."),
        }
    }

    fn validate_subject_punctuation(&mut self) {
        if self.rule_ignored(Rule::SubjectPunctuation) {
            return;
        }

        match self.subject.chars().last() {
            Some(character) => {
                if character.is_ascii_punctuation() {
                    self.add_violation(
                        Rule::SubjectPunctuation,
                        format!("Subject ends with punctuation: {}", character),
                    )
                }
            }
            None => error!("No first character found of subject."),
        }
    }

    fn validate_subject_ticket_numbers(&mut self) {
        if self.rule_ignored(Rule::SubjectTicketNumber) {
            return;
        }

        let subject = &self.subject;
        if SUBJECT_WITH_TICKET.is_match(subject) {
            self.add_violation(
                Rule::SubjectTicketNumber,
                format!("Subject includes a ticket number."),
            )
        } else if SUBJECT_WITH_FIX_TICKET.is_match(subject) {
            self.add_violation(
                Rule::SubjectTicketNumber,
                format!("Subject includes a ticket number."),
            )
        }
    }

    fn validate_subject_cliches(&mut self) {
        if self.rule_ignored(Rule::SubjectCliche) {
            return;
        }

        let subject = &self.subject;
        if subject.to_lowercase().starts_with("wip ") {
            self.add_violation(
                Rule::SubjectCliche,
                format!("Subject is a 'Work in Progress' commit."),
            )
        } else if subject.to_lowercase() == "wip".to_string() {
            self.add_violation(
                Rule::SubjectCliche,
                format!("Subject is a 'Work in Progress' commit."),
            )
        } else if subject == &"Fix test".to_string() {
            self.add_violation(
                Rule::SubjectCliche,
                format!("Subject is a 'Fix test' commit."),
            )
        } else if subject == &"Fix bug".to_string() {
            self.add_violation(
                Rule::SubjectCliche,
                format!("Subject is a 'Fix bug' commit."),
            )
        }
    }

    fn validate_message_presence(&mut self) {
        if self.rule_ignored(Rule::MessagePresence) {
            return;
        }

        let length = self.message.len();
        if length == 0 {
            self.add_violation(Rule::MessagePresence, "Message is not present.".to_string())
        } else if length < 10 {
            self.add_violation(
                Rule::MessagePresence,
                "Message body is less than 10 characters long.".to_string(),
            )
        }
    }

    fn validate_message_line_length(&mut self) {
        if self.rule_ignored(Rule::MessageLineTooLong) {
            return;
        }

        match Self::check_line_lengths(self.message.lines()) {
            Some((rule, message)) => self.add_violation(rule, message),
            None => {}
        }
    }

    fn check_line_lengths(lines: std::str::Lines) -> Option<(Rule, String)> {
        for (_index, raw_line) in lines.enumerate() {
            let line = raw_line.trim();
            let length = line.len();
            if length > 72 {
                if URL_REGEX.is_match(line) {
                    continue;
                }
                return Some((
                    Rule::MessageLineTooLong,
                    "One or more lines in the message are longer than 72 characters.".to_string(),
                ));
            }
        }
        None
    }

    fn add_violation(&mut self, rule: Rule, message: String) {
        self.violations.push(Violation { rule, message })
    }
}

#[derive(Debug, PartialEq)]
pub enum Rule {
    MergeCommit,
    NeedsRebase,
    SubjectTooLong,
    SubjectTooShort,
    SubjectMood,
    SubjectCapitalization,
    SubjectPunctuation,
    SubjectTicketNumber,
    SubjectCliche,
    MessagePresence,
    MessageLineTooLong,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Rule::MergeCommit => "MergeCommit",
            Rule::NeedsRebase => "NeedsRebase",
            Rule::SubjectTooLong => "SubjectTooLong",
            Rule::SubjectTooShort => "SubjectTooShort",
            Rule::SubjectMood => "SubjectMood",
            Rule::SubjectCapitalization => "SubjectCapitalization",
            Rule::SubjectPunctuation => "SubjectPunctuation",
            Rule::SubjectTicketNumber => "SubjectTicketNumber",
            Rule::SubjectCliche => "SubjectCliche",
            Rule::MessagePresence => "MessagePresence",
            Rule::MessageLineTooLong => "MessageLineTooLong",
        };
        write!(f, "{}", label)
    }
}

#[derive(Debug)]
pub struct Violation {
    pub rule: Rule,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::{Commit, Rule, Violation, MOOD_WORDS};

    fn commit(subject: String, message: String) -> Commit {
        Commit::new(
            Some("SHA LONG".to_string()),
            Some("SHA SHORT".to_string()),
            subject,
            message,
        )
    }

    fn validated_commit(subject: String, message: String) -> Commit {
        let mut commit = commit(subject, message);
        commit.validate();
        commit
    }

    fn has_violation(violations: &Vec<Violation>, rule: Rule) -> bool {
        violations.iter().find(|&v| v.rule == rule).is_some()
    }

    #[test]
    fn test_validate_subject_merge_commit() {
        let commit1 = validated_commit("I am not a merge commit".to_string(), "".to_string());
        assert!(!has_violation(&commit1.violations, Rule::MergeCommit));

        let commit2 = validated_commit(
            "Merge pull request #123 from repo".to_string(),
            "".to_string(),
        );
        assert!(!has_violation(&commit2.violations, Rule::MergeCommit));

        let commit3 = validated_commit(
            "Merge branch 'main' into develop".to_string(),
            "".to_string(),
        );
        assert!(has_violation(&commit3.violations, Rule::MergeCommit));

        let ignore_commit = validated_commit(
            "Merge branch 'main' into develop".to_string(),
            "gitlint:disable MergeCommit".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::MergeCommit));
    }

    #[test]
    fn test_validate_needs_rebase() {
        let commit1 = validated_commit("I don't need to be rebased".to_string(), "".to_string());
        assert!(!has_violation(&commit1.violations, Rule::NeedsRebase));

        let commit2 = validated_commit(
            "fixup! I don't need to be rebased".to_string(),
            "".to_string(),
        );
        assert!(has_violation(&commit2.violations, Rule::NeedsRebase));

        let commit3 = validated_commit(
            "squash! I don't need to be rebased".to_string(),
            "".to_string(),
        );
        assert!(has_violation(&commit3.violations, Rule::NeedsRebase));

        let ignore_commit = validated_commit(
            "fixup! I don't need to be rebased".to_string(),
            "gitlint:disable NeedsRebase".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::NeedsRebase));
    }

    #[test]
    fn test_validate_subject_line_length() {
        let commit = validated_commit("a".repeat(50).to_string(), "".to_string());
        let violations = commit.violations;
        assert!(!has_violation(&violations, Rule::SubjectTooShort));
        assert!(!has_violation(&violations, Rule::SubjectTooLong));

        let short_commit = validated_commit("a".repeat(4).to_string(), "".to_string());
        assert!(has_violation(
            &short_commit.violations,
            Rule::SubjectTooShort
        ));

        let long_commit = validated_commit("a".repeat(51).to_string(), "".to_string());
        assert!(has_violation(&long_commit.violations, Rule::SubjectTooLong));

        let ignore_commit = validated_commit(
            "a".repeat(51).to_string(),
            "gitlint:disable SubjectTooLong".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::SubjectTooLong));
    }

    #[test]
    fn test_validate_subject_mood() {
        let mut invalid_subjects = vec![];
        for word in MOOD_WORDS.iter() {
            invalid_subjects.push(format!("{} test", word));
            let mut chars = word.chars();
            let capitalized_word = match chars.next() {
                None => panic!("Could not capitalize word: {}", word),
                Some(letter) => letter.to_uppercase().collect::<String>() + chars.as_str(),
            };
            invalid_subjects.push(format!("{} test", capitalized_word));
        }
        for subject in invalid_subjects {
            let commit = validated_commit(subject.to_string(), "".to_string());
            assert!(
                has_violation(&commit.violations, Rule::SubjectMood),
                "Subject was not considered invalid: {}",
                subject
            );
        }

        let ignore_commit = validated_commit(
            "Fixed test".to_string(),
            "gitlint:disable SubjectMood".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::SubjectMood));
    }

    #[test]
    fn test_validate_subject_capitalization() {
        let commit1 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(!has_violation(
            &commit1.violations,
            Rule::SubjectCapitalization
        ));

        let commit2 = validated_commit("fix test".to_string(), "".to_string());
        assert!(has_violation(
            &commit2.violations,
            Rule::SubjectCapitalization
        ));

        let ignore_commit = validated_commit(
            "fix test".to_string(),
            "gitlint:disable SubjectCapitalization".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::SubjectCapitalization));
    }

    #[test]
    fn test_validate_subject_punctuation() {
        let commit1 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(!has_violation(
            &commit1.violations,
            Rule::SubjectPunctuation
        ));

        let commit2 = validated_commit("Fix test.".to_string(), "".to_string());
        assert!(has_violation(&commit2.violations, Rule::SubjectPunctuation));

        let commit3 = validated_commit("Fix test!".to_string(), "".to_string());
        assert!(has_violation(&commit3.violations, Rule::SubjectPunctuation));

        let commit4 = validated_commit("Fix test?".to_string(), "".to_string());
        assert!(has_violation(&commit4.violations, Rule::SubjectPunctuation));

        let ignore_commit = validated_commit(
            "Fix test.".to_string(),
            "gitlint:disable SubjectPunctuation".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::SubjectPunctuation));
    }

    #[test]
    fn test_validate_subject_ticket() {
        let invalid_subjects = vec![
            "JIRA-1234",
            "Fix JIRA-1234 lorem",
            "Fix #1234",
            "Fixed #1234",
            "Fixes #1234",
            "Fixing #1234",
            "Fix #1234 lorem",
            "Fix: #1234 lorem",
            "Fix my-org/repo#1234 lorem",
            "Fix https://examplegithosting.com/my-org/repo#1234 lorem",
            "Commit fixes #1234",
            "Close #1234",
            "Closed #1234",
            "Closes #1234",
            "Closing #1234",
            "Close #1234 lorem",
            "Close: #1234 lorem",
            "Commit closes #1234",
            "Resolve #1234",
            "Resolved #1234",
            "Resolves #1234",
            "Resolving #1234",
            "Resolve #1234 lorem",
            "Resolve: #1234 lorem",
            "Commit resolves #1234",
            "Implement #1234",
            "Implemented #1234",
            "Implements #1234",
            "Implementing #1234",
            "Implement #1234 lorem",
            "Implement: #1234 lorem",
            "Commit implements #1234",
        ];
        for subject in invalid_subjects {
            let commit = validated_commit(subject.to_string(), "".to_string());
            assert!(
                has_violation(&commit.violations, Rule::SubjectTicketNumber),
                "Subject was not considered invalid: {}",
                subject
            );
        }

        let ignore_ticket_number = validated_commit(
            "Fix bug with 'JIRA-1234' type commits".to_string(),
            "gitlint:disable SubjectTicketNumber".to_string(),
        );
        assert!(!has_violation(
            &ignore_ticket_number.violations,
            Rule::SubjectPunctuation
        ));

        let ignore_issue_number = validated_commit(
            "Fix bug with 'Fix #1234' type commits".to_string(),
            "gitlint:disable SubjectTicketNumber".to_string(),
        );
        assert!(!has_violation(
            &ignore_issue_number.violations,
            Rule::SubjectPunctuation
        ));
    }

    #[test]
    fn test_validate_subject_cliches() {
        let commit1 = validated_commit("I am not a cliche".to_string(), "".to_string());
        assert!(!has_violation(&commit1.violations, Rule::SubjectCliche));

        let wip_prefix_uppercase = validated_commit("WIP something".to_string(), "".to_string());
        assert!(has_violation(
            &wip_prefix_uppercase.violations,
            Rule::SubjectCliche
        ));

        let wip_prefix_lowercase = validated_commit("wip something".to_string(), "".to_string());
        assert!(has_violation(
            &wip_prefix_lowercase.violations,
            Rule::SubjectCliche
        ));

        let wip_only_uppercase = validated_commit("WIP".to_string(), "".to_string());
        assert!(has_violation(
            &wip_only_uppercase.violations,
            Rule::SubjectCliche
        ));

        let wip_only_lowercase = validated_commit("wip".to_string(), "".to_string());
        assert!(has_violation(
            &wip_only_lowercase.violations,
            Rule::SubjectCliche
        ));

        let commit3 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(has_violation(&commit3.violations, Rule::SubjectCliche));

        let commit4 = validated_commit("Fix test for some feature".to_string(), "".to_string());
        assert!(!has_violation(&commit4.violations, Rule::SubjectCliche));

        let commit5 = validated_commit("Fix bug".to_string(), "".to_string());
        assert!(has_violation(&commit5.violations, Rule::SubjectCliche));

        let commit6 = validated_commit("Fix bug for some feature".to_string(), "".to_string());
        assert!(!has_violation(&commit6.violations, Rule::SubjectCliche));

        let ignore_commit = validated_commit(
            "WIP".to_string(),
            "gitlint:disable SubjectCliche".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::SubjectCliche));
    }

    #[test]
    fn test_validate_message_presence() {
        let commit1 = validated_commit("Subject".to_string(), "Hello I am a message.".to_string());
        assert!(!has_violation(&commit1.violations, Rule::MessagePresence));

        let commit2 = validated_commit("Subject".to_string(), "".to_string());
        assert!(has_violation(&commit2.violations, Rule::MessagePresence));

        let commit3 = validated_commit("Subject".to_string(), "Short.".to_string());
        assert!(has_violation(&commit3.violations, Rule::MessagePresence));

        let commit4 = validated_commit("Subject".to_string(), "...".to_string());
        assert!(has_violation(&commit4.violations, Rule::MessagePresence));

        let ignore_commit = validated_commit(
            "Subject".to_string(),
            "gitlint:disable MessagePresence".to_string(),
        );
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::MessagePresence));
    }

    #[test]
    fn test_validate_message_line_length() {
        let message1 = ["Hello I am a message.", "Line 2.", &"a".repeat(72)].join("\n");
        let commit1 = validated_commit("Subject".to_string(), message1);
        assert!(!has_violation(
            &commit1.violations,
            Rule::MessageLineTooLong
        ));

        let message2 = ["a".repeat(72), "a".repeat(73)].join("\n");
        let commit2 = validated_commit("Subject".to_string(), message2);
        assert!(has_violation(&commit2.violations, Rule::MessageLineTooLong));

        let message3 = [
            "This message is accepted.".to_string(),
            "This a long line with a link https://tomdebruijn.com/posts/git-is-about-communication/".to_string()
        ].join("\n");
        let commit3 = validated_commit("Subject".to_string(), message3);
        assert!(!has_violation(
            &commit3.violations,
            Rule::MessageLineTooLong
        ));

        let message4 = [
            "This message is accepted.".to_string(),
            "This a long line with a link http://tomdebruijn.com/posts/git-is-about-communication/"
                .to_string(),
        ]
        .join("\n");
        let commit4 = validated_commit("Subject".to_string(), message4);
        assert!(!has_violation(
            &commit4.violations,
            Rule::MessageLineTooLong
        ));

        let message5 = [
            "This a too long line with only protocols http:// https:// which is not accepted."
                .to_string(),
        ]
        .join("\n");
        let commit5 = validated_commit("Subject".to_string(), message5);
        assert!(has_violation(&commit5.violations, Rule::MessageLineTooLong));

        let ignore_message = [
            "a".repeat(72),
            "a".repeat(73),
            "gitlint:disable MessageLineTooLong".to_string(),
        ]
        .join("\n");
        let ignore_commit = validated_commit("Subject".to_string(), ignore_message);
        let violations = ignore_commit.violations;
        assert!(!has_violation(&violations, Rule::MessageLineTooLong));
    }
}
