use std::borrow::Cow;
use std::cmp::Ordering;

use ropey::RopeSlice;

use crate::grapheme::{align_next, align_prev, next_boundary, prev_boundary};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub anchor: usize,
    pub head: usize,
    pub column: Option<usize>,
}

impl Range {
    pub fn new(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            column: None,
        }
    }

    pub fn point(pos: usize) -> Self {
        Self::new(pos, pos)
    }

    pub fn from(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn to(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn len(&self) -> usize {
        self.to() - self.from()
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn direction(&self) -> Direction {
        if self.head < self.anchor {
            Direction::Backward
        } else {
            Direction::Forward
        }
    }

    pub fn flip(self) -> Self {
        Self {
            anchor: self.head,
            head: self.anchor,
            column: self.column,
        }
    }

    pub fn with_direction(self, direction: Direction) -> Self {
        if self.direction() == direction {
            self
        } else {
            self.flip()
        }
    }

    pub fn contains(&self, pos: usize) -> bool {
        self.from() <= pos && pos < self.to()
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        self.from() == other.from() || (self.to() > other.from() && other.to() > self.from())
    }

    pub fn merge(self, other: Self) -> Self {
        if self.direction() == Direction::Backward && other.direction() == Direction::Backward {
            Self::new(self.anchor.max(other.anchor), self.head.min(other.head))
        } else {
            Self::new(self.from().min(other.from()), self.to().max(other.to()))
        }
    }

    pub fn cursor(&self, text: RopeSlice) -> usize {
        if self.head > self.anchor {
            prev_boundary(text, self.head)
        } else {
            self.head
        }
    }

    pub fn cursor_line(&self, text: RopeSlice) -> usize {
        text.char_to_line(self.cursor(text))
    }

    pub fn put_cursor(self, text: RopeSlice, pos: usize, extend: bool) -> Self {
        if !extend {
            return Self::point(pos).grapheme_aligned(text);
        }
        let anchor = if self.head >= self.anchor && pos < self.anchor {
            next_boundary(text, self.anchor)
        } else if self.head < self.anchor && pos >= self.anchor {
            prev_boundary(text, self.anchor)
        } else {
            self.anchor
        };
        if anchor <= pos {
            Self::new(anchor, next_boundary(text, pos))
        } else {
            Self::new(anchor, pos)
        }
    }

    pub fn min_width_1(self, text: RopeSlice) -> Self {
        if self.is_empty() {
            Self {
                head: next_boundary(text, self.head),
                ..self
            }
        } else {
            self
        }
    }

    pub fn grapheme_aligned(self, text: RopeSlice) -> Self {
        let (anchor, head) = match self.anchor.cmp(&self.head) {
            Ordering::Equal => {
                let pos = align_prev(text, self.anchor);
                (pos, pos)
            }
            Ordering::Less => (align_prev(text, self.anchor), align_next(text, self.head)),
            Ordering::Greater => (align_next(text, self.anchor), align_prev(text, self.head)),
        };
        Self {
            anchor,
            head,
            column: self.column.filter(|_| head == self.head),
        }
    }

    pub fn line_range(&self, text: RopeSlice) -> (usize, usize) {
        let from = self.from();
        let to = if self.is_empty() {
            self.to()
        } else {
            prev_boundary(text, self.to()).max(from)
        };
        (text.char_to_line(from), text.char_to_line(to))
    }

    pub fn fragment<'a>(&self, text: RopeSlice<'a>) -> Cow<'a, str> {
        text.slice(self.from()..self.to()).into()
    }
}

#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    ranges: Vec<Range>,
    primary: usize,
}

impl Selection {
    pub fn new(ranges: Vec<Range>, primary: usize) -> Self {
        assert!(primary < ranges.len(), "primary range out of bounds");
        Self { ranges, primary }.normalize()
    }

    pub fn single(anchor: usize, head: usize) -> Self {
        Self::new(vec![Range::new(anchor, head)], 0)
    }

