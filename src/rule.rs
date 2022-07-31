use std::fmt;

use crate::branch::Branch;
use crate::commit::Commit;
use crate::issue::Issue;
use crate::rules::*;

const BASE_URL: &str = "https://lintje.dev/docs/";

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
            Rule::RebaseCommit => RebaseCommit::new().validate(commit),
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
            Rule::MessageSkipBuildTag => MessageSkipBuildTag::new().validate(commit),
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
            | Rule::DiffPresence => panic!("Unknown rule for branch validation: {}", self),
            Rule::BranchNameLength => BranchNameLength::new().validate(branch),
            Rule::BranchNameTicketNumber => BranchNameTicketNumber::new().validate(branch),
            Rule::BranchNamePunctuation => BranchNamePunctuation::new().validate(branch),
            Rule::BranchNameCliche => BranchNameCliche::new().validate(branch),
        }
    }

    pub fn link(&self) -> String {
        let path = match self {
            Rule::MergeCommit => "commit-type/#mergecommit",
            Rule::RebaseCommit => "commit-type/#rebasecommit",
            Rule::SubjectLength => "commit-subject/#subjectlength",
            Rule::SubjectMood => "commit-subject/#subjectmood",
            Rule::SubjectWhitespace => "commit-subject/#subjectwhitespace",
            Rule::SubjectCapitalization => "commit-subject/#subjectcapitalization",
            Rule::SubjectPunctuation => "commit-subject/#subjectpunctuation",
            Rule::SubjectTicketNumber => "commit-subject/#subjectticketnumber",
            Rule::SubjectPrefix => "commit-subject/#subjectprefix",
            Rule::SubjectBuildTag => "commit-subject/#subjectbuildtag",
            Rule::SubjectCliche => "commit-subject/#subjectcliche",
            Rule::MessageEmptyFirstLine => "commit-message/#messageemptyfirstline",
            Rule::MessagePresence => "commit-message/#messagepresence",
            Rule::MessageLineLength => "commit-message/#messagelinelength",
            Rule::MessageSkipBuildTag => "commit-messsage/#messageskipbuildtag",
            Rule::MessageTicketNumber => "commit-message/#messageticketnumber",
            Rule::DiffPresence => "commit-type/#diffpresence",
            Rule::BranchNameTicketNumber => "branch/#branchnameticketnumber",
            Rule::BranchNameLength => "branch/#branchnamelength",
            Rule::BranchNamePunctuation => "branch/#branchnamepunctuation",
            Rule::BranchNameCliche => "branch/#branchnamecliche",
        };
        format!("{}rules/{}", BASE_URL, path)
    }
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
            "https://lintje.dev/docs/rules/commit-subject/#subjectlength"
        );
    }
}
