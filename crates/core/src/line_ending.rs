use ropey::RopeSlice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub const NATIVE: Self = if cfg!(windows) { Self::Crlf } else { Self::Lf };

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
        }
    }

    pub fn len_chars(self) -> usize {
        self.as_str().len()
    }

    pub fn detect(text: RopeSlice) -> Option<Self> {
        Self::of_line(text.line(0))
    }

    pub fn of_line(line: RopeSlice) -> Option<Self> {
        let last = line.len_chars().checked_sub(1)?;
        if line.char(last) != '\n' {
            None
        } else if last > 0 && line.char(last - 1) == '\r' {
            Some(Self::Crlf)
        } else {
            Some(Self::Lf)
        }
    }

    pub fn of_str(s: &str) -> Option<Self> {
        if s.ends_with("\r\n") {
            Some(Self::Crlf)
        } else if s.ends_with('\n') {
            Some(Self::Lf)
        } else {
            None
        }
    }
}

pub fn line_end(text: RopeSlice, line: usize) -> usize {
    let slice = text.line(line);
    let terminator = LineEnding::of_line(slice).map_or(0, LineEnding::len_chars);
    text.line_to_char(line) + slice.len_chars() - terminator
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn detects_the_first_line_ending() {
        assert_eq!(
            LineEnding::detect(Rope::from("a\nb\r\n").slice(..)),
            Some(LineEnding::Lf)
        );
        assert_eq!(
            LineEnding::detect(Rope::from("a\r\nb\n").slice(..)),
            Some(LineEnding::Crlf)
        );
    }

    #[test]
    fn detects_nothing_without_line_breaks() {
        assert_eq!(LineEnding::detect(Rope::from("abc").slice(..)), None);
        assert_eq!(LineEnding::detect(Rope::new().slice(..)), None);
    }

    #[test]
    fn reads_the_terminator_of_a_line() {
        let rope = Rope::from("a\r\nb\nc");
        assert_eq!(LineEnding::of_line(rope.line(0)), Some(LineEnding::Crlf));
        assert_eq!(LineEnding::of_line(rope.line(1)), Some(LineEnding::Lf));
        assert_eq!(LineEnding::of_line(rope.line(2)), None);
    }

    #[test]
    fn reads_the_terminator_of_a_str() {
        assert_eq!(LineEnding::of_str("x\r\n"), Some(LineEnding::Crlf));
        assert_eq!(LineEnding::of_str("x\n"), Some(LineEnding::Lf));
        assert_eq!(LineEnding::of_str("x\r"), None);
        assert_eq!(LineEnding::of_str("x"), None);
    }

    #[test]
    fn renders_as_str() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::Crlf.as_str(), "\r\n");
    }

    #[test]
    fn line_end_excludes_the_terminator() {
        let rope = Rope::from("ab\r\ncd\nef");
        let text = rope.slice(..);
        assert_eq!(line_end(text, 0), 2);
        assert_eq!(line_end(text, 1), 6);
        assert_eq!(line_end(text, 2), 9);
    }

    #[test]
    fn only_lf_and_crlf_break_lines() {
        assert_eq!(Rope::from("a\rb\u{2028}c\u{c}d\u{85}e").len_lines(), 1);
    }
}