    pub fn point(pos: usize) -> Self {
        Self::single(pos, pos)
    }

    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    pub fn primary(&self) -> Range {
        self.ranges[self.primary]
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Range> {
        self.ranges.iter()
    }

    pub fn transform(mut self, mut f: impl FnMut(Range) -> Range) -> Self {
        for range in &mut self.ranges {
            *range = f(*range);
        }
        self.normalize()
    }

    pub fn push(mut self, range: Range) -> Self {
        self.ranges.push(range);
        self.primary = self.ranges.len() - 1;
        self.normalize()
    }

    pub fn remove(mut self, index: usize) -> Self {
        if self.ranges.len() == 1 {
            return self;
        }
        let _ = self.ranges.remove(index);
        if index < self.primary || self.primary == self.ranges.len() {
            self.primary -= 1;
        }
        self
    }

    pub fn into_single(self) -> Self {
        Self {
            ranges: vec![self.primary()],
            primary: 0,
        }
    }

    pub fn with_primary(mut self, index: usize) -> Self {
        assert!(index < self.ranges.len(), "primary range out of bounds");
        self.primary = index;
        self
    }

    pub fn cursors(self, text: RopeSlice) -> Self {
        self.transform(|range| Range::point(range.cursor(text)))
    }

    pub fn ensure_invariants(self, text: RopeSlice) -> Self {
        self.transform(|range| range.min_width_1(text).grapheme_aligned(text))
    }

    pub fn fragments<'a>(&self, text: RopeSlice<'a>) -> impl Iterator<Item = Cow<'a, str>> {
        self.ranges.iter().map(move |range| range.fragment(text))
    }

    fn normalize(mut self) -> Self {
        if self.ranges.len() < 2 {
            return self;
        }
        let mut primary = self.ranges[self.primary];
        self.ranges.sort_unstable_by_key(Range::from);
        let mut merged: Vec<Range> = Vec::with_capacity(self.ranges.len());
        for range in self.ranges {
            match merged.last_mut() {
                Some(last) if last.overlaps(&range) => {
                    let union = last.merge(range);
                    if *last == primary || range == primary {
                        primary = union;
                    }
                    *last = union;
                }
                _ => merged.push(range),
            }
        }
        self.primary = merged.iter().position(|r| *r == primary).unwrap_or(0);
        self.ranges = merged;
        self
    }
}

impl<'a> IntoIterator for &'a Selection {
    type Item = &'a Range;
    type IntoIter = std::slice::Iter<'a, Range>;

