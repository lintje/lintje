use crate::issue::{Context, Issue, Position};
use crate::rule::{rule_by_name, Rule};
use crate::utils::{character_count_for_bytes_index, line_length_stats};
use core::ops::Range;
use regex::{Regex, RegexBuilder};

lazy_static! {
    // Jira project keys are at least 2 uppercase characters long.
    // AB-123
    // JIRA-123
    static ref SUBJECT_WITH_TICKET: Regex = Regex::new(r"[A-Z]{2,}-\d+").unwrap();
    // Match all GitHub and GitLab keywords
    static ref CONTAINS_FIX_TICKET: Regex =
        Regex::new(r"([fF]ix(es|ed|ing)?|[cC]los(e|es|ed|ing)|[rR]esolv(e|es|ed|ing)|[iI]mplement(s|ed|ing)?):? ([^\s]*[\w\-_/]+)?[#!]{1}\d+").unwrap();
    // Match "Part of #123"
    static ref LINK_TO_TICKET: Regex = {
        let mut tempregex = RegexBuilder::new(r"(part of|related):? ([^\s]*[\w\-_/]+)?[#!]{1}\d+");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };

    static ref URL_REGEX: Regex = Regex::new(r"https?://\w+").unwrap();
    static ref CODE_BLOCK_LINE_WITH_LANGUAGE: Regex = Regex::new(r"^\s*```\s*([\w]+)?$").unwrap();
    static ref CODE_BLOCK_LINE_END: Regex = Regex::new(r"^\s*```$").unwrap();
}

