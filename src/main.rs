#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]
#![deny(unused_import_braces)]
#![deny(non_ascii_idents)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::if_not_else)]
#![warn(clippy::match_same_arms)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::trivially_copy_pass_by_ref)]
#![warn(clippy::doc_markdown)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use log::LevelFilter;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use termcolor::{ColorChoice, StandardStream, WriteColor};

mod branch;
mod command;
mod commit;
mod config;
mod formatter;
mod git;
mod issue;
mod logger;
mod rule;
mod rules;
#[cfg(test)]
mod test;
mod utils;

use branch::Branch;
use commit::Commit;
use config::{fetch_options, Lint, ValidationContext};
use formatter::{formatted_branch_issue, formatted_commit_issue};
use git::{
    fetch_and_parse_branch, fetch_and_parse_commits, is_commit_ignored, parse_commit_file,
    repo_has_changesets,
};
use issue::IssueType;
use logger::Logger;
use utils::pluralize;

fn main() {
    let options = fetch_options();
    init_logger(options.debug);
    match handle_command(&options) {
        Ok(_) => (),
        Err(e) => {
            // No println because the error message ends in a line break
            eprint!("Lintje encountered an error: {}", e);
            std::process::exit(2)
        }
    }
}

fn handle_command(options: &Lint) -> Result<(), String> {
    if let Some(hook) = &options.install_hook {
        return match git::hooks::install_hook(hook) {
            Ok(file) => {
                println!("Succesfully installed Lintje in hook: '{}'", file);
                Ok(())
            }
            Err(e) => Err(e),
        };
    }

    let validate_changesets = repo_has_changesets();
    let context = ValidationContext {
        changesets: validate_changesets,
    };
    debug!("Validating with context: {context:?}");
    let commits = match &options.hook_message_file {
        Some(hook_message_file) => lint_commit_hook(hook_message_file)?,
        None => lint_commit(&options.selection)?,
    };
    let branch = if options.branch_validation {
        Some(lint_branch()?)
    } else {
        None
    };
    match print_lint_result(commits, branch, options, &context) {
        Ok(_) => Ok(()),
        Err(error) => Err(format!(
            "Error encountered while printing output: {}",
            error
        )),
    }
}

fn lint_branch() -> Result<Branch, String> {
    fetch_and_parse_branch()
}

fn lint_commit(selection: &Option<String>) -> Result<Vec<Commit>, String> {
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

            let commit = parse_commit_file(&contents);
            vec![commit]
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

fn print_lint_result(
    mut commits: Vec<Commit>,
    branch: Option<Branch>,
    options: &Lint,
    context: &ValidationContext,
) -> io::Result<()> {
    let mut out = buffer_writer(options.color());
    let mut error_count = 0;
    let mut hint_count = 0;
    let mut commit_count = 0;
    let mut ignored_commit_count = 0;
    let mut branch_message = "";

    for commit in commits.iter_mut() {
        if is_commit_ignored(commit) {
            ignored_commit_count += 1;
            continue;
        }
        commit.validate(context);
        commit_count += 1;
        if !commit.is_valid() {
            for issue in &commit.issues {
                let show = match issue.r#type {
                    IssueType::Error => {
                        error_count += 1;
                        true
                    }
                    IssueType::Hint => {
                        hint_count += 1;
                        options.hints
                    }
                };
                if show {
                    formatted_commit_issue(&mut out, commit, issue)?;
                }
            }
        }
    }
    debug!("Commits: {:?}", commits);

    if let Some(branch) = branch {
        debug!("Branch: {:?}", branch);
        branch_message = " and branch";
        if !branch.is_valid() {
            for issue in &branch.issues {
                match issue.r#type {
                    IssueType::Error => error_count += 1,
                    IssueType::Hint => hint_count += 1,
                }
                formatted_branch_issue(&mut out, &branch, issue)?;
            }
        }
    }

    let commit_label = pluralize("commit", commit_count);
    write!(
        out,
        "{} {}{} inspected, ",
        commit_count, commit_label, branch_message
    )?;
    print_issue_counts(&mut out, error_count, hint_count, options.hints)?;
    if ignored_commit_count > 0 || options.debug {
        let ignored_commit_label = pluralize("commit", ignored_commit_count);
        write!(
            out,
            " ({} {} ignored)",
            ignored_commit_count, ignored_commit_label
        )?;
    }
    writeln!(out)?;
    if error_count > 0 {
        std::process::exit(1)
    }
    Ok(())
}

