use std::fmt;
use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

use crate::branch::Branch;
use crate::commit::Commit;
use crate::issue::{Context, ContextType, Issue, IssueType, Position};
use crate::utils::display_width;

enum Prefix {
    Pipe,
    Scissors,
    Note,
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Prefix::Pipe => " | ",
            Prefix::Scissors => "~~~",
            Prefix::Note => " = ",
        };
        write!(f, "{}", label)
    }
}

struct ContextLine {
    prefix: Prefix,
    width: usize,
    line_number: Option<usize>,
}

pub fn red_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Red));
    cs
}

pub fn bright_red_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Red));
    cs.set_intense(true);
    cs
}

pub fn yellow_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Yellow));
    cs
}

pub fn green_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Green));
    cs
}

pub fn blue_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Blue));
    cs
}

fn muted_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Blue));
    cs.set_intense(true);
    cs
}

pub fn issue_type_color(issue_type: &IssueType) -> ColorSpec {
    match issue_type {
        IssueType::Error => red_color(),
        IssueType::Hint => blue_color(),
    }
}

pub fn formatted_position(out: &mut impl WriteColor, position: &Position) -> io::Result<()> {
    match position {
        Position::Subject { line, column } | Position::MessageLine { line, column } => {
            write!(out, ":{}:{}", line, column)?;
        }
        Position::Branch { column } => {
            write!(out, ":{}", column)?;
        }
        Position::Diff => (),
    }

    Ok(())
}

