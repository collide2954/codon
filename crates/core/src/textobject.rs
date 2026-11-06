use std::cmp::Reverse;

use ropey::RopeSlice;

use crate::chars::{CharClass, classify, is_line_ending};
use crate::movement::last_line;
use crate::pairs::{BRACKETS, find_pair};
use crate::search::{nth_char_after, nth_char_before};
use crate::selection::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextObject {
    Inside,
    Around,
}

pub fn word(text: RopeSlice, range: Range, object: TextObject, long: bool) -> Range {
    let cursor = range.cursor(text);
    let class_at = |pos: usize| text.get_char(pos).map(|c| word_class(c, long));
    let Some(class) =
        class_at(cursor).filter(|class| matches!(class, CharClass::Word | CharClass::Punctuation))
    else {
        return Range::point(cursor);
    };
    let mut start = cursor;
    while start > 0 && class_at(start - 1) == Some(class) {
        start -= 1;
    }
    let mut end = cursor + 1;
    while class_at(end) == Some(class) {
        end += 1;
    }
    if object == TextObject::Inside {
        return Range::new(start, end);
    }
    let after = text.chars_at(end).take_while(|&c| is_space(c)).count();
    if after > 0 {
        Range::new(start, end + after)
    } else {
        let before = text
            .chars_at(start)
            .reversed()
            .take_while(|&c| is_space(c))
            .count();
        Range::new(start - before, end)
    }
}

fn word_class(c: char, long: bool) -> CharClass {
    match classify(c) {
        CharClass::Punctuation if long => CharClass::Word,
        class => class,
    }
}

fn is_space(c: char) -> bool {
    c.is_whitespace() && !is_line_ending(c)
}

pub fn pair(
    text: RopeSlice,
    range: Range,
    object: TextObject,
    pair: (char, char),
    count: usize,
) -> Range {
    find_pair(text, range.cursor(text), pair, count)
        .map_or(range, |(open, close)| surrounding(open, close, object))
}

pub fn quote(
    text: RopeSlice,
    range: Range,
    object: TextObject,
    quote: char,
    count: usize,
) -> Range {
    let pos = range.cursor(text);
    if pos >= text.len_chars() || text.char(pos) == quote {
        return range;
    }
    let open = nth_char_before(text, quote, pos, count);
    let close = nth_char_after(text, quote, pos + 1, count);
    open.zip(close)
        .map_or(range, |(open, close)| surrounding(open, close, object))
}

pub fn closest_pair(text: RopeSlice, range: Range, object: TextObject, count: usize) -> Range {
    let pos = range.cursor(text);
    let mut enclosing: Vec<(usize, usize)> = BRACKETS
        .into_iter()
        .flat_map(|pair| (1..=count).map_while(move |level| find_pair(text, pos, pair, level)))
        .filter(|&(open, close)| open <= range.from() && close + 1 >= range.to())
        .collect();
    enclosing.sort_unstable_by_key(|&(open, _)| Reverse(open));
    enclosing
        .get(count.saturating_sub(1))
        .map_or(range, |&(open, close)| surrounding(open, close, object))
}

fn surrounding(open: usize, close: usize, object: TextObject) -> Range {
    match object {
        TextObject::Inside => Range::new(open + 1, close),
        TextObject::Around => Range::new(open, close + 1),
    }
}

