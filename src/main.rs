#[macro_use]
extern crate log;

#[cfg(test)]
extern crate predicates;

use log::LevelFilter;
use structopt::StructOpt;

mod commit;
mod git;
mod logger;

use git::fetch_and_parse_commits;
use logger::Logger;

#[derive(StructOpt, Debug)]
#[structopt(name = "gitlint")]
struct GitLint {
    /// Prints debug information
    #[structopt(short, long)]
    debug: bool,

    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Lint commits
    ///
    /// Usage examples
    ///
    ///     gitlint lint
    ///       Validate the latest commit.
    ///
    ///     gitlint lint HEAD~5..HEAD
    ///       Validate the last 5 commits.
    ///
    #[structopt(name = "lint", verbatim_doc_comment)]
    Lint(Options),
}

#[derive(Debug, StructOpt)]
struct Options {
    /// Lint only commits in the specified revision range. When no <revision range> is specified,
    /// it defaults to only linting the latest commit.
    // #[structopt(parse(from_os_str))]
    #[structopt(name = "revision range")]
    revision_range: Option<String>,
}

fn main() {
    let args = GitLint::from_args();
    init_logger(args.debug);
    match args.command {
        Command::Lint(command) => {
            lint(command);
        }
    }
}

fn lint(options: Options) {
    let commit_result = fetch_and_parse_commits(options.revision_range);
    debug!("Commits: {:?}", commit_result);
    match commit_result {
        Ok(commits) => {
            let mut valid = true;
            for commit in commits {
                if !commit.is_valid() {
                    println!("{}: {}", commit.short_sha, commit.subject);
                    for validation in commit.validations {
                        valid = false;
                        println!("  {}: {}", validation.kind, validation.message);
                    }
                }
            }
            if !valid {
                std::process::exit(1)
            }
        }
        Err(e) => error!("{}", e),
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
            std::process::exit(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use predicates::prelude::*;
    use std::fs;
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
                "Failed to make commit!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output.status.code().expect("foo"),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
        create_commit(&dir, "Initial commit", "");
        for (subject, message) in commits {
            create_commit(&dir, subject, message)
        }
    }

    fn create_commit(dir: &PathBuf, subject: &str, message: &str) {
        let mut args = vec![
            "commit".to_string(),
            "--allow-empty".to_string(),
            format!("-m {}", subject),
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
            .expect(&format!("Failed to make commit: {}, {}", subject, message));
        if !output.status.success() {
            panic!(
                "Failed to make commit!\nExit code: {}\nSDTOUT: {}\nSTDERR: {}",
                output.status.code().expect("foo"),
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
    fn test_single_commit_valid() {
        compile_bin();
        let dir = test_dir("single_commit_valid");
        create_test_repo(
            &dir,
            &[("Test commit", "I am a test commit, short but valid.")],
        );

        let mut cmd = assert_cmd::Command::cargo_bin("gitlint").unwrap();
        let assert = cmd.arg("lint").current_dir(dir).assert().success();
        assert.stdout("");
    }

    #[test]
    fn test_single_commit_invalid() {
        compile_bin();
        let dir = test_dir("single_commit_invalid");
        create_test_repo(&dir, &[("added some code", ""), ("Fixing tests", "")]);

        let mut cmd = assert_cmd::Command::cargo_bin("gitlint").unwrap();
        let assert = cmd.arg("lint").current_dir(dir).assert().failure().code(1);
        assert.stdout(predicate::str::contains(
            "Fixing tests\n\
            \x20\x20SubjectMood: Subject is not imperative mood.\n\
            \x20\x20MessagePresence: Message is not present.",
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

        let mut cmd = assert_cmd::Command::cargo_bin("gitlint").unwrap();
        let assert = cmd
            .args(&["lint", "HEAD~2..HEAD"])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert
            .stdout(predicate::str::contains(
                "added some code\n\
                \x20\x20SubjectMood: Subject is not imperative mood.\n\
                \x20\x20SubjectCapitalization: Subject does not start with a capital letter.",
            ))
            .stdout(predicate::str::contains(
                "Fixing tests\n\
                \x20\x20SubjectMood: Subject is not imperative mood.\n\
                \x20\x20MessagePresence: Message is not present.",
            ));
    }
}
