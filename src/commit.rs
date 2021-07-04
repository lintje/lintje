use crate::rule::{rule_by_name, Rule, Violation};
use regex::Regex;

lazy_static! {
    static ref SUBJECT_WITH_TICKET: Regex = Regex::new(r"[A-Z]+-\d+").unwrap();
    // Match all GitHub and GitLab keywords
    static ref SUBJECT_WITH_FIX_TICKET: Regex =
        Regex::new(r"([fF]ix(es|ed|ing)?|[cC]los(e|es|ed|ing)|[rR]esolv(e|es|ed|ing)|[iI]mplement(s|ed|ing)?):? ([^\s]*[\w\-_/]+)?#\d+").unwrap();
    static ref URL_REGEX: Regex = Regex::new(r"https?://\w+").unwrap();
    static ref CODE_BLOCK_LINE_WITH_LANGUAGE: Regex = Regex::new(r"^\s*```\s*([\w]+)?$").unwrap();
    static ref CODE_BLOCK_LINE_END: Regex = Regex::new(r"^\s*```$").unwrap();
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
        "tests",
        "tested",
        "testing",
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
            subject: subject.trim_end().to_string(),
            message: message.trim().to_string(),
            ignored_rules,
            violations: Vec::<Violation>::new(),
        }
    }

    pub fn find_ignored_rules(message: &String) -> Vec<Rule> {
        let disable_prefix = "gitlint:disable ";
        let mut ignored = vec![];
        for line in message.lines().into_iter() {
            if line.starts_with(disable_prefix) {
                let name = &line[disable_prefix.len()..];
                match rule_by_name(name) {
                    Some(rule) => ignored.push(rule),
                    None => warn!("Attempted to ignore unknown rule: {}", name),
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
        self.validate_subject_whitespace();
        self.validate_subject_capitalization();
        self.validate_subject_punctuation();
        self.validate_subject_ticket_numbers();
        self.validate_subject_cliches();
        self.validate_message_presence();
        self.validate_message_line_length();
    }

    // Note: Some merge commits are ignored in git.rs and won't be validated here, because they are
    // Pull/Merge Requests, which are valid.
    fn validate_merge_commit(&mut self) {
        if self.rule_ignored(Rule::MergeCommit) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("Merge branch") {
            self.add_violation(
                Rule::MergeCommit,
                format!("Rebase branches on the base branch, rather than merging the base branch with a merge commit.")
            )
        }
    }

    fn validate_needs_rebase(&mut self) {
        if self.rule_ignored(Rule::NeedsRebase) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("fixup! ") {
            self.add_violation(
                Rule::NeedsRebase,
                format!("Squash fixup commits before merging."),
            )
        } else if subject.starts_with("squash! ") {
            self.add_violation(
                Rule::NeedsRebase,
                format!("Squash squash commits before merging."),
            )
        }
    }

    fn validate_subject_line_length(&mut self) {
        let length = self.subject.chars().count();
        if length > 50 {
            if self.rule_ignored(Rule::SubjectLength) {
                return;
            }

            self.add_violation(
                Rule::SubjectLength,
                format!(
                    "Subject is too long: {} characters. Shorten the subject to max 50 characters.",
                    length
                ),
            )
        }
        if length < 5 {
            if self.rule_ignored(Rule::SubjectLength) {
                return;
            }

            self.add_violation(
                Rule::SubjectLength,
                format!(
                    "Subject is too short: {} characters. Describe the change in more detail.",
                    length
                ),
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
                        "Use the imperative mood for the commit subject.".to_string(),
                    )
                }
            }
            None => {
                error!("SubjectMood validation failure: No first word found of commit subject.")
            }
        }
    }

    fn validate_subject_whitespace(&mut self) {
        if self.rule_ignored(Rule::SubjectWhitespace) {
            return;
        }

        match self.subject.chars().nth(0) {
            Some(character) => {
                if character.is_whitespace() {
                    self.add_violation(
                        Rule::SubjectWhitespace,
                        "Remove leading whitespace from the commit subject.".to_string(),
                    )
                }
            }
            None => {
                error!("SubjectWhitespace validation failure: No first character found of subject.")
            }
        }
    }

    fn validate_subject_capitalization(&mut self) {
        if self.rule_ignored(Rule::SubjectCapitalization) {
            return;
        }

        match self.subject.chars().nth(0) {
            Some(character) => {
                if character.is_lowercase() {
                    self.add_violation(
                        Rule::SubjectCapitalization,
                        "Start the commit subject a capital letter.".to_string(),
                    )
                }
            }
            None => {
                error!("SubjectCapitalization validation failure: No first character found of subject.")
            }
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
                        format!(
                            "Remove punctuation from the end of the commit subject: {}",
                            character
                        ),
                    )
                }
            }
            None => {
                error!("SubjectPunctuation validation failure: No last character found of subject.")
            }
        }
    }

    fn validate_subject_ticket_numbers(&mut self) {
        if self.rule_ignored(Rule::SubjectTicketNumber) {
            return;
        }

        let subject = &self.subject;
        if SUBJECT_WITH_TICKET.is_match(subject) || SUBJECT_WITH_FIX_TICKET.is_match(subject) {
            self.add_violation(
                Rule::SubjectTicketNumber,
                format!("Remove the ticket number from the commit subject. Move it to the message body."),
            )
        }
    }

    fn validate_subject_cliches(&mut self) {
        if self.rule_ignored(Rule::SubjectCliche) {
            return;
        }

        let subject = &self.subject;
        let wip_commit = subject.to_lowercase().starts_with("wip ")
            || subject.to_lowercase() == "wip".to_string();
        if wip_commit {
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

        let length = self.message.chars().count();
        if length == 0 {
            self.add_violation(
                Rule::MessagePresence,
                "Add a message body to provide more context about the change and why it was made."
                    .to_string(),
            )
        } else if length < 10 {
            self.add_violation(
                Rule::MessagePresence,
                "Add a longer message body to provide more context about the change and why it was made.".to_string(),
            )
        }
    }

    fn validate_message_line_length(&mut self) {
        if self.rule_ignored(Rule::MessageLineLength) {
            return;
        }

        let mut code_block_style = CodeBlockStyle::None;
        let mut previous_line_was_empty_line = false;
        let mut violations = vec![];
        for (index, raw_line) in self.message.lines().enumerate() {
            let line = raw_line.trim_end();
            let length = line.chars().count();
            match code_block_style {
                CodeBlockStyle::Fenced => {
                    if CODE_BLOCK_LINE_END.is_match(line) {
                        code_block_style = CodeBlockStyle::None
                    }
                }
                CodeBlockStyle::Indenting => {
                    if !line.starts_with("    ") {
                        code_block_style = CodeBlockStyle::None;
                    }
                }
                CodeBlockStyle::None => {
                    if CODE_BLOCK_LINE_WITH_LANGUAGE.is_match(line) {
                        code_block_style = CodeBlockStyle::Fenced
                    } else if line.starts_with("    ") && previous_line_was_empty_line {
                        code_block_style = CodeBlockStyle::Indenting
                    }
                }
            }
            if code_block_style != CodeBlockStyle::None {
                // When in a code block, skip line length validation
                continue;
            }
            if length > 72 {
                if URL_REGEX.is_match(line) {
                    continue;
                }
                violations.push((
                    Rule::MessageLineLength,
                    format!("Line {} of the message body is too long. Shorten the line to maximum 72 characters.", index + 1),
                ))
            }
            previous_line_was_empty_line = line.trim() == "";
        }

        for (rule, message) in violations {
            self.add_violation(rule, message);
        }
    }

    fn add_violation(&mut self, rule: Rule, message: String) {
        self.violations.push(Violation { rule, message })
    }
}

