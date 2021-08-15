lazy_static! {
    static ref OTHER_PUNCTUATION: Vec<char> = vec!['…', '⋯',];
}

pub fn is_punctuation(character: &char) -> bool {
    character.is_ascii_punctuation() || OTHER_PUNCTUATION.contains(&character)
}
