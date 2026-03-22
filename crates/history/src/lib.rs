mod error;
mod history;
mod traits;

pub mod prelude;

pub use error::HistoryError;
pub use history::{EqualsFn, History, Iter as HistoryIter};
pub use traits::HistoryLike;