#[derive(PartialEq)]
enum CodeBlockStyle {
    None,
    Fenced,
    Indenting,
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

    fn assert_commit_valid_for(commit: Commit, rule: &Rule) {
        assert!(
            !has_violation(&commit.violations, rule),
            "Commit was not considered valid: {:?}",
            commit
        );
    }

    fn assert_commit_invalid_for(commit: Commit, rule: &Rule) {
        assert!(
            has_violation(&commit.violations, rule),
            "Commit was not considered invalid: {:?}",
            commit
        );
    }

    fn assert_commit_subject_as_valid(subject: &str, rule: &Rule) {
        let commit = validated_commit(subject.to_string(), "".to_string());
        assert_commit_valid_for(commit, rule);
    }

    fn assert_commit_subjects_as_valid(subjects: Vec<&str>, rule: &Rule) {
        for subject in subjects {
            assert_commit_subject_as_valid(subject, rule)
        }
    }

    fn assert_commit_subject_as_invalid(subject: &str, rule: &Rule) {
        let commit = validated_commit(subject.to_string(), "".to_string());
        assert_commit_invalid_for(commit, rule);
    }

    fn assert_commit_subjects_as_invalid(subjects: Vec<&str>, rule: &Rule) {
        for subject in subjects {
            assert_commit_subject_as_invalid(subject, rule)
        }
    }

