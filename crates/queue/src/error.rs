use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    InvalidCapacity {
        capacity: usize,
    },
    InsufficientCapacity {
        current_size: usize,
        requested_capacity: usize,
    },
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity { capacity } => {
                write!(
                    f,
                    "[CircularQueue] capacity is expected to be a positive integer, but got ({capacity})."
                )
            }
            Self::InsufficientCapacity {
                current_size,
                requested_capacity,
            } => {
                write!(
                    f,
                    "[CircularQueue] failed to resize, the new queue space is insufficient. current size ({current_size}), requested capacity ({requested_capacity})."
                )
            }
        }
    }
}

impl std::error::Error for QueueError {}
