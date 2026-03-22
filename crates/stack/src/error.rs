use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackError {
    InvalidCapacity { capacity: usize },
}

impl fmt::Display for StackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity { capacity } => {
                write!(
                    f,
                    "[CircularStack] capacity is expected to be a positive integer, but got ({capacity})."
                )
            }
        }
    }
}

impl std::error::Error for StackError {}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::StackError;

    #[test]
    fn display_should_format_invalid_capacity() {
        let err = StackError::InvalidCapacity { capacity: 0 };

        assert_eq!(
            err.to_string(),
            "[CircularStack] capacity is expected to be a positive integer, but got (0)."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = StackError::InvalidCapacity { capacity: 1 };

        assert!(StdError::source(&err).is_none());
    }
}
