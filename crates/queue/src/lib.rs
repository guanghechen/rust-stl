mod circular_queue;
mod error;
mod linked_deque;
mod priority_queue;
mod traits;

pub mod prelude;

pub use circular_queue::CircularQueue;
pub use collection::{Collection, Disposable};
pub use error::QueueError;
pub use linked_deque::LinkedDeque;
pub use priority_queue::{PriorityQueue, PriorityQueueLike};
pub use traits::{CircularQueueLike, DequeLike, QueueLike};
