#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate predicates;

use log::LevelFilter;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use structopt::StructOpt;

mod commit;
mod git;
mod logger;
mod rule;

use git::{fetch_and_parse_commits, parse_commit_file_format};
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
    /// Lint the contents of a specific file. Useful for the commit-msg Git hook.
    #[structopt(long, parse(from_os_str))]
    file: Option<PathBuf>,
    /// Lint only commits in the specified revision range. When no <revision range> is specified,
    /// it defaults to only linting the latest commit.
    #[structopt(name = "revision range")]
    revision_range: Option<String>,
}

fn main() {
    let args = GitLint::from_args();
    init_logger(args.debug);
    match args.command {
        Command::Lint(command) => match lint(command) {
            Ok(_) => (),
            Err(e) => {
                error!("An error occurred: {}", e);
                std::process::exit(2)
            }
        },
    }
}

fn lint(options: Options) -> Result<(), String> {
    let commits = match options.file {
        Some(filename) => match File::open(&filename) {
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
                match parse_commit_file_format(&contents, git::cleanup_mode(), git::comment_char())
                {
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
        },
        None => {
            let commit_result = fetch_and_parse_commits(options.revision_range);
            match commit_result {
                Ok(commits) => commits,
                Err(e) => return Err(e),
            }
        }
    };

    debug!("Commits: {:?}", commits);
    let commit_count = commits.len();
    let mut violation_count = 0;
    for commit in commits {
        if !commit.is_valid() {
            match commit.short_sha {
                Some(sha) => println!("{}: {}", sha, commit.subject),
                None => println!("{}", commit.subject),
            }

            for violation in commit.violations {
                violation_count += 1;
                println!("  {}: {}", violation.rule, violation.message);
            }
        }
    }

    if violation_count > 0 {
        println!("");
    }
    let plural = if commit_count != 1 { "s" } else { "" };
    println!(
        "{} commit{} inspected, {} violations detected",
        commit_count, plural, violation_count
    );
    if violation_count == 0 {
        Ok(())
    } else {
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
            "--no-gpg-sign".to_string(),
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
        assert.stdout("1 commit inspected, 0 violations detected\n");
    }

    #[test]
    fn test_single_commit_invalid() {
        compile_bin();
        let dir = test_dir("single_commit_invalid");
        create_test_repo(&dir, &[("added some code", ""), ("Fixing tests", "")]);

        let mut cmd = assert_cmd::Command::cargo_bin("gitlint").unwrap();
        let assert = cmd.arg("lint").current_dir(dir).assert().failure().code(1);
        assert
            .stdout(predicate::str::contains(
                "Fixing tests\n\
                \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
                \x20\x20MessagePresence: Add a message body to provide more context about the change and why it was made.",
            ))
            .stdout(predicate::str::contains(
                "1 commit inspected, 2 violations detected\n",
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
                \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
                \x20\x20SubjectCapitalization: Start the commit subject a capital letter.",
            ))
            .stdout(predicate::str::contains(
                "Fixing tests\n\
                \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
                \x20\x20MessagePresence: Add a message body to provide more context about the change and why it was made.",
            ))
            .stdout(predicate::str::contains(
                "2 commits inspected, 4 violations detected\n",
            ));
    }

    #[test]
    fn test_file_option() {
        compile_bin();
        let dir = test_dir("commit_file_option");
        create_test_repo(&dir, &[]);
        let filename = "commit_message_file";
        let commit_file = dir.join(filename);
        let mut file = File::create(&commit_file).unwrap();
        file.write_all(b"added some code\n\nThis is a message.")
            .unwrap();

        let mut cmd = assert_cmd::Command::cargo_bin("gitlint").unwrap();
        let assert = cmd
            .args(&["lint", &format!("--file={}", filename)])
            .current_dir(dir)
            .assert()
            .failure()
            .code(1);
        assert.stdout(predicate::str::contains(
            "added some code\n\
             \x20\x20SubjectMood: Use the imperative mood for the commit subject.\n\
             \x20\x20SubjectCapitalization: Start the commit subject a capital letter.",
        ));
    }

    #[test]
    fn test_file_option_without_file() {
        compile_bin();
        let dir = test_dir("commit_file_option_without_file");
        create_test_repo(&dir, &[]);
        let filename = "commit_message_file";

        let mut cmd = assert_cmd::Command::cargo_bin("gitlint").unwrap();
        let assert = cmd
            .args(&["lint", &format!("--file={}", filename)])
            .current_dir(dir)
            .assert()
            .failure()
            .code(2);
        assert.stdout(predicate::str::contains(
            "Unable to open commit message file: commit_message_file",
        ));
    }
}
