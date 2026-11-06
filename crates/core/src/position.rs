use std::borrow::Cow;

use ropey::RopeSlice;

use crate::grapheme::{graphemes, width};
use crate::line_ending::line_end;

pub fn grapheme_columns(grapheme: RopeSlice, column: usize, tab_width: usize) -> usize {
    if grapheme == "\t" {
        let tab_width = tab_width.max(1);
        tab_width - column % tab_width
    } else {
        width(&Cow::from(grapheme))
    }
}

pub fn visual_column(text: RopeSlice, pos: usize, tab_width: usize) -> usize {
    let line_start = text.line_to_char(text.char_to_line(pos));
    graphemes(text.slice(line_start..pos)).fold(0, |column, grapheme| {
        column + grapheme_columns(grapheme, column, tab_width)
    })
}

pub fn char_at_visual_column(
    text: RopeSlice,
    line: usize,
    column: usize,
    tab_width: usize,
) -> usize {
    let start = text.line_to_char(line);
    let end = line_end(text, line);
    let mut pos = start;
    let mut reached = 0;
    for grapheme in graphemes(text.slice(start..end)) {
        reached += grapheme_columns(grapheme, reached, tab_width);
        if reached > column {
            return pos;
        }
        pos += grapheme.len_chars();
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn tabs_advance_to_the_next_stop() {
        let rope = Rope::from("a\tb");
        assert_eq!(grapheme_columns(rope.slice(1..2), 1, 4), 3);
        assert_eq!(grapheme_columns(rope.slice(1..2), 4, 4), 4);
        assert_eq!(grapheme_columns(rope.slice(0..1), 0, 4), 1);
    }

    #[test]
    fn measures_columns_from_the_line_start() {
        let rope = Rope::from("x\na\tb中y");
        let text = rope.slice(..);
        assert_eq!(visual_column(text, 2, 4), 0);
        assert_eq!(visual_column(text, 4, 4), 4);
        assert_eq!(visual_column(text, 5, 4), 5);
        assert_eq!(visual_column(text, 6, 4), 7);
    }

    #[test]
    fn finds_the_grapheme_covering_a_column() {
        let rope = Rope::from("x\na\tbc\n中y");
        let text = rope.slice(..);
        assert_eq!(char_at_visual_column(text, 1, 0, 4), 2);
        assert_eq!(char_at_visual_column(text, 1, 2, 4), 3);
        assert_eq!(char_at_visual_column(text, 1, 4, 4), 4);
        assert_eq!(char_at_visual_column(text, 2, 1, 4), 7);
        assert_eq!(char_at_visual_column(text, 2, 2, 4), 8);
    }

    #[test]
    fn clamps_columns_past_the_end_to_the_line_end() {
        let rope = Rope::from("ab\ncd");
        let text = rope.slice(..);
        assert_eq!(char_at_visual_column(text, 0, 9, 4), 2);
        assert_eq!(char_at_visual_column(text, 1, 9, 4), 5);
    }
}
