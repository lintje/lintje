# Git Lint

Git Lint is an opinionated linter for Git.

- [Rules documentation][rules]

Git Lint is primarily focussed on the English languages, other languages may
not be compatible with every [rule][rules].

## Example

Given the last commit in a project is this:

```
Fix bug
```

When running `gitlint lint` to lint the last commit, the output will be:

```
$ gitlint lint
6962010: Fix bug
  SubjectCliche: Subject is a 'Fix bug' commit.
  MessagePresence: Add a message body to provide more context about the change
    and why it was made.

1 commit inspected, 2 violations detected
```

## Installation

TODO

### Supported Operating Systems

- Apple macOS:
    - x86 64-bit
    - ARM 64-bit (Apple Silicon)
- Linux:
    - x86 64-bit
    - ARM 64-bit

## Usage

```
# Lint the most recent commit on the current branch
gitlint lint
# Is the same as:
gitlint lint HEAD~1..HEAD

# Select a range of commits
# Lint the last 5 commits:
gitlint lint HEAD~5..HEAD
# Lint the difference between two branches
gitlint lint main..develop
```

It's recommended to add Git Lint to your CI setup to lint the range of commits
added by a Pull Request or job.

### Exit codes

Git Lint will exit with the following status codes in these situations:

- `0` (Success) - No violations have been found. The commit is accepted.
- `1` (Failure) - One or more violations have been found. The commit is not
  accepted.
- `2` (Error) - An internal error occurred and the program had to exit. This is
  probably a bug, please report it in the [issue tracker][issues].

### Git hook

To lint the commit locally immediately after writing the commit message, use a
Git hook. To add it, run the following:

```
echo "gitlint lint-hook --message-file=\$1" >> .git/hooks/commit-msg
chmod +x .git/hooks/pre-commit
```

## Rules

For more information on which rules are linted on, see the [rules docs
page][rules].

## Configuration

Git lint does not have a configuration file where you can
enable/disable/configure certain rules for an entire project.

Instead it's possible to [ignore specific rules per
commit](#ignoring-rules-per-commit).

### Ignoring rules per commit

To ignore a rule in a specific commit, use the magic `gitlint:disable` comment.

Start a new line (preferably at the end of the commit message) that starts with
`gitlint:disable` and continue specifying the rule you want to ignore, such as:
`gitlint:disable SubjectPunctuation`.

Example commit with multiple ignored rules:

```
This is a commit subject!!

This is a commit message line 1.
Here is some more content of the commit message is very long for valid reasons.

gitlint:disable SubjectPunctuation
gitlint:disable MessageLineTooLong
```

## Development

### Setup

Make sure [Rust](https://www.rust-lang.org/) is installed before continuing.

```
cargo build
```

### Testing

```
cargo test
```

### Building

[Docker](https://www.docker.com/) is required to build all the different target
releases using [cross](https://github.com/rust-embedded/cross).

To build all different targets, run the build script:

```
script/build
```

The build output can be found in the `dist/` directory.

### Releases

See [Building](#building) for more information about the build step.

To release all different targets, run the release script:

```
script/release
```

The release will be pushed to GitHub.

[rules]: doc/rules.md
[issues]: https://github.com/tombruijn/gitlint-private/issues
