use core::ops::Range;
use regex::{Regex, RegexBuilder};

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;
use crate::utils::character_count_for_bytes_index;

lazy_static! {
    static ref SUBJECT_WITH_BUILD_TAGS: Regex = {
        let mut tempregex =
            RegexBuilder::new(r"(\[(skip [\w\s_-]+|[\w\s_-]+ skip|no ci)\]|\*\*\*NO_CI\*\*\*)");
        tempregex.case_insensitive(true);
        tempregex.multi_line(false);
        tempregex.build().unwrap()
    };
}

pub struct SubjectBuildTag {}

impl SubjectBuildTag {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for SubjectBuildTag {
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let subject = &commit.subject.to_string();
        if let Some(captures) = SUBJECT_WITH_BUILD_TAGS.captures(subject) {
            match captures.get(1) {
                Some(tag) => {
                    let line_count = commit.message.lines().count();
                    let base_line_count = if line_count == 0 { 3 } else { line_count + 2 };
                    let context = vec![
                        Context::subject_removal_suggestion(
                            subject.to_string(),
                            tag.range(),
                            "Remove the build tag from the subject".to_string(),
                        ),
                        Context::message_line_addition(
                            base_line_count,
                            tag.as_str().to_string(),
                            Range {
                                start: 0,
                                end: tag.range().len(),
                            },
                            "Move build tag to message body".to_string(),
                        ),
                    ];
                    Some(vec![Issue::error(
                        Rule::SubjectBuildTag,
                        format!("The `{}` build tag was found in the subject", tag.as_str()),
                        Position::Subject {
                            line: 1,
                            column: character_count_for_bytes_index(&commit.subject, tag.start()),
                        },
                        context,
                    )])
                }
                None => {
                    error!("SubjectBuildTag: Unable to fetch build tag from subject.");
                    None
                }
            }
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
        SubjectBuildTag::new().validate(&commit)
    }

    fn assert_subject_as_valid(subject: &str) {
        assert_eq!(validate(&commit(subject, "")), None);
    }

    fn assert_subject_as_invalid(subject: &str) {
        assert!(validate(&commit(subject, "")).is_some());
    }

    #[test]
    fn valid_subjects() {
        assert_subject_as_valid("Add exception for no ci build tag");
    }

    #[test]
    fn invalid_subjects() {
        let build_tags = vec![
            // General
            "[ci skip]",
            "[skip ci]",
            "[no ci]",
            // AppVeyor
            "[skip appveyor]",
            // Azure
            "[azurepipelines skip]",
            "[skip azurepipelines]",
            "[azpipelines skip]",
            "[skip azpipelines]",
            "[azp skip]",
            "[skip azp]",
            "***NO_CI***",
            // GitHub Actions
            "[actions skip]",
            "[skip actions]",
            // Travis
            "[travis skip]",
            "[skip travis]",
            "[travis ci skip]",
            "[skip travis ci]",
            "[travis-ci skip]",
            "[skip travis-ci]",
            "[travisci skip]",
            "[skip travisci]",
            // Other custom tags that match the format
            "[skip me]",
            "[skip changeset]",
            "[skip review]",
        ];
        for tag in build_tags.iter() {
            assert_subject_as_invalid(&format!("Update README {}", tag));
        }
    }

    #[test]
    fn build_tag_detail() {
        let issue = first_issue(validate(&commit("Edit CHANGELOG [skip ci]", "")));
        assert_eq!(
            issue.message,
            "The `[skip ci]` build tag was found in the subject"
        );
        assert_eq!(issue.position, subject_position(16));
        assert_contains_issue_output(
            &issue,
            "1 | Edit CHANGELOG [skip ci]\n\
               |                --------- Remove the build tag from the subject\n\
              ~~~\n\
             3 | [skip ci]\n\
               | +++++++++ Move build tag to message body",
        );
    }
}