    fn has_violation(violations: &Vec<Violation>, rule: &Rule) -> bool {
        violations.iter().find(|&v| &v.rule == rule).is_some()
    }

    #[test]
    fn test_validate_subject_merge_commit() {
        assert_commit_subject_as_valid("I am not a merge commit", &Rule::MergeCommit);
        assert_commit_subject_as_valid("Merge pull request #123 from repo", &Rule::MergeCommit);
        assert_commit_subject_as_invalid("Merge branch 'main' into develop", &Rule::MergeCommit);

        let ignore_commit = validated_commit(
            "Merge branch 'main' into develop".to_string(),
            "gitlint:disable MergeCommit".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::MergeCommit);
    }

    #[test]
    fn test_validate_needs_rebase() {
        assert_commit_subject_as_valid("I don't need a rebase", &Rule::NeedsRebase);
        assert_commit_subject_as_invalid("fixup! I don't need a rebase", &Rule::NeedsRebase);
        assert_commit_subject_as_invalid("squash! I don't need a rebase", &Rule::NeedsRebase);

        let ignore_commit = validated_commit(
            "fixup! I don't need to be rebased".to_string(),
            "gitlint:disable NeedsRebase".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::NeedsRebase);
    }

    #[test]
    fn test_validate_subject_line_length() {
        let subject = "a".repeat(50);
        assert_commit_subject_as_valid(subject.as_str(), &Rule::SubjectLength);

        let short_subject = "a".repeat(4);
        assert_commit_subject_as_invalid(short_subject.as_str(), &Rule::SubjectLength);

        let long_subject = "a".repeat(51);
        assert_commit_subject_as_invalid(long_subject.as_str(), &Rule::SubjectLength);

        let emoji_subject = "✨".repeat(50);
        assert_commit_subject_as_valid(emoji_subject.as_str(), &Rule::SubjectLength);

        let hiragana_short_subject = "あ".repeat(50);
        assert_commit_subject_as_valid(hiragana_short_subject.as_str(), &Rule::SubjectLength);

        let hiragana_long_subject = "あ".repeat(51);
        assert_commit_subject_as_invalid(hiragana_long_subject.as_str(), &Rule::SubjectLength);

        let ignore_commit = validated_commit(
            "a".repeat(51).to_string(),
            "gitlint:disable SubjectLength".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::SubjectLength);
    }

    #[test]
    fn test_validate_subject_mood() {
        let subjects = vec!["Fix test"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectMood);

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
            assert_commit_subject_as_invalid(subject.as_str(), &Rule::SubjectMood);
        }