pub fn paragraph(text: RopeSlice, range: Range, object: TextObject, count: usize) -> Range {
    let lines = last_line(text) + 1;
    let blank = |line: usize| text.line(line).chars().all(char::is_whitespace);
    let run_end = |from: usize| {
        let kind = blank(from);
        (from..lines)
            .find(|&line| blank(line) != kind)
            .unwrap_or(lines)
    };
    let cursor_line = range.cursor_line(text).min(lines - 1);
    let on_blank = blank(cursor_line);
    let mut start = cursor_line;
    while start > 0 && blank(start - 1) == on_blank {
        start -= 1;
    }
    let mut end = run_end(start);
    let count = count.max(1);
    let extra_runs = match object {
        TextObject::Inside => count - 1,
        TextObject::Around => 2 * count - 1,
    };
    for _ in 0..extra_runs {
        if end < lines {
            end = run_end(end);
        }
    }
    if object == TextObject::Around && !on_blank && !blank(end - 1) {
        while start > 0 && blank(start - 1) {
            start -= 1;
        }
    }
    Range::new(text.line_to_char(start), text.line_to_char(end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transformed;
    use TextObject::{Around, Inside};

    fn check(before: &str, after: &str, f: impl Fn(RopeSlice, Range) -> Range) {
        assert_eq!(transformed(before, f), after, "from {before:?}");
    }

    #[test]
    fn selects_inside_and_around_words() {
        check("hel#[l|]#o world", "#[hello|]# world", |t, r| {
            word(t, r, Inside, false)
        });
        check("hel#[l|]#o world", "#[hello |]#world", |t, r| {
            word(t, r, Around, false)
        });
        check("hello wor#[l|]#d", "hello#[ world|]#", |t, r| {
            word(t, r, Around, false)
        });
        check("foo.b#[a|]#r", "foo.#[bar|]#", |t, r| {
            word(t, r, Inside, false)
        });
        check("foo.b#[a|]#r baz", "#[foo.bar|]# baz", |t, r| {
            word(t, r, Inside, true)
        });
        check("a#[ |]# b", "a#[ |]# b", |t, r| word(t, r, Inside, false));
    }

    #[test]
    fn selects_inside_and_around_brackets() {
        let parens = ('(', ')');
        check("(a#[b|]#c)", "(#[abc|]#)", |t, r| {
            pair(t, r, Inside, parens, 1)
        });
        check("(a#[b|]#c)", "#[(abc)|]#", |t, r| {
            pair(t, r, Around, parens, 1)
        });
        check("#[(|]#ab)", "(#[ab|]#)", |t, r| {
            pair(t, r, Inside, parens, 1)
        });
        check("(ab#[)|]#", "(#[ab|]#)", |t, r| {
            pair(t, r, Inside, parens, 1)
        });
        check("{\n  #[x|]#\n}", "{#[\n  x\n|]#}", |t, r| {
            pair(t, r, Inside, ('{', '}'), 1)
        });
    }

    #[test]
    fn counts_select_outer_brackets() {
        let parens = ('(', ')');
        check("((a#[b|]#)c)", "((#[ab|]#)c)", |t, r| {
            pair(t, r, Inside, parens, 1)
        });
        check("((a#[b|]#)c)", "(#[(ab)c|]#)", |t, r| {
            pair(t, r, Inside, parens, 2)
        });
    }

    #[test]
    fn missing_pairs_leave_the_range_alone() {
        check("a#[b|]#c", "a#[b|]#c", |t, r| {
            pair(t, r, Inside, ('(', ')'), 1)
        });
        check("a#[b|]#c", "a#[b|]#c", |t, r| quote(t, r, Inside, '"', 1));
    }

    #[test]
    fn selects_inside_and_around_quotes() {
        check("\"a#[b|]#c\"", "\"#[abc|]#\"", |t, r| {
            quote(t, r, Inside, '"', 1)
        });
        check("\"a#[b|]#c\"", "#[\"abc\"|]#", |t, r| {
            quote(t, r, Around, '"', 1)
        });
        check("#[\"|]#abc\"", "#[\"|]#abc\"", |t, r| {
            quote(t, r, Inside, '"', 1)
        });
    }

    #[test]
    fn closest_pair_picks_the_innermost_enclosing_bracket() {
        check("[(a#[b|]#)]", "[(#[ab|]#)]", |t, r| {
            closest_pair(t, r, Inside, 1)
        });
        check("[(a#[b|]#)]", "[#[(ab)|]#]", |t, r| {
            closest_pair(t, r, Around, 1)
        });
        check("[(a#[b|]#)]", "[#[(ab)|]#]", |t, r| {
            closest_pair(t, r, Inside, 2)
        });
        check("{a(b)#[c|]#}", "{#[a(b)c|]#}", |t, r| {
            closest_pair(t, r, Inside, 1)
        });
    }

    #[test]
    fn selects_paragraphs() {
        check("a\n#[b|]#\n\nc\n", "#[a\nb\n|]#\nc\n", |t, r| {
            paragraph(t, r, Inside, 1)
        });
        check("a\n#[b|]#\n\nc\n", "#[a\nb\n\n|]#c\n", |t, r| {
            paragraph(t, r, Around, 1)
        });
        check("a\n\n#[c|]#\n", "a\n#[\nc\n|]#", |t, r| {
            paragraph(t, r, Around, 1)
        });
        check("a\n#[\n|]#\nb", "a\n#[\n\n|]#b", |t, r| {
            paragraph(t, r, Inside, 1)
        });
        check("#[a|]#\n\nb\n\nc", "#[a\n\nb\n\n|]#c", |t, r| {
            paragraph(t, r, Around, 2)
        });
    }
}
