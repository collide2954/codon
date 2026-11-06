use ropey::RopeSlice;

use super::Movement;
use crate::selection::{Direction, Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Find {
    To,
    Till,
}

pub fn find_char(
    text: RopeSlice,
    range: Range,
    target: char,
    direction: Direction,
    find: Find,
    count: usize,
    movement: Movement,
) -> Range {
    let cursor = range.cursor(text);
    let found = match direction {
        Direction::Forward => find_forward(text, target, cursor, count, find),
        Direction::Backward => find_backward(text, target, cursor, count, find),
    };
    found.map_or(range, |pos| {
        let base = match movement {
            Movement::Move => Range::point(cursor),
            Movement::Extend => range,
        };
        base.put_cursor(text, pos, true)
    })
}

fn find_forward(
    text: RopeSlice,
    target: char,
    cursor: usize,
    count: usize,
    find: Find,
) -> Option<usize> {
    let start = (cursor + 1).min(text.len_chars());
    match find {
        Find::To => nth_after(text, target, start, count),
        Find::Till => {
            let skip = usize::from(text.get_char(start) == Some(target));
            nth_after(text, target, start, count + skip).map(|pos| pos.saturating_sub(1))
        }
    }
}

fn find_backward(
    text: RopeSlice,
    target: char,
    cursor: usize,
    count: usize,
    find: Find,
) -> Option<usize> {
    match find {
        Find::To => nth_before(text, target, cursor, count),
        Find::Till => {
            let skip = usize::from(cursor > 0 && text.get_char(cursor - 1) == Some(target));
            nth_before(text, target, cursor, count + skip).map(|pos| pos + 1)
        }
    }
}

fn nth_after(text: RopeSlice, target: char, start: usize, n: usize) -> Option<usize> {
    text.chars_at(start)
        .enumerate()
        .filter(|&(_, c)| c == target)
        .nth(n.checked_sub(1)?)
        .map(|(offset, _)| start + offset)
}

fn nth_before(text: RopeSlice, target: char, end: usize, n: usize) -> Option<usize> {
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
    use crate::testing::transformed;
    use Direction::{Backward, Forward};

    fn find(
        target: char,
        direction: Direction,
        find: Find,
        count: usize,
        movement: Movement,
    ) -> impl Fn(RopeSlice, Range) -> Range + Copy {
        move |text, range| find_char(text, range, target, direction, find, count, movement)
    }

    #[test]
    fn finds_forward_inclusively() {
        let to_b = find('b', Forward, Find::To, 1, Movement::Move);
        assert_eq!(transformed("#[a|]#bcbd", to_b), "#[ab|]#cbd");
        let second_b = find('b', Forward, Find::To, 2, Movement::Move);
        assert_eq!(transformed("#[a|]#bcbd", second_b), "#[abcb|]#d");
    }

    #[test]
    fn finds_forward_up_to_the_target() {
        let till_c = find('c', Forward, Find::Till, 1, Movement::Move);
        assert_eq!(transformed("#[a|]#bcbd", till_c), "#[ab|]#cbd");
        assert_eq!(transformed("a#[b|]#cdc", till_c), "a#[bcd|]#c");
    }

    #[test]
    fn finds_backward() {
        let to_b = find('b', Backward, Find::To, 1, Movement::Move);
        assert_eq!(transformed("abcb#[d|]#", to_b), "abc#[|bd]#");
        let till_c = find('c', Backward, Find::Till, 1, Movement::Move);
        assert_eq!(transformed("abcb#[d|]#", till_c), "abc#[|bd]#");
    }

    #[test]
    fn finds_across_lines() {
        let to_c = find('c', Forward, Find::To, 1, Movement::Move);
        assert_eq!(transformed("#[a|]#b\nc", to_c), "#[ab\nc|]#");
    }

    #[test]
    fn extends_an_existing_selection() {
        let to_d = find('d', Forward, Find::To, 1, Movement::Extend);
        assert_eq!(transformed("#[ab|]#cbd", to_d), "#[abcbd|]#");
    }

    #[test]
    fn leaves_the_range_alone_when_nothing_matches() {
        let to_z = find('z', Forward, Find::To, 1, Movement::Move);
        assert_eq!(transformed("#[a|]#bc", to_z), "#[a|]#bc");
    }
}