#[derive(Debug)]
pub struct Commit {
    pub long_sha: Option<String>,
    pub short_sha: Option<String>,
    pub email: Option<String>,
    pub subject: String,
    pub message: String,
    pub has_changes: bool,
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
        has_changes: bool,
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
            has_changes,
            ignored: false,
            ignored_rules,
            issues: Vec::<Issue>::new(),
        }
    }

    pub fn find_ignored_rules(message: &str) -> Vec<Rule> {
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
            self.validate_subject_ticket_numbers();
            self.validate_message_ticket_numbers();
            self.validate_rule(&Rule::MessageEmptyFirstLine);
            self.validate_rule(&Rule::MessagePresence);
            self.validate_message_line_length();
        }
        self.validate_changes();
    }

    fn validate_rule(&mut self, rule: &Rule) {
        if self.rule_ignored(rule) {
            return;
        }

        let instance = rule.instance();
        match instance.validate(self) {
            Some(mut issues) => {
                self.issues.append(&mut issues);
            }
            None => {
                debug!("No issues found for rule '{}'", rule);
            }
        }
    }

    fn validate_subject_ticket_numbers(&mut self) {
        if self.rule_ignored(&Rule::SubjectTicketNumber) {
            return;
        }

        let subject = &self.subject.to_string();
        if let Some(captures) = SUBJECT_WITH_TICKET.captures(subject) {
            match captures.get(0) {
                Some(capture) => self.add_subject_ticket_number_error(capture),
                None => {
                    error!(
                        "SubjectTicketNumber: Unable to fetch ticket number match from subject."
                    );
                }
            };
        }
        if let Some(captures) = CONTAINS_FIX_TICKET.captures(subject) {
            match captures.get(0) {
                Some(capture) => self.add_subject_ticket_number_error(capture),
                None => {
                    error!(
                        "SubjectTicketNumber: Unable to fetch ticket number match from subject."
                    );
                }
            };
        }
    }

    fn add_subject_ticket_number_error(&mut self, capture: regex::Match) {
        let subject = self.subject.to_string();
        let line_count = self.message.lines().count();
        let base_line_count = if line_count == 0 { 3 } else { line_count + 2 };
        let context = vec![
            Context::subject_error(
                subject,
                capture.range(),
                "Remove the ticket number from the subject".to_string(),
            ),
            Context::message_line(base_line_count, "".to_string()),
            Context::message_line_addition(
                base_line_count + 1,
                capture.as_str().to_string(),
                Range {
                    start: 0,
                    end: capture.range().len(),
                },
                "Move the ticket number to the message body".to_string(),
            ),
        ];
        self.add_subject_error(
            Rule::SubjectTicketNumber,
            "The subject contains a ticket number".to_string(),
            character_count_for_bytes_index(&self.subject, capture.start()),
            context,
        );
    }

    fn validate_message_line_length(&mut self) {
        if self.rule_ignored(&Rule::MessageLineLength) {
            return;
        }

        let mut code_block_style = CodeBlockStyle::None;
        let mut previous_line_was_empty_line = false;
        let mut issues = vec![];
        for (index, raw_line) in self.message.lines().enumerate() {
            let line = raw_line.trim_end();
            let (width, line_stats) = line_length_stats(line, 72);
            match code_block_style {
                CodeBlockStyle::Fenced => {
                    if CODE_BLOCK_LINE_END.is_match(line) {
                        code_block_style = CodeBlockStyle::None;
                    }
                }
                CodeBlockStyle::Indenting => {
                    if !line.starts_with("    ") {
                        code_block_style = CodeBlockStyle::None;
                    }
                }
                CodeBlockStyle::None => {
                    if CODE_BLOCK_LINE_WITH_LANGUAGE.is_match(line) {
                        code_block_style = CodeBlockStyle::Fenced;
                    } else if line.starts_with("    ") && previous_line_was_empty_line {
                        code_block_style = CodeBlockStyle::Indenting;
                    }
                }
            }
            if code_block_style != CodeBlockStyle::None {
                // When in a code block, skip line length validation
                continue;
            }
            if width > 72 {
                if URL_REGEX.is_match(line) {
                    continue;
                }
                let line_number = index + 2; // + 1 for subject + 1 for zero index
                let context = Context::message_line_error(
                    line_number,
                    line.to_string(),
                    Range {
                        start: line_stats.bytes_index,
                        end: line.len(),
                    },
                    "Shorten line to maximum 72 characters".to_string(),
                );
                issues.push((
                    Rule::MessageLineLength,
                    format!(
                        "Line {} in the message body is longer than 72 characters",
                        line_number
                    ),
                    Position::MessageLine {
                        line: line_number,
                        column: line_stats.char_count + 1, // + 1 because the next char is the problem
                    },
                    vec![context],
                ));
            }
            previous_line_was_empty_line = line.trim() == "";
        }

        for (rule, message, position, context) in issues {
            self.add_message_error(rule, message, position, context);
        }
    }

    fn validate_message_ticket_numbers(&mut self) {
        let message = &self.message.to_string();
        if CONTAINS_FIX_TICKET.captures(message).is_none()
            && LINK_TO_TICKET.captures(message).is_none()
        {
            let line_count = message.lines().count() + 1; // + 1 for subject
            let last_line = if line_count == 1 {
                self.subject.to_string()
            } else {
                message.lines().last().unwrap_or("").to_string()
            };
            let context = vec![
                Context::message_line(line_count, last_line),
                // Add empty line for spacing
                Context::message_line(line_count + 1, "".to_string()),
                // Suggestion because it indicates a suggested change?
                Context::message_line_addition(
                    line_count + 2,
                    "Fixes #123".to_string(),
                    Range { start: 0, end: 10 },
                    "Consider adding a reference to a ticket or issue".to_string(),
                ),
            ];
            self.add_hint(
                Rule::MessageTicketNumber,
                "The message body does not contain a ticket or issue number".to_string(),
                Position::MessageLine {
                    line: line_count + 2,
                    column: 1,
                },
                context,
            );
        }
    }

    fn validate_changes(&mut self) {
        if self.rule_ignored(&Rule::DiffPresence) {
            return;
        }

        if !self.has_changes {
            let context_line = "0 files changed, 0 insertions(+), 0 deletions(-)".to_string();
            let context_length = context_line.len();
            let context = Context::diff_error(
                context_line,
                Range {
                    start: 0,
                    end: context_length,
                },
                "Add changes to the commit or remove the commit".to_string(),
            );
            self.add_error(
                Rule::DiffPresence,
                "No file changes found".to_string(),
                Position::Diff,
                vec![context],
            );
        }
    }

    fn add_error(
        &mut self,
        rule: Rule,
        message: String,
        position: Position,
        context: Vec<Context>,
    ) {
        self.issues
            .push(Issue::error(rule, message, position, context));
    }

    fn add_subject_error(
        &mut self,
        rule: Rule,
        message: String,
        column: usize,
        context: Vec<Context>,
    ) {
        self.add_error(
            rule,
            message,
            Position::Subject { line: 1, column },
            context,
        );
    }

    fn add_message_error(
        &mut self,
        rule: Rule,
        message: String,
        position: Position,
        context: Vec<Context>,
    ) {
        self.add_error(rule, message, position, context);
    }

    fn add_hint(&mut self, rule: Rule, message: String, position: Position, context: Vec<Context>) {
        self.issues
            .push(Issue::hint(rule, message, position, context));
    }

    pub fn has_issue(&self, rule: &Rule) -> bool {
        self.issues.iter().any(|issue| &issue.rule == rule)
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
    use crate::issue::Position;
    use crate::rule::Rule;
    use crate::test::formatted_context;
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
    fn test_validate_subject_ticket() {
        let valid_ticket_subjects = vec![
            "This is a normal commit",
            "Fix #", // Not really good subjects, but won't fail on this rule
            "Fix ##123",
            "Fix #a123",
            "Fix !",
            "Fix !!123",
            "Fix !a123",
            "Change A-1 config",
            "Change A-12 config",
        ];
        assert_commit_subjects_as_valid(valid_ticket_subjects, &Rule::SubjectTicketNumber);

        let invalid_ticket_subjects = vec![
            "JI-1",
            "JI-12",
            "JI-1234567890",
            "JIR-1",
            "JIR-12",
            "JIR-1234567890",
            "JIRA-12",
            "JIRA-123",
            "JIRA-1234",
            "JIRA-1234567890",
            "Fix JIRA-1234 lorem",
        ];
        assert_commit_subjects_as_invalid(invalid_ticket_subjects, &Rule::SubjectTicketNumber);

        let ticket_number = validated_commit("Fix JIRA-123 about email validation", "");
        let issue = find_issue(ticket_number.issues, &Rule::SubjectTicketNumber);
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(5));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | Fix JIRA-123 about email validation\n\
             \x20\x20|     ^^^^^^^^ Remove the ticket number from the subject\n\
                \x20~~~\n\
                   3 | \n\
                   4 | JIRA-123\n\
             \x20\x20| -------- Move the ticket number to the message body\n"
        );

        let ticket_number_unicode =
            validated_commit("Fix ❤\u{fe0f} JIRA-123 about email validation", "");
        let issue = find_issue(ticket_number_unicode.issues, &Rule::SubjectTicketNumber);
        assert_eq!(issue.position, subject_position(7));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | Fix ❤️ JIRA-123 about email validation\n\
             \x20\x20|       ^^^^^^^^ Remove the ticket number from the subject\n\
                \x20~~~\n\
                   3 | \n\
                   4 | JIRA-123\n\
             \x20\x20| -------- Move the ticket number to the message body\n"
        );

        let invalid_subjects = vec![
            "Fix {}1234",
            "Fixed {}1234",
            "Fixes {}1234",
            "Fixing {}1234",
            "Fix {}1234 lorem",
            "Fix: {}1234 lorem",
            "Fix my-org/repo{}1234 lorem",
            "Fix https://examplegithosting.com/my-org/repo{}1234 lorem",
            "Commit fixes {}1234",
            "Close {}1234",
            "Closed {}1234",
            "Closes {}1234",
            "Closing {}1234",
            "Close {}1234 lorem",
            "Close: {}1234 lorem",
            "Commit closes {}1234",
            "Resolve {}1234",
            "Resolved {}1234",
            "Resolves {}1234",
            "Resolving {}1234",
            "Resolve {}1234 lorem",
            "Resolve: {}1234 lorem",
            "Commit resolves {}1234",
            "Implement {}1234",
            "Implemented {}1234",
            "Implements {}1234",
            "Implementing {}1234",
            "Implement {}1234 lorem",
            "Implement: {}1234 lorem",
            "Commit implements {}1234",
        ];
        let invalid_issue_subjects = invalid_subjects
            .iter()
            .map(|s| s.replace("{}", "#"))
            .collect();
        assert_commit_subjects_as_invalid(invalid_issue_subjects, &Rule::SubjectTicketNumber);
        let invalid_merge_request_subjects = invalid_subjects
            .iter()
            .map(|s| s.replace("{}", "!"))
            .collect();
        assert_commit_subjects_as_invalid(
            invalid_merge_request_subjects,
            &Rule::SubjectTicketNumber,
        );

        let fix_ticket = validated_commit("Email validation: Fixes #123 for good", "");
        let issue = find_issue(fix_ticket.issues, &Rule::SubjectTicketNumber);
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(19));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | Email validation: Fixes #123 for good\n\
             \x20\x20|                   ^^^^^^^^^^ Remove the ticket number from the subject\n\
                \x20~~~\n\
                   3 | \n\
                   4 | Fixes #123\n\
             \x20\x20| ---------- Move the ticket number to the message body\n"
        );

        let fix_ticket_unicode = validated_commit("Email validatiｏn: Fixes #123", "");
        let issue = find_issue(fix_ticket_unicode.issues, &Rule::SubjectTicketNumber);
        assert_eq!(issue.position, subject_position(19));

        let fix_link = validated_commit("Email validation: Closed org/repo#123 for good", "");
        let issue = find_issue(fix_link.issues, &Rule::SubjectTicketNumber);
        assert_eq!(issue.message, "The subject contains a ticket number");
        assert_eq!(issue.position, subject_position(19));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   1 | Email validation: Closed org/repo#123 for good\n\
             \x20\x20|                   ^^^^^^^^^^^^^^^^^^^ Remove the ticket number from the subject\n\
                \x20~~~\n\
                   3 | \n\
                   4 | Closed org/repo#123\n\
             \x20\x20| ------------------- Move the ticket number to the message body\n"
        );

        let ignore_ticket_number = validated_commit(
            "Fix bug with 'JIRA-1234' type commits".to_string(),
            "lintje:disable SubjectTicketNumber".to_string(),
        );
        assert_commit_valid_for(&ignore_ticket_number, &Rule::SubjectTicketNumber);

        let ignore_issue_number = validated_commit(
            "Fix bug with 'Fix #1234' type commits".to_string(),
            "lintje:disable SubjectTicketNumber".to_string(),
        );
        assert_commit_valid_for(&ignore_issue_number, &Rule::SubjectTicketNumber);

        let ignore_merge_request_number = validated_commit(
            "Fix bug with 'Fix !1234' type commits".to_string(),
            "lintje:disable SubjectTicketNumber".to_string(),
        );
        assert_commit_valid_for(&ignore_merge_request_number, &Rule::SubjectTicketNumber);
    }

    #[test]
    fn test_validate_message_line_length() {
        let message1 = ["Hello I am a message.", "Line 2.", &"a".repeat(72)].join("\n");
        let commit1 = validated_commit("Subject".to_string(), message1);
        assert_commit_valid_for(&commit1, &Rule::MessageLineLength);

        let long_message = ["".to_string(), "a".repeat(72), "a".repeat(73)].join("\n");
        let long_line = validated_commit("Subject", &long_message);
        let issue = find_issue(long_line.issues, &Rule::MessageLineLength);
        assert_eq!(
            issue.message,
            "Line 4 in the message body is longer than 72 characters"
        );
        assert_eq!(issue.position, message_position(4, 73));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   4 | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
             \x20\x20|                                                                         ^ Shorten line to maximum 72 characters\n"
        );

        let message3 = [
            "This message is accepted.".to_string(),
            "This a long line with a link https://tomdebruijn.com/posts/git-is-about-communication/".to_string()
        ].join("\n");
        let commit3 = validated_commit("Subject".to_string(), message3);
        assert_commit_valid_for(&commit3, &Rule::MessageLineLength);

        let message4 = [
            "This message is accepted.".to_string(),
            "This a long line with a link http://tomdebruijn.com/posts/git-is-about-communication/"
                .to_string(),
        ]
        .join("\n");
        let commit4 = validated_commit("Subject".to_string(), message4);
        assert_commit_valid_for(&commit4, &Rule::MessageLineLength);

        let message5 =
            "This a too long line with only protocols http:// https:// which is not accepted."
                .to_string();
        let commit5 = validated_commit("Subject".to_string(), message5);
        assert_commit_invalid_for(&commit5, &Rule::MessageLineLength);

        let long_message =
            "This a too long line with only protocols http:// https://, not accepted!!".to_string();
        let long_line = validated_commit("Subject", &long_message);
        let issue = find_issue(long_line.issues, &Rule::MessageLineLength);
        assert_eq!(
            issue.message,
            "Line 2 in the message body is longer than 72 characters"
        );
        assert_eq!(issue.position, message_position(2, 73));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   2 | This a too long line with only protocols http:// https://, not accepted!!\n\
             \x20\x20|                                                                         ^ Shorten line to maximum 72 characters\n"
        );

        // This emoji display width is 2
        let emoji_short_message = ["✨".repeat(36)].join("\n");
        let emoji_short_commit = validated_commit("Subject".to_string(), emoji_short_message);
        assert_commit_valid_for(&emoji_short_commit, &Rule::MessageLineLength);

        let emoji_long_message = ["✨".repeat(37)].join("\n");
        let emoji_long_commit = validated_commit("Subject".to_string(), emoji_long_message);
        assert_commit_invalid_for(&emoji_long_commit, &Rule::MessageLineLength);

        // Hiragana display width is 2
        let hiragana_short_message = ["あ".repeat(36)].join("\n");
        let hiragana_short_commit = validated_commit("Subject".to_string(), hiragana_short_message);
        assert_commit_valid_for(&hiragana_short_commit, &Rule::MessageLineLength);

        let hiragana_long_message = ["あ".repeat(37)].join("\n");
        let hiragana_long_commit = validated_commit("Subject".to_string(), hiragana_long_message);
        assert_commit_invalid_for(&hiragana_long_commit, &Rule::MessageLineLength);

        let ignore_message = [
            "a".repeat(72),
            "a".repeat(73),
            "lintje:disable MessageLineLength".to_string(),
        ]
        .join("\n");
        let ignore_commit = validated_commit("Subject".to_string(), ignore_message);
        assert_commit_valid_for(&ignore_commit, &Rule::MessageLineLength);
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
            "I am markdown",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "Normal line",
            "``` yaml",
            "I am yaml",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "Normal line",
            "```  elixir ",
            "I am elixir",
            &"d".repeat(73), // Valid, inside code block
            "```",
            "",
            "  ```",
            "  I am elixir",
            &"  d".repeat(73), // Valid, inside fenced indented code block
            "  ```",
            "End of message",
        ]
        .join("\n");
        assert_commit_valid_for(
            &validated_commit("Subject".to_string(), valid_fenced_code_blocks),
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
            &validated_commit(
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
            &validated_commit(
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
            &validated_commit("Subject".to_string(), valid_indented_code_blocks),
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
            &validated_commit(
                "Subject".to_string(),
                invalid_long_ling_outside_indended_code_block,
            ),
            &Rule::MessageLineLength,
        );
    }

    #[test]
    fn test_validate_message_ticket_numbers() {
        let message_with_ticket_number = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            "Fixes #123",
        ]
        .join("\n");
        assert_commit_valid_for(
            &validated_commit("Subject".to_string(), message_with_ticket_number),
            &Rule::MessageTicketNumber,
        );

        let message_with_ticket_number_part_of = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            "Part of #123",
        ]
        .join("\n");
        assert_commit_valid_for(
            &validated_commit("Subject".to_string(), message_with_ticket_number_part_of),
            &Rule::MessageTicketNumber,
        );

        let message_with_ticket_number_related = [
            "Beginning of message.",
            "",
            "Some explanation.",
            "",
            "Related #123",
        ]
        .join("\n");
        assert_commit_valid_for(
            &validated_commit("Subject".to_string(), message_with_ticket_number_related),
            &Rule::MessageTicketNumber,
        );

        let message_without_ticket_number =
            ["", "Beginning of message.", "", "Some explanation."].join("\n");
        let without_ticket_number =
            validated_commit("Subject".to_string(), message_without_ticket_number);
        let issue = find_issue(without_ticket_number.issues, &Rule::MessageTicketNumber);
        assert_eq!(
            issue.message,
            "The message body does not contain a ticket or issue number"
        );
        assert_eq!(issue.position, message_position(7, 1));
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20|\n\
                   5 | Some explanation.\n\
                   6 | \n\
                   7 | Fixes #123\n\
             \x20\x20| ---------- Consider adding a reference to a ticket or issue\n"
        );
    }

    #[test]
    fn test_validate_changes_presense() {
        let with_changes = validated_commit("Subject".to_string(), "\nSome message.".to_string());
        assert_commit_valid_for(&with_changes, &Rule::DiffPresence);

        let mut without_changes = commit_without_file_changes("\nSome Message".to_string());
        without_changes.validate();
        let issue = find_issue(without_changes.issues, &Rule::DiffPresence);
        assert_eq!(issue.message, "No file changes found");
        assert_eq!(issue.position, Position::Diff);
        assert_eq!(
            formatted_context(&issue),
            "|\n\
             | 0 files changed, 0 insertions(+), 0 deletions(-)\n\
             | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Add changes to the commit or remove the commit\n"
        );

        let mut ignore_commit = commit_without_file_changes(
            "\nSome message.\nlintje:disable: DiffPresence".to_string(),
        );
        ignore_commit.validate();
        assert_commit_invalid_for(&ignore_commit, &Rule::DiffPresence);
    }
}
