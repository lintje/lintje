use crate::rule::Rule;
use core::ops::Range;

#[derive(Debug, PartialEq)]
pub enum IssueType {
    Error,
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
}

#[derive(Debug, PartialEq)]
pub enum Position {
    Subject { line: usize, column: usize },
    MessageLine { line: usize, column: usize },
    Diff,
    Branch { column: usize },
}

#[derive(Debug, PartialEq)]
pub struct Context {
    pub line: Option<usize>,
    pub content: String,
    pub range: Option<Range<usize>>,
    pub message: Option<String>,
}

impl Context {
    pub fn subject(content: String) -> Self {
        Self {
            line: Some(1),
            content,
            range: None,
            message: None,
        }
    }

    pub fn subject_hint(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            line: Some(1),
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn message_line(line: usize, content: String) -> Self {
        Self {
            line: Some(line),
            content,
            range: None,
            message: None,
        }
    }

    pub fn message_line_hint(
        line: usize,
        content: String,
        range: Range<usize>,
        message: String,
    ) -> Self {
        Self {
            line: Some(line),
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn diff_hint(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            line: None,
            content,
            range: Some(range),
            message: Some(message),
        }
    }

    pub fn branch_hint(content: String, range: Range<usize>, message: String) -> Self {
        Self {
            line: None,
            content,
            range: Some(range),
            message: Some(message),
        }
    }
}
