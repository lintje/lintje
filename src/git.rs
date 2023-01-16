pub mod hooks;

use regex::Regex;

use crate::branch::Branch;
use crate::command::{run_command, run_command_with_stdin};
use crate::commit::Commit;

const SCISSORS: &str = "------------------------ >8 ------------------------";
const COMMIT_DELIMITER: &str = "------------------------ COMMIT >! ------------------------";
const COMMIT_BODY_DELIMITER: &str = "------------------------ BODY >! ------------------------";
const COMMIT_TRAILERS_DELIMITER: &str =
    "------------------------ TRAILERS >! ------------------------";

lazy_static! {
    pub static ref SUBJECT_WITH_MERGE_REMOTE_BRANCH: Regex =
        Regex::new(r"^Merge branch '.+' of .+ into .+").unwrap();
    static ref SUBJECT_WITH_SQUASH_PR: Regex = Regex::new(r".+ \(#\d+\)$").unwrap();
    static ref MESSAGE_CONTAINS_MERGE_REQUEST_REFERENCE: Regex =
        Regex::new(r"^See merge request .+/.+!\d+$").unwrap();
    static ref SUBJECT_WITH_MERGE_ONLY: Regex =
        Regex::new(r"Merge [a-z0-9]{40} into [a-z0-9]{40}").unwrap();
}

#[derive(Debug, PartialEq)]
enum CleanupMode {
    Strip,
    Whitespace,
    Verbatim,
    Scissors,
    Default,
}

pub fn fetch_and_parse_branch() -> Result<Branch, String> {
    let output = match run_command("git", &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(o) => o,
        Err(e) => {
            debug!("Failed to fetch Git branch: {:?}", e);
            return Err(e.message());
        }
    };
    let name = output.trim().to_string();
    let mut branch = Branch::new(name);
    branch.validate();
    Ok(branch)
}

pub fn fetch_and_parse_commits(selector: &Option<String>) -> Result<Vec<Commit>, String> {
    let mut commits = Vec::<Commit>::new();
    // Format definition per commit:
    // COMMIT_DELIMITER: Tell commits apart, split the string on this later
    // Format:
    //   - Line 1: Commit SHA in long form
    //   - Line 2: Commit author email address
    //   - Line 3 to end of body: Commit subject and message, including trailers
    // COMMIT_TRAILERS_DELIMITER: Separator for commit message body and trailers
    // %(trailers): Trailers, based on https://git-scm.com/docs/git-interpret-trailers/
    // COMMIT_BODY_DELIMITER: Separator for end of the message body and trailers
    // `--name-only`: Prints filenames of files changed
    let commit_format = "%H%n%ae%n%B";
    let mut args = vec![
        "log".to_string(),
        format!(
            "--pretty=\
             {COMMIT_DELIMITER}%n\
             {commit_format}%n\
             {COMMIT_TRAILERS_DELIMITER}%n\
             %(trailers)%n\
             {COMMIT_BODY_DELIMITER}"
        ),
        "--name-only".to_string(),
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
        None => {
            args.push("-n 1".to_string());
            args.push("HEAD".to_string());
        }
    };

    let output = match run_command("git", &args) {
        Ok(o) => o,
        Err(e) => {
            debug!("Failed to fetch Git log: {:?}", e);
            return Err(e.message());
        }
    };
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
    Ok(commits)
}

fn parse_commit(message: &str) -> Option<Commit> {
    let mut long_sha = None;
    let mut email = None;
    let mut subject = None;
    let mut message_lines = vec![];
    let mut message_parts = message.split(COMMIT_TRAILERS_DELIMITER);
    match message_parts.next() {
        Some(body) => {
            for (index, line) in body.lines().enumerate() {
                match index {
                    0 => long_sha = Some(line),
                    1 => email = Some(line.to_string()),
                    2 => subject = Some(line),
                    _ => message_lines.push(line.to_string()),
                }
            }
        }
        None => error!("No commit body found!"),
    }

    let mut extras_str = message_parts
        .next()
        .unwrap_or("")
        .split(COMMIT_BODY_DELIMITER);
    let trailers = extras_str.next().unwrap_or("").trim().to_string();
    let file_changes_str = extras_str.next().unwrap_or("").trim();
    let file_changes = file_changes_str
        .lines()
        .map(std::string::ToString::to_string)
        .collect::<Vec<String>>();
    if file_changes.is_empty() {
        debug!("No stats found for commit '{}'", long_sha.unwrap_or(""));
    } else {
        debug!(
            "Stats line found for commit '{}': {}",
            long_sha.unwrap_or(""),
            file_changes.join(", ")
        );
    }

    // Trailers are included twice, once for the body and once for the trailers. Replace the
    // trailers in the body, we already have them in the trailers string.
    let message_body_str = message_lines.join("\n");
    let message_body = strip_trailers_from_message(&message_body_str, &trailers);

    match (long_sha, subject) {
        (Some(long_sha), subject) => {
            let used_subject = subject.unwrap_or_else(|| {
                debug!("Commit subject not present in message: {:?}", message);
                ""
            });
            Some(Commit::new(
                Some(long_sha.to_string()),
                email,
                used_subject,
                message_body,
                trailers,
                file_changes,
            ))
        }
        _ => {
            debug!("Commit ignored: SHA was not present: {}", message);
            None
        }
    }
}

