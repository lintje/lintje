use crate::branch::Branch;
use crate::commit::Commit;
use crate::formatter::formatted_context as formatted_context_real;
use crate::issue::{Issue, Position};
use std::fs;
use std::io::Write;
use std::path::Path;
use termcolor::{BufferWriter, ColorChoice};

pub const TEST_DIR: &str = "tmp/tests/test_repo";

pub fn prepare_test_dir(dir: &Path) {
    if Path::new(&dir).exists() {
        fs::remove_dir_all(&dir).expect("Could not remove test repo dir");
    }
    fs::create_dir_all(&dir).expect("Could not create test repo dir");
}

pub fn create_file(file_path: &Path, content: &[u8]) -> fs::File {
    let mut file = match fs::File::create(&file_path) {
        Ok(file) => file,
        Err(e) => panic!("Could not create file: {:?}: {}", file_path, e),
    };
    // Write a slice of bytes to the file
    match file.write_all(content) {
        Ok(_) => (),
        Err(e) => panic!("Could not write to file: {:?}: {}", file_path, e),
    }
    file
}

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
        "".to_string(), // Trailers, commonly empty
        vec!["src/main.rs".to_string()],
    )
}

pub fn commit<S: AsRef<str>>(subject: S, message: S) -> Commit {
    commit_with_sha(
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        subject,
        message,
    )
}

pub fn commit_with_trailers<S: AsRef<str>>(subject: S, message: S, trailers: S) -> Commit {
    Commit::new(
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        Some("test@example.com".to_string()),
        subject.as_ref(),
        message.as_ref().to_string(),
        trailers.as_ref().to_string(),
        vec!["src/main.rs".to_string()],
    )
}

pub fn branch(name: &str) -> Branch {
    Branch::new(name.to_string())
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

pub fn assert_contains_issue_output(issue: &Issue, expected_format: &str) {
    let formatted_message = formatted_context(&issue);
    let mut actual_lines = formatted_message.lines();
    actual_lines.next(); // Skip first line which is always empty
    let expected_lines = expected_format.lines();
    let mut asserted_line_number = 0;
    for expected_line in expected_lines {
        asserted_line_number = asserted_line_number + 1;
        let actual_line = actual_lines.next().expect("No new line expected");
        assert!(
            actual_line.contains(expected_line),
            "Lines #{} don't match.\nActual:\n{}\nExpected to contain (indenting may not match):\n{}",
            asserted_line_number,
            formatted_message,
            expected_format
        );
    }
}
