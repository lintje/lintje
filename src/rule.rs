use std::fmt;

use crate::branch::Branch;
use crate::commit::Commit;
use crate::issue::Issue;
use crate::rules::*;

#[derive(Debug, PartialEq)]
pub enum Rule {
    MergeCommit,
    NeedsRebase,
    SubjectLength,
    SubjectMood,
    SubjectWhitespace,
    SubjectCapitalization,
    SubjectPunctuation,
    SubjectTicketNumber,
    SubjectPrefix,
    SubjectBuildTag,
    SubjectCliche,
    MessageEmptyFirstLine,
    MessagePresence,
    MessageLineLength,
    MessageTicketNumber,
    DiffPresence,
    BranchNameTicketNumber,
    BranchNameLength,
    BranchNamePunctuation,
    BranchNameCliche,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Rule::MergeCommit => "MergeCommit",
            Rule::NeedsRebase => "NeedsRebase",
            Rule::SubjectLength => "SubjectLength",
            Rule::SubjectMood => "SubjectMood",
            Rule::SubjectWhitespace => "SubjectWhitespace",
            Rule::SubjectCapitalization => "SubjectCapitalization",
            Rule::SubjectPunctuation => "SubjectPunctuation",
            Rule::SubjectTicketNumber => "SubjectTicketNumber",
            Rule::SubjectPrefix => "SubjectPrefix",
            Rule::SubjectBuildTag => "SubjectBuildTag",
            Rule::SubjectCliche => "SubjectCliche",
            Rule::MessageEmptyFirstLine => "MessageEmptyFirstLine",
            Rule::MessagePresence => "MessagePresence",
            Rule::MessageLineLength => "MessageLineLength",
            Rule::MessageTicketNumber => "MessageTicketNumber",
            Rule::DiffPresence => "DiffPresence",
            Rule::BranchNameTicketNumber => "BranchNameTicketNumber",
            Rule::BranchNameLength => "BranchNameLength",
            Rule::BranchNamePunctuation => "BranchNamePunctuation",
            Rule::BranchNameCliche => "BranchNameCliche",
        };
        write!(f, "{}", label)
    }
}

impl Rule {
    pub fn validate_commit(&self, commit: &Commit) -> Option<Vec<Issue>> {
        match self {
            Rule::MergeCommit => MergeCommit::new().validate(commit),
            Rule::NeedsRebase => NeedsRebase::new().validate(commit),
            Rule::SubjectLength => SubjectLength::new().validate(commit),
            Rule::SubjectMood => SubjectMood::new().validate(commit),
            Rule::SubjectWhitespace => SubjectWhitespace::new().validate(commit),
            Rule::SubjectCapitalization => SubjectCapitalization::new().validate(commit),
            Rule::SubjectPunctuation => SubjectPunctuation::new().validate(commit),
            Rule::SubjectTicketNumber => SubjectTicketNumber::new().validate(commit),
            Rule::SubjectPrefix => SubjectPrefix::new().validate(commit),
            Rule::SubjectBuildTag => SubjectBuildTag::new().validate(commit),
            Rule::SubjectCliche => SubjectCliche::new().validate(commit),
            Rule::MessagePresence => MessagePresence::new().validate(commit),
            Rule::MessageEmptyFirstLine => MessageEmptyFirstLine::new().validate(commit),
            Rule::MessageLineLength => MessageLineLength::new().validate(commit),
            Rule::MessageTicketNumber => MessageTicketNumber::new().validate(commit),
            Rule::DiffPresence => DiffPresence::new().validate(commit),
            Rule::BranchNameTicketNumber
            | Rule::BranchNameLength
            | Rule::BranchNamePunctuation
            | Rule::BranchNameCliche => {
                panic!("Unknown rule for commit validation: {}", self)
            }
        }
    }

    pub fn validate_branch(&self, branch: &Branch) -> Option<Vec<Issue>> {
        match self {
            Rule::MergeCommit
            | Rule::NeedsRebase
            | Rule::SubjectLength
            | Rule::SubjectMood
            | Rule::SubjectWhitespace
            | Rule::SubjectCapitalization
            | Rule::SubjectPunctuation
            | Rule::SubjectTicketNumber
            | Rule::SubjectPrefix
            | Rule::SubjectBuildTag
            | Rule::SubjectCliche
            | Rule::MessagePresence
            | Rule::MessageEmptyFirstLine
            | Rule::MessageLineLength
            | Rule::MessageTicketNumber
            | Rule::DiffPresence
            | Rule::BranchNameCliche => panic!("Unknown rule for commit validation: {}", self),
            Rule::BranchNameLength => BranchNameLength::new().validate(branch),
            Rule::BranchNameTicketNumber => BranchNameTicketNumber::new().validate(branch),
            Rule::BranchNamePunctuation => BranchNamePunctuation::new().validate(branch),
        }
    }
}

pub fn rule_by_name(name: &str) -> Option<Rule> {
    match name {
        "MergeCommit" => Some(Rule::MergeCommit),
        "NeedsRebase" => Some(Rule::NeedsRebase),
        "SubjectLength" => Some(Rule::SubjectLength),
        "SubjectMood" => Some(Rule::SubjectMood),
        "SubjectWhitespace" => Some(Rule::SubjectWhitespace),
        "SubjectCapitalization" => Some(Rule::SubjectCapitalization),
        "SubjectPunctuation" => Some(Rule::SubjectPunctuation),
        "SubjectTicketNumber" => Some(Rule::SubjectTicketNumber),
        "SubjectBuildTag" => Some(Rule::SubjectBuildTag),
        "SubjectPrefix" => Some(Rule::SubjectPrefix),
        "SubjectCliche" => Some(Rule::SubjectCliche),
        "MessageEmptyFirstLine" => Some(Rule::MessageEmptyFirstLine),
        "MessagePresence" => Some(Rule::MessagePresence),
        "MessageLineLength" => Some(Rule::MessageLineLength),
        "MessageTicketNumber" => Some(Rule::MessageTicketNumber),
        "DiffPresence" => Some(Rule::DiffPresence),
        _ => None,
    }
}
