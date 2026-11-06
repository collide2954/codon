use ropey::RopeSlice;
use ropey::iter::Chunks;
use ropey::str_utils::byte_to_char_idx;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};
use unicode_width::UnicodeWidthStr;

pub fn next_boundary(text: RopeSlice, char_idx: usize) -> usize {
    nth_next_boundary(text, char_idx, 1)
}

pub fn prev_boundary(text: RopeSlice, char_idx: usize) -> usize {
    nth_prev_boundary(text, char_idx, 1)
}

pub fn nth_next_boundary(text: RopeSlice, char_idx: usize, n: usize) -> usize {
    let mut byte_idx = text.char_to_byte(char_idx);
    let (mut chunk, mut chunk_byte, mut chunk_char, _) = text.chunk_at_byte(byte_idx);
    let mut cursor = GraphemeCursor::new(byte_idx, text.len_bytes(), true);
    for _ in 0..n {
        loop {
            match cursor.next_boundary(chunk, chunk_byte) {
                Ok(None) => return text.len_chars(),
                Ok(Some(next)) => {
                    byte_idx = next;
                    break;
                }
                Err(GraphemeIncomplete::NextChunk) => {
                    chunk_byte += chunk.len();
                    (chunk, _, chunk_char, _) = text.chunk_at_byte(chunk_byte);
                }
                Err(GraphemeIncomplete::PreContext(end)) => provide_context(&mut cursor, text, end),
                Err(_) => unreachable!(),
            }
        }
    }
    chunk_char + byte_to_char_idx(chunk, byte_idx - chunk_byte)
}

pub fn nth_prev_boundary(text: RopeSlice, char_idx: usize, n: usize) -> usize {
    let mut byte_idx = text.char_to_byte(char_idx);
    let (mut chunk, mut chunk_byte, mut chunk_char, _) = text.chunk_at_byte(byte_idx);
    let mut cursor = GraphemeCursor::new(byte_idx, text.len_bytes(), true);
    for _ in 0..n {
        loop {
            match cursor.prev_boundary(chunk, chunk_byte) {
                Ok(None) => return 0,
                Ok(Some(prev)) => {
                    byte_idx = prev;
                    break;
                }
                Err(GraphemeIncomplete::PrevChunk) => {
                    (chunk, chunk_byte, chunk_char, _) = text.chunk_at_byte(chunk_byte - 1);
                }
                Err(GraphemeIncomplete::PreContext(end)) => provide_context(&mut cursor, text, end),
                Err(_) => unreachable!(),
            }
        }
    }
    chunk_char + byte_to_char_idx(chunk, byte_idx - chunk_byte)
}

pub fn is_boundary(text: RopeSlice, char_idx: usize) -> bool {
    let byte_idx = text.char_to_byte(char_idx);
    let (chunk, chunk_byte, _, _) = text.chunk_at_byte(byte_idx);
    let mut cursor = GraphemeCursor::new(byte_idx, text.len_bytes(), true);
    loop {
        match cursor.is_boundary(chunk, chunk_byte) {
            Ok(result) => return result,
            Err(GraphemeIncomplete::PreContext(end)) => provide_context(&mut cursor, text, end),
            Err(_) => unreachable!(),
        }
    }
}

pub fn align_prev(text: RopeSlice, char_idx: usize) -> usize {
    if is_boundary(text, char_idx) {
        char_idx
    } else {
        prev_boundary(text, char_idx)
    }
}

pub fn align_next(text: RopeSlice, char_idx: usize) -> usize {
    if is_boundary(text, char_idx) {
        char_idx
    } else {
        next_boundary(text, char_idx)
    }
}

fn provide_context(cursor: &mut GraphemeCursor, text: RopeSlice, end: usize) {
    let (chunk, chunk_byte, _, _) = text.chunk_at_byte(end - 1);
    cursor.provide_context(chunk, chunk_byte);
}

pub struct Graphemes<'a> {
    text: RopeSlice<'a>,
    chunks: Chunks<'a>,
    chunk: &'a str,
    chunk_byte: usize,
    cursor: GraphemeCursor,
}

