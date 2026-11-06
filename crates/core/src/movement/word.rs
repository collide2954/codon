use ropey::RopeSlice;

use crate::chars::{CharClass, classify, is_line_ending};
use crate::grapheme::next_boundary;
use crate::selection::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordMotion {
    NextStart,
    NextEnd,
    PrevStart,
    NextLongStart,
    NextLongEnd,
    PrevLongStart,
}

impl WordMotion {
    fn is_forward(self) -> bool {
        !matches!(self, Self::PrevStart | Self::PrevLongStart)
    }

    fn is_long(self) -> bool {
        matches!(
            self,
            Self::NextLongStart | Self::NextLongEnd | Self::PrevLongStart
        )
    }

    fn reached(self, prev: char, next: char) -> bool {
        let boundary = if self.is_long() {
            is_long_word_boundary(prev, next)
        } else {
            classify(prev) != classify(next)
        };
        boundary
            && match self {
                Self::NextStart | Self::NextLongStart => {
                    is_line_ending(next) || !next.is_whitespace()
                }
                _ => !prev.is_whitespace() || is_line_ending(next),
            }
    }
}

fn is_long_word_boundary(a: char, b: char) -> bool {
    match (classify(a), classify(b)) {
        (CharClass::Word, CharClass::Punctuation) | (CharClass::Punctuation, CharClass::Word) => {
            false
        }
        (a, b) => a != b,
    }
}

pub fn move_word(text: RopeSlice, range: Range, motion: WordMotion, count: usize) -> Range {
    let forward = motion.is_forward();
    if (forward && range.head == text.len_chars()) || (!forward && range.head == 0) {
        return range;
    }
    let cursor = range.cursor(text);
    let after = next_boundary(text, cursor);
    let mut current = if forward {
        Range::new(cursor, after)
    } else {
        Range::new(after, cursor)
    };
    for _ in 0..count {
        let stepped = step(text, current, motion);
        if stepped == current {
            break;
        }
        current = stepped;
    }
    current.grapheme_aligned(text)
}

fn step(text: RopeSlice, origin: Range, motion: WordMotion) -> Range {
    let forward = motion.is_forward();
    let advance = |pos: usize| {
        if forward {
            pos + 1
        } else {
            pos.saturating_sub(1)
        }
    };
    let mut chars = text.chars_at(origin.head);
    if !forward {
        chars.reverse();
    }
    let mut prev = chars.prev();
    if prev.is_some() {
        chars.next();
    }
    let mut anchor = origin.anchor;
    let mut head = origin.head;
    while let Some(c) = chars.next() {
        if is_line_ending(c) {
            prev = Some(c);
            head = advance(head);
        } else {
            chars.prev();
            break;
        }
    }
    if prev.is_some_and(is_line_ending) {
        anchor = head;
    }
    let start = head;
    for next in chars {
        if prev.is_none_or(|prev| motion.reached(prev, next)) {
            if head != start {
                break;
            }
            anchor = head;
        }
        prev = Some(next);
        head = advance(head);
    }
    Range::new(anchor, head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::transformed;
    use WordMotion::*;

    fn word(motion: WordMotion, count: usize) -> impl Fn(RopeSlice, Range) -> Range {
        move |text, range| move_word(text, range, motion, count)
    }

    #[test]
    fn next_word_start_selects_the_word_and_its_trailing_space() {
        assert_eq!(
            transformed("#[h|]#ello world", word(NextStart, 1)),
            "#[hello |]#world"
        );
        assert_eq!(
            transformed("#[hello |]#world", word(NextStart, 1)),
            "hello #[world|]#"
        );
        assert_eq!(
            transformed("#[f|]#oo.bar", word(NextStart, 1)),
            "#[foo|]#.bar"
        );
    }

    #[test]
    fn next_word_start_stops_at_line_ends() {
        assert_eq!(
            transformed("#[f|]#oo\nbar", word(NextStart, 1)),
            "#[foo|]#\nbar"
        );
        assert_eq!(
            transformed("#[foo|]#\nbar", word(NextStart, 1)),
            "foo\n#[bar|]#"
        );
    }

    #[test]
    fn counts_repeat_the_motion() {
        assert_eq!(
            transformed("#[h|]#ello big world", word(NextStart, 2)),
            "hello #[big |]#world"
        );
    }

    #[test]
    fn next_word_end_selects_up_to_the_end() {
        assert_eq!(
            transformed("#[h|]#ello world", word(NextEnd, 1)),
            "#[hello|]# world"
        );
        assert_eq!(
            transformed("#[hello|]# world", word(NextEnd, 1)),
            "hello#[ world|]#"
        );
    }

    #[test]
    fn prev_word_start_selects_backward() {
        assert_eq!(
            transformed("hello #[w|]#orld", word(PrevStart, 1)),
            "#[|hello ]#world"
        );
        assert_eq!(
            transformed("foo.#[b|]#ar", word(PrevStart, 1)),
            "foo#[|.]#bar"
        );
        assert_eq!(
            transformed("foo\n#[b|]#ar", word(PrevStart, 1)),
            "#[|foo]#\nbar"
        );
        assert_eq!(transformed("#[a|]#bc", word(PrevStart, 1)), "#[|a]#bc");
    }

    #[test]
    fn long_words_include_punctuation() {
        assert_eq!(
            transformed("#[f|]#oo.bar baz", word(NextLongStart, 1)),
            "#[foo.bar |]#baz"
        );
        assert_eq!(
            transformed("#[f|]#oo.bar baz", word(NextLongEnd, 1)),
            "#[foo.bar|]# baz"
        );
        assert_eq!(
            transformed("foo.bar #[b|]#az", word(PrevLongStart, 1)),
            "#[|foo.bar ]#baz"
        );
    }

    #[test]
    fn stays_put_at_the_end_of_the_text() {
        assert_eq!(transformed("abc#[|]#", word(NextStart, 1)), "abc#[|]#");
    }
}
