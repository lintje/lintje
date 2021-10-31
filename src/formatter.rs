use std::io;
use termcolor::{Color, ColorSpec, WriteColor};

use crate::branch::Branch;
use crate::commit::Commit;
use crate::rule::Violation;
use crate::utils::display_width;

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

pub fn green_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Green));
    cs
}

fn muted_color() -> ColorSpec {
    let mut cs = ColorSpec::new();
    cs.set_fg(Some(Color::Blue));
    cs.set_intense(true);
    cs
}

pub fn formatted_commit_violation(
    out: &mut impl WriteColor,
    commit: &Commit,
    violation: &Violation,
) -> io::Result<()> {
    out.set_color(&red_color())?;
    write!(out, "{}", violation.rule)?;
    out.reset()?;
    writeln!(out, ": {}", violation.message)?;
    write!(out, "  ")?;
    let sha = match &commit.short_sha {
        Some(sha) => sha,
        None => "0000000",
    };
    out.set_color(&muted_color())?;
    write!(out, "{}", sha)?;
    if let Some(line_number) = &violation.position.line_number() {
        write!(out, ":{}", line_number)?;
    }
    if let Some(column) = &violation.position.column() {
        write!(out, ":{}", column)?;
    }
    write!(out, ":")?;
    out.reset()?;
    write!(out, " {}", commit.subject)?;
    writeln!(out)?;
    formatted_context(out, violation)?;

    Ok(())
}

pub fn formatted_branch_violation(
    out: &mut impl WriteColor,
    branch: &Branch,
    violation: &Violation,
) -> io::Result<()> {
    out.set_color(&red_color())?;
    write!(out, "{}", violation.rule)?;
    out.reset()?;
    writeln!(out, ": {}", violation.message)?;

    out.set_color(&muted_color())?;
    write!(out, "  Branch")?;
    if let Some(column) = &violation.position.column() {
        write!(out, ":{}", column)?;
    }
    write!(out, ":")?;
    out.reset()?;
    writeln!(out, " {}", branch.name)?;
    formatted_context(out, violation)?;
    Ok(())
}

