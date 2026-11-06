use ropey::{Rope, RopeSlice};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Retain(usize),
    Delete(usize),
    Insert(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assoc {
    Before,
    After,
}

pub type Change = (usize, usize, Option<String>);

#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeSet {
    ops: Vec<Operation>,
    len_before: usize,
    len_after: usize,
}

impl ChangeSet {
    pub fn identity(len: usize) -> Self {
        Self::from_changes(len, [])
    }

    pub fn from_changes(len: usize, changes: impl IntoIterator<Item = Change>) -> Self {
        let mut set = Self::empty();
        let mut last = 0;
        for (from, to, text) in changes {
            if to < last {
                continue;
            }
            let from = from.max(last);
            set.retain(from - last);
            if let Some(text) = text {
                set.insert(text);
            }
            set.delete(to - from);
            last = to;
        }
        set.retain(len - last);
        set
    }

    fn empty() -> Self {
        Self {
            ops: Vec::new(),
            len_before: 0,
            len_after: 0,
        }
    }

    fn retain(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        self.len_before += n;
        self.len_after += n;
        if let Some(Operation::Retain(count)) = self.ops.last_mut() {
            *count += n;
        } else {
            self.ops.push(Operation::Retain(n));
        }
    }

    fn delete(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        self.len_before += n;
        if let Some(Operation::Delete(count)) = self.ops.last_mut() {
            *count += n;
        } else {
            self.ops.push(Operation::Delete(n));
        }
    }

    fn insert(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.len_after += text.chars().count();
        match self.ops.as_mut_slice() {
            [.., Operation::Insert(prev)] | [.., Operation::Insert(prev), Operation::Delete(_)] => {
                prev.push_str(&text);
            }
            [.., last @ Operation::Delete(_)] => {
                let delete = std::mem::replace(last, Operation::Insert(text));
                self.ops.push(delete);
            }
            _ => self.ops.push(Operation::Insert(text)),
        }
    }

    pub fn len_before(&self) -> usize {
        self.len_before
    }

    pub fn len_after(&self) -> usize {
        self.len_after
    }

    pub fn ops(&self) -> &[Operation] {
        &self.ops
    }

    pub fn is_identity(&self) -> bool {
        self.ops.iter().all(|op| matches!(op, Operation::Retain(_)))
    }

    pub fn apply(&self, text: &mut Rope) {
        assert_eq!(text.len_chars(), self.len_before, "change set does not fit");
        let mut pos = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) => pos += n,
                Operation::Delete(n) => text.remove(pos..pos + n),
                Operation::Insert(s) => {
                    text.insert(pos, s);
                    pos += s.chars().count();
                }
            }
        }
    }

    pub fn invert(&self, original: RopeSlice) -> Self {
        assert_eq!(
            original.len_chars(),
            self.len_before,
            "change set does not fit"
        );
        let mut inverted = Self::empty();
        let mut pos = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) => {
                    inverted.retain(*n);
                    pos += n;
                }
                Operation::Delete(n) => {
                    inverted.insert(original.slice(pos..pos + n).to_string());
                    pos += n;
                }
                Operation::Insert(s) => inverted.delete(s.chars().count()),
            }
        }
        inverted
    }

    pub fn compose(self, other: Self) -> Self {
        use Operation::{Delete, Insert, Retain};
        assert_eq!(
            self.len_after, other.len_before,
            "change sets do not compose"
        );
        let mut composed = Self::empty();
        let mut first = self.ops.into_iter();
        let mut second = other.ops.into_iter();
        let mut left = first.next();
        let mut right = second.next();
        loop {
            match (left, right) {
                (None, None) => break,
                (Some(Delete(count)), next) => {
                    composed.delete(count);
                    left = first.next();
                    right = next;
                }
                (next, Some(Insert(text))) => {
                    composed.insert(text);
                    left = next;
                    right = second.next();
                }
                (Some(op), None) | (None, Some(op)) => {
                    unreachable!("unbalanced change sets: {op:?}")
                }
                (Some(Retain(kept)), Some(Retain(retained))) => {
                    let step = kept.min(retained);
                    composed.retain(step);
                    left = remainder(kept, step, Retain).or_else(|| first.next());
                    right = remainder(retained, step, Retain).or_else(|| second.next());
                }
                (Some(Retain(kept)), Some(Delete(deleted))) => {
                    let step = kept.min(deleted);
                    composed.delete(step);
                    left = remainder(kept, step, Retain).or_else(|| first.next());
                    right = remainder(deleted, step, Delete).or_else(|| second.next());
                }
                (Some(Insert(text)), Some(Retain(retained))) => {
                    let len = text.chars().count();
                    let (head, rest) = split_at_char(text, retained);
                    composed.insert(head);
                    left = rest.map(Insert).or_else(|| first.next());
                    right =
                        remainder(retained, len.min(retained), Retain).or_else(|| second.next());
                }
                (Some(Insert(text)), Some(Delete(deleted))) => {
                    let len = text.chars().count();
                    let (_, rest) = split_at_char(text, deleted);
                    left = rest.map(Insert).or_else(|| first.next());
                    right = remainder(deleted, len.min(deleted), Delete).or_else(|| second.next());
                }
            }
        }
        composed
    }

    pub fn map_pos(&self, pos: usize, assoc: Assoc) -> usize {
        let mut old = 0;
        let mut new = 0;
        for op in &self.ops {
            match op {
                Operation::Retain(n) => {
                    if pos < old + n {
                        return new + (pos - old);
                    }
                    old += n;
                    new += n;
                }
                Operation::Delete(n) => {
                    if pos < old + n {
                        return new;
                    }
                    old += n;
                }
                Operation::Insert(s) => {
                    if pos == old && assoc == Assoc::Before {
                        return new;
                    }
                    new += s.chars().count();
                }
            }
        }
        new
    }

    pub fn changes(&self) -> impl Iterator<Item = (usize, usize, Option<&str>)> {
        let mut pos = 0;
        let mut ops = self.ops.iter().peekable();
        std::iter::from_fn(move || {
            loop {
                match ops.next()? {
                    Operation::Retain(n) => pos += n,
                    Operation::Delete(n) => {
                        let from = pos;
                        pos += n;
                        return Some((from, pos, None));
                    }
                    Operation::Insert(s) => {
                        let from = pos;
                        if let Some(Operation::Delete(n)) =
                            ops.next_if(|op| matches!(op, Operation::Delete(_)))
                        {
                            pos += n;
                        }
                        return Some((from, pos, Some(s.as_str())));
                    }
                }
            }
        })
    }
}

