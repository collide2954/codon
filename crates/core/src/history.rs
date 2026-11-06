use ropey::RopeSlice;

use crate::change::ChangeSet;
use crate::selection::Selection;

pub struct Step<'a> {
    pub changes: &'a ChangeSet,
    pub selection: &'a Selection,
}

struct Revision {
    parent: usize,
    last_child: Option<usize>,
    changes: ChangeSet,
    inversion: ChangeSet,
    selection_before: Selection,
    selection_after: Selection,
}

pub struct History {
    revisions: Vec<Revision>,
    current: usize,
}

impl Default for History {
    fn default() -> Self {
        let root = Revision {
            parent: 0,
            last_child: None,
            changes: ChangeSet::identity(0),
            inversion: ChangeSet::identity(0),
            selection_before: Selection::point(0),
            selection_after: Selection::point(0),
        };
        Self {
            revisions: vec![root],
            current: 0,
        }
    }
}

impl History {
    pub fn commit(
        &mut self,
        changes: ChangeSet,
        original: RopeSlice,
        selection_before: Selection,
        selection_after: Selection,
    ) {
        if changes.is_identity() {
            return;
        }
        let index = self.revisions.len();
        self.revisions[self.current].last_child = Some(index);
        self.revisions.push(Revision {
            parent: self.current,
            last_child: None,
            inversion: changes.invert(original),
            changes,
            selection_before,
            selection_after,
        });
        self.current = index;
    }

    pub fn undo(&mut self) -> Option<Step<'_>> {
        if self.current == 0 {
            return None;
        }
        let revision = &self.revisions[self.current];
        self.current = revision.parent;
        Some(Step {
            changes: &revision.inversion,
            selection: &revision.selection_before,
        })
    }

    pub fn redo(&mut self) -> Option<Step<'_>> {
        let child = self.revisions[self.current].last_child?;
        self.current = child;
        let revision = &self.revisions[child];
        Some(Step {
            changes: &revision.changes,
            selection: &revision.selection_after,
        })
    }

    pub fn current(&self) -> usize {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::Change;
    use ropey::Rope;

    fn edit(history: &mut History, rope: &mut Rope, change: Change) {
        let changes = ChangeSet::from_changes(rope.len_chars(), [change]);
        let original = rope.clone();
        changes.apply(rope);
        let after = Selection::point(rope.len_chars());
        history.commit(changes, original.slice(..), Selection::point(0), after);
    }

    fn walk(rope: &mut Rope, step: Option<Step>) -> bool {
        step.map(|step| step.changes.apply(rope)).is_some()
    }

    fn append(text: &str, at: usize) -> Change {
        (at, at, Some(text.to_owned()))
    }

    #[test]
    fn undo_and_redo_walk_back_and_forth() {
        let mut history = History::default();
        let mut rope = Rope::from("a");
        edit(&mut history, &mut rope, append("b", 1));
        edit(&mut history, &mut rope, append("c", 2));

        assert!(walk(&mut rope, history.undo()));
        assert_eq!(rope, "ab");
        assert!(walk(&mut rope, history.undo()));
        assert_eq!(rope, "a");
        assert!(!walk(&mut rope, history.undo()));

        assert!(walk(&mut rope, history.redo()));
        assert!(walk(&mut rope, history.redo()));
        assert_eq!(rope, "abc");
        assert!(!walk(&mut rope, history.redo()));
    }

    #[test]
    fn redo_follows_the_newest_branch() {
        let mut history = History::default();
        let mut rope = Rope::from("a");
        edit(&mut history, &mut rope, append("b", 1));
        walk(&mut rope, history.undo());
        edit(&mut history, &mut rope, append("x", 1));
        assert!(!walk(&mut rope, history.redo()));

        walk(&mut rope, history.undo());
        assert_eq!(rope, "a");
        walk(&mut rope, history.redo());
        assert_eq!(rope, "ax");
    }

    #[test]
    fn steps_restore_the_selection_from_either_side() {
        let mut history = History::default();
        let mut rope = Rope::from("a");
        edit(&mut history, &mut rope, append("bc", 1));
        assert_eq!(history.undo().unwrap().selection, &Selection::point(0));
        assert_eq!(history.redo().unwrap().selection, &Selection::point(3));
    }

    #[test]
    fn identity_changes_are_not_recorded() {
        let mut history = History::default();
        let rope = Rope::from("abc");
        history.commit(
            ChangeSet::identity(3),
            rope.slice(..),
            Selection::point(0),
            Selection::point(0),
        );
        assert_eq!(history.current(), 0);
        assert!(history.undo().is_none());
    }

    #[test]
    fn revisions_identify_states() {
        let mut history = History::default();
        let mut rope = Rope::from("a");
        let root = history.current();
        edit(&mut history, &mut rope, append("b", 1));
        let first = history.current();
        assert_ne!(first, root);
        history.undo();
        assert_eq!(history.current(), root);
        history.redo();
        assert_eq!(history.current(), first);
    }
}
