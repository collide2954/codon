mod find;
mod word;

pub use find::{Find, find_char};
pub use word::{WordMotion, move_word};

use ropey::RopeSlice;

use crate::grapheme::{nth_next_boundary, nth_prev_boundary, prev_boundary};
use crate::line_ending::line_end;
use crate::position::{char_at_visual_column, visual_column};
use crate::selection::{Direction, Range};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Move,
    Extend,
}

pub fn last_line(text: RopeSlice) -> usize {
    let last = text.len_lines() - 1;
    if last > 0 && text.line(last).len_chars() == 0 {
        last - 1
    } else {
        last
    }
}

pub fn line_last_grapheme(text: RopeSlice, line: usize) -> usize {
    prev_boundary(text, line_end(text, line)).max(text.line_to_char(line))
}

pub fn first_non_blank(text: RopeSlice, line: usize) -> Option<usize> {
    let start = text.line_to_char(line);
    text.slice(start..line_end(text, line))
        .chars()
        .position(|c| !c.is_whitespace())
        .map(|offset| start + offset)
}

pub fn move_horizontally(
    text: RopeSlice,
    range: Range,
    direction: Direction,
    count: usize,
    movement: Movement,
) -> Range {
    let cursor = range.cursor(text);
    let pos = match direction {
        Direction::Forward => nth_next_boundary(text, cursor, count),
        Direction::Backward => nth_prev_boundary(text, cursor, count),
    };
    range.put_cursor(text, pos, movement == Movement::Extend)
}

pub fn move_vertically(
    text: RopeSlice,
    range: Range,
    direction: Direction,
    count: usize,
    movement: Movement,
    tab_width: usize,
) -> Range {
    let cursor = range.cursor(text);
    let line = text.char_to_line(cursor);
    let column = range
        .column
        .unwrap_or_else(|| visual_column(text, cursor, tab_width));
    let target = match direction {
        Direction::Forward => (line + count).min(last_line(text).max(line)),
        Direction::Backward => line.saturating_sub(count),
    };
    let pos = char_at_visual_column(text, target, column, tab_width);
    let mut moved = range.put_cursor(text, pos, movement == Movement::Extend);
    moved.column = Some(column);
    moved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{parse, render, transformed};
    use Direction::{Backward, Forward};
    use Movement::{Extend, Move};
    use ropey::Rope;

    fn check(before: &str, after: &str, f: impl Fn(RopeSlice, Range) -> Range) {
        assert_eq!(transformed(before, f), after, "from {before:?}");
    }

    fn horizontal(
        direction: Direction,
        count: usize,
        movement: Movement,
    ) -> impl Fn(RopeSlice, Range) -> Range {
        move |text, range| move_horizontally(text, range, direction, count, movement)
    }

    fn vertical(direction: Direction, movement: Movement) -> impl Fn(RopeSlice, Range) -> Range {
        move |text, range| move_vertically(text, range, direction, 1, movement, 4)
    }

    #[test]
    fn last_line_ignores_the_empty_line_after_a_final_newline() {
        assert_eq!(last_line(Rope::from("a\nb").slice(..)), 1);
        assert_eq!(last_line(Rope::from("a\nb\n").slice(..)), 1);
        assert_eq!(last_line(Rope::from("a\n\n").slice(..)), 1);
        assert_eq!(last_line(Rope::from("").slice(..)), 0);
        assert_eq!(last_line(Rope::from("\n").slice(..)), 0);
    }

    #[test]
    fn line_last_grapheme_sits_before_the_terminator() {
        let rope = Rope::from("ab\r\n\nc\u{301}");
        let text = rope.slice(..);
        assert_eq!(line_last_grapheme(text, 0), 1);
        assert_eq!(line_last_grapheme(text, 1), 4);
        assert_eq!(line_last_grapheme(text, 2), 5);
    }

    #[test]
    fn first_non_blank_skips_indentation() {
        let rope = Rope::from("  \tx y\n   \nz");
        let text = rope.slice(..);
        assert_eq!(first_non_blank(text, 0), Some(3));
        assert_eq!(first_non_blank(text, 1), None);
        assert_eq!(first_non_blank(text, 2), Some(11));
    }

    #[test]
    fn moves_by_graphemes() {
        check("#[a|]#bc", "a#[b|]#c", horizontal(Forward, 1, Move));
        check("a#[b|]#c", "#[a|]#bc", horizontal(Backward, 1, Move));
        check("#[a|]#bcd", "ab#[c|]#d", horizontal(Forward, 2, Move));
        check(
            "#[e\u{301}|]#x",
            "e\u{301}#[x|]#",
            horizontal(Forward, 1, Move),
        );
        check("#[a|]#b", "#[a|]#b", horizontal(Backward, 1, Move));
    }

    #[test]
    fn moves_across_line_endings() {
        check("a#[b|]#\nc", "ab#[\n|]#c", horizontal(Forward, 1, Move));
        check("ab#[\n|]#c", "ab\n#[c|]#", horizontal(Forward, 1, Move));
        check(
            "ab\r\n#[c|]#",
            "ab#[\r\n|]#c",
            horizontal(Backward, 1, Move),
        );
    }

    #[test]
    fn extends_horizontally() {
        check("#[a|]#bc", "#[ab|]#c", horizontal(Forward, 1, Extend));
        check("#[ab|]#c", "#[a|]#bc", horizontal(Backward, 1, Extend));
        check("a#[b|]#c", "#[|ab]#c", horizontal(Backward, 1, Extend));
    }

    #[test]
    fn moves_vertically_keeping_the_column() {
        check("ab#[c|]#\nxyz", "abc\nxy#[z|]#", vertical(Forward, Move));
        check("abc\nx#[y|]#z", "a#[b|]#c\nxyz", vertical(Backward, Move));
    }

    #[test]
    fn short_lines_clamp_but_the_column_sticks() {
        let (text, selection) = parse("abc#[d|]#\nxy\nlong");
        let rope = Rope::from(text.as_str());
        let text_slice = rope.slice(..);
        let down = |selection: crate::Selection| {
            selection
                .transform(|range| move_vertically(text_slice, range, Forward, 1, Move, 4))
                .ensure_invariants(text_slice)
        };
        let once = down(selection);
        assert_eq!(render(&text, &once), "abcd\nxy#[\n|]#long");
        assert_eq!(render(&text, &down(once)), "abcd\nxy\nlon#[g|]#");
    }

    #[test]
    fn stops_at_the_first_and_last_lines() {
        check("#[a|]#b\ncd", "#[a|]#b\ncd", vertical(Backward, Move));
        check("ab\n#[c|]#d", "ab\n#[c|]#d", vertical(Forward, Move));
        check("ab\n#[c|]#d\n", "ab\n#[c|]#d\n", vertical(Forward, Move));
    }

    #[test]
    fn extends_vertically() {
        check("#[a|]#bc\ndef", "#[abc\nd|]#ef", vertical(Forward, Extend));
    }

    #[test]
    fn measures_tabs_and_wide_characters() {
        check(
            "\t#[x|]#\nabcdef",
            "\tx\nabcd#[e|]#f",
            vertical(Forward, Move),
        );
        check("中#[x|]#\nabcd", "中x\nab#[c|]#d", vertical(Forward, Move));
    }
}
