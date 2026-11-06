pub mod chars;
pub mod grapheme;
pub mod line_ending;
pub mod selection;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use line_ending::LineEnding;
pub use ropey::{Rope, RopeSlice};
pub use selection::{Direction, Range, Selection};
