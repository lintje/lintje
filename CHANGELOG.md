# Lintje changelog

## Next version

- Ignore SubjectLength rule if the subject already has a SubjectCliche
  violation. This reduces the number of violation that are printed when a
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