pub fn formatted_context(out: &mut impl WriteColor, violation: &Violation) -> io::Result<()> {
    let mut first_line = true;
    let line_number_width = &violation
        .context
        .iter()
        .map(|l| match l.source.line_number() {
            Some(line_number) => (line_number + 1).to_string().chars().count() + 1,
            None => 0,
        })
        .max()
        .unwrap_or(0)
        + 2;

    for context in &violation.context {
        let plain_line_number = if let Some(line_number) = context.source.line_number() {
            format!("{} ", line_number + 1)
        } else {
            "".to_string()
        };
        let line_prefix = format!("{:>spaces$}", plain_line_number, spaces = line_number_width);
        let empty_prefix = " ".repeat(line_prefix.len());
        if first_line {
            // Add empty line to give some space between violation and commit lines
            out.set_color(&muted_color())?;
            write!(out, "{}|", empty_prefix)?;
            out.reset()?;
            writeln!(out)?;
        }

        out.set_color(&muted_color())?;
        write!(out, "{}|", line_prefix)?;
        out.reset()?;
        // Add line that provides context to the violation
        let content = &context.source.content();
        // Print tabs as 4 spaces because that will render more consistently than render the tab
        // character
        let formatted_content = content.replace("\t", "    ");
        writeln!(out, " {}", formatted_content)?;

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
            out.set_color(&muted_color())?;
            write!(out, "{}|", empty_prefix)?;
            out.set_color(&bright_red_color())?;
            write!(out, " {}{} {}", leading_spaces, underline, hint.message)?;
            out.reset()?;
            writeln!(out)?;
        }
        first_line = false;
    }
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::{formatted_branch_violation, formatted_commit_violation};
    use crate::branch::Branch;
    use crate::commit::Commit;
    use crate::rule::{Context, Position, Rule, Violation};
    use crate::utils::test::formatted_context;
    use core::ops::Range;
    use termcolor::{BufferWriter, ColorChoice};

    fn commit<S: AsRef<str>>(sha: Option<String>, subject: S, message: S) -> Commit {
        Commit::new(
            sha,
            Some("test@example.com".to_string()),
            subject.as_ref().to_string(),
            message.as_ref().to_string(),
            true,
        )
    }

    fn subject_violation_hint(value: &str, message: &str, range: Range<usize>) -> Violation {
        let context = Context::subject_hint(value.to_string(), range, message.to_string());
        Violation {
            rule: Rule::SubjectLength,
            message: "Dummy message".to_string(),
            position: Position::Subject { column: 0 },
            context: vec![context],
        }
    }

    fn commit_violation(commit: &Commit, violation: &Violation) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Never);
        let mut out = bufwtr.buffer();
        match formatted_commit_violation(&mut out, commit, violation) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format commit violation: {:?}", e),
        }
    }

    fn commit_violation_color(commit: &Commit, violation: &Violation) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Always);
        let mut out = bufwtr.buffer();
        match formatted_commit_violation(&mut out, commit, violation) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format commit violation: {:?}", e),
        }
    }

    fn branch_violation(branch: &Branch, violation: &Violation) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Never);
        let mut out = bufwtr.buffer();
        match formatted_branch_violation(&mut out, branch, violation) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format branch violation: {:?}", e),
        }
    }

    fn branch_violation_color(branch: &Branch, violation: &Violation) -> String {
        let bufwtr = BufferWriter::stdout(ColorChoice::Always);
        let mut out = bufwtr.buffer();
        match formatted_branch_violation(&mut out, branch, violation) {
            Ok(()) => String::from_utf8_lossy(out.as_slice()).to_string(),
            Err(e) => panic!("Unable to format branch violation: {:?}", e),
        }
    }

    #[test]
    fn test_formatted_commit_violation_with_color() {
        let commit = commit(None, "Subject", "Message");
        let context = vec![
            Context::subject("Subject".to_string()),
            Context::message_line(0, "Message body".to_string()),
            Context::message_line_hint(
                1,
                "Message body line".to_string(),
                Range { start: 1, end: 3 },
                "The hint".to_string(),
            ),
        ];
        let violation = Violation {
            rule: Rule::SubjectLength,
            message: "The error message".to_string(),
            position: Position::Subject { column: 1 },
            context,
        };
        let output = commit_violation_color(&commit, &violation);
        assert_eq!(
            output,
            "\u{1b}[0m\u{1b}[31mSubjectLength\u{1b}[0m: The error message\n\
            \x20\x20\u{1b}[0m\u{1b}[38;5;12m0000000:1:1:\u{1b}[0m Subject\n\
            \u{1b}[0m\u{1b}[38;5;12m    |\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  1 |\u{1b}[0m Subject\n\
            \u{1b}[0m\u{1b}[38;5;12m  2 |\u{1b}[0m Message body\n\
            \u{1b}[0m\u{1b}[38;5;12m  3 |\u{1b}[0m Message body line\n\
            \u{1b}[0m\u{1b}[38;5;12m    |\u{1b}[0m\u{1b}[38;5;9m  ^^ The hint\u{1b}[0m\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_violation_without_sha() {
        let commit = commit(None, "Subject", "Message");
        let context = vec![Context::subject("Subject".to_string())];
        let violation = Violation {
            rule: Rule::SubjectLength,
            message: "The error message".to_string(),
            position: Position::Subject { column: 1 },
            context,
        };
        let output = commit_violation(&commit, &violation);
        assert_eq!(
            output,
            "SubjectLength: The error message\n\
            \x20\x200000000:1:1: Subject\n\
            \x20\x20  |\n\
            \x20\x201 | Subject\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_violation_subject() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::subject("Subject".to_string())];
        let violation = Violation {
            rule: Rule::SubjectLength,
            message: "The error message".to_string(),
            position: Position::Subject { column: 1 },
            context,
        };
        let output = commit_violation(&commit, &violation);
        assert_eq!(
            output,
            "SubjectLength: The error message\n\
            \x20\x201234567:1:1: Subject\n\
            \x20\x20  |\n\
            \x20\x201 | Subject\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_violation_subject_hint() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::subject_hint(
            "Subject".to_string(),
            Range { start: 1, end: 3 },
            "The hint".to_string(),
        )];
        let violation = Violation {
            rule: Rule::SubjectMood,
            message: "The error message".to_string(),
            position: Position::Subject { column: 2 },
            context,
        };
        let output = commit_violation(&commit, &violation);
        assert_eq!(
            output,
            "SubjectMood: The error message\n\
            \x20\x201234567:1:2: Subject\n\
            \x20\x20  |\n\
            \x20\x201 | Subject\n\
            \x20\x20  |  ^^ The hint\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_violation_message_line() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::message_line(9, "Message line".to_string())];
        let violation = Violation {
            rule: Rule::MessageLineLength,
            message: "The error message".to_string(),
            position: Position::MessageLine {
                line: 10,
                column: 50,
            },
            context,
        };
        let output = commit_violation(&commit, &violation);
        assert_eq!(
            output,
            "MessageLineLength: The error message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20   |\n\
            \x20\x2011 | Message line\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_violation_message_line_hint() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![
            Context::message_line(9, "Message line".to_string()),
            Context::message_line_hint(
                10,
                "Message line with hint".to_string(),
                Range { start: 3, end: 10 },
                "My hint".to_string(),
            ),
        ];
        let violation = Violation {
            rule: Rule::MessageLineLength,
            message: "The error message".to_string(),
            position: Position::MessageLine {
                line: 10,
                column: 50,
            },
            context,
        };
        let output = commit_violation(&commit, &violation);
        assert_eq!(
            output,
            "MessageLineLength: The error message\n\
            \x20\x201234567:11:50: Subject\n\
            \x20\x20   |\n\
            \x20\x2011 | Message line\n\
            \x20\x2012 | Message line with hint\n\
            \x20\x20   |    ^^^^^^^ My hint\n\n"
        );
    }

    #[test]
    fn test_formatted_commit_violation_diff_hint() {
        let commit = commit(Some("1234567".to_string()), "Subject", "Message");
        let context = vec![Context::diff_hint(
            "Diff line".to_string(),
            Range { start: 3, end: 5 },
            "My hint".to_string(),
        )];
        let violation = Violation {
            rule: Rule::DiffPresence,
            message: "The error message".to_string(),
            position: Position::Diff,
            context,
        };
        let output = commit_violation(&commit, &violation);
        assert_eq!(
            output,
            "DiffPresence: The error message\n\
            \x20\x201234567: Subject\n\
            \x20\x20|\n\
            \x20\x20| Diff line\n\
            \x20\x20|    ^^ My hint\n\n"
        );
    }

    #[test]
    fn test_formatted_branch_violation_branch_hint() {
        let branch = Branch::new("branch-name".to_string());
        let context = vec![Context::branch_hint(
            "branch-name".to_string(),
            Range { start: 3, end: 5 },
            "My hint".to_string(),
        )];
        let violation = Violation {
            rule: Rule::BranchNameLength,
            message: "The error message".to_string(),
            position: Position::Branch { column: 3 },
            context,
        };
        let output = branch_violation(&branch, &violation);
        assert_eq!(
            output,
            "BranchNameLength: The error message\n\
            \x20\x20Branch:3: branch-name\n\
            \x20\x20|\n\
            \x20\x20| branch-name\n\
            \x20\x20|    ^^ My hint\n\n"
        );
    }

    #[test]
    fn test_formatted_branch_violation_branch_hint_with_color() {
        let branch = Branch::new("branch-name".to_string());
        let context = vec![Context::branch_hint(
            "branch-name".to_string(),
            Range { start: 3, end: 5 },
            "My hint".to_string(),
        )];
        let violation = Violation {
            rule: Rule::BranchNameLength,
            message: "The error message".to_string(),
            position: Position::Branch { column: 3 },
            context,
        };
        let output = branch_violation_color(&branch, &violation);
        assert_eq!(
            output,
            "\u{1b}[0m\u{1b}[31mBranchNameLength\u{1b}[0m: The error message\n\
            \u{1b}[0m\u{1b}[38;5;12m  Branch:3:\u{1b}[0m branch-name\n\
            \u{1b}[0m\u{1b}[38;5;12m  |\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  |\u{1b}[0m branch-name\n\
            \u{1b}[0m\u{1b}[38;5;12m  |\u{1b}[0m\u{1b}[38;5;9m    ^^ My hint\u{1b}[0m\n\n"
        );
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
            formatted_context(&violation),
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
            formatted_context(&violation),
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
            formatted_context(&violation),
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
            formatted_context(&violation),
            "|\n\
             | Some diff\n\
             |  ^^ A message\n"
        );
    }

    #[test]
    fn formatted_context_ascii() {
        let v_start = subject_violation_hint("Lorem ipsum", "A lorem", Range { start: 0, end: 5 });
        assert_eq!(
            formatted_context(&v_start),
            "\x20\x20|\n\
                   1 | Lorem ipsum\n\
             \x20\x20| ^^^^^ A lorem\n"
        );

        let v_end = subject_violation_hint("Lorem ipsum", "A sum", Range { start: 8, end: 11 });
        assert_eq!(
            formatted_context(&v_end),
            "\x20\x20|\n\
                   1 | Lorem ipsum\n\
             \x20\x20|         ^^^ A sum\n"
        );

        let v_middle = subject_violation_hint("Lorem ipsum", "A space", Range { start: 5, end: 6 });
        assert_eq!(
            formatted_context(&v_middle),
            "\x20\x20|\n\
                   1 | Lorem ipsum\n\
             \x20\x20|      ^ A space\n"
        );
    }

    #[test]
    fn formatted_context_whitespace() {
        let v_space = subject_violation_hint(" Lorem ipsum", "A space", Range { start: 0, end: 1 });
        assert_eq!(
            formatted_context(&v_space),
            "\x20\x20|\n\
                   1 |  Lorem ipsum\n\
             \x20\x20| ^ A space\n"
        );

        let v_space =
            subject_violation_hint("\x20Lorem ipsum", "A space", Range { start: 0, end: 1 });
        assert_eq!(
            formatted_context(&v_space),
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
            formatted_context(&v_tab),
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
            formatted_context(&v),
            "\x20\x20|\n\
                   1 | This is a̐ char with an accent\n\
             \x20\x20|         ^ Mark accent\n"
        );
    }

    #[test]
    fn formatted_context_emoji() {
        let v = subject_violation_hint("Aa😀Bb", "Mark emoji", Range { start: 2, end: 4 });
        assert_eq!(
            formatted_context(&v),
            "\x20\x20|\n\
                   1 | Aa😀Bb\n\
             \x20\x20|   ^^ Mark emoji\n"
        );

        let v = subject_violation_hint("Aa👍Bb", "Mark emoji", Range { start: 2, end: 4 });
        assert_eq!(
            formatted_context(&v),
            "\x20\x20|\n\
                   1 | Aa👍Bb\n\
             \x20\x20|   ^^ Mark emoji\n"
        );

        let v = subject_violation_hint(
            "Fix ❤️ in controller Fix #123",
            "Mark fix ticket",
            Range { start: 25, end: 33 },
        );
        assert_eq!(
            formatted_context(&v),
            "\x20\x20|\n\
                   1 | Fix ❤️ in controller Fix #123\n\
             \x20\x20|                     ^^^^^^^^ Mark fix ticket\n"
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
            formatted_context(&v),
            "\x20\x20|\n\
                   1 | ああああああああああああああああああああああああああ\n\
             \x20\x20|                                                   ^^ Mark double width character\n"
        );
    }
}
