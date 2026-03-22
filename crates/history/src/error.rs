use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryError {
    InvalidCapacity { capacity: usize },
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity { capacity } => {
                write!(
                    f,
                    "[History] capacity is expected to be a positive integer, but got ({capacity})."
                )
            }
        }
    }
}

impl std::error::Error for HistoryError {}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::HistoryError;

    #[test]
    fn display_should_format_invalid_capacity() {
        let err = HistoryError::InvalidCapacity { capacity: 0 };

        assert_eq!(
            err.to_string(),
            "[History] capacity is expected to be a positive integer, but got (0)."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = HistoryError::InvalidCapacity { capacity: 1 };

        assert!(StdError::source(&err).is_none());
    }
}
