use regex::Regex;

pub mod branch_name_cliche;
pub mod branch_name_length;
pub mod branch_name_punctuation;
pub mod branch_name_ticket_number;
pub mod diff_changeset;
pub mod diff_presence;
pub mod merge_commit;
pub mod message_empty_first_line;
pub mod message_line_length;
pub mod message_presence;
pub mod message_skip_build_tag;
pub mod message_ticket_number;
pub mod message_trailer_line;
pub mod rebase_commit;
pub mod subject_build_tag;
pub mod subject_capitalization;
pub mod subject_cliche;
pub mod subject_length;
pub mod subject_mood;
pub mod subject_prefix;
pub mod subject_punctuation;
pub mod subject_ticket_number;
pub mod subject_whitespace;

pub use branch_name_cliche::BranchNameCliche;
pub use branch_name_length::BranchNameLength;
pub use branch_name_punctuation::BranchNamePunctuation;
pub use branch_name_ticket_number::BranchNameTicketNumber;
pub use diff_changeset::DiffChangeset;
pub use diff_presence::DiffPresence;
pub use merge_commit::MergeCommit;
pub use message_empty_first_line::MessageEmptyFirstLine;
pub use message_line_length::MessageLineLength;
pub use message_presence::MessagePresence;
pub use message_skip_build_tag::MessageSkipBuildTag;
pub use message_ticket_number::MessageTicketNumber;
pub use message_trailer_line::MessageTrailerLine;
pub use rebase_commit::RebaseCommit;
pub use subject_build_tag::SubjectBuildTag;
pub use subject_capitalization::SubjectCapitalization;
pub use subject_cliche::SubjectCliche;
pub use subject_length::SubjectLength;
pub use subject_mood::SubjectMood;
pub use subject_prefix::SubjectPrefix;
pub use subject_punctuation::SubjectPunctuation;
pub use subject_ticket_number::SubjectTicketNumber;
pub use subject_whitespace::SubjectWhitespace;

pub static ISSUE_NUMBER_REFERENCE: &str = r"
    (
        https?://[^\s]+/| # Match entire URL
        [\w\-_\.]+/[\w\-_\.]+[\#!]| # Repo shorthand format: org/repo#123 or org/repo!123
        [\#!] # Only an issue or PR symbol
    )
    \d+ # Ends in an issue/PR number
";

lazy_static! {
    // Match all GitHub and GitLab keywords
    pub static ref CONTAINS_FIX_TICKET: Regex = Regex::new(&format!(r"(?xi)
        (fix(es|ed|ing)?|clos(e|es|ed|ing)|resolv(e|es|ed|ing)|implement(s|ed|ing)?) # Includes keyword
        :? # Optional colon
        \s+
        {ISSUE_NUMBER_REFERENCE}
    ")).unwrap();

    pub static ref CO_AUTHOR_REFERENCE: Regex =
        Regex::new(r"(?im)^co-authored-by: [\w\s\-]+\s+<[^\s]+[@]+[^\s]+>").unwrap();
}
