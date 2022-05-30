# Lintje changelog

## Next version

### Added

- Check commit messages for "Part of #123" substring for MessageTicketNumber
  hint. This is also accepted along with "Fixes #123" and "Closes #123".

### Changed

- Print CLI flags in a most common usage based order. Flags to are opposites of
  each other, like `--color` and `--no-color`, are grouped together.

## 0.7.0

### Added

- Introducing suggestions! Lintje will print more detailed hints to resolve
  errors. Rules that have been updated with suggestions are the
  SubjectTicketNumber and SubjectBuildTag rules.
  ```
  SubjectTicketNumber: The subject contains a ticket number
    9aa9ca2:1:10: Fix bug. Closes #123
      |
    1 | Fix bug. Closes #123
      |          ^^^^^^^^^^^ Remove the ticket number from the subject
     ~~~
    9 | Closes #123
      | ----------- Move the ticket number to the message body
  ```
- Introducing hints! Lintje will print messages for issues that are not
  necessarily errors with the new MessageTicketNumber rule, but reminders to
  add ticket numbers to the commit message body. Hints can be turned off with
  the `--no-hints` flag.
  ```
  MessageTicketNumber: The message body does not contain a ticket or issue number
    9aa9ca2:10:1: Commit subject
       |
     8 | Last line of the current commit message body.
     9 |
    10 | Fixes #123
       | ---------- Consider adding a reference to a ticket or issue
  ```
- Add color to output. Highlight important output labels, such as rule names,
  the underlining, and the result status. Experimental color output can be
  enabled with the new `--color` flag.

### Changed

- Change "violations" to "issues" in preparation for different types issues
  Lintje can return in the future.

### Fixed

- Improve output when a Git error is encountered.
- Reduce false positive detection for ticket numbers in branch names. Branch
  names with version numbers in them, like `ruby-3` and `elixir-1.12`, are now
  valid.

## 0.6.1

- Ignore other rules if a commit has a MergeCommit or NeedsRebase violation.
  When these violations occur the commit needs to be rebased, so any other
  issues will hopefully be resolved in the rebase, such missing message body,
  or subject length. This will reduce the number of violations printed and
  focus on the important violations.
- The Git scissor line (for cleanup mode scissors) will not be interpreted as
  the commit subject line, if it's the first line in a Git commit hook file. It
  will instead consider the commit as having an empty subject and message body.
  This will prevent any unexpected violations on the scissor line when the Git
  commit process is aborted by removing the subject and message body from the
  Git commit message file.
- The Git scissor line will be interpreted as the end of every commit message.
  This previously only applied for the scissors cleanup mode. This improves
  support for `git commit`'s `--verbose` flag and `--cleanup` option. In
  verbose mode the scissor line is also present in the Git commit default
  message content, but is not included in the committed message body.
- Don't consider trailing whitespace as part of the line length in the scissors
  cleanup mode.
- Improve leading empty line detection and ignore these lines in every cleanup
  mode except "verbatim". This way leading empty lines are not interpreted as
  subjects and Lintje won't print violations about those empty lines as
  subjects.
- Improve leading comment line detection and ignore these lines in the
  default/"strip" cleanup mode. This way leading comment lines are not
  interpreted as subjects and Lintje won't print violations about those
  subjects.

## 0.6.0

- Improve Unicode support for SubjectLength and MessageLineLength rules.
    - Characters with accents such as `a̐` are no longer counted as two
      characters.
    - Double width characters now count towards a width of two.
    - Emoji with a larger display width are now counted with
      their display width. This means it's no longer possible to write a
      subject of 50 emoji in a subject, only 25 emoji that have a render with
      of two, for example.
- Improved violation messages.
    - When a violation of the Lintje rules are found the message that gets
      printed will includes more context about the problem it found. It will
      highlight where exactly the problem was detected in a commit subject,
      commit message, commit diff or branch name to make it easier to resolve
      the problem.

## 0.5.0

- Ignore SubjectLength rule if the subject already has a SubjectCliche
  violation. This reduces the number of violations that are printed when a
  SubjectCliche violation means writing a longer subject anyway.
- Ignore SubjectCapitalization and MessagePresence rules if the subject already
  has a NeedsRebase violation. To fix a NeedsRebase violation the commit needs
  to be rebased into the commit it's marked to fixup or squash, and there will
  be no need to fix the capitalization or add a message body.