pub fn formatted_commit_issue(
    out: &mut impl WriteColor,
    commit: &Commit,
    issue: &Issue,
) -> io::Result<()> {
    out.set_color(&issue_type_color(&issue.r#type))?;
    write!(out, "{}[{}]", issue.r#type, issue.rule)?;
    out.reset()?;
    writeln!(out, ": {}", issue.message)?;
    write!(out, "  ")?;
    let sha = match &commit.short_sha {
        Some(sha) => sha,
        None => "0000000",
    };
    out.set_color(&muted_color())?;
    write!(out, "{}", sha)?;
    formatted_position(out, &issue.position)?;
    write!(out, ":")?;
    out.reset()?;
    write!(out, " {}", commit.subject)?;
    writeln!(out)?;
    formatted_context(out, issue)?;

    Ok(())
}

pub fn formatted_branch_issue(
    out: &mut impl WriteColor,
    branch: &Branch,
    issue: &Issue,
) -> io::Result<()> {
    out.set_color(&issue_type_color(&issue.r#type))?;
    write!(out, "{}[{}]", issue.r#type, issue.rule)?;
    out.reset()?;
    writeln!(out, ": {}", issue.message)?;

    out.set_color(&muted_color())?;
    write!(out, "  Branch")?;
    formatted_position(out, &issue.position)?;
    write!(out, ":")?;
    out.reset()?;
    writeln!(out, " {}", branch.name)?;
    formatted_context(out, issue)?;
    Ok(())
}

fn line_number_width(contexts: &[Context]) -> usize {
    let default_indent = 1;
    contexts
        .iter()
        .map(|l| match l.line {
            Some(line_number) => line_number.to_string().chars().count() + 1,
            None => 0,
        })
        .max()
        .unwrap_or(0)
        + default_indent
}

fn context_line(out: &mut impl WriteColor, detail: &ContextLine) -> io::Result<()> {
    out.set_color(&muted_color())?;
    let width = detail.width;
    if let Some(line_number) = detail.line_number {
        let line_prefix = format!("{:>spaces$}", line_number.to_string(), spaces = width);
        write!(out, "{}", line_prefix)?;
    } else {
        let empty_prefix = " ".repeat(width);
        write!(out, "{}", empty_prefix)?;
    }
    write!(out, "{}", detail.prefix)?;
    out.reset()?;
    Ok(())
}

pub fn formatted_context(out: &mut impl WriteColor, issue: &Issue) -> io::Result<()> {
    let mut first_line = true;
    let line_number_width = line_number_width(&issue.context);

    for context in &issue.context {
        if first_line {
            // Add empty line to give some space between issue and commit lines
            context_line(
                out,
                &ContextLine {
                    prefix: Prefix::Pipe,
                    width: line_number_width,
                    line_number: None,
                },
            )?;
            writeln!(out)?;
        }

        let prefix = match context.r#type {
            ContextType::Gap => Prefix::Scissors,
            ContextType::Plain
            | ContextType::Error
            | ContextType::Addition
            | ContextType::Removal => Prefix::Pipe,
        };

        context_line(
            out,
            &ContextLine {
                prefix,
                width: line_number_width,
                line_number: context.line,
            },
        )?;

        // Add line that provides context to the issue
        let content = &context.content;
        // Print tabs as 4 spaces because that will render more consistently than render the tab
        // character
        let formatted_content = content.replace("\t", "    ");
        writeln!(out, "{}", formatted_content)?;

        // Add underline to the content if any
        if let Some(range) = &context.range {
            let range_start = range.start;
            let leading = match content.get(0..range_start) {
                Some(v) => display_width(v),
                None => range_start,
            };
            let range_end = range.end;
            let rest = match content.get(range_start..range_end) {
                Some(v) => display_width(v),
                None => range.len(),
            };
            let (message_color, underline_char) = match context.r#type {
                ContextType::Plain | ContextType::Gap => {
                    error!(
                        "Unknown scenario occured with '{:?}' formatting",
                        context.r#type
                    );
                    (None, "x")
                }
                ContextType::Error => (Some(bright_red_color()), "^"),
                ContextType::Addition => (Some(green_color()), "+"),
                ContextType::Removal => (Some(yellow_color()), "-"),
            };

            let leading_spaces = " ".repeat(leading);
            let underline = underline_char.repeat(rest);
            context_line(
                out,
                &ContextLine {
                    prefix: Prefix::Pipe,
                    width: line_number_width,
                    line_number: None,
                },
            )?;
            if let Some(color) = message_color {
                out.set_color(&color)?;
            }
            write!(out, "{}{}", leading_spaces, underline)?;
            // Add hint message if any
            if let Some(message) = &context.message {
                write!(out, " {}", message)?;
            }
            out.reset()?;
            writeln!(out)?;
        }
        first_line = false;
    }

    // Add empty line to give some space between issues and help lines
    context_line(
        out,
        &ContextLine {
            prefix: Prefix::Pipe,
            width: line_number_width,
            line_number: None,
        },
    )?;
    writeln!(out)?;

    context_line(
        out,
        &ContextLine {
            prefix: Prefix::Note,
            width: line_number_width,
            line_number: None,
        },
    )?;
    writeln!(out, "help: {}", issue.rule.link())?;
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::{formatted_branch_issue, formatted_commit_issue};
    use crate::branch::Branch;
    use crate::commit::Commit;
    use crate::issue::{Context, Issue, Position};
    use crate::rule::Rule;
    use crate::test::formatted_context;
    use core::ops::Range;
    use termcolor::{BufferWriter, ColorChoice};

    fn commit<S: AsRef<str>>(sha: Option<String>, subject: S, message: S) -> Commit {
        Commit::new(
            sha,
            Some("test@example.com".to_string()),
            subject.as_ref(),
            message.as_ref().to_string(),
            "".to_string(),
            vec!["src/main.rs".to_string()],
        )
    }

    fn subject_issue_error(value: &str, message: &str, range: Range<usize>) -> Issue {
        let context = Context::subject_error(value.to_string(), range, message.to_string());
        Issue::error(
            Rule::SubjectLength,
            "Dummy message".to_string(),
            Position::Subject { line: 1, column: 0 },
            vec![context],
        )
    }

    fn commit_issue(commit: &Commit, issue: &Issue) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Never);
        let mut out = bufwtr.buffer();
        match formatted_commit_issue(&mut out, commit, issue) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format commit issue: {:?}", e),
        }
    }

    fn commit_issue_color(commit: &Commit, issue: &Issue) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Always);
        let mut out = bufwtr.buffer();
        match formatted_commit_issue(&mut out, commit, issue) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format commit issue: {:?}", e),
        }
    }

    fn branch_issue(branch: &Branch, issue: &Issue) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Never);
        let mut out = bufwtr.buffer();
        match formatted_branch_issue(&mut out, branch, issue) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format branch issue: {:?}", e),
        }
    }

    fn branch_issue_color(branch: &Branch, issue: &Issue) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Always);
        let mut out = bufwtr.buffer();
        match formatted_branch_issue(&mut out, branch, issue) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format branch issue: {:?}", e),
        }
    }

    #[test]
    fn test_formatted_commit_error_with_color() {
        let commit = commit(None, "Subject", "Message");
        let context = vec![
            Context::subject("Subject".to_string()),
            Context::message_line(2, "Message body".to_string()),
            Context::message_line_error(
                3,
                "Message body line".to_string(),
                Range { start: 1, end: 3 },
                "The error hint".to_string(),
            ),
        ];
        let issue = Issue::error(
            Rule::SubjectLength,
            "The error message".to_string(),
            Position::Subject { line: 1, column: 1 },
            context,
        );
        let output = commit_issue_color(&commit, &issue);
        assert_eq!(
            output,
            "\u{1b}[0m\u{1b}[31mError[SubjectLength]\u{1b}[0m: The error message\n\
            \x20\x20\u{1b}[0m\u{1b}[38;5;12m0000000:1:1:\u{1b}[0m Subject\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  1 | \u{1b}[0mSubject\n\
            \u{1b}[0m\u{1b}[38;5;12m  2 | \u{1b}[0mMessage body\n\
            \u{1b}[0m\u{1b}[38;5;12m  3 | \u{1b}[0mMessage body line\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\u{1b}[0m\u{1b}[38;5;9m ^^ The error hint\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    = \u{1b}[0mhelp: https://r.lintje.dev/r/SubjectLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_hint_with_color() {
        let commit = commit(None, "Subject", "Message");
        let context = vec![
            Context::subject("Subject".to_string()),
            Context::message_line(2, "Message body".to_string()),
            Context::message_line_addition(
                3,
                "Message body line".to_string(),
                Range { start: 1, end: 3 },
                "The hint".to_string(),
            ),
        ];
        let issue = Issue::hint(
            Rule::SubjectLength,
            "The hint message".to_string(),
            Position::Subject { line: 1, column: 1 },
            context,
        );
        let output = commit_issue_color(&commit, &issue);
        assert_eq!(
            output,
            "\u{1b}[0m\u{1b}[34mHint[SubjectLength]\u{1b}[0m: The hint message\n\
            \x20\x20\u{1b}[0m\u{1b}[38;5;12m0000000:1:1:\u{1b}[0m Subject\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  1 | \u{1b}[0mSubject\n\
            \u{1b}[0m\u{1b}[38;5;12m  2 | \u{1b}[0mMessage body\n\
            \u{1b}[0m\u{1b}[38;5;12m  3 | \u{1b}[0mMessage body line\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\u{1b}[0m\u{1b}[32m ++ The hint\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    = \u{1b}[0mhelp: https://r.lintje.dev/r/SubjectLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_without_sha() {
        let commit = commit(None, "Subject", "Message");
        let context = vec![Context::subject("Subject".to_string())];
        let issue = Issue::error(
            Rule::SubjectLength,
            "The error message".to_string(),
            Position::Subject { line: 1, column: 1 },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[SubjectLength]: The error message\n\
            \x20\x200000000:1:1: Subject\n\
            \x20\x20  | \n\
            \x20\x201 | Subject\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://r.lintje.dev/r/SubjectLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_subject() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::subject("Subject".to_string())];
        let issue = Issue::error(
            Rule::SubjectLength,
            "The error message".to_string(),
            Position::Subject { line: 1, column: 1 },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[SubjectLength]: The error message\n\
            \x20\x201234567:1:1: Subject\n\
            \x20\x20  | \n\
            \x20\x201 | Subject\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://r.lintje.dev/r/SubjectLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_subject_error() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::subject_error(
            "Subject".to_string(),
            Range { start: 1, end: 3 },
            "The hint".to_string(),
        )];
        let issue = Issue::error(
            Rule::SubjectMood,
            "The error message".to_string(),
            Position::Subject { line: 1, column: 2 },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[SubjectMood]: The error message\n\
            \x20\x201234567:1:2: Subject\n\
            \x20\x20  | \n\
            \x20\x201 | Subject\n\
            \x20\x20  |  ^^ The hint\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://r.lintje.dev/r/SubjectMood\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_message_line() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::message_line(11, "Message line".to_string())];
        let issue = Issue::error(
            Rule::MessageLineLength,
            "The error message".to_string(),
            Position::MessageLine {
                line: 11,
                column: 50,
            },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[MessageLineLength]: The error message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20   | \n\
            \x20\x2011 | Message line\n\
            \x20\x20   | \n\
            \x20\x20   = help: https://r.lintje.dev/r/MessageLineLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_message_line_error() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![
            Context::message_line(11, "Message line".to_string()),
            Context::message_line_error(
                12,
                "Message line with hint".to_string(),
                Range { start: 3, end: 10 },
                "My hint".to_string(),
            ),
        ];
        let issue = Issue::error(
            Rule::MessageLineLength,
            "The error message".to_string(),
            Position::MessageLine {
                line: 11,
                column: 50,
            },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[MessageLineLength]: The error message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20   | \n\
            \x20\x2011 | Message line\n\
            \x20\x2012 | Message line with hint\n\
            \x20\x20   |    ^^^^^^^ My hint\n\
            \x20\x20   | \n\
            \x20\x20   = help: https://r.lintje.dev/r/MessageLineLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_message_line_error_without_message() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![
            Context::message_line(11, "Message line".to_string()),
            Context::message_line_error_without_message(
                12,
                "Message line without hint".to_string(),
                Range { start: 3, end: 10 },
            ),
        ];
        let issue = Issue::error(
            Rule::MessageLineLength,
            "The error message".to_string(),
            Position::MessageLine {
                line: 11,
                column: 50,
            },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[MessageLineLength]: The error message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20   | \n\
            \x20\x2011 | Message line\n\
            \x20\x2012 | Message line without hint\n\
            \x20\x20   |    ^^^^^^^\n\
            \x20\x20   | \n\
            \x20\x20   = help: https://r.lintje.dev/r/MessageLineLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_message_line_addition() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![
            Context::message_line(11, "Message line".to_string()),
            Context::message_line_addition(
                12,
                "Message line with addition".to_string(),
                Range { start: 3, end: 10 },
                "My addition suggestion".to_string(),
            ),
        ];
        let issue = Issue::hint(
            Rule::MessageLineLength,
            "The hint message".to_string(),
            Position::MessageLine {
                line: 11,
                column: 50,
            },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Hint[MessageLineLength]: The hint message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20   | \n\
            \x20\x2011 | Message line\n\
            \x20\x2012 | Message line with addition\n\
            \x20\x20   |    +++++++ My addition suggestion\n\
            \x20\x20   | \n\
            \x20\x20   = help: https://r.lintje.dev/r/MessageLineLength\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_diff_error() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::diff_error(
            "Diff line".to_string(),
            Range { start: 3, end: 5 },
            "My suggestion".to_string(),
        )];
        let issue = Issue::error(
            Rule::DiffPresence,
            "The error message".to_string(),
            Position::Diff,
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[DiffPresence]: The error message\n\
            \x20\x201234567: Subject\n\
            \x20\x20| \n\
            \x20\x20| Diff line\n\
            \x20\x20|    ^^ My suggestion\n\
            \x20\x20| \n\
            \x20\x20= help: https://r.lintje.dev/r/DiffPresence\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_issue_diff_addition() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::diff_addition(
            "Diff line".to_string(),
            Range { start: 5, end: 9 },
            "My addition".to_string(),
        )];
        let issue = Issue::error(
            Rule::DiffChangeset,
            "The error message".to_string(),
            Position::Diff,
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Error[DiffChangeset]: The error message\n\
            \x20\x201234567: Subject\n\
            \x20\x20| \n\
            \x20\x20| Diff line\n\
            \x20\x20|      ++++ My addition\n\
            \x20\x20| \n\
            \x20\x20= help: https://r.lintje.dev/r/DiffChangeset\n\n"
        );
    }

    #[test]
    fn test_formatted_branch_issue_branch_error() {
        let branch = Branch::new("branch-name".to_string());
        let context = vec![Context::branch_error(
            "branch-name".to_string(),
            Range { start: 3, end: 5 },
            "My hint".to_string(),
        )];
        let issue = Issue::error(
            Rule::BranchNameLength,
            "The error message".to_string(),
            Position::Branch { column: 3 },
            context,
        );
        let output = branch_issue(&branch, &issue);
        assert_eq!(
            output,
            "Error[BranchNameLength]: The error message\n\
            \x20\x20Branch:3: branch-name\n\
            \x20\x20| \n\
            \x20\x20| branch-name\n\
            \x20\x20|    ^^ My hint\n\
            \x20\x20| \n\
            \x20\x20= help: https://r.lintje.dev/r/BranchNameLength\n\n"
        );
    }

    #[test]
    fn test_formatted_branch_issue_branch_error_with_color() {
        let branch = Branch::new("branch-name".to_string());
        let context = vec![Context::branch_error(
            "branch-name".to_string(),
            Range { start: 3, end: 5 },
            "My hint".to_string(),
        )];
        let issue = Issue::error(
            Rule::BranchNameLength,
            "The error message".to_string(),
            Position::Branch { column: 3 },
            context,
        );
        let output = branch_issue_color(&branch, &issue);
        assert_eq!(
            output,
            "\u{1b}[0m\u{1b}[31mError[BranchNameLength]\u{1b}[0m: The error message\n\
            \u{1b}[0m\u{1b}[38;5;12m  Branch:3:\u{1b}[0m branch-name\n\
            \u{1b}[0m\u{1b}[38;5;12m  | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  | \u{1b}[0mbranch-name\n\
            \u{1b}[0m\u{1b}[38;5;12m  | \u{1b}[0m\u{1b}[0m\u{1b}[38;5;9m   ^^ My hint\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  = \u{1b}[0mhelp: https://r.lintje.dev/r/BranchNameLength\n\n"
        );
    }

    #[test]
    fn formatted_context_subject() {
        let context = vec![
            Context::subject("Subject".to_string()),
            Context::message_line(2, "".to_string()),
            Context::message_line(3, "Line 1".to_string()),
        ];
        let issue = Issue::error(
            Rule::SubjectLength,
            "Dummy message".to_string(),
            Position::Subject { line: 0, column: 0 },
            context,
        );
        assert_eq!(
            formatted_context(&issue),
            "\x20 | \n\
                1 | Subject\n\
                2 | \n\
                3 | Line 1\n\
             \x20 | \n\
             \x20 = help: https://r.lintje.dev/r/SubjectLength\n"
        );
    }

    #[test]
    fn formatted_context_message_multi_line() {
        let context = vec![
            Context::message_line(9, "Line 9".to_string()),
            Context::message_line(10, "Line 10".to_string()),
            Context::message_line(11, "Line 11".to_string()),
            Context::message_line_error(
                12,
                "Line 12".to_string(),
                Range { start: 1, end: 2 },
                "Message".to_string(),
            ),
        ];
        let issue = Issue::error(
            Rule::MessageLineLength,
            "Dummy message".to_string(),
            Position::MessageLine { line: 1, column: 0 },
            context,
        );
        assert_eq!(
            formatted_context(&issue),
            "\x20\x20 | \n\
                \x209 | Line 9\n\
                   10 | Line 10\n\
                   11 | Line 11\n\
                   12 | Line 12\n\
             \x20\x20 |  ^ Message\n\
             \x20\x20 | \n\
             \x20\x20 = help: https://r.lintje.dev/r/MessageLineLength\n"
        );
    }

    #[test]
    fn formatted_context_branch() {
        let context = vec![Context::branch_error(
            "branch-name".to_string(),
            Range { start: 1, end: 3 },
            "A message".to_string(),
        )];
        let issue = Issue::error(
            Rule::BranchNameLength,
            "Dummy message".to_string(),
            Position::Branch { column: 0 },
            context,
        );
        assert_eq!(
            formatted_context(&issue),
            "| \n\
             | branch-name\n\
             |  ^^ A message\n\
             | \n\
             = help: https://r.lintje.dev/r/BranchNameLength\n"
        );
    }

    #[test]
    fn formatted_context_branch_removal_suggestion() {
        let context = vec![Context::branch_removal_suggestion(
            "branch-name".to_string(),
            Range { start: 1, end: 3 },
            "A message".to_string(),
        )];
        let issue = Issue::error(
            Rule::BranchNameLength,
            "Dummy message".to_string(),
            Position::Branch { column: 0 },
            context,
        );
        assert_eq!(
            formatted_context(&issue),
            "| \n\
             | branch-name\n\
             |  -- A message\n\
             | \n\
             = help: https://r.lintje.dev/r/BranchNameLength\n"
        );
    }

    #[test]
    fn formatted_context_diff() {
        let context = vec![Context::diff_error(
            "Some diff".to_string(),
            Range { start: 1, end: 3 },
            "A message".to_string(),
        )];
        let issue = Issue::error(
            Rule::DiffPresence,
            "Dummy message".to_string(),
            Position::Diff,
            context,
        );
        assert_eq!(
            formatted_context(&issue),
            "| \n\
             | Some diff\n\
             |  ^^ A message\n\
             | \n\
             = help: https://r.lintje.dev/r/DiffPresence\n"
        );
    }

    #[test]
    fn formatted_context_line_with_gap() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![
            Context::diff_line("Some diff line".to_string()),
            Context::gap(),
            Context::message_line(1, "Message line 10".to_string()),
        ];
        let issue = Issue::hint(
            Rule::MessageLineLength,
            "The hint message".to_string(),
            Position::MessageLine {
                line: 11,
                column: 50,
            },
            context,
        );
        let output = commit_issue(&commit, &issue);
        assert_eq!(
            output,
            "Hint[MessageLineLength]: The hint message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20  | \n\
            \x20\x20  | Some diff line\n\
            \x20\x20 ~~~\n\
            \x20\x201 | Message line 10\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://r.lintje.dev/r/MessageLineLength\n\n"
        );
    }

    #[test]
    fn formatted_context_ascii() {
        let v_start = subject_issue_error("Lorem ipsum", "A lorem", Range { start: 0, end: 5 });
        assert_eq!(
            formatted_context(&v_start),
            "\x20\x20| \n\
                   1 | Lorem ipsum\n\
             \x20\x20| ^^^^^ A lorem\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );

        let v_end = subject_issue_error("Lorem ipsum", "A sum", Range { start: 8, end: 11 });
        assert_eq!(
            formatted_context(&v_end),
            "\x20\x20| \n\
                   1 | Lorem ipsum\n\
             \x20\x20|         ^^^ A sum\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );

        let v_middle = subject_issue_error("Lorem ipsum", "A space", Range { start: 5, end: 6 });
        assert_eq!(
            formatted_context(&v_middle),
            "\x20\x20| \n\
                   1 | Lorem ipsum\n\
             \x20\x20|      ^ A space\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );
    }

    #[test]
    fn formatted_context_whitespace() {
        let v_space = subject_issue_error(" Lorem ipsum", "A space", Range { start: 0, end: 1 });
        assert_eq!(
            formatted_context(&v_space),
            "\x20\x20| \n\
                   1 |  Lorem ipsum\n\
             \x20\x20| ^ A space\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );

        let v_space = subject_issue_error("\x20Lorem ipsum", "A space", Range { start: 0, end: 1 });
        assert_eq!(
            formatted_context(&v_space),
            "\x20\x20| \n\
                   1 | \x20Lorem ipsum\n\
             \x20\x20| ^ A space\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );

        let v_tab = subject_issue_error(
            "\tLorem ipsum",
            "A tab",
            Range {
                start: 0,
                end: "\t".len(),
            },
        );
        assert_eq!(
            formatted_context(&v_tab),
            "\x20\x20| \n\
                   1 |     Lorem ipsum\n\
             \x20\x20| ^^^^ A tab\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );
    }

    #[test]
    fn formatted_context_accents() {
        // This accented character is two characters, the `a` and the accent, but renders as one
        // column. The character is 3 bytes.
        //
        // This test makes sure the formatted_context function points to the single column, because
        // it has a display width of one, and not two columns because it's two characters.
        let v = subject_issue_error(
            "This is a̐ char with an accent",
            "Mark accent",
            Range { start: 8, end: 11 },
        );
        assert_eq!(
            formatted_context(&v),
            "\x20\x20| \n\
                   1 | This is a̐ char with an accent\n\
             \x20\x20|         ^ Mark accent\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );
    }

    #[test]
    fn formatted_context_emoji() {
        let v = subject_issue_error("Aa😀Bb", "Mark emoji", Range { start: 2, end: 4 });
        assert_eq!(
            formatted_context(&v),
            "\x20\x20| \n\
                   1 | Aa😀Bb\n\
             \x20\x20|   ^^ Mark emoji\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );

        let v = subject_issue_error("Aa👍Bb", "Mark emoji", Range { start: 2, end: 4 });
        assert_eq!(
            formatted_context(&v),
            "\x20\x20| \n\
                   1 | Aa👍Bb\n\
             \x20\x20|   ^^ Mark emoji\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );

        let v = subject_issue_error(
            "Fix ❤️ in controller Fix #123",
            "Mark fix ticket",
            Range { start: 25, end: 33 },
        );
        assert_eq!(
            formatted_context(&v),
            "\x20\x20| \n\
                   1 | Fix ❤️ in controller Fix #123\n\
             \x20\x20|                     ^^^^^^^^ Mark fix ticket\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );
    }

    #[test]
    fn formatted_context_double_width() {
        let v = subject_issue_error(
            "ああああああああああああああああああああああああああ",
            "Mark double width character",
            Range { start: 75, end: 78 },
        );
        assert_eq!(
            formatted_context(&v),
            "\x20\x20| \n\
                   1 | ああああああああああああああああああああああああああ\n\
             \x20\x20|                                                   ^^ Mark double width character\n\
             \x20\x20| \n\
             \x20\x20= help: https://r.lintje.dev/r/SubjectLength\n"
        );
    }
}
