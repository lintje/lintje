use crate::utils::display_width;
use core::ops::Range;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Rule {
    MergeCommit,
    NeedsRebase,
    SubjectLength,
    SubjectMood,
    SubjectWhitespace,
    SubjectCapitalization,
    SubjectPunctuation,
    SubjectTicketNumber,
    SubjectPrefix,
    SubjectBuildTag,
    SubjectCliche,
    MessageEmptyFirstLine,
    MessagePresence,
    MessageLineLength,
    DiffPresence,
    BranchNameTicketNumber,
    BranchNameLength,
    BranchNamePunctuation,
    BranchNameCliche,
}

#[derive(Debug, PartialEq)]
pub struct Violation {
    pub rule: Rule,
    pub message: String,
    pub position: Position,
    pub context: Vec<Context>,
}

#[derive(Debug, PartialEq)]
pub enum Position {
    Subject { column: usize },
    MessageLine { line: usize, column: usize },
    Diff,
    Branch { column: usize },
}

impl Position {
    pub fn line_number(&self) -> Option<usize> {
        match self {
            Self::Subject { column: _ } => Some(1),
            Self::MessageLine { line, column: _ } => Some(*line + 1),
            Self::Diff => None,
            Self::Branch { column: _ } => None,
        }
    }

    pub fn column(&self) -> Option<usize> {
        match self {
            Self::Subject { column } => Some(*column),
            Self::MessageLine { line: _, column } => Some(*column),
            Self::Diff => None,
            Self::Branch { column } => Some(*column),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Source {
    Subject { content: String },
    MessageLine { line: usize, content: String },
    Branch { content: String },
    Diff { content: String },
}

#[derive(Debug, PartialEq)]
pub struct Context {
    pub source: Source,
    pub hint: Option<Hint>,
}

impl Source {
    pub fn line_number(&self) -> Option<usize> {
        match self {
            Self::Subject { content: _ } => Some(0),
            Self::MessageLine { line, content: _ } => Some(*line + 1),
            _ => None,
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Self::Subject { content } => &*content,
            Self::MessageLine { line: _, content } => &*content,
            Self::Branch { content } => &*content,
            Self::Diff { content } => &*content,
        }
    }
}

impl Context {
    pub fn subject(content: String) -> Self {
        Self {
            source: Source::Subject { content },
            hint: None,
        }
    }

    pub fn subject_hint(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            source: Source::Subject { content },
            hint: Some(Hint { range, message }),
        }
    }

    pub fn message_line(line: usize, content: String) -> Self {
        Self {
            source: Source::MessageLine { line, content },
            hint: None,
        }
    }

    pub fn message_line_hint(
        line: usize,
        content: String,
        range: Range<usize>,
        message: String,
    ) -> Self {
        Self {
            source: Source::MessageLine { line, content },
            hint: Some(Hint { range, message }),
        }
    }

    pub fn diff_hint(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            source: Source::Diff { content },
            hint: Some(Hint { range, message }),
        }
    }

    pub fn branch_hint(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            source: Source::Branch { content },
            hint: Some(Hint { range, message }),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Hint {
    pub range: Range<usize>,
    pub message: String,
}

impl Violation {
    pub fn formatted_context(&self) -> String {
        let mut s = String::from("");
        let mut first_line = true;
        let line_number_width = &self
            .context
            .iter()
            .map(|l| match l.source.line_number() {
                Some(line_number) => (line_number + 1).to_string().chars().count() + 1,
                None => 0,
            })
            .max()
            .unwrap_or(0);

        for context in &self.context {
            let plain_line_number = if let Some(line_number) = context.source.line_number() {
                format!("{} ", line_number + 1)
            } else {
                "".to_string()
            };
            let line_prefix = format!("{:>spaces$}", plain_line_number, spaces = line_number_width);
            let empty_prefix = " ".repeat(line_prefix.len());
            if first_line {
                // Add empty line to give some space between violation and commit lines
                s.push_str(&format!("{}|\n", empty_prefix));
            }

            // Add line that provides context to the violation
            let content = &context.source.content();
            // Print tabs as 4 spaces because that will render more consistenly than render the tab
            // character
            let formatted_content = content.replace("\t", "    ");
            s.push_str(&format!("{}| {}\n", line_prefix, formatted_content));

            // Add a hint if any
            if let Some(hint) = &context.hint {
                let range_start = hint.range.start;
                let leading = match content.get(0..range_start) {
                    Some(v) => display_width(v),
                    None => range_start,
                };
                let range_end = hint.range.end;
                let rest = match content.get(range_start..range_end) {
                    Some(v) => display_width(v),
                    None => hint.range.len(),
                };

                let leading_spaces = " ".repeat(leading);
                let underline = "^".repeat(rest);
                let message = format!("{}{} {}", leading_spaces, underline, hint.message);
                s.push_str(&format!("{}| {}\n", empty_prefix, message));
            }
            first_line = false;
        }
        s
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Rule::MergeCommit => "MergeCommit",
            Rule::NeedsRebase => "NeedsRebase",
            Rule::SubjectLength => "SubjectLength",
            Rule::SubjectMood => "SubjectMood",
            Rule::SubjectWhitespace => "SubjectWhitespace",
            Rule::SubjectCapitalization => "SubjectCapitalization",
            Rule::SubjectPunctuation => "SubjectPunctuation",
            Rule::SubjectTicketNumber => "SubjectTicketNumber",
            Rule::SubjectPrefix => "SubjectPrefix",
            Rule::SubjectBuildTag => "SubjectBuildTag",
            Rule::SubjectCliche => "SubjectCliche",
            Rule::MessageEmptyFirstLine => "MessageEmptyFirstLine",
            Rule::MessagePresence => "MessagePresence",
            Rule::MessageLineLength => "MessageLineLength",
            Rule::DiffPresence => "DiffPresence",
            Rule::BranchNameTicketNumber => "BranchNameTicketNumber",
            Rule::BranchNameLength => "BranchNameLength",
            Rule::BranchNamePunctuation => "BranchNamePunctuation",
            Rule::BranchNameCliche => "BranchNameCliche",
        };
        write!(f, "{}", label)
    }
}

pub fn rule_by_name(name: &str) -> Option<Rule> {
    match name {
        "MergeCommit" => Some(Rule::MergeCommit),
        "NeedsRebase" => Some(Rule::NeedsRebase),
        "SubjectLength" => Some(Rule::SubjectLength),
        "SubjectMood" => Some(Rule::SubjectMood),
        "SubjectWhitespace" => Some(Rule::SubjectWhitespace),
        "SubjectCapitalization" => Some(Rule::SubjectCapitalization),
        "SubjectPunctuation" => Some(Rule::SubjectPunctuation),
        "SubjectTicketNumber" => Some(Rule::SubjectTicketNumber),
        "SubjectBuildTag" => Some(Rule::SubjectBuildTag),
        "SubjectPrefix" => Some(Rule::SubjectPrefix),
        "SubjectCliche" => Some(Rule::SubjectCliche),
        "MessageEmptyFirstLine" => Some(Rule::MessageEmptyFirstLine),
        "MessagePresence" => Some(Rule::MessagePresence),
        "MessageLineLength" => Some(Rule::MessageLineLength),
        "DiffPresence" => Some(Rule::DiffPresence),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Context, Position, Rule, Violation};
    use core::ops::Range;

    fn subject_violation_hint(value: &str, message: &str, range: Range<usize>) -> Violation {
        let context = Context::subject_hint(value.to_string(), range, message.to_string());
        Violation {
            rule: Rule::SubjectLength,
            message: "Dummy message".to_string(),
            position: Position::Subject { column: 0 },
            context: vec![context],
        }
    }

    #[test]
    fn formatted_context_subject() {
        let context = vec![
            Context::subject("Subject".to_string()),
            Context::message_line(0, "".to_string()),
            Context::message_line(1, "Line 1".to_string()),
        ];
        let violation = Violation {
            rule: Rule::SubjectLength,
            message: "Dummy message".to_string(),
            position: Position::MessageLine { line: 0, column: 0 },
            context,
        };
        assert_eq!(
            violation.formatted_context(),
            "\x20 |\n\
                1 | Subject\n\
                2 | \n\
                3 | Line 1\n"
        );
    }

    #[test]
    fn formatted_context_message_multi_line() {
        let context = vec![
            Context::message_line(7, "Line 9".to_string()),
            Context::message_line(8, "Line 10".to_string()),
            Context::message_line(9, "Line 11".to_string()),
            Context::message_line_hint(
                10,
                "Line 12".to_string(),
                Range { start: 1, end: 2 },
                "Message".to_string(),
            ),
        ];
        let violation = Violation {
            rule: Rule::SubjectLength,
            message: "Dummy message".to_string(),
            position: Position::MessageLine { line: 0, column: 0 },
            context,
        };
        assert_eq!(
            violation.formatted_context(),
            "\x20\x20 |\n\
                \x209 | Line 9\n\
                   10 | Line 10\n\
                   11 | Line 11\n\
                   12 | Line 12\n\
             \x20\x20 |  ^ Message\n"
        );
    }

    #[test]
    fn formatted_context_branch() {
        let context = vec![Context::branch_hint(
            "branch-name".to_string(),
            Range { start: 1, end: 3 },
            "A message".to_string(),
        )];
        let violation = Violation {
            rule: Rule::BranchNameLength,
            message: "Dummy message".to_string(),
            position: Position::Branch { column: 0 },
            context,
        };
        assert_eq!(
            violation.formatted_context(),
            "|\n\
             | branch-name\n\
             |  ^^ A message\n"
        );
    }

    #[test]
    fn formatted_context_diff() {
        let context = vec![Context::diff_hint(
            "Some diff".to_string(),
            Range { start: 1, end: 3 },
            "A message".to_string(),
        )];
        let violation = Violation {
            rule: Rule::DiffPresence,
            message: "Dummy message".to_string(),
            position: Position::Diff,
            context,
        };
        assert_eq!(
            violation.formatted_context(),
            "|\n\
             | Some diff\n\
             |  ^^ A message\n"
        );
    }

    #[test]
    fn formatted_context_ascii() {
        let v_start = subject_violation_hint("Lorem ipsum", "A lorem", Range { start: 0, end: 5 });
        assert_eq!(
            v_start.formatted_context(),
            "\x20\x20|\n\
                   1 | Lorem ipsum\n\
             \x20\x20| ^^^^^ A lorem\n"
        );

        let v_end = subject_violation_hint("Lorem ipsum", "A sum", Range { start: 8, end: 11 });
        assert_eq!(
            v_end.formatted_context(),
            "\x20\x20|\n\
                   1 | Lorem ipsum\n\
             \x20\x20|         ^^^ A sum\n"
        );

        let v_middle = subject_violation_hint("Lorem ipsum", "A space", Range { start: 5, end: 6 });
        assert_eq!(
            v_middle.formatted_context(),
            "\x20\x20|\n\
                   1 | Lorem ipsum\n\
             \x20\x20|      ^ A space\n"
        );
    }

    #[test]
    fn formatted_context_whitespace() {
        let v_space = subject_violation_hint(" Lorem ipsum", "A space", Range { start: 0, end: 1 });
        assert_eq!(
            v_space.formatted_context(),
            "\x20\x20|\n\
                   1 |  Lorem ipsum\n\
             \x20\x20| ^ A space\n"
        );

        let v_space =
            subject_violation_hint("\x20Lorem ipsum", "A space", Range { start: 0, end: 1 });
        assert_eq!(
            v_space.formatted_context(),
            "\x20\x20|\n\
                   1 | \x20Lorem ipsum\n\
             \x20\x20| ^ A space\n"
        );

        let v_tab = subject_violation_hint(
            "\tLorem ipsum",
            "A tab",
            Range {
                start: 0,
                end: "\t".len(),
            },
        );
        assert_eq!(
            v_tab.formatted_context(),
            "\x20\x20|\n\
                   1 |     Lorem ipsum\n\
             \x20\x20| ^^^^ A tab\n"
        );
    }

    #[test]
    fn formatted_context_accents() {
        // This accented character is two characters, the `a` and the accent, but renders as one
        // column. The character is 3 bytes.
        //
        // This test makes sure the formatted_context function points to the single column, because
        // it has a display width of one, and not two columns because it's two characters.
        let v = subject_violation_hint(
            "This is a̐ char with an accent",
            "Mark accent",
            Range { start: 8, end: 11 },
        );
        assert_eq!(
            v.formatted_context(),
            "\x20\x20|\n\
                   1 | This is a̐ char with an accent\n\
             \x20\x20|         ^ Mark accent\n"
        );
    }

    #[test]
    fn formatted_context_emoji() {
        let v = subject_violation_hint("Aa😀Bb", "Mark emoji", Range { start: 2, end: 4 });
        assert_eq!(
            v.formatted_context(),
            "\x20\x20|\n\
                   1 | Aa😀Bb\n\
             \x20\x20|   ^^ Mark emoji\n"
        );

        let v = subject_violation_hint("Aa👍Bb", "Mark emoji", Range { start: 2, end: 4 });
        assert_eq!(
            v.formatted_context(),
            "\x20\x20|\n\
                   1 | Aa👍Bb\n\
             \x20\x20|   ^^ Mark emoji\n"
        );
    }

    #[test]
    fn formatted_context_double_width() {
        let v = subject_violation_hint(
            "ああああああああああああああああああああああああああ",
            "Mark double width character",
            Range { start: 75, end: 78 },
        );
        assert_eq!(
            v.formatted_context(),
            "\x20\x20|\n\
                   1 | ああああああああああああああああああああああああああ\n\
             \x20\x20|                                                   ^^ Mark double width character\n"
        );
    }
}
