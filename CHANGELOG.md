# Lintje changelog

## Next version

- Better handling of Git commands when they fail. Print an error message when a
  Git command fails (like calling `git log`) and when Git is not
  installed.

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
