#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate predicates;

use log::LevelFilter;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

mod branch;
mod command;
mod commit;
mod git;
mod logger;
mod rule;

use branch::Branch;
use commit::Commit;
use git::{fetch_and_parse_branch, fetch_and_parse_commits, parse_commit_hook_format};
use logger::Logger;

#[derive(StructOpt, Debug)]
#[structopt(name = "lintje", verbatim_doc_comment)]
/**
Lint Git commits and branch name.

## Usage examples

    lintje
      Validate the latest commit.

    lintje HEAD
      Validate the latest commit.

    lintje 3a561ef766c2acfe5da478697d91758110b8b24c
      Validate a single specific commit.

    lintje HEAD~5..HEAD
      Validate the last 5 commits.

    lintje main..develop
      Validate the difference between the main and develop branch.

    lintje --hook-message-file=.git/COMMIT_EDITMSG
      Lints the given commit message file from the commit-msg hook.

    lintje --no-branch
      Disable branch name validation.
*/
struct Lint {
    /// Prints debug information
    #[structopt(long)]
    debug: bool,

    /// Lint the contents the Git hook commit-msg commit message file.
    #[structopt(long, parse(from_os_str))]
    hook_message_file: Option<PathBuf>,

    /// Disable branch validation
    #[structopt(long = "no-branch")]
    no_branch_validation: bool,

    /// Lint commits by Git commit SHA or by a range of commits. When no <commit> is specified, it
    /// defaults to linting the latest commit.
    #[structopt(name = "commit (range)")]
    selection: Option<String>,
}

fn main() {
    let args = Lint::from_args();
    init_logger(args.debug);
    let commit_result = match args.hook_message_file {
        Some(hook_message_file) => lint_commit_hook(&hook_message_file),
        None => lint_commit(args.selection),
    };
    let branch_result = if args.no_branch_validation {
        None
    } else {
        Some(lint_branch())
    };
    handle_lint_result(commit_result, branch_result);
}

fn lint_branch() -> Result<Branch, String> {
    fetch_and_parse_branch()
}

fn lint_commit(selection: Option<String>) -> Result<Vec<Commit>, String> {
    fetch_and_parse_commits(selection)
}

fn lint_commit_hook(filename: &Path) -> Result<Vec<Commit>, String> {
    let commits = match File::open(filename) {
        Ok(mut file) => {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => {}
                Err(e) => {
                    return Err(format!(
                        "Unable to read commit message file contents: {}\n{}",
                        filename.to_str().unwrap(),
                        e
                    ));
                }
            };
            match parse_commit_hook_format(&contents, git::cleanup_mode(), git::comment_char()) {
                Some(commit) => vec![commit],
                None => vec![],
            }
        }
        Err(e) => {
            return Err(format!(
                "Unable to open commit message file: {}\n{}",
                filename.to_str().unwrap(),
                e
            ));
        }
    };
    Ok(commits)
}

fn handle_lint_result(
    commit_result: Result<Vec<Commit>, String>,
    branch_result: Option<Result<Branch, String>>,
) {
    let mut violation_count = 0;
    let mut commit_count = 0;
    let mut branch_message = "";

    if let Ok(ref commits) = commit_result {
        debug!("Commits: {:?}", commits);
        commit_count = commits.len();
        for commit in commits {
            if !commit.is_valid() {
                match &commit.short_sha {
                    Some(sha) => println!("{}: {}", sha, commit.subject),
                    None => println!("{}", commit.subject),
                }

                for violation in &commit.violations {
                    violation_count += 1;
                    println!("  {}: {}", violation.rule, violation.message);
                }
            }
        }
    }
    let mut branch_error = None;
    if let Some(result) = branch_result {
        match result {
            Ok(ref branch) => {
                debug!("Branch: {:?}", branch);
                branch_message = " and branch";
                if !branch.is_valid() {
                    println!("Branch: {}", branch.name);
                    for violation in &branch.violations {
                        violation_count += 1;
                        println!("  {}: {}", violation.rule, violation.message);
                    }
                }
            }
            Err(error) => branch_error = Some(error),
        }
    }

    if violation_count > 0 {
        println!();
    }
    let plural = if commit_count != 1 { "s" } else { "" };
    println!(
        "{} commit{}{} inspected, {} violations detected",
        commit_count, plural, branch_message, violation_count
    );
    let mut has_error = false;
    if commit_result.is_err() {
        has_error = true;
        error!("An error occurred validating commits: {:?}", commit_result);
    }
    if branch_error.is_some() {
        has_error = true;
        error!(
            "An error occurred validating the branch: {:?}",
            &branch_error
        );
    }
    if has_error {
        std::process::exit(2)
    }
    if violation_count > 0 {
        std::process::exit(1)
    }
}

