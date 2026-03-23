use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsapError {
    InvalidNodeCount { n: usize },
    NodeOutOfRange { node: usize, n: usize },
    SourceEqualsSink { node: usize },
    NegativeCapacity { cap: i64 },
    NotInitialized,
}

impl fmt::Display for IsapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNodeCount { n } => {
                write!(f, "[Isap] node count must be positive, but got ({n}).")
            }
            Self::NodeOutOfRange { node, n } => {
                write!(f, "[Isap] node ({node}) is out of range [0, {n}).")
            }
            Self::SourceEqualsSink { node } => {
                write!(
                    f,
                    "[Isap] source and sink must be different, but both are ({node})."
                )
            }
            Self::NegativeCapacity { cap } => {
                write!(
                    f,
                    "[Isap] edge capacity must be non-negative, but got ({cap})."
                )
            }
            Self::NotInitialized => write!(f, "[Isap] algorithm is not initialized."),
        }
    }
}

impl std::error::Error for IsapError {}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::IsapError;

    #[test]
    fn display_should_be_readable() {
        assert_eq!(
            IsapError::InvalidNodeCount { n: 0 }.to_string(),
            "[Isap] node count must be positive, but got (0)."
        );
        assert_eq!(
            IsapError::NodeOutOfRange { node: 5, n: 3 }.to_string(),
            "[Isap] node (5) is out of range [0, 3)."
        );
        assert_eq!(
            IsapError::NegativeCapacity { cap: -1 }.to_string(),
            "[Isap] edge capacity must be non-negative, but got (-1)."
        );
        assert_eq!(
            IsapError::SourceEqualsSink { node: 7 }.to_string(),
            "[Isap] source and sink must be different, but both are (7)."
        );
        assert_eq!(
            IsapError::NotInitialized.to_string(),
            "[Isap] algorithm is not initialized."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = IsapError::NotInitialized;
        assert!(StdError::source(&err).is_none());
    }
}