    fn into_iter(self) -> Self::IntoIter {
        self.ranges.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    fn pair(range: Range) -> (usize, usize) {
        (range.anchor, range.head)
    }

    fn pairs(selection: &Selection) -> Vec<(usize, usize)> {
        selection.iter().copied().map(pair).collect()
    }

    #[test]
    fn bounds_ignore_direction() {
        let range = Range::new(5, 2);
        assert_eq!((range.from(), range.to(), range.len()), (2, 5, 3));
        assert!(!range.is_empty());
        assert!(Range::point(3).is_empty());
    }

    #[test]
    fn reports_and_changes_direction() {
        assert_eq!(Range::new(1, 4).direction(), Direction::Forward);
        assert_eq!(Range::new(4, 1).direction(), Direction::Backward);
        assert_eq!(Range::point(2).direction(), Direction::Forward);
        assert_eq!(pair(Range::new(1, 4).flip()), (4, 1));
        assert_eq!(
            pair(Range::new(1, 4).with_direction(Direction::Backward)),
            (4, 1)
        );
        assert_eq!(
            pair(Range::new(1, 4).with_direction(Direction::Forward)),
            (1, 4)
        );
    }

    #[test]
    fn contains_is_half_open() {
        let range = Range::new(4, 2);
        assert!(!range.contains(1));
        assert!(range.contains(2));
        assert!(range.contains(3));
        assert!(!range.contains(4));
    }

    #[test]
    fn overlap_excludes_touching_ranges() {
        assert!(Range::new(0, 2).overlaps(&Range::new(1, 3)));
        assert!(Range::new(0, 4).overlaps(&Range::new(1, 2)));
        assert!(!Range::new(0, 2).overlaps(&Range::new(2, 4)));
        assert!(Range::point(2).overlaps(&Range::new(2, 4)));
        assert!(Range::point(0).overlaps(&Range::point(0)));
    }

    #[test]
    fn merge_stays_backward_only_when_both_are() {
        assert_eq!(pair(Range::new(3, 1).merge(Range::new(5, 2))), (5, 1));
        assert_eq!(pair(Range::new(1, 3).merge(Range::new(5, 2))), (1, 5));
    }

    #[test]
    fn cursor_sits_on_the_last_grapheme_of_a_forward_range() {
        let rope = Rope::from("e\u{301}xy");
        let text = rope.slice(..);
        assert_eq!(Range::new(0, 2).cursor(text), 0);
        assert_eq!(Range::new(0, 3).cursor(text), 2);
        assert_eq!(Range::new(3, 0).cursor(text), 0);
        assert_eq!(Range::point(3).cursor(text), 3);
    }

    #[test]
    fn cursor_line_follows_the_cursor() {
        let rope = Rope::from("ab\ncd");
        let text = rope.slice(..);
        assert_eq!(Range::new(0, 3).cursor_line(text), 0);
        assert_eq!(Range::new(0, 4).cursor_line(text), 1);
    }

    #[test]
    fn min_width_1_widens_points() {
        let rope = Rope::from("ab\u{301}");
        let text = rope.slice(..);
        assert_eq!(pair(Range::point(1).min_width_1(text)), (1, 3));
        assert_eq!(pair(Range::point(3).min_width_1(text)), (3, 3));
        assert_eq!(pair(Range::new(2, 0).min_width_1(text)), (2, 0));
    }

    #[test]
    fn grapheme_aligned_grows_outward() {
        let rope = Rope::from("e\u{301}x");
        let text = rope.slice(..);
        assert_eq!(pair(Range::point(1).grapheme_aligned(text)), (0, 0));
        assert_eq!(pair(Range::new(1, 2).grapheme_aligned(text)), (0, 2));
        assert_eq!(pair(Range::new(2, 1).grapheme_aligned(text)), (2, 0));
    }

    #[test]
    fn put_cursor_moves_without_extending() {
        let rope = Rope::from("abc");
        let range = Range::new(0, 1).put_cursor(rope.slice(..), 2, false);
        assert_eq!(pair(range), (2, 2));
    }

    #[test]
    fn put_cursor_extends_to_include_the_target() {
        let rope = Rope::from("abcd");
        let text = rope.slice(..);
        assert_eq!(pair(Range::new(0, 1).put_cursor(text, 2, true)), (0, 3));
        assert_eq!(pair(Range::new(2, 3).put_cursor(text, 0, true)), (3, 0));
        assert_eq!(pair(Range::new(2, 0).put_cursor(text, 3, true)), (1, 4));
    }

    #[test]
    fn line_range_excludes_a_trailing_line_start() {
        let rope = Rope::from("ab\ncd\nef");
        let text = rope.slice(..);
        assert_eq!(Range::new(0, 3).line_range(text), (0, 0));
        assert_eq!(Range::new(0, 4).line_range(text), (0, 1));
        assert_eq!(Range::point(4).line_range(text), (1, 1));
    }

    #[test]
    fn fragment_is_the_covered_text() {
        let rope = Rope::from("hello world");
        assert_eq!(Range::new(11, 6).fragment(rope.slice(..)), "world");
    }

    #[test]
    fn new_sorts_and_merges_overlapping_ranges() {
        let ranges = vec![Range::new(5, 7), Range::new(0, 2), Range::new(1, 3)];
        let selection = Selection::new(ranges, 0);
        assert_eq!(pairs(&selection), [(0, 3), (5, 7)]);
        assert_eq!(selection.primary_index(), 1);
    }

    #[test]
    fn merging_into_the_primary_keeps_it_primary() {
        let selection = Selection::new(vec![Range::new(0, 2), Range::new(1, 3)], 1);
        assert_eq!(pairs(&selection), [(0, 3)]);
        assert_eq!(selection.primary_index(), 0);
    }

    #[test]
    fn touching_ranges_stay_separate() {
        let selection = Selection::new(vec![Range::new(0, 2), Range::new(2, 4)], 0);
        assert_eq!(pairs(&selection), [(0, 2), (2, 4)]);
    }

    #[test]
    fn push_makes_the_new_range_primary() {
        let selection = Selection::single(0, 1).push(Range::new(4, 5));
        assert_eq!(pairs(&selection), [(0, 1), (4, 5)]);
        assert_eq!(selection.primary_index(), 1);
    }

    #[test]
    fn remove_keeps_a_sensible_primary() {
        let three = Selection::new(
            vec![Range::new(0, 1), Range::new(2, 3), Range::new(4, 5)],
            1,
        );
        let removed_primary = three.clone().remove(1);
        assert_eq!(pairs(&removed_primary), [(0, 1), (4, 5)]);
        assert_eq!(removed_primary.primary(), Range::new(4, 5));

        let removed_before = three.clone().remove(0);
        assert_eq!(removed_before.primary(), Range::new(2, 3));

        let removed_last_primary = three.with_primary(2).remove(2);
        assert_eq!(removed_last_primary.primary(), Range::new(2, 3));

        assert_eq!(pairs(&Selection::single(0, 1).remove(0)), [(0, 1)]);
    }

    #[test]
    fn into_single_keeps_the_primary() {
        let selection = Selection::new(vec![Range::new(0, 1), Range::new(2, 3)], 1);
        assert_eq!(pairs(&selection.into_single()), [(2, 3)]);
    }

    #[test]
    fn transform_renormalizes() {
        let selection = Selection::new(vec![Range::new(0, 1), Range::new(4, 5)], 1);
        let merged = selection.transform(|range| Range::new(range.anchor, 5));
        assert_eq!(pairs(&merged), [(0, 5)]);
    }

    #[test]
    fn cursors_collapse_each_range() {
        let rope = Rope::from("abcdef");
        let selection = Selection::new(vec![Range::new(0, 2), Range::new(5, 3)], 0);
        assert_eq!(pairs(&selection.cursors(rope.slice(..))), [(1, 1), (3, 3)]);
    }

    #[test]
    fn ensure_invariants_gives_every_range_a_grapheme() {
        let rope = Rope::from("abc");
        let selection = Selection::new(vec![Range::point(0), Range::point(2)], 0);
        let fixed = selection.ensure_invariants(rope.slice(..));
        assert_eq!(pairs(&fixed), [(0, 1), (2, 3)]);
    }

    #[test]
    fn fragments_follow_range_order() {
        let rope = Rope::from("one two");
        let selection = Selection::new(vec![Range::new(4, 7), Range::new(0, 3)], 0);
        let fragments: Vec<_> = selection.fragments(rope.slice(..)).collect();
        assert_eq!(fragments, ["one", "two"]);
    }
}
