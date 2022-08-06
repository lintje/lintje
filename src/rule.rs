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
            Rule::DiffChangeset => Box::new(DiffChangeset::new()),
            Rule::DiffPresence => Box::new(DiffPresence::new()),
            Rule::BranchNameTicketNumber
            | Rule::BranchNameLength
            | Rule::BranchNamePunctuation
            | Rule::BranchNameCliche => {
                panic!("Unknown rule for commit validation: {}", self)
            }
        };
        if let Some(dependent_rules) = rule_validator.dependent_rules() {
            for dependent_rule in dependent_rules {
                if !commit.checked_rules.contains(&dependent_rule) {
                    panic!(
                        "Commit rules were checked out of order. Rule '{}' has a dependency on '{}', which has not been validated yet.",
                        self,
                        dependent_rule
                    );
                }
            }
        }
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
            | Rule::DiffChangeset
            | Rule::DiffPresence => panic!("Unknown rule for branch validation: {}", self),
            Rule::BranchNameLength => Box::new(BranchNameLength::new()),
            Rule::BranchNameTicketNumber => Box::new(BranchNameTicketNumber::new()),
            Rule::BranchNamePunctuation => Box::new(BranchNamePunctuation::new()),
            Rule::BranchNameCliche => Box::new(BranchNameCliche::new()),
        };
        if let Some(dependent_rules) = rule_validator.dependent_rules() {
            for dependent_rule in dependent_rules {
                if !branch.checked_rules.contains(&dependent_rule) {
                    panic!(
                        "Branch rules were checked out of order. Rule '{}' has a dependency on '{}', which has not been validated yet.",
                        self,
                        dependent_rule
                    );
                }
            }
        }
        rule_validator.validate(branch)
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
            Rule::DiffChangeset => "commit-type/#diffchangeset",
            Rule::DiffPresence => "commit-type/#diffpresence",
            Rule::BranchNameTicketNumber => "branch/#branchnameticketnumber",
            Rule::BranchNameLength => "branch/#branchnamelength",
            Rule::BranchNamePunctuation => "branch/#branchnamepunctuation",
            Rule::BranchNameCliche => "branch/#branchnamecliche",
        };
        format!("{}rules/{}", BASE_URL, path)
    }
}

pub trait RuleValidator<T> {
    fn dependent_rules(&self) -> Option<Vec<Rule>>;
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
            "https://lintje.dev/docs/rules/commit-subject/#subjectlength"
        );
    }
}
