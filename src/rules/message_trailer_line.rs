use core::ops::Range;
use regex::Regex;

use crate::commit::Commit;
use crate::issue::{Context, Issue, Position};
use crate::rule::Rule;
use crate::rule::RuleValidator;

lazy_static! {
    static ref TRAILER_LINE: Regex = Regex::new(
        r"(?i)^(co-authored-by|signed-off-by|helped-by):\s+([\w\s\-]+\s+<[^\s]+[@]+[^\s]+>)"
    )
    .unwrap();
}

pub struct MessageTrailerLine {}

impl MessageTrailerLine {
    pub fn new() -> Self {
        Self {}
    }
}

impl RuleValidator<Commit> for MessageTrailerLine {
    // Test if the "Co-authored-by" line is always at the end of the message body
    // https://docs.github.com/en/pull-requests/committing-changes-to-your-project/creating-and-editing-commits/creating-a-commit-with-multiple-authors
    fn validate(&self, commit: &Commit) -> Option<Vec<Issue>> {
        let message = &commit.message.trim_end();

        let mut context = vec![];
        let mut context_additions = vec![];
        // Track where the first co-authored-by line was found that is not in the right place
        let mut first_line_issue_occurrence = None;
        // Track where the last issue occurred
        let mut last_issue_occurrence = None;

        for (line_index, line) in message.lines().enumerate() {
            if let Some(captures) = TRAILER_LINE.captures(line) {
                let full_capture = if let Some(capture) = captures.get(0) {
                    capture
                } else {
                    error!("MessageTrailerLine: Unable to fetch capture 0");
                    return None;
                };
                let type_capture = if let Some(capture) = captures.get(1) {
                    capture
                } else {
                    error!("MessageTrailerLine: Unable to fetch capture 1");
                    return None;
                };
                // +1 for subject line
                // +1 for zero index
                let line_number = line_index + 2;
                // Update to the latest position, the first occurence in the message
                if first_line_issue_occurrence.is_none() {
                    first_line_issue_occurrence = Some(line_number);
                }
                if last_issue_occurrence.is_some()
                    && line_index > last_issue_occurrence.unwrap_or(0) + 1
                {
                    // Add a gap if two lines with an issue are more than 1 line apart
                    context.push(Context::gap());
                }
                context.push(Context::message_line_removal_suggestion(
                    line_number,
                    line.to_string(),
                    full_capture.range(),
                    format!(
                        "Remove the {} reference in the message body",
                        type_capture.as_str().to_lowercase()
                    ),
                ));
                // Store for later, when we can calculate the new line count more easily.
                context_additions.push((
                    line.to_string(),
                    type_capture.as_str().to_lowercase(),
                    full_capture.range(),
                ));
                last_issue_occurrence = Some(line_index);
            }
        }

        if context.is_empty() {
            return None;
        }

        // Show a visual gap between the errors and suggestions
        context.push(Context::gap());

        // +1 for subject line
        let mut new_last_line = message.lines().count() + 1;
        if commit.trailers.is_empty() {
            // Add new trailer line, which is an empty line, because none was found
            new_last_line += 1;
            context.push(Context::message_line_addition(
                new_last_line,
                "".to_string(),
                Range { start: 0, end: 3 },
                "Add a new empty trailer line at the end of the message body".to_string(),
            ));
        } else {
            // +1 for the existing empty trailer separator line
            new_last_line += commit.trailers.lines().count() + 1;
        }

        // Add additions to the context, based on the line count and co-authored-by lines that
        // were found.
        for (line, trailer_type, range) in context_additions.drain(..) {
            new_last_line += 1;
            context.push(Context::message_line_addition(
                new_last_line,
                line.to_string(),
                range,
                format!(
                    "Move {} reference to the end of the message body",
                    trailer_type
                ),
            ));
        }

        Some(vec![Issue::error(
            Rule::MessageTrailerLine,
            "Trailer line is not at the end of the message body".to_string(),
            Position::MessageLine {
                line: first_line_issue_occurrence.unwrap(),
                column: 1,
            },
            context,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;

    fn validate(commit: &Commit) -> Option<Vec<Issue>> {
        MessageTrailerLine::new().validate(commit)
    }

    #[test]
    fn without_co_author() {
        let commit = commit(
            "Subject".to_string(),
            "\nI am a message without a co authored by line.\nSome other line.\n".to_string(),
        );
        assert_eq!(validate(&commit), None);
    }

    #[test]
    fn with_one_co_author_without_trailers() {
        let commit = commit(
            "Subject",
            "\n\
            I am a message with a co authored by line on the last line.\n\
            Some other line.\n\
            \n\
            Co-authored-by: Person A <other@example.com>\n\
            \n\
            Some other line at the end.",
        );
        let issue = first_issue(validate(&commit));
        assert_eq!(
            issue.message,
            "Trailer line is not at the end of the message body"
        );
        assert_eq!(issue.position, message_position(6, 1));
        assert_contains_issue_output(
            &issue,
            " 6 | Co-authored-by: Person A <other@example.com>\n\
                | -------------------------------------------- Remove the co-authored-by reference in the message body\n\
               ~~~\n\
              9 |\n\
                | +++ Add a new empty trailer line at the end of the message\n\
             10 | Co-authored-by: Person A <other@example.com>\n\
                | ++++++++++++++++++++++++++++++++++++++++++++ Move co-authored-by reference to the end of the message body",
        );
    }

    #[test]
    fn with_one_co_author_with_trailers() {
        let commit = commit_with_trailers(
            "Subject",
            "\n\
            I am a message with a co authored by line on the last line.\n\
            Some other line.\n\
            \n\
            Co-authored-by: Person A <other@example.com>\n\
            \n\
            Some other line at the end.",
            "Co-authored-by: Tom <email@domain.com>\nSigned-off-by: Tom <email@domain.com>",
        );
        let issue = first_issue(validate(&commit));
        assert_eq!(
            issue.message,
            "Trailer line is not at the end of the message body"
        );
        assert_eq!(issue.position, message_position(6, 1));
        assert_contains_issue_output(
            &issue,
            " 6 | Co-authored-by: Person A <other@example.com>\n\
                | -------------------------------------------- Remove the co-authored-by reference in the message body\n\
               ~~~\n\
             12 | Co-authored-by: Person A <other@example.com>\n\
                | ++++++++++++++++++++++++++++++++++++++++++++ Move co-authored-by reference to the end of the message body",
        );
    }

    #[test]
    fn with_multiple_co_authors() {
        let commit = commit(
            "Subject",
            "\n\
            I am a message with a co authored by line on the last line.\n\
            Some other line.\n\
            \n\
            Co-authored-by: Person A <other@example.com>\n\
            \n\
            Co-authored-by: Person B <other@example.com>\n\
            Co-authored-by: Person C <other@example.com>\n\
            \n\
            Some other line at the end.",
        );
        let issue = first_issue(validate(&commit));
        assert_eq!(
            issue.message,
            "Trailer line is not at the end of the message body"
        );
        assert_eq!(issue.position, message_position(6, 1));
        assert_contains_issue_output(
            &issue,
            " 6 | Co-authored-by: Person A <other@example.com>\n\
                | -------------------------------------------- Remove the co-authored-by reference in the message body\n\
                ~~~\n\
              8 | Co-authored-by: Person B <other@example.com>\n\
                | -------------------------------------------- Remove the co-authored-by reference in the message body\n\
              9 | Co-authored-by: Person C <other@example.com>\n\
                | -------------------------------------------- Remove the co-authored-by reference in the message body\n\
               ~~~\n\
             12 |\n\
                | +++ Add a new empty trailer line at the end of the message\n\
             13 | Co-authored-by: Person A <other@example.com>\n\
                | ++++++++++++++++++++++++++++++++++++++++++++ Move co-authored-by reference to the end of the message body\n\
             14 | Co-authored-by: Person B <other@example.com>\n\
                | ++++++++++++++++++++++++++++++++++++++++++++ Move co-authored-by reference to the end of the message body\n\
             15 | Co-authored-by: Person C <other@example.com>\n\
                | ++++++++++++++++++++++++++++++++++++++++++++ Move co-authored-by reference to the end of the message body",
        );
    }

    #[test]
    fn with_signed_off_by() {
        let commit = commit(
            "Subject",
            "\n\
            Some message line.\n\
            \n\
            Signed-off-by: Person A <other@example.com>\n\
            \n\
            Some other line at the end.",
        );
        let issue = first_issue(validate(&commit));
        assert_eq!(
            issue.message,
            "Trailer line is not at the end of the message body"
        );
        assert_eq!(issue.position, message_position(5, 1));
        assert_contains_issue_output(
            &issue,
            "5 | Signed-off-by: Person A <other@example.com>\n\
               | ------------------------------------------- Remove the signed-off-by reference in the message body\n\
              ~~~\n\
             8 |\n\
               | +++ Add a new empty trailer line at the end of the message\n\
             9 | Signed-off-by: Person A <other@example.com>\n\
               | +++++++++++++++++++++++++++++++++++++++++++ Move signed-off-by reference to the end of the message body",
        );
    }

    #[test]
    fn with_helped_off_by() {
        let commit = commit(
            "Subject",
            "\n\
            Some message line.\n\
            \n\
            Helped-by: Person A <other@example.com>\n\
            \n\
            Some other line at the end.",
        );
        let issue = first_issue(validate(&commit));
        assert_eq!(
            issue.message,
            "Trailer line is not at the end of the message body"
        );
        assert_eq!(issue.position, message_position(5, 1));
        assert_contains_issue_output(
            &issue,
            "5 | Helped-by: Person A <other@example.com>\n\
               | --------------------------------------- Remove the helped-by reference in the message body\n\
              ~~~\n\
             8 |\n\
               | +++ Add a new empty trailer line at the end of the message\n\
             9 | Helped-by: Person A <other@example.com>\n\
               | +++++++++++++++++++++++++++++++++++++++ Move helped-by reference to the end of the message body",
        );
    }
}
