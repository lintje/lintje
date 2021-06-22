use std::process::Command;

use crate::commit::Commit;

pub fn fetch_and_parse_commits(
    revision_range: Option<String>,
) -> Result<Vec<Commit>, std::io::Error> {
    let mut commits = Vec::<Commit>::new();
    let mut command = Command::new("git");
    let range = match revision_range {
        Some(s) => s,
        None => "HEAD~1..HEAD".to_string(),
    };
    let delimiter = "------------------------ >8 ------------------------";

    // Format definition per commit
    // Line 1: Commit SHA in long form
    // Line 2: Commit SHA in abbreviated form
    // Line 3 to second to last: Commit subject and message
    // Line last: Delimiter to tell commits apart
    let format = "%H%n%h%n%B";

    command.args(&["log", &format!("--pretty={}{}", format, delimiter), &range]);
    match command.output() {
        Ok(raw_output) => {
            let output = format!("{}", String::from_utf8_lossy(&raw_output.stdout));
            let messages = output.split(delimiter);
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
            error!("{}", e);
            std::process::exit(1);
        }
    };
    Ok(commits)
}

fn parse_commit(message: &str) -> Option<Commit> {
    let mut long_sha = None;
    let mut short_sha = None;
    let mut subject = None;
    let mut message_lines = Vec::<&str>::new();
    for (index, line) in message.lines().enumerate() {
        match index {
            0 => long_sha = Some(line),
            1 => short_sha = Some(line),
            2 => subject = Some(line),
            _ => message_lines.push(line),
        }
    }
    match (long_sha, short_sha, subject) {
        (Some(long_sha), Some(short_sha), Some(subject)) => {
            let mut commit = Commit::new(
                long_sha.to_string(),
                short_sha.to_string(),
                subject.to_string(),
                message_lines.join("\n"),
            );
            if !ignored(&commit) {
                commit.validate();
                Some(commit)
            } else {
                debug!("Commit is ignored: {:?}", commit);
                None
            }
        }
        _ => {
            debug!("Commit SHA or subject not present: {}", message);
            None
        }
    }
}

fn ignored(commit: &Commit) -> bool {
    if commit.subject.starts_with("Merge pull request") {
        return true;
    }
    if commit.subject.starts_with("Merge branch") && commit.message.contains("See merge request !")
    {
        return true;
    }

    return false;
}

#[cfg(test)]
mod tests {
    use super::parse_commit;

    #[test]
    fn test_parse_commit() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        aaaaaaa
        This is a subject
        \n\
        This is my multi line message.\n\
        Line 2.",
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(commit.short_sha, "aaaaaaa");
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "This is my multi line message.\nLine 2.");
        assert!(commit.validations.is_empty());
    }

    #[test]
    fn test_parse_commit_with_errors() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        aaaaaaa
        This is a subject",
        );

        assert!(result.is_some());
        let commit = result.unwrap();
        assert_eq!(commit.long_sha, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(commit.short_sha, "aaaaaaa");
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "");
        assert!(!commit.validations.is_empty());
    }

    #[test]
    fn test_parse_commit_ignore_merge_commit_pull_request() {
        let result = parse_commit(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        aaaaaaa
        Merge pull request #123 from tombruijn/repo\n\
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
        aaaaaaa
         Merge branch 'branch' into 'main' \n\
        \n\
        This is my multi line message.\n\
        Line 2.

        See merge request !123",
        );

        assert!(result.is_none());
    }
}