- Add DiffPresence rule. This rule whether or not the commit has any changes or
  not. When a commit is empty, it will print a violation.
- Improve SubjectLength violation message when the subject is completely empty.
- Remove error messages from output when the commit subject is empty.
- Match more build tags in the SubjectBuildTag rule. It now also matches all
  tags that match the format of "[skip *]" and "[* skip]", rather than a
  previously fixed list of build tags.
- Match fewer substrings as ticket numbers, strings like "A-1" no longer
  matches.
- Ignore SubjectCapitalization rule if the subject already has a SubjectPrefix
  violation. This reduces the number of violations that are printed when a
  prefix is found in the commit, which is the violation that takes priority.

## 0.4.1

- Fix error handling for Git hook mode when no `core.commentChar` or
  `commit.cleanup` is configured in Git.

## 0.4.0

- Better handling of Git commands when they fail. Print an error message when a
  Git command fails (like calling `git log`) and when Git is not
  installed.
- Improve SubjectCliche rule to catch plurals of words (e.g. "fix tests") and
  check for more subject prefixes like "add fix", "update code", "remove file".
- Improve wording of the SubjectCapitalization violation message.
- Add branch name validation.
    - Can be disabled with the `--no-branch` flag.
    - New BranchNameTicketNumber rule to scan branch names for ticket numbers,
      and `fix-###` formats. Ticket numbers are accepted as long as the name is
      more than a combination of a prefix and number.
    - New BranchNameLength rule checks for a minimum branch name length of four
      characters.
    - New BranchNamePunctuation rule checks for a branch names starting or
      ending with punctuation.
    - New BranchNameCliche rule checks for a branch names is a cliché.
      "fix-bug" or "add-test" branches are no longer accepted.
- Fix emoji false positives in SubjectPunctuation. It will no longer match on
  numbers and * and # as emoji at the start of a subject.
- Ignore commits made by GitHub bots. Project members can't always ensure that
  all bots follow the rules set by Lintje.
- Add Debian installation method. More information in the
  [installation docs](doc/installation.md).
- Print the singular "violation" label when Lintje only finds one violation.
- Print number of ignored commits, if any commits are ignored.

## 0.3.1

- Improve MergeCommit rule to fail on less types of merge commits. A local
  merge commit into the repository's base branch is accepted, but a merge
  commit merging a remote branch or two non-base-branch into one another are
  not. In the future this may warn on certain local merges again.
- Ignore merge commits for tags. These commits are local merges that will be
  ignored for checks for now. They may trigger the MergeCommit rule in the
  future, when local merges can be detected.
- Fix GitLab merge commit detection, to ignore those commits. It previously
  only scanned for Merge Request reference IDs, but now scans for the full
  `org/repo!id` reference used by GitLab in Merge Request merge commits.
- Better detect GitLab Merge Request references. Update the SubjectTicketNumber
  rule to also detect references to Merge Requests in GitLab that uses
  exclamation marks `!` instead of `#`.

## 0.3.0

- Add SubjectBuildTag rule to check for "skip ci" tags in the subject. These
  tags should be moved to the message body.
- Add SubjectPrefix rule to explicitly check for prefixes in subjects, like
  "fix: bug", "fix!: bug", "fix(scope): bug", and suggest to remove them.
- Update SubjectCliche to catch more types of clichés, such as only "fix". The
  check is now also case insensitive, so "Fix", "fix" and "FIX" are all caught.

## 0.2.0

- Add MessageEmptyFirstLine rule that checks if the line after the subject line
  is empty. If it's not empty that line is considered part of the commit's
  subject.
- Validate commits without a subject. Previously these commits would be ignored
  and Lintje would not validate them, missing very undescriptive commits.
- Add Alpine Linux musl compatible build.
- Don't validate commit's which are squash commits from GitHub Pull Requests.
  It's not recommended to rewrite merge commits after they've been made, so
  they will be ignored.
- Expand SubjectPunctuation rule to also scan for punctuation at the start of
  the subject, not just the end. Subjects should not start with punctuation.
- Expand SubjectPunctuation rule to also scan for emoji at the start of
  the subject. Subjects should not start with an emoji as a prefix.
- Expand SubjectPunctuation rule to check for more Unicode punctuation.
- Add Microsoft Windows release build.

## 0.1.0

Initial release.