fn print_issue_counts(
    out: &mut impl WriteColor,
    error_count: usize,
    hint_count: usize,
    show_hints: bool,
) -> io::Result<()> {
    // Errors
    let error_color = if error_count > 0 {
        formatter::red_color()
    } else {
        formatter::green_color()
    };
    out.set_color(&error_color)?;
    write!(
        out,
        "{} {} detected",
        error_count,
        pluralize("error", error_count)
    )?;
    out.reset()?;

    if hint_count > 0 && show_hints {
        // Hints
        write!(out, ", {} {}", hint_count, pluralize("hint", hint_count))?;
    }
    Ok(())
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

/// Returns a `StandardStream` configured to write with color or not based on the config flag set by
/// the user.
fn buffer_writer(color: bool) -> StandardStream {
    let color_choice = if color {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    };
    StandardStream::stdout(color_choice)
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    use predicates::prelude::*;
    use regex::Regex;
    use std::fs::File;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::Once;

    use crate::test::TEST_DIR;

    static COMPILE_ONCE: Once = Once::new();

    fn test_dir(name: &str) -> PathBuf {
        Path::new(TEST_DIR).join(name)
    }

    fn create_test_repo(dir: &Path) {
        prepare_test_dir(&dir);
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
        create_commit(
            dir,
            "Initial commit",
            "Initial commit with message body.\nlintje:disable DiffPresence",
        );
    }

    fn checkout_branch(dir: &Path, name: &str) {
        let output = Command::new("git")
            .args(&["checkout", "-b", name])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|_| panic!("Could not checkout branch: {}", name));
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

    fn create_commit(dir: &Path, subject: &str, message: &str) {
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

    fn create_commit_with_file(dir: &Path, subject: &str, message: &str, filename: &str) {
        create_dummy_file(&dir.join(&filename));
        stage_files(dir);
        create_commit(dir, subject, message)
    }

    fn create_dummy_file(file_path: &Path) {
        create_file(file_path, b"I am a test file!");
    }

    fn stage_files(dir: &Path) {
        let output = Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|e| panic!("Failed to add files to commit: {:?}", e));
        if !output.status.success() {
            panic!(
                "Failed to add files to commit!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output
                    .status
                    .code()
                    .expect("Could not fetch status code of git add"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }

    fn configure_git_cleanup_mode(dir: &Path, mode: &str) {
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

    fn configure_git_comment_char(dir: &Path, character: &str) {
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

    fn normalize_output(output: &[u8]) -> String {
        // Replace dynamic commit short SHA with 0000000 dummy placeholder
        let regexp = Regex::new("([a-z0-9]{7})(:\\d:\\d)").unwrap();
        let raw_output = String::from_utf8_lossy(output);
        regexp.replace_all(&raw_output, "0000000$2").to_string()
    }

    // Compile the `lintje` executable, but only one for the entire test suite.
    // This way it doesn't compile Lintje for every test again.
    fn compile_bin() {
        COMPILE_ONCE.call_once(|| {
            Command::new("cargo")
                .args(&["build"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("Could not compile debug target!");
        });
    }

    #[test]
    fn test_version_option() {
        compile_bin();
        let dir = test_dir("version_option");
        create_test_repo(&dir);

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.arg("--version").current_dir(dir).assert().success();
        assert.stdout(predicate::str::is_match(format!("lintje \\d+\\.\\d+\\.\\d+")).unwrap());
    }

    #[test]
    fn test_commit_by_sha() {
        compile_bin();
        let dir = test_dir("commit_by_sha");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "", "file");
        let output = Command::new("git")
            .args(&["log", "--pretty=%H", "-n 1"])
            .current_dir(&dir)
            .output()
            .expect("Failed to fetch commit SHA.");
        let sha = String::from_utf8_lossy(&output.stdout);
        let short_sha = sha.get(0..7).expect("Unable to build short commit SHA");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", &sha])
            .current_dir(dir)
            .assert()
            .failure();
        assert
            .stdout(
                predicate::str::is_match(format!("{}:\\d+:\\d+: Test commit", short_sha)).unwrap(),
            )
            .stdout(predicate::str::contains("1 commit and branch inspected"));
    }

    #[test]
    fn test_single_commit_valid() {
        compile_bin();
        let dir = test_dir("single_commit_valid");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout("1 commit and branch inspected, 0 errors detected\n");
    }

    #[test]
    fn test_single_commit_valid_with_color() {
        compile_bin();
        let dir = test_dir("single_commit_valid_with_color");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout(
            "1 commit and branch inspected, \u{1b}[0m\u{1b}[32m0 errors detected\u{1b}[0m\n",
        );
    }

    #[test]
    fn test_single_commit_with_hint() {
        compile_bin();
        let dir = test_dir("single_commit_valid_hint");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.args(["--no-color"]).current_dir(dir).assert().success();
        assert
            .stdout(predicate::str::contains(
                "Hint[MessageTicketNumber]: The message body does not contain a ticket or issue number",
            ))
            .stdout(predicate::str::contains(
                "1 commit and branch inspected, 0 errors detected, 1 hint\n",
            ));
    }

    #[test]
    fn test_single_commit_in_repository() {
        compile_bin();
        let dir = test_dir("single_commit_in_repository");
        create_test_repo(&dir);

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout("1 commit and branch inspected, 0 errors detected\n");
    }

    #[test]
    fn test_single_commit_invalid() {
        compile_bin();
        let dir = test_dir("single_commit_invalid");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Fixing tests", "", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);

        let output = normalize_output(&assert.get_output().stdout);
        assert_eq!(
            output,
            "Error[SubjectCliche]: The subject does not explain the change in much detail\n\
            \x20\x200000000:1:1: Fixing tests\n\
            \x20\x20  | \n\
            \x20\x201 | Fixing tests\n\
            \x20\x20  | ^^^^^^^^^^^^ Describe the change in more detail\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://lintje.dev/docs/rules/commit-subject/#subjectcliche\n\
            \n\
            Error[SubjectMood]: The subject does not use the imperative grammatical mood\n\
            \x20\x200000000:1:1: Fixing tests\n\
            \x20\x20  | \n\
            \x20\x201 | Fixing tests\n\
            \x20\x20  | ^^^^^^ Use the imperative mood for the subject\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://lintje.dev/docs/rules/commit-subject/#subjectmood\n\
            \n\
            Error[MessagePresence]: No message body was found\n\
            \x20\x200000000:3:1: Fixing tests\n\
            \x20\x20  | \n\
            \x20\x201 | Fixing tests\n\
            \x20\x202 | \n\
            \x20\x203 | \n\
            \x20\x20  | +++ Add a message that describes the change and why it was made\n\
            \x20\x20  | \n\
            \x20\x20  = help: https://lintje.dev/docs/rules/commit-message/#messagepresence\n\
            \n\
            1 commit and branch inspected, 3 errors detected\n"
        );
    }

    #[test]
    fn test_single_commit_invalid_with_color() {
        compile_bin();
        let dir = test_dir("with_color");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Fixing tests", "", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);

        let output = normalize_output(&assert.get_output().stdout);
        assert_eq!(
            output,
            "\u{1b}[0m\u{1b}[31mError[SubjectCliche]\u{1b}[0m: The subject does not explain the change in much detail\n\
            \x20\x20\u{1b}[0m\u{1b}[38;5;12m0000000:1:1:\u{1b}[0m Fixing tests\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  1 | \u{1b}[0mFixing tests\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\u{1b}[0m\u{1b}[38;5;9m^^^^^^^^^^^^ Describe the change in more detail\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    = \u{1b}[0mhelp: https://lintje.dev/docs/rules/commit-subject/#subjectcliche\n\
            \n\
            \u{1b}[0m\u{1b}[31mError[SubjectMood]\u{1b}[0m: The subject does not use the imperative grammatical mood\n\
            \x20\x20\u{1b}[0m\u{1b}[38;5;12m0000000:1:1:\u{1b}[0m Fixing tests\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  1 | \u{1b}[0mFixing tests\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\u{1b}[0m\u{1b}[38;5;9m^^^^^^ Use the imperative mood for the subject\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    = \u{1b}[0mhelp: https://lintje.dev/docs/rules/commit-subject/#subjectmood\n\
            \n\
            \u{1b}[0m\u{1b}[31mError[MessagePresence]\u{1b}[0m: No message body was found\n\
            \x20\x20\u{1b}[0m\u{1b}[38;5;12m0000000:3:1:\u{1b}[0m Fixing tests\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  1 | \u{1b}[0mFixing tests\n\
            \u{1b}[0m\u{1b}[38;5;12m  2 | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m  3 | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\u{1b}[0m\u{1b}[32m+++ Add a message that describes the change and why it was made\u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    | \u{1b}[0m\n\
            \u{1b}[0m\u{1b}[38;5;12m    = \u{1b}[0mhelp: https://lintje.dev/docs/rules/commit-message/#messagepresence\n\
            \n\
            1 commit and branch inspected, \u{1b}[0m\u{1b}[31m3 errors detected\u{1b}[0m\n"
        );
    }

    #[test]
    fn test_single_commit_invalid_one_issue() {
        compile_bin();
        let dir = test_dir("single_commit_invalid_one_issue");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Valid commit subject", "", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert
            .stdout(predicate::str::contains(
                "Error[MessagePresence]: No message body was found",
            ))
            .stdout(predicate::str::contains(
                "1 commit and branch inspected, 1 error detected",
            ));
    }

    #[test]
    fn test_single_commit_invalid_without_file_changes() {
        compile_bin();
        let dir = test_dir("single_commit_invalid_without_file_changes");
        create_test_repo(&dir);
        create_commit(&dir, "Valid commit subject", "");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert
            .stdout(predicate::str::contains(
                "Error[MessagePresence]: No message body was found",
            ))
            .stdout(predicate::str::contains(
                "Error[DiffPresence]: No file changes found",
            ))
            .stdout(predicate::str::contains(
                "1 commit and branch inspected, 2 errors detected",
            ));
    }

    #[test]
    fn test_single_commit_ignored() {
        compile_bin();
        let dir = test_dir("single_commit_ignored");
        create_test_repo(&dir);
        create_commit_with_file(
            &dir,
            "Merge pull request #123 from tombruijn/repo",
            "",
            "file",
        );

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout("0 commits and branch inspected, 0 errors detected (1 commit ignored)\n");
    }

    #[test]
    fn test_single_commit_ignored_with_color() {
        compile_bin();
        let dir = test_dir("single_commit_ignored_with_color");
        create_test_repo(&dir);
        create_commit_with_file(
            &dir,
            "Merge pull request #123 from tombruijn/repo",
            "",
            "file",
        );

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout("0 commits and branch inspected, \u{1b}[0m\u{1b}[32m0 errors detected\u{1b}[0m (1 commit ignored)\n");
    }

    #[test]
    fn test_single_commit_with_debug() {
        compile_bin();
        let dir = test_dir("single_commit_with_debug");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Valid commit subject", "Valid message body", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-hints", "--debug"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout(predicate::str::contains(
            "1 commit and branch inspected, 0 errors detected (0 commits ignored)",
        ));
    }

    #[test]
    fn test_multiple_commit_invalid() {
        compile_bin();
        let dir = test_dir("multiple_commits_invalid");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "added some code", "This is a message.", "file1");
        create_commit_with_file(&dir, "Fixing tests", "", "file2");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "HEAD~2..HEAD"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        let output = normalize_output(&assert.get_output().stdout);

        assert!(predicate::str::contains(
            "Error[SubjectMood]: The subject does not use the imperative grammatical mood\n\
            \x20\x200000000:1:1: Fixing tests\n"
        )
        .eval(&output));
        assert!(predicate::str::contains(
            "Error[SubjectCapitalization]: The subject does not start with a capital letter\n\
            \x20\x200000000:1:1: added some code\n"
        )
        .eval(&output));
        assert.stdout(predicate::str::contains(
            "2 commits and branch inspected, 5 errors detected",
        ));
    }

    #[test]
    fn test_lint_hook() {
        compile_bin();
        let dir = test_dir("commit_file_option");
        create_test_repo(&dir);
        let filename = "commit_message_file";
        let commit_file = dir.join(filename);
        let mut file = File::create(&commit_file).unwrap();
        file.write_all(b"added some code\n\nThis is a message.")
            .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", &format!("--hook-message-file={}", filename)])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert
            .stdout(predicate::str::contains(
                "Error[SubjectMood]: The subject does not use the imperative grammatical mood",
            ))
            .stdout(predicate::str::contains(
                "Error[SubjectCapitalization]: The subject does not start with a capital letter",
            ))
            .stdout(predicate::str::contains(
                "Error[DiffPresence]: No file changes found",
            ))
            .stdout(predicate::str::contains(
                "1 commit and branch inspected, 3 errors detected",
            ));
    }

    #[test]
    fn test_file_option_with_file_changes() {
        compile_bin();
        let dir = test_dir("commit_file_option_with_file_changes");
        create_test_repo(&dir);
        create_dummy_file(&dir.join("file name"));
        stage_files(&dir);
        let filename = "commit_message_file";
        let commit_file = dir.join(filename);
        let mut file = File::create(&commit_file).unwrap();
        file.write_all(b"Valid subject\n\nValid message body.")
            .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", &format!("--hook-message-file={}", filename)])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout(predicate::str::contains(
            "1 commit and branch inspected, 0 errors detected",
        ));
    }

    #[test]
    fn test_file_option_with_scissors_cleanup() {
        compile_bin();
        let dir = test_dir("commit_file_option_with_scissors_cleanup_default_comment_char");
        create_test_repo(&dir);
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
            .args(["--no-color", &format!("--hook-message-file={}", filename)])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert.stdout(predicate::str::contains("Error[MessagePresence]: "));
    }

    #[test]
    fn test_file_option_with_scissors_cleanup_custom_comment_char() {
        compile_bin();
        let dir = test_dir("commit_file_option_with_scissors_cleanup_custom_comment_char");
        create_test_repo(&dir);
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
            .args(["--no-color", &format!("--hook-message-file={}", filename)])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert.stdout(predicate::str::contains("Error[MessagePresence]: "));
    }

    #[test]
    fn test_file_option_without_file() {
        compile_bin();
        let dir = test_dir("commit_file_option_without_file");
        create_test_repo(&dir);
        let filename = "commit_message_file";

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", &format!("--hook-message-file={}", filename)])
            .current_dir(dir)
            .assert()
            .failure()
            .code(2);
        assert.stderr(predicate::str::contains(
            "Lintje encountered an error: Unable to open commit message file: commit_message_file",
        ));
    }

    #[test]
    fn does_not_validate_changesets_in_repo_without_changesets() {
        compile_bin();
        let dir = test_dir("no_changesets");
        create_test_repo(&dir);

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.args(["--no-color"]).current_dir(dir).assert().success();
        assert.stdout(predicate::str::contains("Hint[DiffChangeset]").not());
    }

    #[test]
    fn validate_changesets_in_repo_with_changesets() {
        compile_bin();
        let dir = test_dir("with_changesets");
        create_test_repo(&dir);

        // Add commit with changeset
        let changeset_dir = dir.join(".changesets");
        std::fs::create_dir_all(&changeset_dir).expect("Could not create changeset dir");
        create_dummy_file(&changeset_dir.join("some-changeset.md"));
        stage_files(&changeset_dir);
        create_commit(
            &changeset_dir,
            "Add changeset for changelog",
            "Add a changeset file",
        );

        // Create new commit to be validated without changeset
        create_commit_with_file(&dir, "Test commit", "I am a test commit", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.args(["--no-color"]).current_dir(dir).assert().success();
        assert.stdout(predicate::str::contains("Hint[DiffChangeset]"));
    }

    #[test]
    fn test_branch_valid() {
        compile_bin();
        let dir = test_dir("branch_valid");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit.", "file");
        checkout_branch(&dir, "my-branch");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd.arg("--no-color").current_dir(dir).assert().success();
        assert.stdout(predicate::str::contains(
            "1 commit and branch inspected, 0 errors detected",
        ));
    }

    #[test]
    fn test_branch_invalid() {
        compile_bin();
        let dir = test_dir("branch_invalid");
        create_test_repo(&dir);
        checkout_branch(&dir, "fix-123");
        create_commit_with_file(&dir, "Test commit", "I am a test commit.", "file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-hints"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert
            .stdout(predicate::str::contains(
                "Error[BranchNameTicketNumber]: A ticket number was detected in the branch name\n\
                \x20\x20Branch:1: fix-123\n\
                \x20\x20| \n\
                \x20\x20| fix-123\n\
                \x20\x20| ------- Remove the ticket number from the branch name or expand the branch name with more details\n"
            ))
            .stdout(predicate::str::contains(
                "Error[BranchNameCliche]: The branch name does not explain the change in much detail\n\
                \x20\x20Branch:1: fix-123\n\
                \x20\x20| \n\
                \x20\x20| fix-123\n\
                \x20\x20| ^^^^^^^ Describe the change in more detail\n"
            ))
            .stdout(predicate::str::contains(
                "1 commit and branch inspected, 2 errors detected",
            ));
    }

    #[test]
    fn test_no_branch_validation() {
        compile_bin();
        let dir = test_dir("branch_invalid_disabled");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit.", "file");
        checkout_branch(&dir, "fix-123");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .args(["--no-color", "--no-branch"])
            .current_dir(dir)
            .assert()
            .success();
        assert.stdout(predicate::str::contains(
            "1 commit inspected, 0 errors detected",
        ));
    }

    #[test]
    fn options_file_present() {
        compile_bin();
        let dir = test_dir("options_config_file_present");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit.", "file");
        let config_filename = Path::new("options.txt");
        let config_file = dir.join(config_filename);
        create_file(&config_file, b"--debug\n--no-branch");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .arg("--no-color")
            .env("LINTJE_OPTIONS_PATH", config_filename)
            .current_dir(dir)
            .assert()
            .success();

        // Prints a debug statement (options file)
        // Does not validate branch (options file)
        // But does not print the output with color (CLI flag)
        assert
            .stderr("") // Does not print any warnings to STDERR
            .stdout(predicate::str::contains("[DEBUG]"))
            .stdout(predicate::str::contains(
                "1 commit inspected, 0 errors detected",
            ));
    }

    #[test]
    fn options_file_missing() {
        compile_bin();
        let dir = test_dir("options_config_file_missing");
        create_test_repo(&dir);
        create_commit_with_file(&dir, "Test commit", "I am a test commit.", "file");
        let config_filename = Path::new("options.txt");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .env("LINTJE_OPTIONS_PATH", config_filename)
            .current_dir(dir)
            .assert()
            .success();

        assert.stderr(predicate::str::contains(
            "ERROR: Configured LINTJE_OPTIONS_PATH does not exist or is not a file. Path: \'options.txt\'",
        ));
    }

    #[test]
    fn git_hook_install_commit_msg() {
        compile_bin();
        let dir = test_dir("git_hook_install_commit_msg");
        create_test_repo(&dir);

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .current_dir(&dir)
            .arg("--install-hook=commit-msg")
            .assert()
            .success();

        assert.stdout(predicate::str::contains(
            "Succesfully installed Lintje in hook: '.git/hooks/commit-msg'",
        ));

        let hook_file = dir.join(".git/hooks/commit-msg");
        let contents = std::fs::read_to_string(&hook_file).expect("Can't read hook file");
        assert_eq!(contents, "\n\nlintje --hook-message-file=$1");

        let permissions = hook_file.metadata().unwrap().permissions();
        assert_eq!(format!("{:#o}", permissions.mode()), "0o100744");
    }

    #[test]
    fn git_hook_install_post_commit() {
        compile_bin();
        let dir = test_dir("git_hook_install_post_commit");
        create_test_repo(&dir);

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .current_dir(&dir)
            .arg("--install-hook=post-commit")
            .assert()
            .success();

        assert.stdout(predicate::str::contains(
            "Succesfully installed Lintje in hook: '.git/hooks/post-commit'",
        ));

        let hook_file = dir.join(".git/hooks/post-commit");
        let contents = std::fs::read_to_string(hook_file).expect("Can't read hook file");
        assert_eq!(contents, "\n\nlintje");
    }

    #[test]
    fn git_hook_install_with_file_exists() {
        compile_bin();
        let dir = test_dir("git_hook_install_with_file_exists");
        create_test_repo(&dir);

        // Write some content to the file beforehand
        let hook_file = dir.join(".git/hooks/post-commit");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&hook_file)
            .expect("Could not open file");
        file.write_all(b"Other content")
            .expect("Could not write to file");

        let mut cmd = assert_cmd::Command::cargo_bin("lintje").unwrap();
        let assert = cmd
            .current_dir(&dir)
            .arg("--install-hook=post-commit")
            .assert()
            .success();

        assert.stdout(predicate::str::contains(
            "Succesfully installed Lintje in hook: '.git/hooks/post-commit'",
        ));

        let contents = std::fs::read_to_string(hook_file).expect("Can't read hook file");
        assert_eq!(contents, "Other content\n\nlintje");
    }
}
