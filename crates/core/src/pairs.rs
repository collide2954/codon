use ropey::RopeSlice;

pub const BRACKETS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

pub fn bracket_pair(c: char) -> Option<(char, char)> {
    BRACKETS
        .into_iter()
        .find(|&(open, close)| c == open || c == close)
}

pub fn find_pair(
    text: RopeSlice,
    pos: usize,
    pair: (char, char),
    level: usize,
) -> Option<(usize, usize)> {
    if pos >= text.len_chars() || level == 0 {
        return None;
    }
    Some((
        find_open(text, pos, pair, level)?,
        find_close(text, pos, pair, level)?,
    ))
}

pub fn match_bracket(text: RopeSlice, pos: usize) -> Option<usize> {
    let c = text.get_char(pos)?;
    let pair = bracket_pair(c)?;
    if c == pair.0 {
        find_close(text, pos, pair, 1)
    } else {
        find_open(text, pos, pair, 1)
    }
}

fn find_open(
    text: RopeSlice,
    pos: usize,
    (open, close): (char, char),
    level: usize,
) -> Option<usize> {
    let mut remaining = level;
    if text.char(pos) == open {
        remaining -= 1;
        if remaining == 0 {
            return Some(pos);
        }
    }
    let before = text.chars_at(pos).reversed();
    unmatched(before, close, open, remaining).map(|offset| pos - 1 - offset)
}

fn find_close(
    text: RopeSlice,
    pos: usize,
    (open, close): (char, char),
    level: usize,
) -> Option<usize> {
    let mut remaining = level;
    if text.char(pos) == close {
        remaining -= 1;
        if remaining == 0 {
            return Some(pos);
        }
    }
    let after = text.chars_at(pos + 1);
    unmatched(after, open, close, remaining).map(|offset| pos + 1 + offset)
}

fn unmatched(
    chars: impl Iterator<Item = char>,
    nested: char,
    target: char,
    mut remaining: usize,
) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, c) in chars.enumerate() {
        if c == nested {
            depth += 1;
        } else if c == target {
            if depth == 0 {
                remaining -= 1;
                if remaining == 0 {
                    return Some(offset);
                }
            } else {
                depth -= 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn knows_both_sides_of_a_bracket() {
        assert_eq!(bracket_pair('('), Some(('(', ')')));
        assert_eq!(bracket_pair('}'), Some(('{', '}')));
        assert_eq!(bracket_pair('x'), None);
    }

    #[test]
    fn matches_brackets_in_both_directions() {
        let rope = Rope::from("(a[b]c)");
        let text = rope.slice(..);
        assert_eq!(match_bracket(text, 0), Some(6));
        assert_eq!(match_bracket(text, 2), Some(4));
        assert_eq!(match_bracket(text, 4), Some(2));
        assert_eq!(match_bracket(text, 6), Some(0));
        assert_eq!(match_bracket(text, 1), None);
    }

    #[test]
    fn matching_respects_nesting_and_balance() {
        let rope = Rope::from("((x))(");
        let text = rope.slice(..);
        assert_eq!(match_bracket(text, 0), Some(4));
        assert_eq!(match_bracket(text, 1), Some(3));
        assert_eq!(match_bracket(text, 5), None);
    }

    #[test]
    fn finds_enclosing_pairs_by_level() {
        let rope = Rope::from("((ab)c)");
        let text = rope.slice(..);
        let parens = ('(', ')');
        assert_eq!(find_pair(text, 2, parens, 1), Some((1, 4)));
        assert_eq!(find_pair(text, 2, parens, 2), Some((0, 6)));
        assert_eq!(find_pair(text, 2, parens, 3), None);
        assert_eq!(find_pair(text, 5, parens, 1), Some((0, 6)));
    }

    #[test]
    fn a_bracket_under_the_cursor_is_the_first_level() {
        let rope = Rope::from("((a))");
        let text = rope.slice(..);
        let parens = ('(', ')');
        assert_eq!(find_pair(text, 1, parens, 1), Some((1, 3)));
        assert_eq!(find_pair(text, 3, parens, 1), Some((1, 3)));
        assert_eq!(find_pair(text, 1, parens, 2), Some((0, 4)));
    }
}
