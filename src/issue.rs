use crate::rule::Rule;
use core::ops::Range;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum IssueType {
    Error,
    Hint,
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            IssueType::Error => "Error",
            IssueType::Hint => "Hint",
        };
        write!(f, "{}", label)
    }
}

#[derive(Debug, PartialEq)]
pub struct Issue {
    pub r#type: IssueType,
    pub rule: Rule,
    pub message: String,
    pub position: Position,
    pub context: Vec<Context>,
}

impl Issue {
    pub fn error(rule: Rule, message: String, position: Position, context: Vec<Context>) -> Self {
        Self {
            r#type: IssueType::Error,
            rule,
            message,
            position,
            context,
        }
    }

    pub fn hint(rule: Rule, message: String, position: Position, context: Vec<Context>) -> Self {
        Self {
            r#type: IssueType::Hint,
            rule,
            message,
            position,
            context,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Position {
    Subject { line: usize, column: usize },
    MessageLine { line: usize, column: usize },
    Diff,
    Branch { column: usize },
}

#[derive(Debug, PartialEq)]
pub enum ContextType {
    Plain,
    Gap,
    Error,
    Addition,
    Removal,
}

#[derive(Debug, PartialEq)]
pub struct Context {
    pub r#type: ContextType,
    pub line: Option<usize>,
    pub content: String,
    pub range: Option<Range<usize>>,
    pub message: Option<String>,
}

impl Context {
    pub fn gap() -> Self {
        Self {
            r#type: ContextType::Gap,
            line: None,
            content: "".to_string(), // TODO: Make option?
            range: None,
            message: None,
        }
    }

    pub fn subject(content: String) -> Self {
        Self {
            r#type: ContextType::Plain,
            line: Some(1),
            content,
            range: None,
            message: None,
        }
    }

    pub fn subject_error(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            r#type: ContextType::Error,
            line: Some(1),
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn subject_removal_suggestion(
        content: String,
        range: Range<usize>,
        message: String,
    ) -> Self {
        Self {
            r#type: ContextType::Removal,
            line: Some(1),
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn message_line(line: usize, content: String) -> Self {
        Self {
            r#type: ContextType::Plain,
            line: Some(line),
            content,
            range: None,
            message: None,
        }
    }

    pub fn message_line_error(
        line: usize,
        content: String,
        range: Range<usize>,
        message: String,
    ) -> Self {
        Self {
            r#type: ContextType::Error,
            line: Some(line),
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn message_line_error_without_message(
        line: usize,
        content: String,
        range: Range<usize>,
    ) -> Self {
        Self {
            r#type: ContextType::Error,
            line: Some(line),
            content,
            range: Some(range),
            message: None,
        }
    }

    pub fn message_line_addition(
        line: usize,
        content: String,
        range: Range<usize>,
        message: String,
    ) -> Self {
        Self {
            r#type: ContextType::Addition,
            line: Some(line),
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn diff_line(content: String) -> Self {
        Self {
            r#type: ContextType::Plain,
            line: None,
            content,
            range: None,
            message: None,
        }
    }

    pub fn diff_error(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            r#type: ContextType::Error,
            line: None,
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn diff_addition(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            r#type: ContextType::Addition,
            line: None,
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn branch_error(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            r#type: ContextType::Error,
            line: None,
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn branch_removal_suggestion(
        content: String,
        range: Range<usize>,
        message: String,
    ) -> Self {
        Self {
            r#type: ContextType::Removal,
            line: None,
            content,
            range: Some(range),
            message: Some(message),
        }
    }
}
