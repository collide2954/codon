use ropey::RopeSlice;

pub fn nth_char_after(text: RopeSlice, target: char, start: usize, n: usize) -> Option<usize> {
    text.chars_at(start)
        .enumerate()
        .filter(|&(_, c)| c == target)
        .nth(n.checked_sub(1)?)
        .map(|(offset, _)| start + offset)
}

pub fn nth_char_before(text: RopeSlice, target: char, end: usize, n: usize) -> Option<usize> {
    text.chars_at(end)
        .reversed()
        .enumerate()
        .filter(|&(_, c)| c == target)
        .nth(n.checked_sub(1)?)
        .map(|(offset, _)| end - 1 - offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn finds_the_nth_char_after_a_position() {
        let rope = Rope::from("abcabc");
        let text = rope.slice(..);
        assert_eq!(nth_char_after(text, 'a', 0, 1), Some(0));
        assert_eq!(nth_char_after(text, 'a', 1, 1), Some(3));
        assert_eq!(nth_char_after(text, 'a', 0, 2), Some(3));
        assert_eq!(nth_char_after(text, 'a', 0, 3), None);
        assert_eq!(nth_char_after(text, 'a', 0, 0), None);
    }

    #[test]
    fn finds_the_nth_char_before_a_position() {
        let rope = Rope::from("abcabc");
        let text = rope.slice(..);
        assert_eq!(nth_char_before(text, 'c', 6, 1), Some(5));
        assert_eq!(nth_char_before(text, 'c', 5, 1), Some(2));
        assert_eq!(nth_char_before(text, 'c', 6, 2), Some(2));
        assert_eq!(nth_char_before(text, 'c', 2, 1), None);
    }
}