pub fn graphemes(text: RopeSlice<'_>) -> Graphemes<'_> {
    let mut chunks = text.chunks();
    let chunk = chunks.next().unwrap_or_default();
    Graphemes {
        text,
        chunks,
        chunk,
        chunk_byte: 0,
        cursor: GraphemeCursor::new(0, text.len_bytes(), true),
    }
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = RopeSlice<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.cursor.cur_cursor();
        let end = loop {
            match self.cursor.next_boundary(self.chunk, self.chunk_byte) {
                Ok(None) => return None,
                Ok(Some(end)) => break end,
                Err(GraphemeIncomplete::NextChunk) => {
                    self.chunk_byte += self.chunk.len();
                    self.chunk = self.chunks.next().unwrap_or_default();
                }
                Err(GraphemeIncomplete::PreContext(end)) => {
                    provide_context(&mut self.cursor, self.text, end);
                }
                Err(_) => unreachable!(),
            }
        };
        Some(if start >= self.chunk_byte {
            RopeSlice::from(&self.chunk[start - self.chunk_byte..end - self.chunk_byte])
        } else {
            self.text.byte_slice(start..end)
        })
    }
}

pub fn width(grapheme: &str) -> usize {
    if grapheme.as_bytes().first().is_some_and(u8::is_ascii) {
        1
    } else {
        grapheme.width().clamp(1, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn steps_over_single_chars() {
        let rope = Rope::from("abc");
        assert_eq!(next_boundary(rope.slice(..), 0), 1);
        assert_eq!(prev_boundary(rope.slice(..), 2), 1);
    }

    #[test]
    fn steps_over_combining_marks() {
        let rope = Rope::from("e\u{301}x");
        assert_eq!(next_boundary(rope.slice(..), 0), 2);
        assert_eq!(prev_boundary(rope.slice(..), 2), 0);
    }

    #[test]
    fn treats_crlf_as_one_grapheme() {
        let rope = Rope::from("a\r\nb");
        assert_eq!(next_boundary(rope.slice(..), 1), 3);
        assert_eq!(prev_boundary(rope.slice(..), 3), 1);
    }

    #[test]
    fn clamps_at_the_edges() {
        let rope = Rope::from("ab");
        assert_eq!(next_boundary(rope.slice(..), 2), 2);
        assert_eq!(prev_boundary(rope.slice(..), 0), 0);
        assert_eq!(next_boundary(Rope::new().slice(..), 0), 0);
    }

    #[test]
    fn steps_n_graphemes() {
        let rope = Rope::from("ab\u{301}cd");
        assert_eq!(nth_next_boundary(rope.slice(..), 0, 2), 3);
        assert_eq!(nth_prev_boundary(rope.slice(..), 5, 3), 1);
        assert_eq!(nth_next_boundary(rope.slice(..), 0, 10), 5);
    }

    #[test]
    fn detects_boundaries() {
        let rope = Rope::from("e\u{301}x");
        let text = rope.slice(..);
        assert!(is_boundary(text, 0));
        assert!(!is_boundary(text, 1));
        assert!(is_boundary(text, 2));
        assert!(is_boundary(text, 3));
    }

    #[test]
    fn aligns_to_boundaries() {
        let rope = Rope::from("e\u{301}x");
        let text = rope.slice(..);
        assert_eq!(align_prev(text, 1), 0);
        assert_eq!(align_next(text, 1), 2);
        assert_eq!(align_next(text, 2), 2);
        assert_eq!(align_prev(text, 2), 2);
    }

    #[test]
    fn steps_across_chunk_boundaries() {
        let rope = Rope::from("e\u{301}".repeat(3000));
        assert!(rope.chunks().count() > 1);
        let text = rope.slice(..);
        let mut pos = 0;
        while pos < text.len_chars() {
            let next = next_boundary(text, pos);
            assert_eq!(next, pos + 2);
            assert_eq!(prev_boundary(text, next), pos);
            assert!(!is_boundary(text, pos + 1));
            pos = next;
        }
    }

    #[test]
    fn iterates_graphemes() {
        let rope = Rope::from("ae\u{301}\r\n中");
        let found: Vec<String> = graphemes(rope.slice(..)).map(String::from).collect();
        assert_eq!(found, ["a", "e\u{301}", "\r\n", "中"]);
    }

    #[test]
    fn iterates_graphemes_across_chunks() {
        let rope = Rope::from("e\u{301}".repeat(3000));
        assert_eq!(graphemes(rope.slice(..)).count(), 3000);
        assert!(graphemes(rope.slice(..)).all(|g| g == "e\u{301}"));
    }

    #[test]
    fn measures_display_width() {
        assert_eq!(width("a"), 1);
        assert_eq!(width("中"), 2);
        assert_eq!(width("e\u{301}"), 1);
        assert_eq!(width("😀"), 2);
        assert_eq!(width("\u{200b}"), 1);
        assert_eq!(width("\u{1}"), 1);
    }
}