pub fn parse_commit_file(contents: &str) -> Commit {
    let (subject, message) = parse_commit_hook_format(contents, &cleanup_mode(), &comment_char());
    let trailers = parse_trailers_from_message([subject.to_owned(), message.to_owned()].join("\n"));
    let message = strip_trailers_from_message(&message, &trailers);
    // Run the diff command to fetch the current staged changes and determine if the commit is
    // empty or not. The contents of the commit message file is too unreliable as it depends on
    // user config and how the user called the `git commit` command.
    let file_changes = current_file_changes();
    Commit::new(None, None, &subject, message, trailers, file_changes)
}

fn parse_commit_hook_format(
    file_contents: &str,
    cleanup_mode: &CleanupMode,
    comment_char: &str,
) -> (String, String) {
    let mut subject = None;
    let mut message_lines = vec![];
    let scissor_line = format!("{} {}", comment_char, SCISSORS);
    debug!("Using clean up mode: {:?}", cleanup_mode);
    debug!("Using config core.commentChar: {:?}", comment_char);
    for line in file_contents.lines() {
        // A scissor line has been detected.
        //
        // A couple reasons why this could happen:
        //
        // - A scissor line was found in cleanup mode "scissors". All content after this line is
        //   ignored.
        // - A scissor line was found in a different cleanup mode with the `--verbose` option.
        //   Lintje cannot detect this verbose mode so it assumes it's for the verbose mode
        //   and ignores all content after this line.
        // - The commit message is entirely empty, leaving only the comments added to the file by
        //   Git. Unless `--allow-empty-message` is specified this is the user telling Git it stop
        //   the commit process.
        if line == scissor_line {
            debug!("Found scissors line. Stop parsing message.");
            break;
        }

        // The first non-empty line is the subject line in every cleanup mode but the Verbatim
        // mode.
        if subject.is_none() {
            if cleanup_mode == &CleanupMode::Verbatim {
                // Set subject, doesn't matter what the content is. Even empty lines are considered
                // subjects in Verbatim cleanup mode.
                subject = Some(line.to_string());
            } else if let Some(cleaned_line) = cleanup_line(line, cleanup_mode, comment_char) {
                if !cleaned_line.is_empty() {
                    // Skip leading empty lines in every other cleanup mode than Verbatim.
                    subject = Some(cleaned_line);
                }
            }
            // Skips this line if the cleanup mode is Strip and the line is a comment.
            // See when the `cleanup_line` function returns `None`.
            continue;
        }

        if let Some(cleaned_line) = cleanup_line(line, cleanup_mode, comment_char) {
            message_lines.push(cleaned_line);
        }
    }
    let used_subject = subject.unwrap_or_else(|| {
        debug!("Commit subject not present in message: {:?}", file_contents);
        "".to_string()
    });

    (used_subject, message_lines.join("\n"))
}

fn parse_trailers_from_message(message: String) -> String {
    match run_command_with_stdin("git", &["interpret-trailers", "--only-trailers"], message) {
        Ok(stdout) => stdout.trim().to_string(),
        Err(e) => {
            error!(
                "Unable to determine commit message trailers.\nError: {:?}",
                e
            );
            "".to_string()
        }
    }
}

fn cleanup_line(line: &str, cleanup_mode: &CleanupMode, comment_char: &str) -> Option<String> {
    match cleanup_mode {
        CleanupMode::Default | CleanupMode::Strip => {
            if line.starts_with(comment_char) {
                return None;
            }
            Some(line.trim_end().to_string())
        }
        CleanupMode::Whitespace | CleanupMode::Scissors => Some(line.trim_end().to_string()),
        CleanupMode::Verbatim => Some(line.to_string()),
    }
}

