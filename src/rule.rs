use std::fmt;

use crate::branch::Branch;
use crate::commit::Commit;
use crate::issue::Issue;
use crate::rules::*;

const REDIRECTOR_DOMAIN: &str = "https://r.lintje.dev/";

#[derive(Debug, PartialEq)]
pub enum Rule {
    MergeCommit,
    RebaseCommit,
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
    MessageSkipBuildTag,
    MessageTicketNumber,
    MessageTrailerLine,
    DiffChangeset,
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
            Rule::RebaseCommit => "RebaseCommit",
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
            Rule::MessageSkipBuildTag => "MessageSkipBuildTag",
            Rule::MessageTicketNumber => "MessageTicketNumber",
            Rule::MessageTrailerLine => "MessageTrailerLine",
            Rule::DiffChangeset => "DiffChangeset",
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
        let rule_validator: Box<dyn RuleValidator<Commit>> = match self {
            Rule::MergeCommit => Box::new(MergeCommit::new()),
            Rule::RebaseCommit => Box::new(RebaseCommit::new()),
            Rule::SubjectLength => Box::new(SubjectLength::new()),
            Rule::SubjectMood => Box::new(SubjectMood::new()),
            Rule::SubjectWhitespace => Box::new(SubjectWhitespace::new()),
            Rule::SubjectCapitalization => Box::new(SubjectCapitalization::new()),
            Rule::SubjectPunctuation => Box::new(SubjectPunctuation::new()),
            Rule::SubjectTicketNumber => Box::new(SubjectTicketNumber::new()),
            Rule::SubjectPrefix => Box::new(SubjectPrefix::new()),
            Rule::SubjectBuildTag => Box::new(SubjectBuildTag::new()),
            Rule::SubjectCliche => Box::new(SubjectCliche::new()),
            Rule::MessagePresence => Box::new(MessagePresence::new()),
            Rule::MessageEmptyFirstLine => Box::new(MessageEmptyFirstLine::new()),
            Rule::MessageLineLength => Box::new(MessageLineLength::new()),
            Rule::MessageSkipBuildTag => Box::new(MessageSkipBuildTag::new()),
            Rule::MessageTicketNumber => Box::new(MessageTicketNumber::new()),
            Rule::MessageTrailerLine => Box::new(MessageTrailerLine::new()),
            Rule::DiffChangeset => Box::new(DiffChangeset::new()),
            Rule::DiffPresence => Box::new(DiffPresence::new()),
            Rule::BranchNameTicketNumber
            | Rule::BranchNameLength
            | Rule::BranchNamePunctuation
            | Rule::BranchNameCliche => {
                panic!("Unknown rule for commit validation: {}", self)
            }
        };
        rule_validator.validate(commit)
    }

    pub fn validate_branch(&self, branch: &Branch) -> Option<Vec<Issue>> {
        let rule_validator: Box<dyn RuleValidator<Branch>> = match self {
            Rule::MergeCommit
            | Rule::RebaseCommit
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
            | Rule::MessageSkipBuildTag
            | Rule::MessageTicketNumber
            | Rule::MessageTrailerLine
            | Rule::DiffChangeset
            | Rule::DiffPresence => panic!("Unknown rule for branch validation: {}", self),
            Rule::BranchNameLength => Box::new(BranchNameLength::new()),
            Rule::BranchNameTicketNumber => Box::new(BranchNameTicketNumber::new()),
            Rule::BranchNamePunctuation => Box::new(BranchNamePunctuation::new()),
            Rule::BranchNameCliche => Box::new(BranchNameCliche::new()),
        };
        rule_validator.validate(branch)
    }

    pub fn link(&self) -> String {
        format!("{}r/{}", REDIRECTOR_DOMAIN, self)
    }
}

pub trait RuleValidator<T> {
    fn validate(&self, commit: &T) -> Option<Vec<Issue>>;
}

pub fn rule_by_name(name: &str) -> Option<Rule> {
    match name {
        "MergeCommit" => Some(Rule::MergeCommit),
        "RebaseCommit" | "NeedsRebase" => Some(Rule::RebaseCommit),
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
        "MessageSkipBuildTag" => Some(Rule::MessageSkipBuildTag),
        "MessageTicketNumber" => Some(Rule::MessageTicketNumber),
        "MessageTrailerLine" => Some(Rule::MessageTrailerLine),
        "DiffChangeset" => Some(Rule::DiffChangeset),
        "DiffPresence" => Some(Rule::DiffPresence),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Rule;

    #[test]
    fn link_to_docs() {
        assert_eq!(
            Rule::SubjectLength.link(),
            "https://r.lintje.dev/r/SubjectLength"
        );
    }
}
