use regex::Regex;
use std::process::Command;

use crate::commit::{Commit, SUBJECT_WITH_MERGE_REMOTE_BRANCH};

const SCISSORS: &str = "------------------------ >8 ------------------------";
const COMMIT_DELIMITER: &str = "------------------------ COMMIT >! ------------------------";

lazy_static! {
    static ref SUBJECT_WITH_SQUASH_PR: Regex = Regex::new(r".+ \(#\d+\)$").unwrap();
}

#[derive(Debug, PartialEq)]
pub enum CleanupMode {
    Strip,
    Whitespace,
    Verbatim,
    Scissors,
    Default,
}

pub fn fetch_and_parse_commits(selector: Option<String>) -> Result<Vec<Commit>, String> {
    let mut commits = Vec::<Commit>::new();
    let mut command = Command::new("git");

    // Format definition per commit
    // Line 1: Commit SHA in long form
    // Line 2 to second to last: Commit subject and message
    // Line last: Delimiter to tell commits apart
    let format = "%H%n%B";
    let mut args = vec![
        "log".to_string(),
        format!("--pretty={}{}", format, COMMIT_DELIMITER),
    ];
    match selector {
        Some(selection) => {
            let selection = selection.trim().to_string();
            if !selection.contains("..") {
                // Only select one commit if no commit range was selected
                args.push("-n 1".to_string());
            }
            args.push(selection);
        }
        None => args.push("HEAD~1..HEAD".to_string()),
    };

    command.args(&args);
    match command.output() {
        Ok(raw_output) => {
            let output = String::from_utf8_lossy(&raw_output.stdout);
            let messages = output.split(COMMIT_DELIMITER);
            for message in messages {
                let trimmed_message = message.trim();
                if !trimmed_message.is_empty() {
                    match parse_commit(trimmed_message) {
                        Some(commit) => commits.push(commit),
                        None => debug!("Commit ignored: {:?}", message),
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!(
                "Unable to fetch commits from Git: {:?}\n{}",
                args, e
            ))
        }
    };
    Ok(commits)
}

fn parse_commit(message: &str) -> Option<Commit> {
    let mut long_sha = None;
    let mut subject = None;
    let mut message_lines = vec![];
    for (index, line) in message.lines().enumerate() {
        match index {
            0 => long_sha = Some(line),
            1 => subject = Some(line),
            _ => message_lines.push(line),
        }
    }
    match (long_sha, subject) {
        (Some(long_sha), subject) => {
            let used_subject = subject.unwrap_or_else(|| {
                debug!("Commit subject not present in message: {:?}", message);
                ""
            });
            commit_or_none(
                Some(long_sha.to_string()),
                used_subject.to_string(),
                message_lines,
            )
        }
        _ => {
            debug!("Commit ignored: SHA was not present: {}", message);
            None
        }
    }
}

pub fn parse_commit_hook_format(
    message: &str,
    cleanup_mode: CleanupMode,
    comment_char: String,
) -> Option<Commit> {
    let mut subject = None;
    let mut message_lines = Vec::<&str>::new();
    let scissor_line = format!("{} {}", comment_char, SCISSORS);
    debug!("Using clean up mode: {:?}", cleanup_mode);
    debug!("Using config core.commentChar: {:?}", comment_char);
    for (index, mut line) in message.lines().enumerate() {
        match index {
            0 => subject = Some(line),
            _ => {
                match cleanup_mode {
                    CleanupMode::Scissors => {
                        if line == scissor_line {
                            debug!("Found scissors line. Stop parsing message.");
                            break;
                        }
                    }
                    CleanupMode::Default | CleanupMode::Strip => {
                        line = line.trim_end();
                        if line.starts_with(&comment_char) {
                            continue;
                        }
                    }
                    CleanupMode::Verbatim => {}
                    CleanupMode::Whitespace => {
                        line = line.trim_end();
                    }
                }
                message_lines.push(line)
            }
        }
    }
    let used_subject = subject.unwrap_or_else(|| {
        debug!("Commit subject not present in message: {:?}", message);
        ""
    });
    commit_or_none(None, used_subject.to_string(), message_lines)
}

fn commit_or_none(sha: Option<String>, subject: String, message: Vec<&str>) -> Option<Commit> {
    let mut commit = Commit::new(sha, subject, message.join("\n"));
    if !ignored(&commit) {
        commit.validate();
        Some(commit)
    } else {
        None
    }
}

fn ignored(commit: &Commit) -> bool {
    let subject = &commit.subject;
    if subject.starts_with("Merge tag ") {
        debug!(
            "Ignoring commit because it's a merge commit of a tag: {}",
            subject
        );
        return true;
    }
    if subject.starts_with("Merge pull request") {
        debug!(
            "Ignoring commit because it's a 'merge pull request' commit: {}",
            subject
        );
        return true;
    }
    if subject.starts_with("Merge branch") && commit.message.contains("See merge request !") {
        debug!(
            "Ignoring commit because it's a 'merge request' commit: {}",
            subject
        );
        return true;
    }
    if SUBJECT_WITH_SQUASH_PR.is_match(subject) {
        // Subject ends with a GitHub squash PR marker: ` (#123)`
        debug!(
            "Ignoring commit because it's a 'merge pull request' squash commit: {}",
            subject
        );
        return true;
    }
    if subject.starts_with("Merge branch ") && !SUBJECT_WITH_MERGE_REMOTE_BRANCH.is_match(subject) {
        debug!(
            "Ignoring commit because it's a local merge commit: {}",
            subject
        );
        return true;
    }

    false
}

pub fn cleanup_mode() -> CleanupMode {
    let mut command = Command::new("git");
    command.args(&["config", "commit.cleanup"]);
    match command.output() {
        Ok(raw_output) => match String::from_utf8_lossy(&raw_output.stdout).trim() {
            "default" => CleanupMode::Default,
            "scissors" => CleanupMode::Scissors,
            "strip" => CleanupMode::Strip,
            "verbatim" => CleanupMode::Verbatim,
            "whitespace" => CleanupMode::Whitespace,
            "" => CleanupMode::Default,
            option => {
                info!(
                    "Unsupported commit.cleanup config: {}\nFalling back on 'default'.",
                    option
                );
                CleanupMode::Default
            }
        },
        Err(e) => {
            error!("Unable to determine Git's commit.cleanup config: {}", e);
            CleanupMode::Default
        }
    }
}

pub fn comment_char() -> String {
    let mut command = Command::new("git");
    command.args(&["config", "core.commentChar"]);
    match command.output() {
        Ok(raw_output) => {
            let character = String::from_utf8_lossy(&raw_output.stdout)
                .trim()
                .to_string();
            if character.is_empty() {
                debug!("No Git core.commentChar config found. Using default `#` character.");
                "#".to_string()
            } else {
                character
            }
        }
        Err(e) => {
            error!("Unable to determine Git's core.commentChar config: {}", e);
            "#".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_commit, parse_commit_hook_format, CleanupMode};

    #[test]
    fn test_parse_commit() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        This is a subject\n\
        \n\
        This is my multi line message.\n\
        Line 2.",
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "\nThis is my multi line message.\nLine 2.");
        assert!(commit.violations.is_empty());
    }

    #[test]
    fn test_parse_commit_with_errors() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        This is a subject",
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "");
        assert!(!commit.violations.is_empty());
    }

    #[test]
    fn test_parse_commit_empty() {
        let result = parse_commit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n");

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.subject, "");
        assert_eq!(commit.message, "");
        assert!(!commit.violations.is_empty());
    }

    #[test]
    fn test_parse_commit_ignore_tag_merge_commit() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        Merge tag 'v1.2.3' into main",
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commit_ignore_merge_commit_pull_request() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        Merge pull request #123 from tombruijn/repo\n\
        \n\
        This is my multi line message.\n\
        Line 2.",
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commit_ignore_squash_merge_commit_pull_request() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        Fix some issue that's squashed (#123)\n\
        \n\
        This is my multi line message.\n\
        Line 2.",
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commit_ignore_merge_commits_merge_request() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        Merge branch 'branch' into 'main'\n\
        \n\
        This is my multi line message.\n\
        Line 2.

        See merge request !123",
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commit_ignore_merge_commits_without_into() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        Merge branch 'branch'",
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_parse_commit_merge_remote_commits() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        Merge branch 'branch' of github.com/org/repo into branch",
        );

        assert!(result.is_some());
    }

    #[test]
    fn test_parse_commit_hook_format() {
        let result = parse_commit_hook_format(
            "This is a subject\n\nThis is a message.",
            CleanupMode::Default,
            "#".to_string(),
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "\nThis is a message.");
    }

    #[test]
    fn test_parse_commit_hook_format_without_message() {
        let result =
            parse_commit_hook_format("This is a subject", CleanupMode::Default, "#".to_string());

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "");
    }

    // Same as Default
    #[test]
    fn test_parse_commit_hook_format_with_strip() {
        let result = parse_commit_hook_format(
            "This is a subject\n\
            \n\
            This is the message body.  \n\
            # This is a commented line.\n\
            \n\
            Another line.\n\
            \n\
            # Other things that are not part of the message.\n\
            ",
            CleanupMode::Strip,
            "#".to_string(),
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(
            commit.message,
            "\nThis is the message body.\n\nAnother line.\n"
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_strip_custom_comment_char() {
        let result = parse_commit_hook_format(
            "This is a subject\n\
            \n\
            This is the message body.  \n\
            - This is a commented line.\n\
            \n\
            Another line.\n\
            \n\
            - Other things that are not part of the message.\n\
            ",
            CleanupMode::Strip,
            "-".to_string(),
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(
            commit.message,
            "\nThis is the message body.\n\nAnother line.\n"
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_scissors() {
        let result = parse_commit_hook_format(
            "This is a subject\n\
            \n\
            This is the message body.\n\
            \n\
            This is line 2.\n\
            # ------------------------ >8 ------------------------\n\
            Other things that are not part of the message.\n\
            ",
            CleanupMode::Scissors,
            "#".to_string(),
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(
            commit.message,
            "\nThis is the message body.\n\nThis is line 2."
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_verbatim() {
        let result = parse_commit_hook_format(
            "This is a subject\n\
            \n\
            This is the message body.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            ",
            CleanupMode::Verbatim,
            "#".to_string(),
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(
            commit.message,
            "\nThis is the message body.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            "
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_whitespace() {
        let result = parse_commit_hook_format(
            "This is a subject\n\
            \n\
            This is the message body.  \n\
            \n\
            This is line 2.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            ",
            CleanupMode::Whitespace,
            "#".to_string(),
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, None);
        assert_eq!(commit.short_sha, None);
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(
            commit.message,
            "\nThis is the message body.\n\
            \n\
            This is line 2.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            "
        );
    }
}
