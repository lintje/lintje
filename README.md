# Lintje

Lintje is an opinionated linter for Git. It lints commit messages based on a
preconfigured set of [rules][rules] focussed on promoting communication between
people. The idea is to write commits meant for other people reading them during
reviews and debug sessions 2+ months from now.

- __No configuration__. Don't spend time configuring your Git commit linter and
  instead adopt a preconfigured set of [rules][rules].
- __Portable__. Lintje is a Rust project built for several
  [Operating Systems](#supported-operating-systems) and has no dependencies.
  Drop it into your project and it works.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Rules documentation][rules]
- [Configuration](#configuration)
- [Development documentation](#development)

## Example

Given the last commit in a project is this:

```
Fix bug
```

When running `lintje` to lint the last commit, the output will be:

```
$ lintje
6162010: Fix bug
  SubjectCliche: Subject is a 'Fix bug' commit.
  MessagePresence: Add a message body to provide more context about the change
    and why it was made.

1 commit inspected, 2 violations detected
```

## Installation

### macOS

To install Lintje on macOS use the [Homebrew](https://brew.sh/) tap.

```
brew tap tombruijn/lintje
brew install lintje
```

### Linux

To install Lintje on Linux download the Linux archive from the latest release
on the [releases page](https://github.com/tombruijn/lintje/releases). Then
extract it to a directory in your `$PATH` so the `lintje` executable is
available in any directory.

### Supported Operating Systems

- Apple macOS:
    - x86 64-bit (`x86_64-apple-darwin`)
    - ARM 64-bit (`aarch64-apple-darwin`) (Apple Silicon)
- Linux GNU:
    - x86 64-bit (`x86_64-unknown-linux-gnu`)
    - ARM 64-bit (`aarch64-unknown-linux-gnu`)

## Usage

```
# Lint the most recent commit on the current branch
lintje
# Is the same as:
lintje HEAD
# Lint a specific commit
lintje 3a561ef766c2acfe5da478697d91758110b8b24c

# Select a range of commits
# Lint the last 5 commits:
lintje HEAD~5..HEAD
# Lint the difference between two branches
lintje main..develop
```

It's recommended to add Lintje to your CI setup to lint the range of commits
added by a Pull Request or job.

### Exit codes

Lintje will exit with the following status codes in these situations:

- `0` (Success) - No violations have been found. The commit is accepted.
- `1` (Failure) - One or more violations have been found. The commit is not
  accepted.
- `2` (Error) - An internal error occurred and the program had to exit. This is
  probably a bug, please report it in the [issue tracker][issues].

### Git hook

To lint the commit locally immediately after writing the commit message, use a
Git hook. To add it, run the following:

```
echo "lintje --hook-message-file=\$1" >> .git/hooks/commit-msg
chmod +x .git/hooks/commit-msg
```

If Lintje fails the commit is aborted. The message you entered is available
in `.git/COMMIT_EDITMSG` and you can restart your commit message with:

```
git commit --edit --file=.git/COMMIT_EDITMSG
```

Personally I don't like how it fails the commit process and makes the commit
message harder to reach to use again. It also makes making fixup commits really
difficult. Instead I prefer not failing the commit hook and amending the commit
afterwards to fix any issues that came up. The example below will have Lintje
output the issues it found, but still make the commit. You can then amend the
commit to fix any issues it found afterwards.

```
echo "lintje --hook-message-file=\$1 || echo \"\\\nLintje failure\"" >> .git/hooks/commit-msg
chmod +x .git/hooks/commit-msg
```

### Git alias

It's possible to set up an alias with Git to use `git lint` as the command
instead, or any other alias you prefer.

Set up your alias with the following line.

```
git config --global alias.lint '!lintje'
```

You'll then be able to call it like the examples below and any other methods
listed in [usage](#usage).

```
git lint
git lint main..develop
```

## Rules

For more information on which rules are validated on, see the [rules docs
page][rules].

## Configuration

Lintje does not have a configuration file where you can enable/disable/configure
certain rules for an entire project.

Instead it's possible to [ignore specific rules per
commit](#ignoring-rules-per-commit).

### Ignoring rules per commit

It's possible to ignore certain rules for a commit, but this be used very
infrequently. If you think Lintje should handle a certain scenario better,
please [create an issue][issues] explaining your use case.

To ignore a rule in a specific commit, use the magic `lintje:disable` comment.

Start a new line (preferably at the end of the commit message) that starts with
`lintje:disable` and continue specifying the rule you want to ignore, such as:
`lintje:disable SubjectPunctuation`.

Example commit with multiple ignored rules:

```
This is a commit subject!!

This is a commit message line 1.
Here is some more content of the commit message is very long for valid reasons.

lintje:disable SubjectPunctuation
lintje:disable MessageLineTooLong
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

Before release all the supported targets will be build. See
[Building](#building) for more information about the build step.

To release all different targets, run the release script:

```
script/release
```

The release will be pushed to GitHub.

Finally update the
[Lintje Homebrew tap](https://github.com/tombruijn/homebrew-lintje).

[rules]: doc/rules.md
[issues]: https://github.com/tombruijn/lintje/issues
