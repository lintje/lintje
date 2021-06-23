use regex::Regex;
use std::fmt;

lazy_static! {
    static ref SUBJECT_WITH_TICKET: Regex = Regex::new(r"[A-Z]+-\d+").unwrap();
    // Match all GitHub and GitLab keywords
    static ref SUBJECT_WITH_FIX_TICKET: Regex = Regex::new(r"([fF]ix(es|ed|ing)?|[cC]los(e|es|ed|ing)|[rR]esolv(e|es|ed|ing)|[iI]mplement(s|ed|ing)?):? ([^\s]*[\w\-_/]+)?#\d+").unwrap();
    static ref URL_REGEX: Regex = Regex::new(r"https?://\w+").unwrap();
}

#[derive(Debug)]
pub struct Commit {
    pub long_sha: Option<String>,
    pub short_sha: Option<String>,
    pub subject: String,
    pub message: String,
    pub validations: Vec<Validation>,
    pub ignored_rules: Vec<RuleType>,
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
            validations: Vec::<Validation>::new(),
        }
    }

    pub fn find_ignored_rules(message: &String) -> Vec<RuleType> {
        let disable_prefix = "gitlint:disable ";
        let mut ignored = vec![];
        for (_index, line) in message.lines().enumerate() {
            if line.starts_with(disable_prefix) {
                let rule = match &line[disable_prefix.len()..] {
                    "MergeCommit" => Some(RuleType::MergeCommit),
                    "NeedsRebase" => Some(RuleType::NeedsRebase),
                    "SubjectTooLong" => Some(RuleType::SubjectTooLong),
                    "SubjectTooShort" => Some(RuleType::SubjectTooShort),
                    "SubjectMood" => Some(RuleType::SubjectMood),
                    "SubjectCapitalization" => Some(RuleType::SubjectCapitalization),
                    "SubjectPunctuation" => Some(RuleType::SubjectPunctuation),
                    "SubjectTicketNumber" => Some(RuleType::SubjectTicketNumber),
                    "SubjectCliche" => Some(RuleType::SubjectCliche),
                    "MessagePresence" => Some(RuleType::MessagePresence),
                    "MessageLineTooLong" => Some(RuleType::MessageLineTooLong),
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

    fn rule_ignored(&self, rule: RuleType) -> bool {
        self.ignored_rules.contains(&rule)
    }

    pub fn is_valid(&self) -> bool {
        self.validations.is_empty()
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
        if self.rule_ignored(RuleType::MergeCommit) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("Merge branch") {
            self.add_error(RuleType::MergeCommit, format!("Commit is a merge commit."))
        }
    }

    fn validate_needs_rebase(&mut self) {
        if self.rule_ignored(RuleType::NeedsRebase) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("fixup! ") {
            self.add_error(RuleType::NeedsRebase, format!("Subject is a fixup commit."))
        } else if subject.starts_with("squash! ") {
            self.add_error(
                RuleType::NeedsRebase,
                format!("Subject is a squash commit."),
            )
        }
    }

    fn validate_subject_line_length(&mut self) {
        let length = self.subject.len();
        if length > 50 {
            if self.rule_ignored(RuleType::SubjectTooLong) {
                return;
            }

            self.add_error(
                RuleType::SubjectTooLong,
                format!("Subject length is too long: {} characters.", length),
            )
        }
        if length < 5 {
            if self.rule_ignored(RuleType::SubjectTooShort) {
                return;
            }

            self.add_error(
                RuleType::SubjectTooShort,
                format!("Subject length is too short: {} characters.", length),
            )
        }
    }

    fn validate_subject_mood(&mut self) {
        if self.rule_ignored(RuleType::SubjectMood) {
            return;
        }

        match self.subject.split(" ").nth(0) {
            Some(word) => {
                if word.ends_with("ed") || word.ends_with("ing") {
                    self.add_error(
                        RuleType::SubjectMood,
                        "Subject is not imperative mood.".to_string(),
                    )
                }
            }
            None => error!("No first word found of subject."),
        }
    }

    fn validate_subject_capitalization(&mut self) {
        if self.rule_ignored(RuleType::SubjectCapitalization) {
            return;
        }

        match self.subject.chars().nth(0) {
            Some(character) => {
                if !character.is_uppercase() {
                    self.add_error(
                        RuleType::SubjectCapitalization,
                        "Subject does not start with a capital letter.".to_string(),
                    )
                }
            }
            None => error!("No first character found of subject."),
        }
    }

    fn validate_subject_punctuation(&mut self) {
        if self.rule_ignored(RuleType::SubjectPunctuation) {
            return;
        }

        match self.subject.chars().last() {
            Some(character) => {
                if character.is_ascii_punctuation() {
                    self.add_error(
                        RuleType::SubjectPunctuation,
                        format!("Subject ends with punctuation: {}", character),
                    )
                }
            }
            None => error!("No first character found of subject."),
        }
    }

    fn validate_subject_ticket_numbers(&mut self) {
        if self.rule_ignored(RuleType::SubjectTicketNumber) {
            return;
        }

        let subject = &self.subject;
        if SUBJECT_WITH_TICKET.is_match(subject) {
            self.add_error(
                RuleType::SubjectTicketNumber,
                format!("Subject includes a ticket number."),
            )
        } else if SUBJECT_WITH_FIX_TICKET.is_match(subject) {
            self.add_error(
                RuleType::SubjectTicketNumber,
                format!("Subject includes a ticket number."),
            )
        }
    }

    fn validate_subject_cliches(&mut self) {
        if self.rule_ignored(RuleType::SubjectCliche) {
            return;
        }

        let subject = &self.subject;
        if subject.to_lowercase().starts_with("wip ") {
            self.add_error(
                RuleType::SubjectCliche,
                format!("Subject is a 'Work in Progress' commit."),
            )
        } else if subject.to_lowercase() == "wip".to_string() {
            self.add_error(
                RuleType::SubjectCliche,
                format!("Subject is a 'Work in Progress' commit."),
            )
        } else if subject == &"Fix test".to_string() {
            self.add_error(
                RuleType::SubjectCliche,
                format!("Subject is a 'Fix test' commit."),
            )
        } else if subject == &"Fix bug".to_string() {
            self.add_error(
                RuleType::SubjectCliche,
                format!("Subject is a 'Fix bug' commit."),
            )
        }
    }

    fn validate_message_presence(&mut self) {
        if self.rule_ignored(RuleType::MessagePresence) {
            return;
        }

        let length = self.message.len();
        if length == 0 {
            self.add_error(
                RuleType::MessagePresence,
                "Message is not present.".to_string(),
            )
        } else if length < 10 {
            self.add_error(
                RuleType::MessagePresence,
                "Message body is less than 10 characters long.".to_string(),
            )
        }
    }

    fn validate_message_line_length(&mut self) {
        if self.rule_ignored(RuleType::MessageLineTooLong) {
            return;
        }

        match Self::check_line_lengths(self.message.lines()) {
            Some((kind, message)) => self.add_error(kind, message),
            None => {}
        }
    }

    fn check_line_lengths(lines: std::str::Lines) -> Option<(RuleType, String)> {
        for (_index, raw_line) in lines.enumerate() {
            let line = raw_line.trim();
            let length = line.len();
            if length > 72 {
                if URL_REGEX.is_match(line) {
                    continue;
                }
                return Some((
                    RuleType::MessageLineTooLong,
                    "One or more lines in the message are longer than 72 characters.".to_string(),
                ));
            }
        }
        None
    }

    fn add_error(&mut self, kind: RuleType, message: String) {
        self.validations.push(Validation { kind, message })
    }
}

#[derive(Debug, PartialEq)]
pub enum RuleType {
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

impl fmt::Display for RuleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            RuleType::MergeCommit => "MergeCommit",
            RuleType::NeedsRebase => "NeedsRebase",
            RuleType::SubjectTooLong => "SubjectTooLong",
            RuleType::SubjectTooShort => "SubjectTooShort",
            RuleType::SubjectMood => "SubjectMood",
            RuleType::SubjectCapitalization => "SubjectCapitalization",
            RuleType::SubjectPunctuation => "SubjectPunctuation",
            RuleType::SubjectTicketNumber => "SubjectTicketNumber",
            RuleType::SubjectCliche => "SubjectCliche",
            RuleType::MessagePresence => "MessagePresence",
            RuleType::MessageLineTooLong => "MessageLineTooLong",
        };
        write!(f, "{}", label)
    }
}

#[derive(Debug)]
pub struct Validation {
    pub kind: RuleType,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::{Commit, RuleType, Validation};

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

    fn has_validation(validations: &Vec<Validation>, validation: RuleType) -> bool {
        validations.iter().find(|&v| v.kind == validation).is_some()
    }

    #[test]
    fn test_validate_subject_merge_commit() {
        let commit1 = validated_commit("I am not a merge commit".to_string(), "".to_string());
        assert!(!has_validation(&commit1.validations, RuleType::MergeCommit));

        let commit2 = validated_commit(
            "Merge pull request #123 from repo".to_string(),
            "".to_string(),
        );
        assert!(!has_validation(&commit2.validations, RuleType::MergeCommit));

        let commit3 = validated_commit(
            "Merge branch 'main' into develop".to_string(),
            "".to_string(),
        );
        assert!(has_validation(&commit3.validations, RuleType::MergeCommit));

        let ignore_commit = validated_commit(
            "Merge branch 'main' into develop".to_string(),
            "gitlint:disable MergeCommit".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::MergeCommit));
    }

    #[test]
    fn test_validate_needs_rebase() {
        let commit1 = validated_commit("I don't need to be rebased".to_string(), "".to_string());
        assert!(!has_validation(&commit1.validations, RuleType::NeedsRebase));

        let commit2 = validated_commit(
            "fixup! I don't need to be rebased".to_string(),
            "".to_string(),
        );
        assert!(has_validation(&commit2.validations, RuleType::NeedsRebase));

        let commit3 = validated_commit(
            "squash! I don't need to be rebased".to_string(),
            "".to_string(),
        );
        assert!(has_validation(&commit3.validations, RuleType::NeedsRebase));

        let ignore_commit = validated_commit(
            "fixup! I don't need to be rebased".to_string(),
            "gitlint:disable NeedsRebase".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::NeedsRebase));
    }

    #[test]
    fn test_validate_subject_line_length() {
        let commit = validated_commit("a".repeat(50).to_string(), "".to_string());
        let validations = commit.validations;
        assert!(!has_validation(&validations, RuleType::SubjectTooShort));
        assert!(!has_validation(&validations, RuleType::SubjectTooLong));

        let short_commit = validated_commit("a".repeat(4).to_string(), "".to_string());
        assert!(has_validation(
            &short_commit.validations,
            RuleType::SubjectTooShort
        ));

        let long_commit = validated_commit("a".repeat(51).to_string(), "".to_string());
        assert!(has_validation(
            &long_commit.validations,
            RuleType::SubjectTooLong
        ));

        let ignore_commit = validated_commit(
            "a".repeat(51).to_string(),
            "gitlint:disable SubjectTooLong".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::SubjectTooLong));
    }

    #[test]
    fn test_validate_subject_mood() {
        let commit1 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(!has_validation(&commit1.validations, RuleType::SubjectMood));

        let commit2 = validated_commit("Fixed test".to_string(), "".to_string());
        assert!(has_validation(&commit2.validations, RuleType::SubjectMood));

        let commit3 = validated_commit("Fixing test".to_string(), "".to_string());
        assert!(has_validation(&commit3.validations, RuleType::SubjectMood));

        let ignore_commit = validated_commit(
            "Fixed test".to_string(),
            "gitlint:disable SubjectMood".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::SubjectMood));
    }

    #[test]
    fn test_validate_subject_capitalization() {
        let commit1 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(!has_validation(
            &commit1.validations,
            RuleType::SubjectCapitalization
        ));

        let commit2 = validated_commit("fix test".to_string(), "".to_string());
        assert!(has_validation(
            &commit2.validations,
            RuleType::SubjectCapitalization
        ));

        let ignore_commit = validated_commit(
            "fix test".to_string(),
            "gitlint:disable SubjectCapitalization".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(
            &validations,
            RuleType::SubjectCapitalization
        ));
    }

    #[test]
    fn test_validate_subject_punctuation() {
        let commit1 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(!has_validation(
            &commit1.validations,
            RuleType::SubjectPunctuation
        ));

        let commit2 = validated_commit("Fix test.".to_string(), "".to_string());
        assert!(has_validation(
            &commit2.validations,
            RuleType::SubjectPunctuation
        ));

        let commit3 = validated_commit("Fix test!".to_string(), "".to_string());
        assert!(has_validation(
            &commit3.validations,
            RuleType::SubjectPunctuation
        ));

        let commit4 = validated_commit("Fix test?".to_string(), "".to_string());
        assert!(has_validation(
            &commit4.validations,
            RuleType::SubjectPunctuation
        ));

        let ignore_commit = validated_commit(
            "Fix test.".to_string(),
            "gitlint:disable SubjectPunctuation".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::SubjectPunctuation));
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
                has_validation(&commit.validations, RuleType::SubjectTicketNumber),
                "Subject was not considered invalid: {}",
                subject
            );
        }

        let ignore_ticket_number = validated_commit(
            "Fix bug with 'JIRA-1234' type commits".to_string(),
            "gitlint:disable SubjectTicketNumber".to_string(),
        );
        assert!(!has_validation(
            &ignore_ticket_number.validations,
            RuleType::SubjectPunctuation
        ));

        let ignore_issue_number = validated_commit(
            "Fix bug with 'Fix #1234' type commits".to_string(),
            "gitlint:disable SubjectTicketNumber".to_string(),
        );
        assert!(!has_validation(
            &ignore_issue_number.validations,
            RuleType::SubjectPunctuation
        ));
    }

    #[test]
    fn test_validate_subject_cliches() {
        let commit1 = validated_commit("I am not a cliche".to_string(), "".to_string());
        assert!(!has_validation(
            &commit1.validations,
            RuleType::SubjectCliche
        ));

        let wip_prefix_uppercase = validated_commit("WIP something".to_string(), "".to_string());
        assert!(has_validation(
            &wip_prefix_uppercase.validations,
            RuleType::SubjectCliche
        ));

        let wip_prefix_lowercase = validated_commit("wip something".to_string(), "".to_string());
        assert!(has_validation(
            &wip_prefix_lowercase.validations,
            RuleType::SubjectCliche
        ));

        let wip_only_uppercase = validated_commit("WIP".to_string(), "".to_string());
        assert!(has_validation(
            &wip_only_uppercase.validations,
            RuleType::SubjectCliche
        ));

        let wip_only_lowercase = validated_commit("wip".to_string(), "".to_string());
        assert!(has_validation(
            &wip_only_lowercase.validations,
            RuleType::SubjectCliche
        ));

        let commit3 = validated_commit("Fix test".to_string(), "".to_string());
        assert!(has_validation(
            &commit3.validations,
            RuleType::SubjectCliche
        ));

        let commit4 = validated_commit("Fix test for some feature".to_string(), "".to_string());
        assert!(!has_validation(
            &commit4.validations,
            RuleType::SubjectCliche
        ));

        let commit5 = validated_commit("Fix bug".to_string(), "".to_string());
        assert!(has_validation(
            &commit5.validations,
            RuleType::SubjectCliche
        ));

        let commit6 = validated_commit("Fix bug for some feature".to_string(), "".to_string());
        assert!(!has_validation(
            &commit6.validations,
            RuleType::SubjectCliche
        ));

        let ignore_commit = validated_commit(
            "WIP".to_string(),
            "gitlint:disable SubjectCliche".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::SubjectCliche));
    }

    #[test]
    fn test_validate_message_presence() {
        let commit1 = validated_commit("Subject".to_string(), "Hello I am a message.".to_string());
        assert!(!has_validation(
            &commit1.validations,
            RuleType::MessagePresence
        ));

        let commit2 = validated_commit("Subject".to_string(), "".to_string());
        assert!(has_validation(
            &commit2.validations,
            RuleType::MessagePresence
        ));

        let commit3 = validated_commit("Subject".to_string(), "Short.".to_string());
        assert!(has_validation(
            &commit3.validations,
            RuleType::MessagePresence
        ));

        let commit4 = validated_commit("Subject".to_string(), "...".to_string());
        assert!(has_validation(
            &commit4.validations,
            RuleType::MessagePresence
        ));

        let ignore_commit = validated_commit(
            "Subject".to_string(),
            "gitlint:disable MessagePresence".to_string(),
        );
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::MessagePresence));
    }

    #[test]
    fn test_validate_message_line_length() {
        let message1 = ["Hello I am a message.", "Line 2.", &"a".repeat(72)].join("\n");
        let commit1 = validated_commit("Subject".to_string(), message1);
        assert!(!has_validation(
            &commit1.validations,
            RuleType::MessageLineTooLong
        ));

        let message2 = ["a".repeat(72), "a".repeat(73)].join("\n");
        let commit2 = validated_commit("Subject".to_string(), message2);
        assert!(has_validation(
            &commit2.validations,
            RuleType::MessageLineTooLong
        ));

        let message3 = [
            "This message is accepted.".to_string(),
            "This a long line with a link https://tomdebruijn.com/posts/git-is-about-communication/".to_string()
        ].join("\n");
        let commit3 = validated_commit("Subject".to_string(), message3);
        assert!(!has_validation(
            &commit3.validations,
            RuleType::MessageLineTooLong
        ));

        let message4 = [
            "This message is accepted.".to_string(),
            "This a long line with a link http://tomdebruijn.com/posts/git-is-about-communication/"
                .to_string(),
        ]
        .join("\n");
        let commit4 = validated_commit("Subject".to_string(), message4);
        assert!(!has_validation(
            &commit4.validations,
            RuleType::MessageLineTooLong
        ));

        let message5 = [
            "This a too long line with only protocols http:// https:// which is not accepted."
                .to_string(),
        ]
        .join("\n");
        let commit5 = validated_commit("Subject".to_string(), message5);
        assert!(has_validation(
            &commit5.validations,
            RuleType::MessageLineTooLong
        ));

        let ignore_message = [
            "a".repeat(72),
            "a".repeat(73),
            "gitlint:disable MessageLineTooLong".to_string(),
        ]
        .join("\n");
        let ignore_commit = validated_commit("Subject".to_string(), ignore_message);
        let validations = ignore_commit.validations;
        assert!(!has_validation(&validations, RuleType::MessageLineTooLong));
    }
}