fn init_logger(debug: bool) {
    let level = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let result = log::set_boxed_logger(Box::new(Logger::new())).map(|()| log::set_max_level(level));
    match result {
        Ok(_) => (),
        Err(error) => {
            eprintln!(
                "An error occurred while initialzing the logger. \
                Cannot continue.\n{:?}",
                error
            );
            std::process::exit(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use predicates::prelude::*;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    const TEST_DIR: &str = "tmp/tests/test_repo";

    fn test_dir(name: &str) -> PathBuf {
        Path::new(TEST_DIR).join(name)
    }

    fn create_test_repo(dir: &PathBuf, commits: &[(&str, &str)]) {
        if Path::new(&dir).exists() {
            fs::remove_dir_all(&dir).expect("Could not remove test repo dir");
        }
        fs::create_dir_all(&dir).expect("Could not create test repo dir");
        let output = Command::new("git")
            .args(&["init"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .expect("Could not init test repo!");
        if !output.status.success() {
            panic!(
                "Failed to initialize repo!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output
                    .status
                    .code()
                    .expect("Could not fetch status code of git init"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
        create_commit(dir, "Initial commit", "");
        for (subject, message) in commits {
            create_commit(dir, subject, message)
        }
    }

    fn checkout_branch(dir: &PathBuf, name: &str) {
        let output = Command::new("git")
            .args(&["checkout", "-b", name])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .expect(&format!("Could not checkout branch: {}", name));
        if !output.status.success() {
            panic!(
                "Failed to checkout branch: {}\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                name,
                output
                    .status
                    .code()
                    .expect("Could not fetch status code of git checkout"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }

    fn create_commit(dir: &PathBuf, subject: &str, message: &str) {
        let mut args = vec![
            "commit".to_string(),
            "--no-gpg-sign".to_string(),
            "--allow-empty".to_string(),
            format!("-m{}", subject),
        ];
        if !message.is_empty() {
            let message_arg = format!("-m {}", message);
            args.push(message_arg)
        }
        let output = Command::new("git")
            .args(args.as_slice())
            .current_dir(dir)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("Failed to make commit: {}, {}", subject, message));
        if !output.status.success() {
            panic!(
                "Failed to make commit!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output
                    .status
                    .code()
                    .expect("Could not fetch status code of git commit"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }

    fn configure_git_cleanup_mode(dir: &PathBuf, mode: &str) {
        let output = Command::new("git")
            .args(&["config", "commit.cleanup", mode])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("Failed to configure Git commit.cleanup: {}", mode));
        if !output.status.success() {
            panic!(
                "Failed to configure Git commit.cleanup!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output
                    .status
                    .code()
                    .expect("Could not fetch status code of git config"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }

    fn configure_git_comment_char(dir: &PathBuf, character: &str) {
        let output = Command::new("git")
            .args(&["config", "core.commentChar", character])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("Failed to configure Git core.commentChar: {}", character));
        if !output.status.success() {
            panic!(
                "Failed to configure Git core.commentChar!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output
                    .status
                    .code()
                    .expect("Could not fetch status code of git config"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }

    fn compile_bin() {
        Command::new("cargo")
            .args(&["build"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("Could not compile debug target!");
    }

    #[test]
    fn test_commit_by_sha() {
        compile_bin();
        let dir = test_dir("commit_by_sha");
        create_test_repo(&dir, &[("Test commit", "")]);
        let output = Command::new("git")
            .args(&["log", "--pretty=%H", "-n 1"])
            .current_dir(&dir)
            .output()
            .expect("Failed to fetch commit SHA.");
        let sha = String::from_utf8_lossy(&output.stdout);
        let short_sha = sha.get(0..7).expect("Unable to build short commit SHA");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.arg(sha.to_string()).current_dir(dir).assert().failure();
        assert
            .stdout(predicate::str::contains(&format!(
                "{}: Test commit",
                short_sha
            )))
            .stdout(predicate::str::contains("1 commit and branch inspected"));
    }

    #[test]
    fn test_single_commit_valid() {
        compile_bin();
        let dir = test_dir("single_commit_valid");
        create_test_repo(
            &dir,
            &[("Test commit", "I am a test commit, short but valid.")],
        );

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.current_dir(dir).assert().success();
        assert.stdout(predicate::str::contains(
            "1 commit and branch inspected, 0 violations detected\n",
        ));
    }

    #[test]
    fn test_single_commit_invalid() {
        compile_bin();
        let dir = test_dir("single_commit_invalid");
        create_test_repo(&dir, &[("added some code", ""), ("Fixing tests", "")]);

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.current_dir(dir).assert().failure().code(1);
        assert
            .stdout(predicate::str::contains(
                "Fixing tests\n\
                \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
                \x20\x20SubjectCliche: Reword the subject to describe the change in more detail.\n\
                \x20\x20MessagePresence: Add a message body to provide more context about the change and why it was made.",
            ))
            .stdout(predicate::str::contains(
                "1 commit and branch inspected, 3 violations detected\n",
            ));
    }

    #[test]
    fn test_multiple_commit_invalid() {
        compile_bin();
        let dir = test_dir("multiple_commits_invalid");
        create_test_repo(
            &dir,
            &[
                ("added some code", "This is a message."),
                ("Fixing tests", ""),
            ],
        );

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .arg(&"HEAD~2..HEAD")
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert
            .stdout(predicate::str::contains(
                "added some code\n\
                \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
                \x20\x20SubjectCapitalization: Start the commit subject with a capital letter.",
            ))
            .stdout(predicate::str::contains(
                "Fixing tests\n\
                \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
                \x20\x20SubjectCliche: Reword the subject to describe the change in more detail.\n\
                \x20\x20MessagePresence: Add a message body to provide more context about the change and why it was made.",
            ))
            .stdout(predicate::str::contains(
                "2 commits and branch inspected, 5 violations detected\n",
            ));
    }

    #[test]
    fn test_lint_hook() {
        compile_bin();
        let dir = test_dir("commit_file_option");
        create_test_repo(&dir, &[]);
        let filename = "commit_message_file";
        let commit_file = dir.join(filename);
        let mut file = File::create(&commit_file).unwrap();
        file.write_all(b"added some code\n\nThis is a message.")
            .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .arg(&format!("--hook-message-file={}", filename))
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert.stdout(predicate::str::contains(
            "added some code\n\
             \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
             \x20\x20SubjectCapitalization: Start the commit subject with a capital letter.",
        ));
    }

    #[test]
    fn test_file_option_with_scissors_cleanup() {
        compile_bin();
        let dir = test_dir("commit_file_option_with_scissors_cleanup_default_comment_char");
        create_test_repo(&dir, &[]);
        configure_git_cleanup_mode(&dir, "scissors");
        let filename = "commit_message_file";
        let commit_file = dir.join(filename);
        let mut file = File::create(&commit_file).unwrap();
        file.write_all(
            b"This is a subject\n\n\
            # ------------------------ >8 ------------------------
            # This is part of the comment that will be ignored
            ",
        )
        .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .arg(&format!("--hook-message-file={}", filename))
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert.stdout(predicate::str::contains("  MessagePresence: "));
    }

    #[test]
    fn test_file_option_with_scissors_cleanup_custom_comment_char() {
        compile_bin();
        let dir = test_dir("commit_file_option_with_scissors_cleanup_custom_comment_char");
        create_test_repo(&dir, &[]);
        configure_git_cleanup_mode(&dir, "scissors");
        configure_git_comment_char(&dir, "-");
        let filename = "commit_message_file";
        let commit_file = dir.join(filename);
        let mut file = File::create(&commit_file).unwrap();
        file.write_all(
            b"This is a subject\n\n\
            - ------------------------ >8 ------------------------
            - This is part of the comment that will be ignored
            ",
        )
        .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .arg(&format!("--hook-message-file={}", filename))
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert.stdout(predicate::str::contains("  MessagePresence: "));
    }

    #[test]
    fn test_file_option_without_file() {
        compile_bin();
        let dir = test_dir("commit_file_option_without_file");
        create_test_repo(&dir, &[]);
        let filename = "commit_message_file";

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .arg(&format!("--hook-message-file={}", filename))
            .current_dir(dir)
            .assert()
            .failure()
            .code(2);
        assert.stdout(predicate::str::contains(
            "Unable to open commit message file: commit_message_file",
        ));
    }

    #[test]
    fn test_branch_valid() {
        compile_bin();
        let dir = test_dir("branch_valid");
        create_test_repo(
            &dir,
            &[("Test commit", "I am a test commit, short but valid.")],
        );
        checkout_branch(&dir, "my-branch");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.current_dir(dir).assert().success();
        assert.stdout(predicate::str::contains(
            "1 commit and branch inspected, 0 violations detected\n",
        ));
    }

    #[test]
    fn test_branch_invalid() {
        compile_bin();
        let dir = test_dir("branch_invalid");
        create_test_repo(
            &dir,
            &[("Test commit", "I am a test commit, short but valid.")],
        );
        checkout_branch(&dir, "fix-123");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.current_dir(dir).assert().failure().code(1);
        assert
            .stdout(predicate::str::contains(
                "Branch: fix-123\n\
                \x20\x20BranchNameTicketNumber: Remove the ticket number from the branch name or expand the branch name with more details.",
            ))
            .stdout(predicate::str::contains(
                    "1 commit and branch inspected, 1 violations detected\n",
            ));
    }

    #[test]
    fn test_no_branch_validation() {
        compile_bin();
        let dir = test_dir("branch_invalid_disabled");
        create_test_repo(
            &dir,
            &[("Test commit", "I am a test commit, short but valid.")],
        );
        checkout_branch(&dir, "fix-123");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.arg("--no-branch").current_dir(dir).assert().success();
        assert.stdout(predicate::str::contains(
            "1 commit inspected, 0 violations detected\n",
        ));
    }
}