pub fn is_commit_ignored(commit: &Commit) -> bool {
    let subject = &commit.subject;
    let message = &commit.message;
    if let Some(email) = &commit.email {
        if email.ends_with("[bot]@users.noreply.github.com") {
            debug!(
                "Ignoring commit because it is from a bot account: {}",
                email
            );
            return true;
        }
    }
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
    if subject.starts_with("Merge branch ")
        && MESSAGE_CONTAINS_MERGE_REQUEST_REFERENCE.is_match(message)
    {
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
    if subject.starts_with("Revert \"")
        && subject.ends_with('"')
        && message.contains("This reverts commit ")
    {
        debug!("Ignoring commit because it's a revert commit: {}", subject);
        return true;
    }
    if SUBJECT_WITH_MERGE_ONLY.is_match(subject) {
        debug!(
            "Ignoring commit because it's a merge into commit: {}",
            subject
        );
        return true;
    }

    false
}

fn cleanup_mode() -> CleanupMode {
    match run_command("git", &["config", "commit.cleanup"]) {
        Ok(stdout) => match stdout.trim() {
            "default" | "" => CleanupMode::Default,
            "scissors" => CleanupMode::Scissors,
            "strip" => CleanupMode::Strip,
            "verbatim" => CleanupMode::Verbatim,
            "whitespace" => CleanupMode::Whitespace,
            option => {
                info!(
                    "Unsupported Git commit.cleanup config: {}\nFalling back on 'default'.",
                    option
                );
                CleanupMode::Default
            }
        },
        Err(e) => {
            let message = format!(
                "Unable to determine Git's commit.cleanup config. \
                Falling back on default commit.cleanup config.\nError: {:?}",
                e
            );
            if e.error.is_exit_code(1) {
                // Git returns exit code 1 if the config option is not set
                // So no need to error when that happens
                debug!("{}", message);
            } else {
                // Other error that we do not expect so print the error
                error!("{}", message);
            }
            CleanupMode::Default
        }
    }
}

fn comment_char() -> String {
    match run_command("git", &["config", "core.commentChar"]) {
        Ok(stdout) => {
            let character = stdout.trim().to_string();
            if character.is_empty() {
                debug!("No Git core.commentChar config found. Using default `#` character.");
                "#".to_string()
            } else {
                character
            }
        }
        Err(e) => {
            let message = format!(
                "Unable to determine Git's core.commentChar config. \
                Falling back on default core.commentChar: `#`\nError: {:?}",
                e
            );
            if e.error.is_exit_code(1) {
                // Git returns exit code 1 if the config option is not set
                // So no need to error when that happens
                debug!("{}", message);
            } else {
                // Other error that we do not expect so print the error
                error!("{}", message);
            }
            "#".to_string()
        }
    }
}

fn current_file_changes() -> Vec<String> {
    match run_command("git", &["diff", "--cached", "--name-only"]) {
        Ok(stdout) => stdout
            .trim()
            .lines()
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>(),
        Err(e) => {
            error!("Unable to determine commit changes.\nError: {:?}", e);
            vec![]
        }
    }
}

pub fn repo_has_changesets() -> bool {
    // Find all changesets directories in the repo
    match run_command(
        "git",
        &[
            "ls-files",
            "--cached",  // Only committed files
            "--ignored", // List all --exclude matches
            "--exclude=.changesets/",
            "--exclude=**/*/.changesets/", // Match sub directories
            "--exclude=.changeset/",
            "--exclude=**/*/.changeset/", // Match sub directories
        ],
    ) {
        Ok(stdout) => {
            // If no output is printed no changeset directory was found
            !stdout.is_empty()
        }
        Err(e) => {
            // Other error that we do not expect so print the error
            let message = format!("Unable to read files from Git repository.\nError: {:?}", e);
            error!("{}", message);
            false
        }
    }
}

pub fn strip_trailers_from_message(message: &str, trailers: &str) -> String {
    let (body, _removed_trailers) = message
        .rsplit_once(&trailers) // Split from the back so only the last occurrence of the trailers is removed
        .unwrap_or(("", ""));
    body.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::Commit;
    use super::{
        is_commit_ignored, parse_commit, parse_commit_hook_format, strip_trailers_from_message,
        CleanupMode, COMMIT_BODY_DELIMITER, COMMIT_TRAILERS_DELIMITER,
    };
    use crate::config::ValidationContext;
    use crate::issue::IssueType;

    fn default_context() -> ValidationContext {
        ValidationContext { changesets: false }
    }

    fn assert_commit_is_invalid(commit: &mut Commit) {
        commit.validate(&default_context());
        assert!(!commit.issues.is_empty());
    }

    fn assert_commit_is_ignored(result: &Option<Commit>) {
        match result {
            Some(commit) => assert!(is_commit_ignored(commit)),
            None => panic!("Result is not a commit!"),
        }
    }

    fn assert_commit_is_not_ignored(result: &Option<Commit>) {
        match result {
            Some(commit) => assert!(!is_commit_ignored(commit)),
            None => panic!("Result is not a commit!"),
        }
    }

    fn commit_with_file_changes(message: &str) -> String {
        format!(
            "{}\n{COMMIT_TRAILERS_DELIMITER}\n{COMMIT_BODY_DELIMITER}\n{}",
            message, "\nsrc/main.rs\nsrc/utils.rs\n"
        )
    }

    fn commit_without_file_changes(message: &str) -> String {
        format!(
            "{}\n{COMMIT_TRAILERS_DELIMITER}\n{COMMIT_BODY_DELIMITER}\n{}",
            message, ""
        )
    }

    fn commit_with_trailers(message: &str, trailers: &str) -> String {
        format!(
            "{message}\n\
             \n\
             {trailers}
             {COMMIT_TRAILERS_DELIMITER}\n\
             {trailers}\n\
             {COMMIT_BODY_DELIMITER}\n\
             src/main.rs\n\
             README.md\n",
        )
    }

    #[test]
    fn test_parse_commit() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        This is a subject\n\
        \n\
        This is my multi line message.\n\
        Line 2.",
        ));

        assert_commit_is_not_ignored(&result);
        let commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.email, Some("test@example.com".to_string()));
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "\nThis is my multi line message.\nLine 2.");
        assert_eq!(commit.trailers, "");
        assert_eq!(
            commit.file_changes,
            vec!["src/main.rs".to_string(), "src/utils.rs".to_string()]
        );
        assert!(!commit
            .issues
            .into_iter()
            .any(|i| i.r#type == IssueType::Error));
    }

    #[test]
    fn test_parse_commit_with_trailers() {
        let result = parse_commit(&commit_with_trailers(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            test@example.com\n\
            This is a subject\n\
            \n\
            This is a message\n",
            "Co-authored-by: Person A <name@domain.com>\n\
             Co-authored-by: Person B <name@domain.com>\n",
        ));

        assert_commit_is_not_ignored(&result);
        let commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.email, Some("test@example.com".to_string()));
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "\nThis is a message");
        assert_eq!(
            commit.trailers,
            "Co-authored-by: Person A <name@domain.com>\n\
             Co-authored-by: Person B <name@domain.com>"
        );
        assert!(commit.has_changes());
    }

    #[test]
    fn test_parse_commit_with_errors() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            test@example.com\n\
            This is a subject",
        ));

        assert_commit_is_not_ignored(&result);
        let mut commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.email, Some("test@example.com".to_string()));
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "");
        assert!(commit.has_changes());
        assert_commit_is_invalid(&mut commit);
    }

    #[test]
    fn test_parse_commit_empty() {
        let result = parse_commit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n");

        assert_commit_is_not_ignored(&result);
        let mut commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.email, None);
        assert_eq!(commit.subject, "");
        assert_eq!(commit.message, "");
        assert!(!commit.has_changes());
        assert_commit_is_invalid(&mut commit);
    }

    #[test]
    fn test_parse_commit_without_file_changes() {
        let result = parse_commit(&commit_without_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            test@example.com\n\
            This is a subject\n\
            \n\
            This is a message.",
        ));

        assert_commit_is_not_ignored(&result);
        let mut commit = result.unwrap();
        assert_eq!(
            commit.long_sha,
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string())
        );
        assert_eq!(commit.short_sha, Some("aaaaaaa".to_string()));
        assert_eq!(commit.email, Some("test@example.com".to_string()));
        assert_eq!(commit.subject, "This is a subject");
        assert_eq!(commit.message, "\nThis is a message.");
        assert_eq!(commit.file_changes, Vec::<String>::new());
        assert!(!commit.has_changes());
        assert_commit_is_invalid(&mut commit);
    }

    #[test]
    fn test_parse_commit_ignore_bot_commit() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            12345678+bot-name[bot]@users.noreply.github.com\n\
            Commit by bot without description",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_ignore_tag_merge_commit() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Merge tag 'v1.2.3' into main",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_ignore_merge_commit_pull_request() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Merge pull request #123 from tombruijn/repo\n\
        \n\
        This is my multi line message.\n\
        Line 2.",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_ignore_squash_merge_commit_pull_request() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Fix some issue that's squashed (#123)\n\
        \n\
        This is my multi line message.\n\
        Line 2.",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_ignore_merge_commits_merge_request() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Merge branch 'branch' into main\n\
        \n\
        This is my multi line message.\n\
        Line 2.\n\
        \n\
        See merge request gitlab-org/repo!123",
        ));

        assert_commit_is_ignored(&result);

        // This is not a full reference, but a shorthand. GitLab merge commits
        // use the full org + repo + Merge Request ID reference.
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Fix some issue\n\
        \n\
        This is my multi line message.\n\
        Line 2.\n\
        \n\
        See merge request !123 for more info about the orignal fix",
        ));

        assert_commit_is_not_ignored(&result);

        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Fix some issue\n\
        \n\
        This is my multi line message.\n\
        Line 2.\n\
        \n\
        Also See merge request !123",
        ));

        assert_commit_is_not_ignored(&result);

        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Fix some issue\n\
        \n\
        This is my multi line message.\n\
        Line 2. See merge request org/repo!123",
        ));

        assert_commit_is_not_ignored(&result);
    }

    #[test]
    fn test_parse_commit_ignore_merge_commits_without_into() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Merge branch 'branch'",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_ignore_revert_commit() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
            test@example.com\n\
            Revert \"Some commit\"\n\
            \n\
            This reverts commit 0d02b90cbf0c79acf9c0b56de00d52389272ec6f",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_merge_remote_commits() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Merge branch 'branch' of github.com/org/repo into branch",
        ));

        assert_commit_is_not_ignored(&result);
    }

    #[test]
    fn test_parse_commit_merge_into_commit() {
        let result = parse_commit(&commit_with_file_changes(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n\
        test@example.com\n\
        Merge 3af48fbbdf7c2bd77c35e829bc7561fb7c660b21 into 17e2def8fbb2a0d500bffb79c7fe85381f24d415",
        ));

        assert_commit_is_ignored(&result);
    }

    #[test]
    fn test_parse_commit_hook_format() {
        let (subject, message) = parse_commit_hook_format(
            "This is a subject\n\nThis is a message.",
            &CleanupMode::Default,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is a message.");
    }

    #[test]
    fn test_parse_commit_hook_format_without_message() {
        let (subject, message) =
            parse_commit_hook_format("This is a subject", &CleanupMode::Default, "#");

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "");
    }

    // Same as Default
    #[test]
    fn test_parse_commit_hook_format_with_strip() {
        let (subject, message) = parse_commit_hook_format(
            "This is a subject  \n\
            \n\
            This is the message body.  \n\
            # This is a commented line.\n\
            \n\
            Another line.\n\
            \n\
            # Other things that are not part of the message.\n\
            ",
            &CleanupMode::Strip,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is the message body.\n\nAnother line.\n");
    }

    #[test]
    fn test_parse_commit_hook_format_with_leading_empty_lines() {
        let (subject, message) = parse_commit_hook_format(
            "  \n\
            This is a subject  \n\
            \n\
            This is the message body.  \n\
            # This is a commented line.\n\
            \n\
            Another line.\n\
            \n\
            # Other things that are not part of the message.\n\
            ",
            &CleanupMode::Strip,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is the message body.\n\nAnother line.\n");
    }

    #[test]
    fn test_parse_commit_hook_format_with_leading_comment_lines() {
        let (subject, message) = parse_commit_hook_format(
            "# This is a comment\n\
            This is a subject  \n\
            \n\
            This is the message body.  \n\
            # This is a commented line.\n\
            \n\
            Another line.\n\
            \n\
            # Other things that are not part of the message.\n\
            ",
            &CleanupMode::Strip,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is the message body.\n\nAnother line.\n");
    }

    #[test]
    fn test_parse_commit_hook_format_with_strip_custom_comment_char() {
        let (subject, message) = parse_commit_hook_format(
            "This is a subject  \n\
            \n\
            This is the message body.  \n\
            - This is a commented line.\n\
            \n\
            Another line.\n\
            \n\
            - Other things that are not part of the message.\n\
            ",
            &CleanupMode::Strip,
            "-",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is the message body.\n\nAnother line.\n");
    }

    #[test]
    fn test_parse_commit_hook_format_with_scissors() {
        let (subject, message) = parse_commit_hook_format(
            "This is a subject  \n\
            \n\
            This is the message body.  \n\
            \n\
            This is line 2.\n\
            # ------------------------ >8 ------------------------\n\
            Other things that are not part of the message.\n\
            ",
            &CleanupMode::Scissors,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is the message body.\n\nThis is line 2.");
    }

    #[test]
    fn test_parse_commit_hook_format_with_scissors_empty_message() {
        let (subject, message) = parse_commit_hook_format(
            "# ------------------------ >8 ------------------------\n\
            Other things that are not part of the message.\n\
            ",
            &CleanupMode::Scissors,
            "#",
        );

        assert_eq!(subject, "");
        assert_eq!(message, "");
    }

    #[test]
    fn test_parse_commit_hook_format_with_verbatim() {
        let (subject, message) = parse_commit_hook_format(
            "This is a subject  \n\
            \n\
            This is the message body.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            ",
            &CleanupMode::Verbatim,
            "#",
        );

        assert_eq!(subject, "This is a subject  ");
        assert_eq!(
            message,
            "\nThis is the message body.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            "
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_verbatim_leading_empty_lines() {
        let (subject, message) = parse_commit_hook_format(
            "  \n\
            This is the message body.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            ",
            &CleanupMode::Verbatim,
            "#",
        );

        assert_eq!(subject, "  ");
        assert_eq!(
            message,
            "This is the message body.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            "
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_whitespace() {
        let (subject, message) = parse_commit_hook_format(
            "This is a subject  \n\
            \n\
            This is the message body.  \n\
            \n\
            This is line 2.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            ",
            &CleanupMode::Whitespace,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(
            message,
            "\nThis is the message body.\n\
            \n\
            This is line 2.\n\
            # This is a comment\n\
            # Other things that are not part of the message.\n\
            Extra suprise!\
            "
        );
    }

    #[test]
    fn test_parse_commit_hook_format_with_whitespace_leading_comment_lines() {
        let (subject, message) = parse_commit_hook_format(
            "# This is a comment\n\
            This is a subject  \n\
            \n\
            This is the message body.",
            &CleanupMode::Whitespace,
            "#",
        );

        assert_eq!(subject, "# This is a comment");
        assert_eq!(message, "This is a subject\n\nThis is the message body.");
    }

    #[test]
    fn test_parse_commit_hook_format_with_strip_and_scissor_line() {
        // Even in default mode the scissor line is seen as the end of the message.
        // This can happen when `git commit` is called with the `--verbose` flag.
        let (subject, message) = parse_commit_hook_format(
            "This is a subject  \n\
            \n\
            This is the message body.  \n\
            # This is a comment before scissor line\n\
            # ------------------------ >8 ------------------------\n\
            # Other things that are not part of the message.\n\
            List of file changes",
            &CleanupMode::Strip,
            "#",
        );

        assert_eq!(subject, "This is a subject");
        assert_eq!(message, "\nThis is the message body.");
    }

    #[test]
    fn strip_trailers_from_message_without_trailers() {
        let result = strip_trailers_from_message("Subject\n\nMy message body\n", "");
        assert_eq!(result, "Subject\n\nMy message body");
    }

    #[test]
    fn strip_trailers_from_message_with_trailers() {
        let result = strip_trailers_from_message(
            "Subject\n\
            \n\
            My message body\n
            \n\
            Co-authored-by: Person A\n",
            "Co-authored-by: Person A",
        );
        assert_eq!(result, "Subject\n\nMy message body");
    }

    #[test]
    fn strip_trailers_from_message_with_multiple_trailers() {
        let result = strip_trailers_from_message(
            "Subject\n\
            \n\
            My message body\n
            \n\
            Co-authored-by: Person A\n\
            Signed-off-by: Person B\n\
            Fix: #123\n",
            "Co-authored-by: Person A\n\
            Signed-off-by: Person B\n\
            Fix: #123",
        );
        assert_eq!(result, "Subject\n\nMy message body");
    }

    #[test]
    fn strip_trailers_from_message_with_duplicate_trailers() {
        let result = strip_trailers_from_message(
            "Subject\n\
            \n\
            Co-authored-by: Person A\n\
            Signed-off-by: Person B\n
            My message body\n
            \n\
            Co-authored-by: Person A\n\
            Signed-off-by: Person B\n",
            "Co-authored-by: Person A\n\
            Signed-off-by: Person B",
        );
        assert_eq!(
            result,
            "Subject\n\
            \n\
            Co-authored-by: Person A\n\
            Signed-off-by: Person B\n
            My message body"
        );
    }
}