fn remainder(total: usize, used: usize, op: fn(usize) -> Operation) -> Option<Operation> {
    (total > used).then(|| op(total - used))
}

fn split_at_char(mut s: String, chars: usize) -> (String, Option<String>) {
    match s.char_indices().nth(chars) {
        Some((byte, _)) => {
            let rest = s.split_off(byte);
            (s, Some(rest))
        }
        None => (s, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert(from: usize, to: usize, text: &str) -> Change {
        (from, to, Some(text.to_owned()))
    }

    fn delete(from: usize, to: usize) -> Change {
        (from, to, None)
    }

    fn apply(text: &str, changes: Vec<Change>) -> String {
        let mut rope = Rope::from(text);
        ChangeSet::from_changes(rope.len_chars(), changes).apply(&mut rope);
        rope.to_string()
    }

    #[test]
    fn inserts_deletes_and_replaces() {
        assert_eq!(apply("hello", vec![insert(0, 0, "oh ")]), "oh hello");
        assert_eq!(apply("hello", vec![delete(1, 3)]), "hlo");
        assert_eq!(apply("hello", vec![insert(1, 3, "EY")]), "hEYlo");
    }

    #[test]
    fn applies_changes_given_in_original_coordinates() {
        let changes = vec![insert(0, 1, "X"), insert(2, 2, "Y"), delete(4, 6)];
        assert_eq!(apply("abcdef", changes), "XbYcd");
    }

    #[test]
    fn clips_overlapping_changes() {
        assert_eq!(apply("abcdef", vec![delete(0, 3), delete(2, 4)]), "ef");
        assert_eq!(
            apply("abcdef", vec![delete(0, 3), insert(1, 1, "x")]),
            "def"
        );
    }

    #[test]
    fn tracks_lengths_on_both_sides() {
        let set = ChangeSet::from_changes(5, [insert(1, 3, "XYZ")]);
        assert_eq!((set.len_before(), set.len_after()), (5, 6));
    }

    #[test]
    fn identity_changes_nothing() {
        assert!(ChangeSet::identity(3).is_identity());
        assert!(ChangeSet::from_changes(3, [delete(1, 1)]).is_identity());
        assert!(!ChangeSet::from_changes(3, [delete(1, 2)]).is_identity());
    }

    #[test]
    fn inversion_restores_the_original() {
        let original = Rope::from("hello world");
        let set = ChangeSet::from_changes(
            11,
            [
                insert(0, 5, "goodbye"),
                insert(6, 6, "cruel "),
                delete(10, 11),
            ],
        );
        let mut rope = original.clone();
        set.apply(&mut rope);
        assert_eq!(rope, "goodbye cruel worl");
        set.invert(original.slice(..)).apply(&mut rope);
        assert_eq!(rope, original);
    }

    #[test]
    fn composition_matches_sequential_application() {
        let first = ChangeSet::from_changes(5, [insert(0, 0, "oh "), insert(5, 5, "!")]);
        let second = ChangeSet::from_changes(9, [delete(0, 3), insert(3, 7, "HELL")]);
        let mut sequential = Rope::from("hello");
        first.apply(&mut sequential);
        second.apply(&mut sequential);
        let mut composed = Rope::from("hello");
        first.compose(second).apply(&mut composed);
        assert_eq!(composed, sequential);
        assert_eq!(composed, "HELLo!");
    }

    #[test]
    fn composition_splits_inserted_text() {
        let first = ChangeSet::from_changes(2, [insert(1, 1, "xyz")]);
        let second = ChangeSet::from_changes(5, [delete(2, 3)]);
        let mut rope = Rope::from("ab");
        first.compose(second).apply(&mut rope);
        assert_eq!(rope, "axzb");
    }

    #[test]
    fn composition_from_an_empty_document() {
        let first = ChangeSet::from_changes(0, [insert(0, 0, "a")]);
        let second = ChangeSet::from_changes(1, [insert(1, 1, "b")]);
        let composed = first.compose(second);
        let mut rope = Rope::new();
        composed.apply(&mut rope);
        assert_eq!(rope, "ab");
    }

    #[test]
    fn maps_positions_around_an_insertion() {
        let set = ChangeSet::from_changes(5, [insert(2, 2, "XY")]);
        assert_eq!(set.map_pos(1, Assoc::Before), 1);
        assert_eq!(set.map_pos(2, Assoc::Before), 2);
        assert_eq!(set.map_pos(2, Assoc::After), 4);
        assert_eq!(set.map_pos(3, Assoc::Before), 5);
        assert_eq!(set.map_pos(5, Assoc::After), 7);
    }

    #[test]
    fn maps_positions_inside_a_deletion_to_its_start() {
        let set = ChangeSet::from_changes(6, [delete(1, 4)]);
        for pos in 1..=4 {
            assert_eq!(set.map_pos(pos, Assoc::After), 1, "{pos}");
        }
        assert_eq!(set.map_pos(5, Assoc::Before), 2);
    }

    #[test]
    fn maps_positions_through_a_replacement() {
        let set = ChangeSet::from_changes(5, [insert(1, 3, "XYZ")]);
        assert_eq!(set.map_pos(1, Assoc::Before), 1);
        assert_eq!(set.map_pos(1, Assoc::After), 4);
        assert_eq!(set.map_pos(2, Assoc::Before), 4);
        assert_eq!(set.map_pos(3, Assoc::Before), 4);
        assert_eq!(set.map_pos(4, Assoc::Before), 5);
    }

    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }

        fn text(&mut self, max: usize) -> String {
            const ALPHABET: [char; 5] = ['a', 'b', 'é', '中', '\n'];
            (0..self.below(max + 1))
                .map(|_| ALPHABET[self.below(ALPHABET.len())])
                .collect()
        }

        fn changes(&mut self, len: usize) -> Vec<Change> {
            let mut changes = Vec::new();
            let mut pos = 0;
            while pos <= len && self.below(4) != 0 {
                let from = pos + self.below(len - pos + 1);
                let to = from + self.below(len - from + 1).min(3);
                let text = (self.below(2) == 0).then(|| self.text(3));
                changes.push((from, to, text));
                pos = to + 1;
            }
            changes
        }
    }

    #[test]
    fn random_edits_compose_and_invert_consistently() {
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        for _ in 0..1000 {
            let original = Rope::from(rng.text(12));
            let first =
                ChangeSet::from_changes(original.len_chars(), rng.changes(original.len_chars()));
            let mut middle = original.clone();
            first.apply(&mut middle);
            let second =
                ChangeSet::from_changes(middle.len_chars(), rng.changes(middle.len_chars()));
            let mut sequential = middle.clone();
            second.apply(&mut sequential);

            let composed = first.compose(second);
            let mut direct = original.clone();
            composed.apply(&mut direct);
            assert_eq!(direct, sequential);

            composed.invert(original.slice(..)).apply(&mut direct);
            assert_eq!(direct, original);
        }
    }

    #[test]
    fn lists_changes_in_original_coordinates() {
        let set = ChangeSet::from_changes(6, [insert(1, 2, "X"), delete(4, 6)]);
        let changes: Vec<_> = set.changes().collect();
        assert_eq!(changes, [(1, 2, Some("X")), (4, 6, None)]);
    }
}
