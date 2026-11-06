pub mod change;
pub mod chars;
pub mod grapheme;
pub mod history;
pub mod line_ending;
pub mod movement;
pub mod position;
pub mod selection;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod transaction;

pub use change::{Assoc, Change, ChangeSet, Operation};
pub use history::History;
pub use line_ending::LineEnding;
pub use ropey::{Rope, RopeSlice};
pub use selection::{Direction, Range, Selection};
pub use transaction::Transaction;
