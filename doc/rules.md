# Rules

All the rules Lintje follows are documented on this page. The heading name
matches the rule name, and can be used to [ignore specific rules per
commit](../README.md#ignoring-rules-per-commit).

_Lintje is primarily focussed on supporting the English language, other
languages may not be compatible with every rule currently. Please
[create an issue](https://github.com/tombruijn/lintje/issues) if you run into
any problems._

## What type of rules is Lintje configured for?

Lintje is written to promote communication between people within Git commits.
Write commit subjects and messages meant for other people reading these commits
during reviews and debug sessions 2+ months from now.

It checks for commits like "Fix bug" and instead prefers commits that explain
changes in more detail. Explain why the change was necessary, what alternatives
were considered and why this solution was chosen. This will provide much needed
context to future readers so they understand what kind of constraints the
commit was made under.

Inspiration for Lintje's rules:

- [Git is about communication](https://tomdebruijn.com/posts/git-is-about-communication/)
  by Tom de Bruijn.
- [A Note About Git Commit Messages](https://tbaggery.com/2008/04/19/a-note-about-git-commit-messages.html)
  by Tim Pope.

Read the rest of this page for the full list of rules Lintje checks on and how
to fix them.

### Git for machines

Lintje does not actively promote machine parsing of commit subjects and
messages for the purposes of generating changelogs automatically.

The audiences of commits and changelogs are different. Commits are written for
people working on a project and changelogs are written for people using the
project. In my opinion a changelog entry should not be based on a Git commit,
but instead be managed with another tool such as
[Changesets](https://github.com/atlassian/changesets), which can also generate
changelogs automatically.

## SubjectLength

The commit's subject is considered too short or too long.

Short commit subjects like "WIP" and "Fix" don't explain the change well
enough. Don't be afraid to dive into a little bit more detail to explain the
change.

The commit's subject should be a maximum of 50 characters long. If the subject
is longer than 50 characters, reword the subject to fit in the maximum subject
length. Use the commit's message body to explain the change in greater detail.

```
# Good
Fix incorrect email validation

# Bad - too short
WIP
wip
Fix

# Bad - too long
One day I woke up and found the solution to this year old bug, the solution...
```

*Note: This rule is skipped if a [SubjectCliche](#subjectcliche) violation is
found.*

## SubjectMood

Write commit subjects in the imperative mood. The commit is not actively
"fixing" an issue, but it is a "fix" for an issue or it does "add" a feature.

Start the subject with something like "Fix ...", but not "Fixes ...", "Fixed
..." or "Fixing ...".

```
# Good
Fix ...
Test ...
Change ...

# Bad
Fixes ...
Fixed ...
Fixing ...
Tests ...
Tested ...
Testing ...
Changes ...
Changed ...
Changing ...
```

(_Where `...` would describe the change in more detail._)

## SubjectWhitespace

The commit's subject starts with a whitespace (space, tab, etc). Remove this
leading whitespace from the subject.

```
# Good
Fix incorrect email validation

# Bad
 Fix incorrect email validation
  Fix incorrect email validation
<TAB>Fix incorrect email validation
```

## SubjectCapitalization

The commit's subject doesn't start with a capital letter. Use a capital letter
to start the subject.

```
# Good
Fix incorrect email validation

# Bad
fix incorrect email validation
```

*Note: This rule is skipped if a [NeedsRebase](#needsrebase) violation is
found. To fix a NeedsRebase violation the commit needs to be rebased into the
commit it's marked to fixup or squash, and there will be no need to fix the
capitalization.*

## SubjectPunctuation

The commit's subject starts or ends with punctuation. Subjects don't need to
end with punctuation.

It may also be that a subject starts with an emoji, subjects also don't need to
start with an emoji as a prefix of some kind.

```
# Good
Fix incorrect email validation

# Bad
Fix incorrect email validation.
Fix incorrect email validation!
Fix incorrect email validation?
.Fix incorrect email validation
!Fix incorrect email validation
?Fix incorrect email validation
📺 Fix my television
👍 All good
🐞 Fix bug in email validation
```

Sometimes commits contain some tag for some machine to parse, like `[ci skip]`
or `[skip ci]` to avoid building the commit on the CI, and save some resources.
This rule will trigger if this tag is part of the commit's subject. Instead
move the tag to the body of the commit message. It's not relevant for the
subject, and the space can instead be used for describe the change in more
detail.

## SubjectTicketNumber

The commit's subject includes a reference to a ticket or issue. Move this to
the message body.

Invalid subject examples:

```
# Bad
Fix #123
I have fixed #123
I have fixed org/repo#123
I have fixed https://github.com/org/repo#123
JIRA-123
Fix JIRA-123 for good
```

## SubjectPrefix

The commit's subject contains a prefix of some kind. Remove prefixes from the
commit subject and reword the subject to explain the change in more detail.

```
# Good
Fix bad validation for user email validation
Refactor the user email validation
Add email validation to user sign up
Add documentation for the user email validation

# Bad
fix: ...
chore: ...
feat: ...
feature: ...
docs: ...
refactor: ...
FIX: ...
fix!: bug...
fix(scope): ...
fix(scope)!: ...
```

## SubjectBuildTag

The commit's subject contains a "skip ci" build tag. This should be moved to
the message body. The skip CI tag doesn't tell anything about what kind of
change was made. It's metadata only for the CI system.

```
# Bad
// General
Update README [ci skip]
Update README [skip ci]
Update README [no ci]
// AppVeyor
Update README [skip appveyor]
// Azure
Update README [azurepipelines skip]
Update README [skip azurepipelines]
Update README [azpipelines skip]
Update README [skip azpipelines]
Update README [azp skip]
Update README [skip azp]
Update README ***NO_CI***",
// GitHub Actions
Update README [actions skip]
Update README [skip actions]
// Travis
Update README [travis skip]
Update README [skip travis]
Update README [travis ci skip]
Update README [skip travis ci]
Update README [travis-ci skip]
Update README [skip travis-ci]
Update README [travisci skip]
Update README [skip travisci]
```

## SubjectCliche

The commit's subject is considered to be a cliché, it's overused and adds
little meaning. This rule scans for subjects that only use two words to
describe a change, usually "fix bug" and "update code" types of subjects. The
words in the example below are the words it scans for.

To resolve this violation, expand the subject to explain the change in more
detail. Describe what type of bug was fixed and what type of change was made.

```
# Bad
WIP
Fix
Fix bug
Fixes test
Fixed issue
Fixing build
Add
Add file
Adds files
Added tests
Adding stuff
Update
Update README
Updates files
Updated tests
Updating stuff
Change
Change README
Changes files
Changed tests
Changing stuff
Remove
Remove file
Removes files
Removed tests
Removing stuff
Delete
Delete file
Deletes files
Deleted tests
Deleting stuff
...
```

## MessageEmptyFirstLine

The line in the commit message body after the subject is not empty. If the line
after the subject is not empty, it is considered part of the subject.

This is the preferred format of a Git commit:

```
Subject line

First message line below an empty line.
```

## MessagePresence

The commit's message body is empty or too short. Add a message body to the
commit to elaborate on _why_ the change was necessary, what alternatives were
considered and why this particular implementation was chosen as a solution.

*Note: This rule is skipped if a [NeedsRebase](#needsrebase) violation is
found. To fix a NeedsRebase violation the commit needs to be rebased into the
commit it's marked to fixup or squash, and there will be no need to add a
message body.*

## MessageLineLength

The commit's message body has one or more lines that are too long. The maximum
line length in a message body is 72 characters. Split sentences and paragraph
across multiples lines.

Lines that include URLs that start with `http://` or `https://` are excluded
from this rule. Lines that are too long inside code blocks are also ignored,
because it's not always possible to reformat code to fit on a 72 character
line.

    # Good - max 72 characters per line
    Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam
    nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat,
    sed diam voluptua.

    # Good - the only too long line includes URL
    Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam.
    Source:
    https://url-to-page-that-is-very-long.org/but-still-valid-for-this-rule.html

    # Good - the only long line is in a code block
    Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam.

    ```
    Example code block with a very long line that will be considered valid!!!!
    ```

    ```md
    Example code block with a very long line that will be considered valid!!!!
    ```

    ``` md
    Example code block with a very long line that will be considered valid!!!!
    ```

    - Valid indented fenced code block inside a list
      ```
      Example code block with a very long line that will be considered valid!
      ```

    # Good - the only long line is in a code block
    Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam.

        Example code block with a very long line that will consider valid!!!!

    # Bad - lines are too long
    Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy aa
    tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.

## MergeCommit

The commit is detected as a merge commit, which is a commit merging one branch
into another. Prefer rebasing feature branches instead of merging base branches
back into them. These commits don't communicate anything meaningful other than
when a person merged changes locally.

Currently only one scenario triggers this rule: when a remote branch is merged
into a local branch.

```
# Merge commit merging a remote branch into a local branch
Merge branch 'develop' of github.com/org/repo into develop
```

When pulling local changes that would create a merge commit, rebase the local
changes on the remote changes instead.

```
# Checkout the branch to pull updates for
git checkout feature-branch
# Rebase local changes on the remote's changes
git pull --rebase origin feature-branch
```

To avoid making these types of commits I recommend configuring a pull strategy
in Git. Either use the `pull.ff only` or `pull.rebase` strategy.

```
# Block pulls that would create a merge commit
# With this config you'll need to pull rebase the remote changes
git config --global pull.ff only

# Automatically rebase local changes on remote changes when using git pull
git config --global pull.rebase true
```

This rule will also try to trigger on local merge commits in the future.

### Notes about merge commits

Merge commits that merge local branches/tags into the repository's branches do
not fail under this rule, they are currently ignored, but may fail on this rule
in the future.

Merge commits made when merging Pull and Merge requests will not fail on this
rule, these commits are ignored entirely. These commits they communicate when a
Pull/Merge request was accepted and merged into the base branch. This includes
commits made by GitHub's "squash and merge" merge strategy.

## NeedsRebase

The commit is detected as a fixup or squash commit. These commits communicate
the intent to squash them into other commits during the next rebase. These
commits should not be send in for review in Pull Requests, and they should not
be merged into main branches.

```
git checkout feature-branch
git rebase --interactive --autosquash main
```

## BranchNameLength

The branch name is detected as too short. A branch name needs to be at least
four characters.

```
# Good branch names
main
develop
trunk
fix-email-validation

# Bad branch names
foo
wip
fix
bug
```

## BranchNameTicketNumber

The branch name is detected to only contain a ticket number or a prefix and
ticket number. Ticket numbers alone don't communicate much, especially if all
branches are formatted this way. Describe the branch in more detail, in a
couple words, to explain what the change is about. Ticket numbers are accepted,
but not as the only thing in the branch name.

```
# Good branch names
123-email-validation
123_fix_email_validation
123/feature-email-validation
fix-123-email-validation
fix_123_email_validation
fix/123-email-validation
feature-123-email-validation
email-validation-123

# Bad branch names - in any capitalization
123
123-fix
123_fix
123/fix
123-feature
fix-123
fix_123
fix/123
feature-123
JIRA-123
```

## BranchNamePunctuation

The branch name starts or ends with punctuation. Branch names should not use
punctuation this way.

```
# Good branch names
fix-email-validation
fix_email_validation
feature/email-validation

# Bad branch names
fix-bug!
fix-bug.
fix-bug'
fix-bug"
!fix-bug
-fix-bug
_fix-bug
~fix-bug
(JIRA-123)
[JIRA-123]
```

## BranchNameCliche

The branch name is considered to be a cliché, it's overused and adds little
meaning. This rule scans for branch names that only use two words to describe a
change, usually "fix-bug" and "add_test" types of branch names. The words in
the example below are the words it scans for.

To resolve this violation, expand the branch name to explain the change in more
detail. Describe what type of bug was fixed and what type of change was made.

```
# Bad
wip
wip-feature
wip_feature
wip/feature
fix
fix-bug
fixes-test
fixed-issue
fixing-build
add
add-file
adds-files
added-tests
adding-stuff
update
update-readme
updates-files
updated-tests
updating-stuff
change
change-readme
changes-files
changed-tests
changing-stuff
remove
remove-file
removes-files
removed-tests
removing-stuff
delete
delete-file
deletes-files
deleted-tests
deleting-stuff
...
```
