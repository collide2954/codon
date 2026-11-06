use crate::selection::{Range, Selection};

struct Open {
    start: usize,
    primary: bool,
    head_first: bool,
}

pub fn parse(input: &str) -> (String, Selection) {
    let mut text = String::new();
    let mut len = 0;
    let mut ranges = Vec::new();
    let mut primary = None;
    let mut open: Option<Open> = None;
    let mut rest = input;
    while let Some(c) = rest.chars().next() {
        if open.is_none() && (rest.starts_with("#[") || rest.starts_with("#(")) {
            let head_first = rest[2..].starts_with('|');
            open = Some(Open {
                start: len,
                primary: rest.starts_with("#["),
                head_first,
            });
            rest = &rest[2 + usize::from(head_first)..];
            continue;
        }
        if let Some(range) = &open {
            let close = if range.primary { "]#" } else { ")#" };
            let head_last = rest.starts_with('|') && rest[1..].starts_with(close);
            if head_last || rest.starts_with(close) {
                assert!(
                    range.head_first != head_last,
                    "a range needs one | at its start or end"
                );
                if range.primary {
                    assert!(primary.is_none(), "more than one primary range");
                    primary = Some(ranges.len());
                }
                ranges.push(if range.head_first {
                    Range::new(len, range.start)
                } else {
                    Range::new(range.start, len)
                });
                rest = &rest[close.len() + usize::from(head_last)..];
                open = None;
                continue;
            }
        }
        text.push(c);
        len += 1;
        rest = &rest[c.len_utf8()..];
    }
    assert!(open.is_none(), "unclosed range");
    let primary = primary.expect("no primary range");
    (text, Selection::new(ranges, primary))
}

pub fn render(text: &str, selection: &Selection) -> String {
    let mut markers = Vec::new();
    for (index, range) in selection.iter().enumerate() {
        let (open, close) = if index == selection.primary_index() {
            ("#[", "]#")
        } else {
            ("#(", ")#")
        };
        let head_first = range.head < range.anchor || range.is_empty();
        let bar = |shown: bool| if shown { "|" } else { "" };
        markers.push((range.from(), format!("{open}{}", bar(head_first))));
        markers.push((range.to(), format!("{}{close}", bar(!head_first))));
    }
    let mut markers = markers.into_iter().peekable();
    let mut out = String::new();
    for (pos, c) in text.chars().enumerate() {
        while let Some((_, marker)) = markers.next_if(|(at, _)| *at == pos) {
            out.push_str(&marker);
        }
        out.push(c);
    }
    markers.for_each(|(_, marker)| out.push_str(&marker));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::Range;

    #[test]
    fn parses_a_forward_range() {
        let (text, selection) = parse("#[h|]#ello");
        assert_eq!(text, "hello");
        assert_eq!(selection, Selection::single(0, 1));
    }

    #[test]
    fn parses_a_backward_range() {
        let (text, selection) = parse("#[|he]#llo");
        assert_eq!(text, "hello");
        assert_eq!(selection, Selection::single(2, 0));
    }

    #[test]
    fn parses_a_point() {
        let (text, selection) = parse("ab#[|]#");
        assert_eq!(text, "ab");
        assert_eq!(selection, Selection::point(2));
    }

    #[test]
    fn parses_secondary_ranges() {
        let (text, selection) = parse("a#(b|)#c#[d|]#");
        assert_eq!(text, "abcd");
        assert_eq!(selection.ranges(), [Range::new(1, 2), Range::new(3, 4)]);
        assert_eq!(selection.primary_index(), 1);
    }

    #[test]
    fn keeps_unicode_positions_in_chars() {
        let (text, selection) = parse("é#[中|]#");
        assert_eq!(text, "é中");
        assert_eq!(selection, Selection::single(1, 2));
    }

    #[test]
    fn renders_what_it_parses() {
        for input in [
            "#[h|]#ello",
            "#[|he]#llo",
            "ab#[|]#",
            "a#(b|)##[c|]#d",
            "#(|a)#b#[|c]#",
        ] {
            let (text, selection) = parse(input);
            assert_eq!(render(&text, &selection), input);
        }
    }

    #[test]
    #[should_panic(expected = "primary")]
    fn requires_a_primary_range() {
        let _ = parse("#(a|)#");
    }
}