        let ignore_commit = validated_commit(
            "Fixed test".to_string(),
            "gitlint:disable SubjectMood".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::SubjectMood);
    }

    #[test]
    fn test_validate_subject_whitespace() {
        let subjects = vec!["Fix test"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectWhitespace);

        let invalid_subjects = vec![" Fix test", "\tFix test", "\x20Fix test"];
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectWhitespace);

        let ignore_commit = validated_commit(
            " Fix test".to_string(),
            "gitlint:disable SubjectWhitespace".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::SubjectWhitespace);
    }

    #[test]
    fn test_validate_subject_capitalization() {
        let subjects = vec!["Fix test"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectCapitalization);

        let invalid_subjects = vec!["fix test"];
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectCapitalization);

        let ignore_commit = validated_commit(
            "fix test".to_string(),
            "gitlint:disable SubjectCapitalization".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::SubjectCapitalization);
    }

    #[test]
    fn test_validate_subject_punctuation() {
        let subjects = vec!["Fix test"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectPunctuation);

        let invalid_subjects = vec!["Fix test.", "Fix test!", "Fix test?", "Fix test:"];
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectPunctuation);

        let ignore_commit = validated_commit(
            "Fix test.".to_string(),
            "gitlint:disable SubjectPunctuation".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::SubjectPunctuation);
    }

    #[test]
    fn test_validate_subject_ticket() {
        assert_commit_subject_as_valid("This is a normal commit", &Rule::SubjectTicketNumber);

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
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectTicketNumber);

        let ignore_ticket_number = validated_commit(
            "Fix bug with 'JIRA-1234' type commits".to_string(),
            "gitlint:disable SubjectTicketNumber".to_string(),
        );
        assert_commit_valid_for(ignore_ticket_number, &Rule::SubjectTicketNumber);

        let ignore_issue_number = validated_commit(
            "Fix bug with 'Fix #1234' type commits".to_string(),
            "gitlint:disable SubjectTicketNumber".to_string(),
        );
        assert_commit_valid_for(ignore_issue_number, &Rule::SubjectTicketNumber);
    }

    #[test]
    fn test_validate_subject_cliches() {
        let subjects = vec![
            "I am not a cliche",
            "Fix test for some feature",
            "Fix bug for some feature",
        ];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectCliche);

        let invalid_subjects = vec![
            "WIP something",
            "wip something",
            "Wip something",
            "WIP",
            "wip",
            "Fix test",
            "Fix bug",
        ];
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectCliche);

        let ignore_commit = validated_commit(
            "WIP".to_string(),
            "gitlint:disable SubjectCliche".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::SubjectCliche);
    }

    #[test]
    fn test_validate_message_presence() {
        let commit1 = validated_commit("Subject".to_string(), "Hello I am a message.".to_string());
        assert_commit_valid_for(commit1, &Rule::MessagePresence);

        let commit2 = validated_commit("Subject".to_string(), "".to_string());
        assert_commit_invalid_for(commit2, &Rule::MessagePresence);

        let commit3 = validated_commit("Subject".to_string(), "Short.".to_string());
        assert_commit_invalid_for(commit3, &Rule::MessagePresence);

        let commit4 = validated_commit("Subject".to_string(), "...".to_string());
        assert_commit_invalid_for(commit4, &Rule::MessagePresence);

        let ignore_commit = validated_commit(
            "Subject".to_string(),
            "gitlint:disable MessagePresence".to_string(),
        );
        assert_commit_valid_for(ignore_commit, &Rule::MessagePresence);
    }

    #[test]
    fn test_validate_message_line_length() {
        let message1 = ["Hello I am a message.", "Line 2.", &"a".repeat(72)].join("\n");
        let commit1 = validated_commit("Subject".to_string(), message1);
        assert_commit_valid_for(commit1, &Rule::MessageLineLength);

        let message2 = ["a".repeat(72), "a".repeat(73)].join("\n");
        let commit2 = validated_commit("Subject".to_string(), message2);
        assert_commit_invalid_for(commit2, &Rule::MessageLineLength);

        let message3 = [
            "This message is accepted.".to_string(),
            "This a long line with a link https://tomdebruijn.com/posts/git-is-about-communication/".to_string()
        ].join("\n");
        let commit3 = validated_commit("Subject".to_string(), message3);
        assert_commit_valid_for(commit3, &Rule::MessageLineLength);

        let message4 = [
            "This message is accepted.".to_string(),
            "This a long line with a link http://tomdebruijn.com/posts/git-is-about-communication/"
                .to_string(),
        ]
        .join("\n");
        let commit4 = validated_commit("Subject".to_string(), message4);
        assert_commit_valid_for(commit4, &Rule::MessageLineLength);

        let message5 = [
            "This a too long line with only protocols http:// https:// which is not accepted."
                .to_string(),
        ]
        .join("\n");
        let commit5 = validated_commit("Subject".to_string(), message5);
        assert_commit_invalid_for(commit5, &Rule::MessageLineLength);

        let hiragana_short_message = ["あ".repeat(72)].join("\n");
        let hiragana_short_commit = validated_commit("Subject".to_string(), hiragana_short_message);
        assert_commit_valid_for(hiragana_short_commit, &Rule::MessageLineLength);

        let hiragana_long_message = ["あ".repeat(73)].join("\n");
        let hiragana_long_commit = validated_commit("Subject".to_string(), hiragana_long_message);
        assert_commit_invalid_for(hiragana_long_commit, &Rule::MessageLineLength);

        let ignore_message = [
            "a".repeat(72),
            "a".repeat(73),
            "gitlint:disable MessageLineLength".to_string(),
        ]
        .join("\n");
        let ignore_commit = validated_commit("Subject".to_string(), ignore_message);
        assert_commit_valid_for(ignore_commit, &Rule::MessageLineLength);
    }

    #[test]
    fn test_validate_message_line_length_in_code_block() {
        let valid_fenced_code_blocks = [
            "Beginning of message.",
            "```",
            &"a".repeat(73), // Valid, inside code block
            &"b".repeat(73),
            &"c".repeat(73),
            "```",
            "Normal line",
            "```md",
            &"I am markdown",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "Normal line",
            "``` yaml",
            &"I am yaml",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "Normal line",
            "```  elixir ",
            &"I am elixir",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "",
            "  ```",
            &"  I am elixir",
            &"  d".repeat(73), // Valid, inside fenced indented code block
            "  ```",
            "End of message",
        ]
        .join("\n");
        assert_commit_valid_for(
            validated_commit("Subject".to_string(), valid_fenced_code_blocks),
            &Rule::MessageLineLength,
        );

        let invalid_long_line_outside_fenced_code_block = [
            "Beginning of message.",
            "```",
            &"a".repeat(73),
            "```",
            &"a".repeat(73), // Long line outside code block is invalid
            "End of message",
        ]
        .join("\n");
        assert_commit_invalid_for(
            validated_commit(
                "Subject".to_string(),
                invalid_long_line_outside_fenced_code_block,
            ),
            &Rule::MessageLineLength,
        );

        let invalid_fenced_code_block_language_identifier = [
            "Beginning of message.",
            "``` m d", // Invald language identifier
            &"a".repeat(73),
            "```",
            "End of message",
        ]
        .join("\n");
        assert_commit_invalid_for(
            validated_commit(
                "Subject".to_string(),
                invalid_fenced_code_block_language_identifier,
            ),
            &Rule::MessageLineLength,
        );

        let valid_indented_code_blocks = [
            "Beginning of message.",
            "",
            "    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "",
            "    ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "    ",
            "    ddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "",
            "End of message",
        ]
        .join("\n");
        assert_commit_valid_for(
            validated_commit("Subject".to_string(), valid_indented_code_blocks),
            &Rule::MessageLineLength,
        );

        let invalid_long_ling_outside_indended_code_block = [
            "Beginning of message.",
            "",
            "    aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "    bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "",
            "    ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "",
            "End of message",
            &"a".repeat(73), // Long line outside code block is invalid
        ]
        .join("\n");
        assert_commit_invalid_for(
            validated_commit(
                "Subject".to_string(),
                invalid_long_ling_outside_indended_code_block,
            ),
            &Rule::MessageLineLength,
        );
    }
}
