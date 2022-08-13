use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

static SKIP_CHANGESET_TAG: &str = "[skip changeset]";

lazy_static! {
    static ref NON_WORD_CHARACTERS: Regex = Regex::new(r"([^\w]+)").unwrap();
}

pub struct DiffChangeset {}

impl DiffChangeset {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for DiffChangeset {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if !commit.has_changes() {
            return None;
        }

        if commit.message.contains(SKIP_CHANGESET_TAG) {
            return None;
        }

        let has_changesets = commit
            .file_changes
            .iter()
            .any(|filename| filename.contains(".changeset/") || filename.contains(".changesets/"));
        if has_changesets {
            return None;
        }

        let diff_line = format!(".changesets/{}.md", parameterize(&commit.subject));
        let diff_line_len = diff_line.len();
        let line_count = commit.message.lines().count();
        let new_line_count = if line_count == 0 { 3 } else { line_count + 2 };
        let tag = "[skip changeset]".to_string();
        let tag_len = tag.len();
        let context = vec![
            Context::diff_addition(
                diff_line,
                Range {
                    start: 0,
                    end: diff_line_len,
                },
                "Add a changeset file for changelog generation".to_string(),
            ),
            Context::gap(),
            Context::message_line_addition(
                new_line_count,
                tag,
                Range {
                    start: 0,
                    end: tag_len,
                },
                "Or add the skip changeset tag to the commit message".to_string(),
            ),
        ];
        Some(vec![Issue::hint(
            Rule::DiffChangeset,
            "No changeset file found in commit".to_string(),
            Position::Diff,
            context,
        )])
    }
}

fn parameterize(filename: &str) -> String {
    NON_WORD_CHARACTERS
        .replace_all(&filename.to_lowercase(), "-")
        .trim_start_matches('-')
        .trim_end_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        DiffChangeset::new().validate(&commit)
    }

    pub fn commit_with_files(files: Vec<String>) -> Commit {
        Commit::new(
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            Some("test@example.com".to_string()),
            "Subject: of commit with /&_|[](){}\\'\"`@!=+*.,#$%^;: chars -- foo.test",
            "Some message".to_string(),
            "".to_string(),
            files,
        )
    }

    #[test]
    fn without_changeset() {
        let issue = first_issue(validate(&commit_with_files(vec![
            "README.md".to_string(),
            "src/main.rs".to_string(),
        ])));
        assert_eq!(issue.message, "No changeset file found in commit");
        assert_eq!(issue.position, Position::Diff);
        assert_contains_issue_output(
            &issue,
            "  | .changesets/subject-of-commit-with-_-chars-foo-test.md\n\
               | ++++++++++++++++++++++++++++++++++++++++++++++++++++++ Add a changeset file for changelog generation\n\
              ~~~\n\
             3 | [skip changeset]\n\
               | ++++++++++++++++ Or add the skip changeset tag to the commit message",
        );
    }

    #[test]
    fn without_changeset_with_skip_tag() {
        let commit = Commit::new(
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            Some("test@example.com".to_string()),
            "Subject",
            "Some message\n[skip changeset]".to_string(),
            "".to_string(),
            vec!["README.md".to_string(), "src/main.rs".to_string()],
        );
        let issues = validate(&commit);
        assert_eq!(issues, None);
    }

    #[test]
    fn with_changesets_in_root_directory() {
        let issues = validate(&commit_with_files(vec![
            "src/main.rs".to_string(),
            ".changesets/changeset-name.md".to_string(),
        ]));
        assert_eq!(issues, None);
    }

    #[test]
    fn with_changeset_in_root_directory() {
        let issues = validate(&commit_with_files(vec![
            "src/main.rs".to_string(),
            ".changeset/changeset-name.md".to_string(),
        ]));
        assert_eq!(issues, None);
    }

    #[test]
    fn with_changesets_in_sub_directory() {
        let issues = validate(&commit_with_files(vec![
            "package/src/main.rs".to_string(),
            "package/.changesets/changeset-name.md".to_string(),
        ]));
        assert_eq!(issues, None);
    }

    #[test]
    fn with_changeset_in_sub_directory() {
        let issues = validate(&commit_with_files(vec![
            "package/src/main.rs".to_string(),
            "package/.changeset/changeset-name.md".to_string(),
        ]));
        assert_eq!(issues, None);
    }
}
