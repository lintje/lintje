use std::fmt;

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
    SubjectCliche,
    MessageEmptyFirstLine,
    MessagePresence,
    MessageLineLength,
}

#[derive(Debug)]
pub struct Violation {
    pub rule: Rule,
    pub message: String,
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
            Rule::SubjectCliche => "SubjectCliche",
            Rule::MessageEmptyFirstLine => "MessageEmptyFirstLine",
            Rule::MessagePresence => "MessagePresence",
            Rule::MessageLineLength => "MessageLineLength",
        };
        write!(f, "{}", label)
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
        "SubjectCliche" => Some(Rule::SubjectCliche),
        "MessageEmptyFirstLine" => Some(Rule::MessageEmptyFirstLine),
        "MessagePresence" => Some(Rule::MessagePresence),
        "MessageLineLength" => Some(Rule::MessageLineLength),
        _ => None,
    }
}
