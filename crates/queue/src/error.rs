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

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::QueueError;

    #[test]
    fn display_should_format_invalid_capacity() {
        let err = QueueError::InvalidCapacity { capacity: 0 };

        assert_eq!(
            err.to_string(),
            "[CircularQueue] capacity is expected to be a positive integer, but got (0)."
        );
    }

    #[test]
    fn display_should_format_insufficient_capacity() {
        let err = QueueError::InsufficientCapacity {
            current_size: 3,
            requested_capacity: 2,
        };

        assert_eq!(
            err.to_string(),
            "[CircularQueue] failed to resize, the new queue space is insufficient. current size (3), requested capacity (2)."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = QueueError::InvalidCapacity { capacity: 1 };

        assert!(StdError::source(&err).is_none());
    }
}
