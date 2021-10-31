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
