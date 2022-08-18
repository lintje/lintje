use clap::{AppSettings, Parser};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const IGNORED_CLAP_ERRORS: [clap::error::ErrorKind; 2] = [
    clap::error::ErrorKind::DisplayHelp,
    clap::error::ErrorKind::DisplayVersion,
];

#[derive(Parser, Debug)]
#[clap(
    name = "lintje",
    version,
    verbatim_doc_comment,
    setting(AppSettings::DeriveDisplayOrder)
)]
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

    lintje --color
      Enable color output.

## Options file

    Use an options file to add defaults every Lintje call. Configure the
    options file path with the `LINTJE_OPTIONS_PATH` system environment
    variable.

         # Linux and macOS example
         export LINTJE_OPTIONS_PATH="$HOME/.config/lintje/options.txt"

    In the options file, specify the options one or more per line.
    Lines starting with the number symbol (#) are ignored.

         # Lintje default options
         # Enable color
         --color
         # Disable hints
         --no-hints

         # Or all one one line
         --color --no-hints

    http://lintje.dev/docs/configuration/#options-file
*/
pub struct Lint {
    /// Disable branch validation
    #[clap(long = "no-branch", parse(from_flag = std::ops::Not::not))]
    pub branch_validation: bool,

    /// Disable hints
    #[clap(long = "no-hints", parse(from_flag = std::ops::Not::not))]
    pub hints: bool,

    /// Enable color output
    #[clap(long = "color")]
    pub color: bool,

    /// Disable color output
    #[clap(long = "no-color")]
    pub no_color: bool,

    /// Lint the contents the Git hook commit-msg commit message file.
    /// This will usually be `.git/COMMIT_EDITMSG`.
    #[clap(long, name = "commit message file path", parse(from_os_str))]
    pub hook_message_file: Option<PathBuf>,

    /// Prints debug information
    #[clap(long)]
    pub debug: bool,

    /// Lint commits by Git commit SHA or by a range of commits. When no <commit> is specified, it
    /// defaults to linting the latest commit.
    #[clap(name = "commit (range)")]
    pub selection: Option<String>,
}

impl Lint {
    /// Return color config option value
    pub fn color(&self) -> bool {
        if self.no_color {
            return false;
        }
        if self.color {
            return true;
        }
        true // By default color is turned on
    }

    pub fn merge(&mut self, options: Vec<String>) {
        self.update_from(options);
    }
}

#[derive(Debug)]
pub struct ValidationContext {
    pub changesets: bool,
}

pub fn fetch_options() -> Lint {
    let cli_opts = cli_options();
    match file_options(env::var("LINTJE_OPTIONS_PATH")) {
        Some((path, file_options)) => {
            // Merge CLI options with options file if a options file was successfully
            // parsed.
            let mut opts = parse_file_options(&path, &file_options);
            opts.merge(cli_opts);
            opts
        }
        None => Lint::parse_from(cli_opts),
    }
}

// Return unparsed CLI options and flags
fn cli_options() -> Vec<String> {
    env::args_os()
        .filter_map(|a| match a.into_string() {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Unable to parse CLI argument: '{:?}'", e);
                None
            }
        })
        .collect::<Vec<String>>()
}

// Return unparsed options file options and flags
fn file_options(env_path: Result<String, std::env::VarError>) -> Option<(PathBuf, Vec<String>)> {
    match env_path {
        Ok(value) => {
            let path = Path::new(&value);
            if path.is_file() {
                match fs::read_to_string(&path) {
                    Ok(contents) => Some((path.to_path_buf(), parse_options_file(&contents))),
                    Err(e) => {
                        eprintln!("ERROR: Lintje options file could not be read: {}", e);
                        None
                    }
                }
            } else {
                eprintln!(
                    "ERROR: Configured LINTJE_OPTIONS_PATH does not exist or is not a file. Path: '{}'",
                    path.display()
                );
                None
            }
        }
        Err(_) => None,
    }
}

fn parse_options_file(contents: &str) -> Vec<String> {
    contents
        .lines()
        .into_iter()
        .filter(|line| !line.starts_with('#')) // Filter out comment lines
        .flat_map(|line| {
            // Split up words so multiple flags on one line work
            line.split(' ')
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()
        })
        .collect::<Vec<String>>()
}

fn parse_file_options(path: &Path, options: &[String]) -> Lint {
    let mut opts = vec!["lintje".to_string()];
    opts.append(&mut options.to_owned());
    match Lint::try_parse_from(&opts) {
        Ok(opts) => opts,
        Err(e) => {
            // Only print parse error when clap errors aren't used to print help or version
            // information
            if !IGNORED_CLAP_ERRORS.contains(&e.kind()) {
                eprintln!("ERROR: Error parsing options file: {:?}", path);
            }
            e.exit()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{file_options, parse_options_file, Lint};
    use crate::test::*;
    use clap::Parser;
    use std::path::{Path, PathBuf};

    fn test_dir(name: &str) -> PathBuf {
        Path::new(TEST_DIR).join(name)
    }

    #[test]
    fn color_flags() {
        // Both color flags set, but --no-color is leading
        assert!(!Lint::parse_from(["lintje", "--color", "--no-color"]).color());

        // Only --color is set
        assert!(Lint::parse_from(["lintje", "--color"]).color());

        // Only --no-color is set
        assert!(!Lint::parse_from(["lintje", "--no-color"]).color());

        // No flags are set
        assert!(Lint::parse_from(["lintje"]).color());
    }

    #[test]
    fn merge_options() {
        let mut opts = Lint::parse_from(vec![
            "lintje".to_string(),
            "--color".to_string(),
            "--no-branch".to_string(),
        ]);
        opts.merge(vec![
            "lintje".to_string(),
            "--no-color".to_string(),
            "--no-hints".to_string(),
        ]);
        assert_eq!(opts.color, true);
        assert_eq!(opts.no_color, true);
        assert_eq!(opts.color(), false);
        assert_eq!(opts.branch_validation, false);
        assert_eq!(opts.hints, false);
    }

    #[test]
    fn options_file_valid() {
        let dir = test_dir("options_file_valid");
        let env_path = dir.join("options.txt");
        prepare_test_dir(&dir);
        create_file(&env_path, b"--color\n--no-hints --no-branch");

        let (path, options) =
            file_options(Ok(env_path.as_path().display().to_string())).expect("No options");
        assert_eq!(path, env_path);
        assert_eq!(options, vec!["--color", "--no-hints", "--no-branch"]);
    }

    #[test]
    fn options_file_invalid() {
        let env_path = PathBuf::from("test_options.txt");

        assert_eq!(
            file_options(Ok(env_path.as_path().display().to_string())),
            None
        );
    }

    #[test]
    fn options_file_none() {
        assert_eq!(file_options(Err(std::env::VarError::NotPresent)), None);
    }

    #[test]
    fn parse_options_file_multi_line() {
        let options = parse_options_file("--color\n--no-hints\n--no-branch");
        assert_eq!(options, vec!["--color", "--no-hints", "--no-branch"]);
    }

    #[test]
    fn parse_options_file_single_line() {
        let options = parse_options_file("--color --no-hints --no-branch");
        assert_eq!(options, vec!["--color", "--no-hints", "--no-branch"]);
    }

    #[test]
    fn parse_options_file_ignore_comments() {
        let options = parse_options_file("# Set color\n--color\n# Disable hints\n--no-hints");
        assert_eq!(options, vec!["--color", "--no-hints"]);
    }
}
