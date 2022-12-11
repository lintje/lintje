use core::ops::Range;
use regex::{Regex, RegexBuilder};

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

lazy_static! {
    static ref TEXT_FILES: Regex = {
        // Only match README and LICENSE files, regardless of extension, sub directory or file
        // prefix.
        let mut regex = RegexBuilder::new(r"[\\/]?([\w\-_]*)(readme|license|code_of_conduct)\.?([^\\/]?\w+)?$");
        regex.case_insensitive(true);
        regex.multi_line(false);
        regex.build().unwrap()
    };
}

static SKIP_TAGS: [&str; 4] = ["[skip ci]", "[ci skip]", "[no ci]", "***NO_CI***"];

pub struct MessageSkipBuildTag {}

impl MessageSkipBuildTag {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessageSkipBuildTag {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        if !commit.has_changes() {
            return None;
        }
        if commit.has_issue(&Rule::SubjectBuildTag) {
            return None;
        }
        for tag in SKIP_TAGS {
            if commit.message.contains(tag) {
                return None;
            }
        }

        let is_text_files = commit
            .file_changes
            .iter()
            .all(|filename| TEXT_FILES.is_match(filename));

        if is_text_files {
            let mut context = vec![];

            let mut i = 0;
            let count = commit.file_changes.len();
            for filename in &commit.file_changes {
                i += 1;
                if i == count {
                    // Only add underline and message to last file in list
                    let filename_len = filename.len();
                    context.push(Context::diff_error(
                        filename.to_string(),
                        Range {
                            start: 0,
                            end: filename_len,
                        },
                        "Only text files were changed".to_string(),
                    ));
                } else {
                    context.push(Context::diff_line(filename.to_string()));
                }
            }
            context.push(Context::gap());

            let line_count = commit.message.lines().count();
            let new_line_count = if line_count == 0 { 3 } else { line_count + 2 };
            let tag = "[skip ci]".to_string();
            let tag_len = tag.len();
            context.push(Context::message_line_addition(
                new_line_count,
                tag,
                Range {
                    start: 0,
                    end: tag_len,
                },
                "Add the skip build tag to the commit message".to_string(),
            ));
            Some(vec![Issue::hint(
                Rule::MessageSkipBuildTag,
                "Consider skipping the build for a text change that does not impact the test suite"
                    .to_string(),
                Position::Diff,
                context,
            )])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MessageSkipBuildTag::new().validate(commit)
    }

    pub fn commit_with_files(files: Vec<String>) -> Commit {
        Commit::new(
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            Some("test@example.com".to_string()),
            "Some subject",
            "Some message".to_string(),
            "".to_string(),
            files,
        )
    }

    #[test]
    fn with_only_text_files() {
        let issue = first_issue(validate(&commit_with_files(vec![
            "README".to_string(),
            "docs/README.md".to_string(),
            "CODE_OF_CONDUCT.md".to_string(),
            "LICENSE".to_string(),
            "MIT-LICENSE.txt".to_string(),
            "MIT_LICENSE.foo".to_string(),
            "license.md".to_string(),
        ])));
        assert_eq!(
            issue.message,
            "Consider skipping the build for a text change that does not impact the test suite"
        );
        assert_eq!(issue.position, Position::Diff);
        assert_contains_issue_output(
            &issue,
            "  | README\n\
               | docs/README.md\n\
               | CODE_OF_CONDUCT.md\n\
               | LICENSE\n\
               | MIT-LICENSE.txt\n\
               | MIT_LICENSE.foo\n\
               | license.md\n\
               | ^^^^^^^^^^ Only text files were changed\n\
              ~~~\n\
             3 | [skip ci]\n\
               | +++++++++ Add the skip build tag to the commit message",
        );
    }

    #[test]
    fn with_mix_files() {
        let issues = validate(&commit_with_files(vec![
            "src/README/md".to_string(),
            "README.html.md".to_string(),
        ]));
        assert_eq!(issues, None);
    }

    #[test]
    fn with_more_than_text_files() {
        let issues = validate(&commit_with_files(vec![
            "README.md".to_string(),
            "src/main.rs".to_string(),
        ]));
        assert_eq!(issues, None);
    }

    #[test]
    fn without_file_changes() {
        let issues = validate(&commit_with_files(vec![]));
        assert_eq!(issues, None);
    }

    #[test]
    fn with_build_tag_in_subject() {
        let mut commit = commit_with_files(vec![]);
        commit.issues.push(Issue::error(
            Rule::SubjectBuildTag,
            "some message".to_string(),
            Position::Subject { line: 1, column: 1 },
            vec![],
        ));
        let issues = validate(&commit);
        assert_eq!(issues, None);
    }

    #[test]
    fn with_skip_build_tag() {
        for tag in SKIP_TAGS {
            let commit = Commit::new(
                Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                Some("test@example.com".to_string()),
                "Some subject",
                format!("Some message {}", tag),
                "".to_string(),
                vec!["README.md".to_string()],
            );

            let issues = validate(&commit);
            assert_eq!(issues, None);
        }
    }
}
