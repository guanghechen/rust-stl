use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DinicError {
    InvalidNodeCount { n: usize },
    NodeOutOfRange { node: usize, n: usize },
    SourceEqualsSink { node: usize },
    NegativeCapacity { cap: i64 },
    NotInitialized,
}

impl fmt::Display for DinicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNodeCount { n } => {
                write!(f, "[Dinic] node count must be positive, but got ({n}).")
            }
            Self::NodeOutOfRange { node, n } => {
                write!(f, "[Dinic] node ({node}) is out of range [0, {n}).")
            }
            Self::SourceEqualsSink { node } => {
                write!(
                    f,
                    "[Dinic] source and sink must be different, but both are ({node})."
                )
            }
            Self::NegativeCapacity { cap } => {
                write!(
                    f,
                    "[Dinic] edge capacity must be non-negative, but got ({cap})."
                )
            }
            Self::NotInitialized => write!(f, "[Dinic] algorithm is not initialized."),
        }
    }
}

impl std::error::Error for DinicError {}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::DinicError;

    #[test]
    fn display_should_be_readable() {
        assert_eq!(
            DinicError::InvalidNodeCount { n: 0 }.to_string(),
            "[Dinic] node count must be positive, but got (0)."
        );
        assert_eq!(
            DinicError::NodeOutOfRange { node: 5, n: 3 }.to_string(),
            "[Dinic] node (5) is out of range [0, 3)."
        );
        assert_eq!(
            DinicError::NegativeCapacity { cap: -1 }.to_string(),
            "[Dinic] edge capacity must be non-negative, but got (-1)."
        );
        assert_eq!(
            DinicError::SourceEqualsSink { node: 7 }.to_string(),
            "[Dinic] source and sink must be different, but both are (7)."
        );
        assert_eq!(
            DinicError::NotInitialized.to_string(),
            "[Dinic] algorithm is not initialized."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = DinicError::NotInitialized;
        assert!(StdError::source(&err).is_none());
    }
}
