use crate::commit::Commit;
use crate::formatter::formatted_context as formatted_context_real;
use crate::issue::{Issue, Position};
use termcolor::{BufferWriter, ColorChoice};

pub fn formatted_context(issue: &Issue) -> String {
    let bufwtr = BufferWriter::stdout(ColorChoice::Never);
    let mut out = bufwtr.buffer();
    match formatted_context_real(&mut out, issue) {
        Ok(()) => {
            // Strip off the two leading spaces per line if any
            // The indenting is tested somewhere else
            String::from_utf8_lossy(out.as_slice())
                .to_string()
                .lines()
                .into_iter()
                .map(|v| v.strip_prefix("  ").unwrap_or(v))
                .collect::<Vec<&str>>()
                .join("\n")
        }
        Err(e) => panic!("Unable to format context issue: {:?}", e),
    }
}

pub fn commit_with_sha<S: AsRef<str>>(sha: Option<String>, subject: S, message: S) -> Commit {
    Commit::new(
        sha,
        Some("test@example.com".to_string()),
        subject.as_ref(),
        message.as_ref().to_string(),
        true,
    )
}

pub fn commit<S: AsRef<str>>(subject: S, message: S) -> Commit {
    commit_with_sha(
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        subject,
        message,
    )
}

pub fn first_issue(issues_option: Option<Vec<Issue>>) -> Issue {
    let issues = issues_option.expect("No issues found");
    assert_eq!(issues.len(), 1);
    issues.into_iter().next().expect("No issue found")
}

pub fn subject_position(column: usize) -> Position {
    Position::Subject { line: 1, column }
}

pub fn message_position(line: usize, column: usize) -> Position {
    Position::MessageLine { line, column }
}
