# Rules

All the rules Git Lint follows are documented on this page. The heading name matches the rule name, and can be used to [ignore specific rules per commit](../README.md#ignoring-rules-per-commit)

## MergeCommit

The commit is detected as a merge commit, which is a commit merging one branch
into another. Prefer rebasing feature branches instead of merging base branches
back into them. These commits don't communicate anything meaningful.

For example, when the base branch of your feature branch has new changes you
need to include in your feature branch as well, rebase the feature branch on
top of the updated base branch.

```
# Checkout the base branch
git checkout main
# Fetch the latest changes
git pull origin main
# Checkout and rebase your feature branch on the base branch
git checkout feature-branch
git rebase --interactive main
```

Commits from Pull and Merge requests will not fail on this rule, they are
ignored, as they communicate when a Pull/Merge request was accepted and merged
into the base branch.

## NeedsRebase

The commit is detected as a fixup or squash commit. These commits communicate
the intent to squash them into other commits during the next rebase. These
commits should not be send in for review in Pull Requests, and they should not
be merged into main branches.

```
git checkout feature-branch
git rebase --interactive --autosquash main
```

## SubjectLength

The commit's subject is considered too short or too long.

Short commit subjects like "WIP" and "Fix" don't explain the change well enough.
Don't be afraid to dive into a little bit more detail to explain the change.

The commit's subject should be a maximum of 50 characters long. If the subject
is longer than 50 characters, reword the subject to fit in the maximum subject
length. Use the commit's message body to explain the change in greater detail.

## SubjectMood

## SubjectCapitalization

The commit's subject doesn't start with a capital letter. Use a capital letter
to start your subject.

## SubjectPunctuation

The commit's subject ends with punctuation. Subjects don't need to end with
punctuation.

## SubjectTicketNumber

The commit's subject includes a reference to a ticker or issue. Move this to
the message body.

Invalid subject examples:

```
Fix #123
I have fixed #123
JIRA-123
Fix JIRA-123 for good
```

## SubjectCliche

The commit's subject is considered to be a cliché, it's overused and adds
little meaning. Expand the subject to be more descriptive.

Some examples:

- WIP
- Fix bug
- Fix test
- ...

## MessagePresence

The commit's message body is empty or too short. Add a message body to the
commit to elaborate on _why_ the change was necessary, what alternatives you
considered and why you chose this particular implementation as a solution.

## MessageLineLength

The commit's message body has one or more lines that are too long. The maximum
line length in a message body is 72 characters. Split sentences and paragraph
across multiples lines.
