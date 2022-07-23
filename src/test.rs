use crate::commit::Commit;
use crate::formatter::formatted_context as formatted_context_real;
use crate::issue::{Issue, Position};
use crate::rule::Rule;
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

pub fn commit_without_file_changes(message: String) -> Commit {
    Commit::new(
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        Some("test@example.com".to_string()),
        "Some subject",
        message,
        false,
    )
}

pub fn validated_commit<S: AsRef<str>>(subject: S, message: S) -> Commit {
    let mut commit = commit(subject, message);
    commit.validate();
    commit
}

pub fn assert_commit_valid_for(commit: &Commit, rule: &Rule) {
    assert!(
        !has_issue(&commit.issues, &rule),
        "Commit was not considered valid: {:?}",
        commit
    );
}

pub fn assert_commit_invalid_for(commit: &Commit, rule: &Rule) {
    assert!(
        has_issue(&commit.issues, &rule),
        "Commit was not considered invalid: {:?}",
        commit
    );
}

pub fn assert_commit_subject_as_valid(subject: &str, rule: &Rule) {
    let commit = validated_commit(subject.to_string(), "".to_string());
    assert_commit_valid_for(&commit, rule);
}

pub fn assert_commit_subjects_as_valid(subjects: Vec<&str>, rule: &Rule) {
    for subject in subjects {
        assert_commit_subject_as_valid(subject, rule)
    }
}

pub fn assert_commit_subject_as_invalid<S: AsRef<str>>(subject: S, rule: &Rule) {
    let commit = validated_commit(subject.as_ref().to_string(), "".to_string());
    assert_commit_invalid_for(&commit, rule);
}

pub fn assert_commit_subjects_as_invalid<S: AsRef<str>>(subjects: Vec<S>, rule: &Rule) {
    for subject in subjects {
        assert_commit_subject_as_invalid(subject, rule)
    }
}

pub fn has_issue(issues: &[Issue], rule: &Rule) -> bool {
    issues.iter().any(|v| &v.rule == rule)
}

pub fn first_issue(issues_option: Option<Vec<Issue>>) -> Issue {
    let issues = issues_option.expect("No issues found");
    assert_eq!(issues.len(), 1);
    issues.into_iter().next().expect("No issue found")
}

pub fn find_issue(issues: Vec<Issue>, rule: &Rule) -> Issue {
    let mut issues = issues.into_iter().filter(|v| &v.rule == rule);
    let issue = match issues.next() {
        Some(issue) => issue,
        None => panic!("No issue of the {} rule found", rule),
    };
    if issues.next().is_some() {
        panic!("More than one issue of the {} rule found", rule)
    }
    issue
}

pub fn subject_position(column: usize) -> Position {
    Position::Subject { line: 1, column }
}

pub fn message_position(line: usize, column: usize) -> Position {
    Position::MessageLine { line, column }
}
