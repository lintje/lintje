use crate::rule::{rule_by_name, Context, Position, Rule, Violation};
use crate::utils::{
    character_count_for_bytes_index, display_width, is_punctuation, line_length_stats,
};
use core::ops::Range;
use regex::{Regex, RegexBuilder};

lazy_static! {
    pub static ref SUBJECT_WITH_MERGE_REMOTE_BRANCH: Regex = Regex::new(r"^Merge branch '.+' of .+ into .+").unwrap();
    static ref SUBJECT_STARTS_WITH_PREFIX: Regex = Regex::new(r"^([\w\(\)/!]+:)\s.*").unwrap();
    // Regex to match emoji, but not all emoji. Emoji using ASCII codepoints like the emojis for
    // the numbers 0-9, and symbols like * and # are not included. Otherwise it would also catches
    // plain numbers 0-9 and those symbols, even when they are not emoji.
    // This regex matches all emoji but subtracts any object with ASCII codepoints.
    // For more information, see:
    // https://github.com/BurntSushi/ripgrep/discussions/1623#discussioncomment-28827
    static ref SUBJECT_STARTS_WITH_EMOJI: Regex = Regex::new(r"^[\p{Emoji}--\p{Ascii}]").unwrap();
    // Jira project keys are at least 2 uppercase characters long.
    // AB-123
    // JIRA-123
    static ref SUBJECT_WITH_TICKET: Regex = Regex::new(r"[A-Z]{2,}-\d+").unwrap();
    // Match all GitHub and GitLab keywords
    static ref SUBJECT_WITH_FIX_TICKET: Regex =
        Regex::new(r"([fF]ix(es|ed|ing)?|[cC]los(e|es|ed|ing)|[rR]esolv(e|es|ed|ing)|[iI]mplement(s|ed|ing)?):? ([^\s]*[\w\-_/]+)?[#!]{1}\d+").unwrap();
    static ref SUBJECT_WITH_CLICHE: Regex = {
        let mut tempregex = RegexBuilder::new(r"^(fix(es|ed|ing)?|add(s|ed|ing)?|(updat|chang|remov|delet)(e|es|ed|ing))(\s+\w+)?$");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
    static ref SUBJECT_WITH_BUILD_TAGS: Regex = {
        let mut tempregex = RegexBuilder::new(r"(\[(skip [\w\s_-]+|[\w\s_-]+ skip|no ci)\]|\*\*\*NO_CI\*\*\*)");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };

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
    pub email: Option<String>,
    pub subject: String,
    pub message: String,
    pub has_changes: bool,
    pub violations: Vec<Violation>,
    pub ignored: bool,
    pub ignored_rules: Vec<Rule>,
}

impl Commit {
    pub fn new(
        long_sha: Option<String>,
        email: Option<String>,
        subject: String,
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
            violations: Vec::<Violation>::new(),
        }
    }

    pub fn find_ignored_rules(message: &str) -> Vec<Rule> {
        let disable_prefix = "lintje:disable ";
        let mut ignored = vec![];
        for line in message.lines().into_iter() {
            if let Some(name) = line.strip_prefix(disable_prefix) {
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
        self.validate_subject_cliches();
        self.validate_subject_line_length();
        self.validate_subject_mood();
        self.validate_subject_whitespace();
        self.validate_subject_prefix();
        self.validate_subject_capitalization();
        self.validate_subject_build_tags();
        self.validate_subject_punctuation();
        self.validate_subject_ticket_numbers();
        self.validate_message_empty_first_line();
        self.validate_message_presence();
        self.validate_message_line_length();
        self.validate_changes();
    }

    // Note: Some merge commits are ignored in git.rs and won't be validated here, because they are
    // Pull/Merge Requests, which are valid.
    fn validate_merge_commit(&mut self) {
        if self.rule_ignored(Rule::MergeCommit) {
            return;
        }

        let subject = &self.subject;
        if SUBJECT_WITH_MERGE_REMOTE_BRANCH.is_match(subject) {
            let subject_length = subject.len();
            let context = Context::subject_hint(
                subject.to_string(),
                Range { start: 0, end: subject_length },
                "Rebase on the remote branch, rather than merging the remote branch into the local branch".to_string(),
            );
            self.add_violation(
                Rule::MergeCommit,
                "A remote merge commit was found".to_string(),
                Position::Subject { column: 1 },
                vec![context],
            )
        }
    }

    fn validate_needs_rebase(&mut self) {
        if self.rule_ignored(Rule::NeedsRebase) {
            return;
        }

        let subject = &self.subject;
        if subject.starts_with("fixup! ") {
            let context = Context::subject_hint(
                self.subject.to_string(),
                Range { start: 0, end: 6 },
                "Rebase fixup commits before pushing or merging".to_string(),
            );
            self.add_violation(
                Rule::NeedsRebase,
                "A fixup commit was found".to_string(),
                Position::Subject { column: 1 },
                vec![context],
            )
        } else if subject.starts_with("squash! ") {
            let context = Context::subject_hint(
                self.subject.to_string(),
                Range { start: 0, end: 7 },
                "Rebase squash commits before pushing or merging".to_string(),
            );
            self.add_violation(
                Rule::NeedsRebase,
                "A squash commit was found".to_string(),
                Position::Subject { column: 1 },
                vec![context],
            )
        }
    }

    fn validate_subject_line_length(&mut self) {
        if self.rule_ignored(Rule::SubjectLength) || self.has_violation(Rule::SubjectCliche) {
            return;
        }

        let (width, line_stats) = line_length_stats(&self.subject, 50);

        if width == 0 {
            let context = Context::subject_hint(
                self.subject.to_string(),
                Range { start: 0, end: 1 },
                "Add a subject to describe the change".to_string(),
            );
            self.add_violation(
                Rule::SubjectLength,
                "The commit has no subject".to_string(),
                Position::Subject { column: 1 },
                vec![context],
            );
            return;
        }

        if width > 50 {
            let total_width_index = self.subject.len();
            let context = Context::subject_hint(
                self.subject.to_string(),
                Range {
                    start: line_stats.bytes_index,
                    end: total_width_index,
                },
                "Shorten the subject to a maximum width of 50 characters".to_string(),
            );
            self.add_violation(
                Rule::SubjectLength,
                format!("The subject of `{}` characters wide is too long", width),
                Position::Subject {
                    column: line_stats.char_count + 1, // + 1 because the next char is the problem
                },
                vec![context],
            );
            return;
        }
        if width < 5 {
            let total_width_index = self.subject.len();
            let context = Context::subject_hint(
                self.subject.to_string(),
                Range {
                    start: 0,
                    end: total_width_index,
                },
                "Describe the change in more detail".to_string(),
            );
            self.add_violation(
                Rule::SubjectLength,
                format!("The subject of `{}` characters wide is too short", width),
                Position::Subject { column: 1 },
                vec![context],
            );
        }
    }

    fn validate_subject_mood(&mut self) {
        if self.rule_ignored(Rule::SubjectMood) {
            return;
        }

        match self.subject.split(' ').next() {
            Some(raw_word) => {
                let word = raw_word.to_lowercase();
                if MOOD_WORDS.contains(&word.as_str()) {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        Range {
                            start: 0,
                            end: word.len(),
                        },
                        "Use the imperative mood for the subject".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectMood,
                        "The subject does not use the imperative grammatical mood".to_string(),
                        Position::Subject { column: 1 },
                        context,
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
        if self.subject.chars().count() == 0 && self.has_violation(Rule::SubjectLength) {
            return;
        }

        match self.subject.chars().next() {
            Some(character) => {
                if character.is_whitespace() {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Remove the leading whitespace from the subject".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectWhitespace,
                        "The subject starts with a whitespace character such as a space or a tab"
                            .to_string(),
                        Position::Subject { column: 1 },
                        context,
                    )
                }
            }
            None => {
                error!("SubjectWhitespace validation failure: No first character found of subject.")
            }
        }
    }

    fn validate_subject_capitalization(&mut self) {
        if self.rule_ignored(Rule::SubjectCapitalization)
            || self.has_violation(Rule::NeedsRebase)
            || self.has_violation(Rule::SubjectPrefix)
        {
            return;
        }
        if self.subject.chars().count() == 0 && self.has_violation(Rule::SubjectLength) {
            return;
        }

        match self.subject.chars().next() {
            Some(character) => {
                if character.is_lowercase() {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Start the subject with a capital letter".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectCapitalization,
                        "The subject does not start with a capital letter".to_string(),
                        Position::Subject { column: 1 },
                        context,
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
        if self.subject.chars().count() == 0 && self.has_violation(Rule::SubjectLength) {
            return;
        }

        if let Some(captures) = SUBJECT_STARTS_WITH_EMOJI.captures(&self.subject) {
            match captures.get(0) {
                Some(emoji) => {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        emoji.range(),
                        "Remove emoji from the start of the subject".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectPunctuation,
                        "The subject starts with an emoji".to_string(),
                        Position::Subject { column: 1 },
                        context,
                    )
                }
                None => {
                    error!("SubjectTicketNumber: Unable to fetch ticket number match from subject.")
                }
            }
        }

        match self.subject.chars().next() {
            Some(character) => {
                if is_punctuation(&character) {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        Range {
                            start: 0,
                            end: character.len_utf8(),
                        },
                        "Remove punctuation from the start of the subject".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectPunctuation,
                        format!(
                            "The subject starts with a punctuation character: `{}`",
                            character
                        ),
                        Position::Subject { column: 1 },
                        context,
                    )
                }
            }
            None => {
                error!(
                    "SubjectPunctuation validation failure: No first character found of subject."
                )
            }
        }

        match self.subject.chars().last() {
            Some(character) => {
                if is_punctuation(&character) {
                    let subject_length = self.subject.len();
                    let context = Context::subject_hint(
                        self.subject.to_string(),
                        Range {
                            start: subject_length - character.len_utf8(),
                            end: subject_length,
                        },
                        "Remove punctuation from the end of the subject".to_string(),
                    );
                    self.add_violation(
                        Rule::SubjectPunctuation,
                        format!(
                            "The subject ends with a punctuation character: `{}`",
                            character
                        ),
                        Position::Subject {
                            column: character_count_for_bytes_index(
                                &self.subject,
                                subject_length - character.len_utf8(),
                            ),
                        },
                        vec![context],
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

        let subject = &self.subject.to_string();
        if let Some(captures) = SUBJECT_WITH_TICKET.captures(subject) {
            match captures.get(0) {
                Some(capture) => {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        capture.range(),
                        "Move the ticket number to the message body".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectTicketNumber,
                        "The subject contains a ticket number".to_string(),
                        Position::Subject {
                            column: character_count_for_bytes_index(&self.subject, capture.start()),
                        },
                        context,
                    )
                }
                None => {
                    error!("SubjectTicketNumber: Unable to fetch ticket number match from subject.")
                }
            }
        }
        if let Some(captures) = SUBJECT_WITH_FIX_TICKET.captures(subject) {
            match captures.get(0) {
                Some(capture) => {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        capture.range(),
                        "Move the ticket number to the message body".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectTicketNumber,
                        "The subject contains a ticket number".to_string(),
                        Position::Subject {
                            column: character_count_for_bytes_index(&self.subject, capture.start()),
                        },
                        context,
                    )
                }
                None => {
                    error!("SubjectTicketNumber: Unable to fetch ticket number match from subject.")
                }
            }
        }
    }

    fn validate_subject_prefix(&mut self) {
        if self.rule_ignored(Rule::SubjectPrefix) {
            return;
        }

        let subject = &self.subject.to_string();
        if let Some(captures) = SUBJECT_STARTS_WITH_PREFIX.captures(subject) {
            // Get first match from captures, the prefix
            match captures.get(1) {
                Some(capture) => {
                    let context = vec![Context::subject_hint(
                        self.subject.to_string(),
                        capture.range(),
                        "Remove the prefix from the subject".to_string(),
                    )];
                    self.add_violation(
                        Rule::SubjectPrefix,
                        format!("Remove the `{}` prefix from the subject", capture.as_str()),
                        Position::Subject { column: 1 },
                        context,
                    )
                }
                None => error!("SubjectPrefix: Unable to fetch prefix capture from subject."),
            }
        }
    }

    fn validate_subject_build_tags(&mut self) {
        if self.rule_ignored(Rule::SubjectBuildTag) {
            return;
        }

        let subject = &self.subject.to_string();
        if let Some(captures) = SUBJECT_WITH_BUILD_TAGS.captures(subject) {
            match captures.get(1) {
                Some(tag) => {
                    let context = Context::subject_hint(
                        subject.to_string(),
                        tag.range(),
                        "Move build tag to message body".to_string(),
                    );
                    self.add_violation(
                        Rule::SubjectBuildTag,
                        format!("The `{}` build tag was found in the subject", tag.as_str()),
                        Position::Subject {
                            column: character_count_for_bytes_index(&self.subject, tag.start()),
                        },
                        vec![context],
                    )
                }
                None => error!("SubjectBuildTag: Unable to fetch build tag from subject."),
            }
        }
    }

    fn validate_subject_cliches(&mut self) {
        if self.rule_ignored(Rule::SubjectCliche) {
            return;
        }

        let subject = &self.subject.to_lowercase();
        let wip_commit = subject.starts_with("wip ") || subject == &"wip".to_string();
        if wip_commit || SUBJECT_WITH_CLICHE.is_match(subject) {
            let context = vec![Context::subject_hint(
                self.subject.to_string(),
                Range {
                    start: 0,
                    end: self.subject.len(),
                },
                "Describe the change in more detail".to_string(),
            )];
            self.add_violation(
                Rule::SubjectCliche,
                "The subject does not explain the change in much detail".to_string(),
                Position::Subject { column: 1 },
                context,
            )
        }
    }

    fn validate_message_empty_first_line(&mut self) {
        if self.rule_ignored(Rule::MessageEmptyFirstLine) {
            return;
        }

        if let Some(line) = self.message.lines().next() {
            if !line.is_empty() {
                let context = vec![
                    Context::subject(self.subject.to_string()),
                    Context::message_line_hint(
                        0,
                        line.to_string(),
                        Range {
                            start: 0,
                            end: line.len(),
                        },
                        "Add an empty line below the subject line".to_string(),
                    ),
                ];
                self.add_violation(
                    Rule::MessageEmptyFirstLine,
                    "No empty line found below the subject".to_string(),
                    Position::MessageLine { line: 1, column: 1 },
                    context,
                )
            }
        }
    }

    fn validate_message_presence(&mut self) {
        if self.rule_ignored(Rule::MessagePresence) || self.has_violation(Rule::NeedsRebase) {
            return;
        }

        let message = &self.message.trim();
        let width = display_width(&message);
        if width == 0 {
            let context = vec![
                Context::subject(self.subject.to_string()),
                Context::message_line(0, "".to_string()),
                Context::message_line_hint(
                    1,
                    "".to_string(),
                    Range { start: 0, end: 1 },
                    "Add a message body with context about the change and why it was made"
                        .to_string(),
                ),
            ];
            self.add_violation(
                Rule::MessagePresence,
                "No message body was found".to_string(),
                Position::MessageLine { line: 2, column: 1 },
                context,
            )
        } else if width < 10 {
            let mut context = vec![];
            let line_count = self.message.lines().count();
            if let Some(line) = self.message.lines().last() {
                context.push(Context::message_line_hint(
                    line_count - 1,
                    line.to_string(),
                    Range {
                        start: 0,
                        end: line.len(),
                    },
                    "Add a longer message with context about the change and why it was made"
                        .to_string(),
                ));
            }
            self.add_violation(
                Rule::MessagePresence,
                "The message body is too short".to_string(),
                Position::MessageLine {
                    line: line_count,
                    column: 1,
                },
                context,
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
            let (width, line_stats) = line_length_stats(line, 72);
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
            if width > 72 {
                if URL_REGEX.is_match(line) {
                    continue;
                }
                let line_index = index + 1; // + 1 for subject
                let line_number = line_index + 1; // + 1 to normalize it
                let context = Context::message_line_hint(
                    index,
                    line.to_string(),
                    Range {
                        start: line_stats.bytes_index,
                        end: line.len(),
                    },
                    "Shorten line to maximum 72 characters".to_string(),
                );
                violations.push((
                    Rule::MessageLineLength,
                    format!(
                        "Line {} in the message body is longer than 72 characters",
                        line_number
                    ),
                    Position::MessageLine {
                        line: line_index,
                        column: line_stats.char_count + 1, // + 1 because the next char is the problem
                    },
                    vec![context],
                ))
            }
            previous_line_was_empty_line = line.trim() == "";
        }

        for (rule, message, position, context) in violations {
            self.add_violation(rule, message, position, context)
        }
    }

    fn validate_changes(&mut self) {
        if self.rule_ignored(Rule::DiffPresence) {
            return;
        }

        if !self.has_changes {
            let context_line = "0 files changed, 0 insertions(+), 0 deletions(-)".to_string();
            let context_length = context_line.len();
            let context = Context::diff_hint(
                context_line,
                Range {
                    start: 0,
                    end: context_length,
                },
                "Add changes to the commit or remove the commit".to_string(),
            );
            self.add_violation(
                Rule::DiffPresence,
                "No file changes found".to_string(),
                Position::Diff,
                vec![context],
            )
        }
    }

    fn add_violation(
        &mut self,
        rule: Rule,
        message: String,
        position: Position,
        context: Vec<Context>,
    ) {
        self.violations.push(Violation {
            rule,
            message,
            position,
            context,
        })
    }

    fn has_violation(&self, rule: Rule) -> bool {
        self.violations
            .iter()
            .any(|violation| violation.rule == rule)
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
    use super::MOOD_WORDS;
    use crate::commit::Commit;
    use crate::formatter::formatted_context;
    use crate::rule::{Position, Rule, Violation};

    fn commit_with_sha<S: AsRef<str>>(sha: Option<String>, subject: S, message: S) -> Commit {
        Commit::new(
            sha,
            Some("test@example.com".to_string()),
            subject.as_ref().to_string(),
            message.as_ref().to_string(),
            true,
        )
    }

    fn commit<S: AsRef<str>>(subject: S, message: S) -> Commit {
        commit_with_sha(
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            subject,
            message,
        )
    }

    fn commit_without_file_changes(message: String) -> Commit {
        Commit::new(
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            Some("test@example.com".to_string()),
            "Some subject".to_string(),
            message,
            false,
        )
    }

    fn validated_commit<S: AsRef<str>>(subject: S, message: S) -> Commit {
        let mut commit = commit(subject, message);
        commit.validate();
        commit
    }

    fn assert_commit_valid_for(commit: &Commit, rule: &Rule) {
        assert!(
            !has_violation(&commit.violations, rule),
            "Commit was not considered valid: {:?}",
            commit
        );
    }

    fn assert_commit_invalid_for(commit: &Commit, rule: &Rule) {
        assert!(
            has_violation(&commit.violations, rule),
            "Commit was not considered invalid: {:?}",
            commit
        );
    }

    fn assert_commit_subject_as_valid(subject: &str, rule: &Rule) {
        let commit = validated_commit(subject.to_string(), "".to_string());
        assert_commit_valid_for(&commit, rule);
    }

    fn assert_commit_subjects_as_valid(subjects: Vec<&str>, rule: &Rule) {
        for subject in subjects {
            assert_commit_subject_as_valid(subject, rule)
        }
    }

    fn assert_commit_subject_as_invalid<S: AsRef<str>>(subject: S, rule: &Rule) {
        let commit = validated_commit(subject.as_ref().to_string(), "".to_string());
        assert_commit_invalid_for(&commit, rule);
    }

    fn assert_commit_subjects_as_invalid<S: AsRef<str>>(subjects: Vec<S>, rule: &Rule) {
        for subject in subjects {
            assert_commit_subject_as_invalid(subject, rule)
        }
    }

    fn has_violation(violations: &Vec<Violation>, rule: &Rule) -> bool {
        violations.iter().any(|v| &v.rule == rule)
    }

    fn find_violation(violations: Vec<Violation>, rule: &Rule) -> Violation {
        let mut violations = violations.into_iter().filter(|v| &v.rule == rule);
        let violation = match violations.next() {
            Some(violation) => violation,
            None => panic!("No violation of the {} rule found", rule),
        };
        if violations.next().is_some() {
            panic!("More than one violation of the {} rule found", rule)
        }
        violation
    }

    fn subject_position(column: usize) -> Position {
        Position::Subject { column }
    }

    fn message_position(line: usize, column: usize) -> Position {
        Position::MessageLine { line, column }
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
    fn test_validate_subject_merge_commit() {
        assert_commit_subject_as_valid("I am not a merge commit", &Rule::MergeCommit);
        assert_commit_subject_as_valid("Merge pull request #123 from repo", &Rule::MergeCommit);
        // Merge into the project's defaultBranch branch
        assert_commit_subject_as_valid("Merge branch 'develop'", &Rule::MergeCommit);
        // Merge a local branch into another local branch
        assert_commit_subject_as_valid(
            "Merge branch 'develop' into feature-branch",
            &Rule::MergeCommit,
        );
        // Merge a remote branch into a local branch
        let commit = validated_commit(
            "Merge branch 'develop' of github.com/org/repo into develop",
            "",
        );
        let violation = find_violation(commit.violations, &Rule::MergeCommit);
        assert_eq!(violation.message, "A remote merge commit was found");
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Merge branch 'develop' of github.com/org/repo into develop\n\
             \x20\x20| ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ \
                Rebase on the remote branch, rather than merging the remote branch into the local branch\n"
        );

        let ignore_commit = validated_commit(
            "Merge branch 'develop' of github.com/org/repo into develop".to_string(),
            "lintje:disable MergeCommit".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::MergeCommit);
    }

    #[test]
    fn test_validate_needs_rebase() {
        assert_commit_subject_as_valid("I don't need a rebase", &Rule::NeedsRebase);

        let fixup = validated_commit("fixup! I need a rebase", "");
        let violation = find_violation(fixup.violations, &Rule::NeedsRebase);
        assert_eq!(violation.message, "A fixup commit was found");
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | fixup! I need a rebase\n\
             \x20\x20| ^^^^^^ Rebase fixup commits before pushing or merging\n"
        );

        let squash = validated_commit("squash! I need a rebase", "");
        let violation = find_violation(squash.violations, &Rule::NeedsRebase);
        assert_eq!(violation.message, "A squash commit was found");
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | squash! I need a rebase\n\
             \x20\x20| ^^^^^^^ Rebase squash commits before pushing or merging\n"
        );

        let ignore_commit = validated_commit(
            "fixup! I don't need to be rebased".to_string(),
            "lintje:disable NeedsRebase".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::NeedsRebase);
    }

    #[test]
    fn test_validate_subject_line_length() {
        assert_commit_subject_as_valid(&"a".repeat(5), &Rule::SubjectLength);
        assert_commit_subject_as_valid(&"a".repeat(50), &Rule::SubjectLength);

        let empty = validated_commit("", "");
        let violation = find_violation(empty.violations, &Rule::SubjectLength);
        assert_eq!(violation.message, "The commit has no subject");
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | \n\
             \x20\x20| ^ Add a subject to describe the change\n"
        );

        let short_subject = validated_commit("a".repeat(4).as_str(), "");
        let violation = find_violation(short_subject.violations, &Rule::SubjectLength);
        assert_eq!(
            violation.message,
            "The subject of `4` characters wide is too short"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | aaaa\n\
             \x20\x20| ^^^^ Describe the change in more detail\n"
        );

        let long_subject = validated_commit("a".repeat(51).as_str(), "");
        let violation = find_violation(long_subject.violations, &Rule::SubjectLength);
        assert_eq!(
            violation.message,
            "The subject of `51` characters wide is too long"
        );
        assert_eq!(violation.position, subject_position(51));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
             \x20\x20|                                                   ^ \
             Shorten the subject to a maximum width of 50 characters\n"
        );

        // Character is two characters, but is counted as 1 column
        assert_eq!("ö̲".chars().count(), 2);
        let accent_subject = validated_commit("A ö̲", "");
        let violation = find_violation(accent_subject.violations, &Rule::SubjectLength);
        assert_eq!(
            violation.message,
            "The subject of `3` characters wide is too short"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | A ö̲\n\
             \x20\x20| ^^^ Describe the change in more detail\n"
        );

        // These emoji display width is 2
        assert_commit_subject_as_valid(&"✨".repeat(25), &Rule::SubjectLength);
        assert_commit_subject_as_invalid(&"✨".repeat(26), &Rule::SubjectLength);

        let emoji_short_subject = validated_commit("👁️‍🗨️", "");
        let violation = find_violation(emoji_short_subject.violations, &Rule::SubjectLength);
        assert_eq!(
            violation.message,
            "The subject of `2` characters wide is too short"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | 👁️‍🗨️\n\
             \x20\x20| ^^ Describe the change in more detail\n"
        );

        // Hiragana display width is 2
        assert_commit_subject_as_valid(&"あ".repeat(25), &Rule::SubjectLength);

        let hiragana_long_subject = validated_commit("あ".repeat(26).as_str(), "");
        let violation = find_violation(hiragana_long_subject.violations, &Rule::SubjectLength);
        assert_eq!(
            violation.message,
            "The subject of `52` characters wide is too long"
        );
        assert_eq!(violation.position, subject_position(26));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | ああああああああああああああああああああああああああ\n\
             \x20\x20|                                                   ^^ \
             Shorten the subject to a maximum width of 50 characters\n"
        );

        let ignore_commit = validated_commit(
            "a".repeat(51).to_string(),
            "lintje:disable SubjectLength".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectLength);

        // Already a SubjectCliche violation, so it's skipped.
        assert_commit_subject_as_valid("wip", &Rule::SubjectLength);
        assert_commit_subject_as_invalid("wip", &Rule::SubjectCliche);
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

        let subject = validated_commit("Fixing bug", "");
        let violation = find_violation(subject.violations, &Rule::SubjectMood);
        assert_eq!(
            violation.message,
            "The subject does not use the imperative grammatical mood"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Fixing bug\n\
             \x20\x20| ^^^^^^ Use the imperative mood for the subject\n"
        );

        let ignore_commit = validated_commit(
            "Fixed test".to_string(),
            "lintje:disable SubjectMood".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectMood);
    }

    #[test]
    fn test_validate_subject_whitespace() {
        let subjects = vec!["Fix test"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectWhitespace);

        let space = validated_commit(" Fix test", "");
        let violation = find_violation(space.violations, &Rule::SubjectWhitespace);
        assert_eq!(
            violation.message,
            "The subject starts with a whitespace character such as a space or a tab"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 |  Fix test\n\
             \x20\x20| ^ Remove the leading whitespace from the subject\n"
        );

        let space = validated_commit("\x20Fix test", "");
        let violation = find_violation(space.violations, &Rule::SubjectWhitespace);
        assert_eq!(
            violation.message,
            "The subject starts with a whitespace character such as a space or a tab"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | \x20Fix test\n\
             \x20\x20| ^ Remove the leading whitespace from the subject\n"
        );

        let tab = validated_commit("\tFix test", "");
        let violation = find_violation(tab.violations, &Rule::SubjectWhitespace);
        assert_eq!(
            violation.message,
            "The subject starts with a whitespace character such as a space or a tab"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 |     Fix test\n\
             \x20\x20| ^^^^ Remove the leading whitespace from the subject\n"
        );

        // Rule is ignored because the subject is empty, a SubjectLength violation
        assert_commit_subject_as_invalid("", &Rule::SubjectLength);
        assert_commit_subject_as_valid("", &Rule::SubjectWhitespace);

        let ignore_commit = validated_commit(
            " Fix test".to_string(),
            "lintje:disable SubjectWhitespace".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectWhitespace);
    }

    #[test]
    fn test_validate_subject_capitalization() {
        let subjects = vec!["Fix test"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectCapitalization);

        let subject = validated_commit("fix test", "");
        let violation = find_violation(subject.violations, &Rule::SubjectCapitalization);
        assert_eq!(
            violation.message,
            "The subject does not start with a capital letter"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | fix test\n\
             \x20\x20| ^ Start the subject with a capital letter\n"
        );

        let ignore_commit = validated_commit(
            "fix test".to_string(),
            "lintje:disable SubjectCapitalization".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectCapitalization);

        // Already a SubjectLength violation, so it's skipped
        assert_commit_subject_as_invalid("", &Rule::SubjectLength);
        assert_commit_subject_as_valid("", &Rule::SubjectCapitalization);

        // Already a NeedsRebase violation, so it's skipped
        let rebase_commit = validated_commit("fixup! foo".to_string(), "".to_string());
        assert_commit_valid_for(&rebase_commit, &Rule::SubjectCapitalization);
        let rebase_commit = validated_commit("fixup! foo".to_string(), "".to_string());
        assert_commit_invalid_for(&rebase_commit, &Rule::NeedsRebase);

        // Already a SubjectPrefix violation, so it's skippe.
        let prefix_commit = validated_commit("chore: foo".to_string(), "".to_string());
        assert_commit_valid_for(&prefix_commit, &Rule::SubjectCapitalization);
        let prefix_commit = validated_commit("chore: foo".to_string(), "".to_string());
        assert_commit_invalid_for(&prefix_commit, &Rule::SubjectPrefix);
    }

    #[test]
    fn test_validate_subject_punctuation() {
        let subjects = vec![
            "Fix test",
            "あ commit",
            "123 digits",
            "0 digit",
            // These should not be allowed, but won't match using the Emoji -- ASCII regex matcher.
            // See the comment for SUBJECT_STARTS_WITH_EMOJI for more information.
            "0️⃣ emoji",
            "﹟emoji",
            "＊emoji",
        ];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectPunctuation);

        let invalid_subjects = vec![
            "Fix test.",
            "Fix test!",
            "Fix test?",
            "Fix test:",
            "Fix test\'",
            "Fix test\"",
            "Fix test…",
            "Fix test⋯",
            ".Fix test",
            "!Fix test",
            "?Fix test",
            ":Fix test",
            "…Fix test",
            "⋯Fix test",
            "📺Fix test",
            "👍Fix test",
            "👍🏻Fix test",
            "[JIRA-123] Fix test",
            "[Bug] Fix test",
            "[chore] Fix test",
            "[feat] Fix test",
            "(feat) Fix test",
            "{fix} Fix test",
            "|fix| Fix test",
            "-fix- Fix test",
            "+fix+ Fix test",
            "*fix* Fix test",
            "%fix% Fix test",
            "@fix Fix test",
        ];
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectPunctuation);

        let start = validated_commit(".Fix test", "");
        let violation = find_violation(start.violations, &Rule::SubjectPunctuation);
        assert_eq!(
            violation.message,
            "The subject starts with a punctuation character: `.`"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | .Fix test\n\
             \x20\x20| ^ Remove punctuation from the start of the subject\n"
        );

        let end = validated_commit("Fix test…", "");
        let violation = find_violation(end.violations, &Rule::SubjectPunctuation);
        assert_eq!(
            violation.message,
            "The subject ends with a punctuation character: `…`"
        );
        assert_eq!(violation.position, subject_position(9));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Fix test…\n\
             \x20\x20|         ^ Remove punctuation from the end of the subject\n"
        );

        let emoji = validated_commit("👍 Fix test", "");
        let violation = find_violation(emoji.violations, &Rule::SubjectPunctuation);
        assert_eq!(violation.message, "The subject starts with an emoji");
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | 👍 Fix test\n\
             \x20\x20| ^^ Remove emoji from the start of the subject\n"
        );

        // Already a empty SubjectLength violation, so it's skipped
        assert_commit_subject_as_invalid("", &Rule::SubjectLength);
        assert_commit_subject_as_valid("", &Rule::SubjectPunctuation);

        let ignore_commit = validated_commit(
            "Fix test.".to_string(),
            "lintje:disable SubjectPunctuation".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectPunctuation);
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
        let violation = find_violation(ticket_number.violations, &Rule::SubjectTicketNumber);
        assert_eq!(violation.message, "The subject contains a ticket number");
        assert_eq!(violation.position, subject_position(5));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Fix JIRA-123 about email validation\n\
             \x20\x20|     ^^^^^^^^ Move the ticket number to the message body\n"
        );

        let ticket_number_unicode = validated_commit("Fix a̐ JIRA-123 about email validation", "");
        let violation =
            find_violation(ticket_number_unicode.violations, &Rule::SubjectTicketNumber);
        assert_eq!(violation.position, subject_position(7));

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
        let violation = find_violation(fix_ticket.violations, &Rule::SubjectTicketNumber);
        assert_eq!(violation.message, "The subject contains a ticket number");
        assert_eq!(violation.position, subject_position(19));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Email validation: Fixes #123 for good\n\
             \x20\x20|                   ^^^^^^^^^^ Move the ticket number to the message body\n"
        );

        let fix_ticket_unicode = validated_commit("Email validatiｏn: Fixes #123", "");
        let violation = find_violation(fix_ticket_unicode.violations, &Rule::SubjectTicketNumber);
        assert_eq!(violation.position, subject_position(19));

        let fix_link = validated_commit("Email validation: Closed org/repo#123 for good", "");
        let violation = find_violation(fix_link.violations, &Rule::SubjectTicketNumber);
        assert_eq!(violation.message, "The subject contains a ticket number");
        assert_eq!(violation.position, subject_position(19));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Email validation: Closed org/repo#123 for good\n\
             \x20\x20|                   ^^^^^^^^^^^^^^^^^^^ Move the ticket number to the message body\n"
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
    fn test_validate_subject_prefix() {
        let subjects = vec!["This is a commit without prefix"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectPrefix);

        let invalid_subjects = vec![
            "fix: bug",
            "fix!: bug",
            "Fix: bug",
            "Fix!: bug",
            "fix(scope): bug",
            "fix(scope)!: bug",
            "Fix(scope123)!: bug",
            "fix(scope/scope): bug",
            "fix(scope/scope)!: bug",
        ];
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectPrefix);

        let prefix = validated_commit("Fix: bug", "");
        let violation = find_violation(prefix.violations, &Rule::SubjectPrefix);
        assert_eq!(
            violation.message,
            "Remove the `Fix:` prefix from the subject"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Fix: bug\n\
             \x20\x20| ^^^^ Remove the prefix from the subject\n"
        );

        let scoped = validated_commit("chore(package)!: some package bug", "");
        let violation = find_violation(scoped.violations, &Rule::SubjectPrefix);
        assert_eq!(
            violation.message,
            "Remove the `chore(package)!:` prefix from the subject"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | chore(package)!: some package bug\n\
             \x20\x20| ^^^^^^^^^^^^^^^^ Remove the prefix from the subject\n"
        );

        let ignore_commit = validated_commit(
            "fix: bug".to_string(),
            "lintje:disable SubjectPrefix".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectPrefix);
    }

    #[test]
    fn test_validate_subject_build_tags() {
        let subjects = vec!["Add exception for no ci build tag"];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectBuildTag);

        let build_tags = vec![
            // General
            "[ci skip]",
            "[skip ci]",
            "[no ci]",
            // AppVeyor
            "[skip appveyor]",
            // Azure
            "[azurepipelines skip]",
            "[skip azurepipelines]",
            "[azpipelines skip]",
            "[skip azpipelines]",
            "[azp skip]",
            "[skip azp]",
            "***NO_CI***",
            // GitHub Actions
            "[actions skip]",
            "[skip actions]",
            // Travis
            "[travis skip]",
            "[skip travis]",
            "[travis ci skip]",
            "[skip travis ci]",
            "[travis-ci skip]",
            "[skip travis-ci]",
            "[travisci skip]",
            "[skip travisci]",
            // Other custom tags that match the format
            "[skip me]",
            "[skip changeset]",
            "[skip review]",
        ];
        let mut invalid_subjects = vec![];
        for tag in build_tags.iter() {
            invalid_subjects.push(format!("Update README {}", tag))
        }
        assert_commit_subjects_as_invalid(invalid_subjects, &Rule::SubjectBuildTag);

        let build_tag = validated_commit("Edit CHANGELOG [skip ci]", "");
        let violation = find_violation(build_tag.violations, &Rule::SubjectBuildTag);
        assert_eq!(
            violation.message,
            "The `[skip ci]` build tag was found in the subject"
        );
        assert_eq!(violation.position, subject_position(16));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Edit CHANGELOG [skip ci]\n\
             \x20\x20|                ^^^^^^^^^ Move build tag to message body\n"
        );

        let ignore_commit = validated_commit(
            "Update README [ci skip]".to_string(),
            "lintje:disable SubjectBuildTag".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectBuildTag);
    }

    #[test]
    fn test_validate_subject_cliches() {
        let subjects = vec![
            "I am not a cliche",
            "Fix user bug",
            "Fix test for some feature",
            "Fix bug for some feature",
            "Fixes bug for some feature",
            "Fixed bug for some feature",
            "Fixing bug for some feature",
        ];
        assert_commit_subjects_as_valid(subjects, &Rule::SubjectCliche);

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

            invalid_subjects.push(format!("{}", uppercase_word));
            invalid_subjects.push(format!("{}", capitalized_word));
            invalid_subjects.push(format!("{}", word));
            invalid_subjects.push(format!("{} test", uppercase_word));
            invalid_subjects.push(format!("{} issue", capitalized_word));
            invalid_subjects.push(format!("{} bug", word));
            invalid_subjects.push(format!("{} readme", word));
            invalid_subjects.push(format!("{} something", word));
        }
        for subject in invalid_subjects {
            assert_commit_subject_as_invalid(subject.as_str(), &Rule::SubjectCliche);
        }

        let wip = validated_commit("WIP", "");
        let violation = find_violation(wip.violations, &Rule::SubjectCliche);
        assert_eq!(
            violation.message,
            "The subject does not explain the change in much detail"
        );
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | WIP\n\
             \x20\x20| ^^^ Describe the change in more detail\n"
        );

        let cliche = validated_commit("Fixed bug", "");
        let violation = find_violation(cliche.violations, &Rule::SubjectCliche);
        assert_eq!(violation.position, subject_position(1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Fixed bug\n\
             \x20\x20| ^^^^^^^^^ Describe the change in more detail\n"
        );

        let ignore_commit = validated_commit(
            "WIP".to_string(),
            "lintje:disable SubjectCliche".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::SubjectCliche);
    }

    #[test]
    fn test_validate_message_first_line_empty() {
        let with_empty_line = validated_commit(
            "Subject".to_string(),
            "\nEmpty line after subject.".to_string(),
        );
        assert_commit_valid_for(&with_empty_line, &Rule::MessageEmptyFirstLine);

        let without_empty_line = validated_commit("Subject", "No empty line after subject");
        let violation = find_violation(without_empty_line.violations, &Rule::MessageEmptyFirstLine);
        assert_eq!(violation.message, "No empty line found below the subject");
        assert_eq!(violation.position, message_position(1, 1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Subject\n\
                   2 | No empty line after subject\n\
             \x20\x20| ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Add an empty line below the subject line\n"
        );

        let ignore_commit = validated_commit(
            "Subject".to_string(),
            "No empty line after subject\nlintje:disable MessageEmptyFirstLine".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::MessageEmptyFirstLine);
    }

    #[test]
    fn test_validate_message_presence() {
        let with_message =
            validated_commit("Subject".to_string(), "Hello I am a message.".to_string());
        assert_commit_valid_for(&with_message, &Rule::MessagePresence);

        let without_message = validated_commit("Subject", "");
        let violation = find_violation(without_message.violations, &Rule::MessagePresence);
        assert_eq!(violation.message, "No message body was found");
        assert_eq!(violation.position, message_position(2, 1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   1 | Subject\n\
                   2 | \n\
                   3 | \n\
             \x20\x20| ^ Add a message body with context about the change and why it was made\n"
        );

        let short = validated_commit("Subject", "\nShort.");
        let violation = find_violation(short.violations, &Rule::MessagePresence);
        assert_eq!(violation.message, "The message body is too short");
        assert_eq!(violation.position, message_position(2, 1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   3 | Short.\n\
             \x20\x20| ^^^^^^ Add a longer message with context about the change and why it was made\n"
        );

        let very_short = validated_commit("Subject".to_string(), "...".to_string());
        let violation = find_violation(very_short.violations, &Rule::MessagePresence);
        assert_eq!(violation.message, "The message body is too short");
        assert_eq!(violation.position, message_position(1, 1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   2 | ...\n\
             \x20\x20| ^^^ Add a longer message with context about the change and why it was made\n"
        );

        let very_short = validated_commit("Subject".to_string(), ".\n.\nShort.\n".to_string());
        let violation = find_violation(very_short.violations, &Rule::MessagePresence);
        assert_eq!(violation.message, "The message body is too short");
        assert_eq!(violation.position, message_position(3, 1));
        assert_eq!(
            formatted_context(&violation),
            "\x20\x20|\n\
                   4 | Short.\n\
             \x20\x20| ^^^^^^ Add a longer message with context about the change and why it was made\n"
        );

        let ignore_commit = validated_commit(
            "Subject".to_string(),
            "lintje:disable MessagePresence".to_string(),
        );
        assert_commit_valid_for(&ignore_commit, &Rule::MessagePresence);

        // Already a NeedsRebase violation, so it's skipped.
        let rebase_commit = validated_commit("fixup! foo".to_string(), "".to_string());
        assert_commit_valid_for(&rebase_commit, &Rule::MessagePresence);
        let rebase_commit = validated_commit("fixup! foo".to_string(), "".to_string());
        assert_commit_invalid_for(&rebase_commit, &Rule::NeedsRebase);
    }

    #[test]
    fn test_validate_message_line_length() {
        let message1 = ["Hello I am a message.", "Line 2.", &"a".repeat(72)].join("\n");
        let commit1 = validated_commit("Subject".to_string(), message1);
        assert_commit_valid_for(&commit1, &Rule::MessageLineLength);

        let long_message = ["".to_string(), "a".repeat(72), "a".repeat(73)].join("\n");
        let long_line = validated_commit("Subject", &long_message);
        let violation = find_violation(long_line.violations, &Rule::MessageLineLength);
        assert_eq!(
            violation.message,
            "Line 4 in the message body is longer than 72 characters"
        );
        assert_eq!(violation.position, message_position(3, 73));
        assert_eq!(
            formatted_context(&violation),
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
        let violation = find_violation(long_line.violations, &Rule::MessageLineLength);
        assert_eq!(
            violation.message,
            "Line 2 in the message body is longer than 72 characters"
        );
        assert_eq!(violation.position, message_position(1, 73));
        assert_eq!(
            formatted_context(&violation),
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
    fn test_validate_changes_presense() {
        let with_changes = validated_commit("Subject".to_string(), "\nSome message.".to_string());
        assert_commit_valid_for(&with_changes, &Rule::DiffPresence);

        let mut without_changes = commit_without_file_changes("\nSome Message".to_string());
        without_changes.validate();
        let violation = find_violation(without_changes.violations, &Rule::DiffPresence);
        assert_eq!(violation.message, "No file changes found");
        assert_eq!(violation.position, Position::Diff);
        assert_eq!(
            formatted_context(&violation),
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
