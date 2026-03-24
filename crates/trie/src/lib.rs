mod error;
mod trie;
mod util;

pub mod prelude;

pub use collection::{Collection, Disposable};
pub use error::TrieError;
pub use trie::{Trie, TrieNodeData, TrieOptions};
pub use util::{alpha_numeric_idx, digit_idx, lowercase_idx, uppercase_idx};
