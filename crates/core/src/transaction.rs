use ropey::{Rope, RopeSlice};

use crate::change::{Change, ChangeSet};
use crate::selection::{Range, Selection};

#[must_use]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    changes: ChangeSet,
    selection: Option<Selection>,
}

impl Transaction {
    pub fn change(text: RopeSlice, changes: impl IntoIterator<Item = Change>) -> Self {
        ChangeSet::from_changes(text.len_chars(), changes).into()
    }

    pub fn change_by_selection(
        text: RopeSlice,
        selection: &Selection,
        f: impl FnMut(&Range) -> Change,
    ) -> Self {
        Self::change(text, selection.iter().map(f))
    }

    pub fn delete_by_selection(
        text: RopeSlice,
        selection: &Selection,
        mut f: impl FnMut(&Range) -> (usize, usize),
    ) -> Self {
        Self::change(
            text,
            selection.iter().map(|range| {
                let (from, to) = f(range);
                (from, to, None)
            }),
        )
    }

    pub fn with_selection(self, selection: Selection) -> Self {
        Self {
            selection: Some(selection),
            ..self
        }
    }

    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    pub fn apply(&self, text: &mut Rope) {
        self.changes.apply(text);
    }
}

impl From<ChangeSet> for Transaction {
    fn from(changes: ChangeSet) -> Self {
        Self {
            changes,
            selection: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::parse;

    fn edited(text: &str, transaction: &Transaction) -> String {
        let mut rope = Rope::from(text);
        transaction.apply(&mut rope);
        rope.to_string()
    }

    #[test]
    fn changes_every_range() {
        let (text, selection) = parse("#[a|]#b#(c|)#d");
        let rope = Rope::from(text.as_str());
        let transaction = Transaction::change_by_selection(rope.slice(..), &selection, |range| {
            (range.from(), range.to(), Some("X".into()))
        });
        assert_eq!(edited(&text, &transaction), "XbXd");
    }

    #[test]
    fn deletes_overlapping_spans_once() {
        let (text, selection) = parse("a#[b|]#c#(d|)#e");
        let rope = Rope::from(text.as_str());
        let transaction = Transaction::delete_by_selection(rope.slice(..), &selection, |range| {
            (range.from().saturating_sub(2), range.to())
        });
        assert_eq!(edited(&text, &transaction), "e");
    }

    #[test]
    fn carries_an_explicit_selection() {
        let rope = Rope::from("abc");
        let transaction =
            Transaction::change(rope.slice(..), [(0, 1, None)]).with_selection(Selection::point(1));
        assert_eq!(transaction.selection(), Some(&Selection::point(1)));
        assert_eq!(edited("abc", &transaction), "bc");
    }
}
