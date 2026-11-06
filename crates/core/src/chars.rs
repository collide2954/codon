#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharClass {
    Eol,
    Whitespace,
    Word,
    Punctuation,
}

pub fn classify(c: char) -> CharClass {
    if is_line_ending(c) {
        CharClass::Eol
    } else if c.is_whitespace() {
        CharClass::Whitespace
    } else if is_word(c) {
        CharClass::Word
    } else {
        CharClass::Punctuation
    }
}

pub fn is_line_ending(c: char) -> bool {
    matches!(c, '\n' | '\r')
}

pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn is_blank(c: char) -> bool {
    matches!(c, ' ' | '\t')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_word_characters() {
        for c in ['a', 'Z', '0', '_', 'é', 'ß', '中'] {
            assert_eq!(classify(c), CharClass::Word, "{c:?}");
        }
    }

    #[test]
    fn classifies_whitespace() {
        for c in [' ', '\t', '\u{a0}'] {
            assert_eq!(classify(c), CharClass::Whitespace, "{c:?}");
        }
    }

    #[test]
    fn classifies_line_endings() {
        for c in ['\n', '\r'] {
            assert_eq!(classify(c), CharClass::Eol, "{c:?}");
        }
    }

    #[test]
    fn classifies_everything_else_as_punctuation() {
        for c in ['.', '(', '+', '-', '"', '😀'] {
            assert_eq!(classify(c), CharClass::Punctuation, "{c:?}");
        }
    }
}
