use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McmfError {
    InvalidInf { inf: i64 },
    InvalidNodeCount { n: usize },
    NodeOutOfRange { node: usize, n: usize },
    SourceEqualsSink { node: usize },
    NegativeCapacity { cap: i64 },
    NotInitialized,
}

impl fmt::Display for McmfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInf { inf } => {
                write!(f, "[Mcmf] inf must be positive, but got ({inf}).")
            }
            Self::InvalidNodeCount { n } => {
                write!(f, "[Mcmf] node count must be positive, but got ({n}).")
            }
            Self::NodeOutOfRange { node, n } => {
                write!(f, "[Mcmf] node ({node}) is out of range [0, {n}).")
            }
            Self::SourceEqualsSink { node } => {
                write!(
                    f,
                    "[Mcmf] source and sink must be different, but both are ({node})."
                )
            }
            Self::NegativeCapacity { cap } => {
                write!(
                    f,
                    "[Mcmf] edge capacity must be non-negative, but got ({cap})."
                )
            }
            Self::NotInitialized => write!(f, "[Mcmf] algorithm is not initialized."),
        }
    }
}

impl std::error::Error for McmfError {}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::McmfError;

    #[test]
    fn display_should_be_readable() {
        assert_eq!(
            McmfError::InvalidInf { inf: 0 }.to_string(),
            "[Mcmf] inf must be positive, but got (0)."
        );
        assert_eq!(
            McmfError::InvalidNodeCount { n: 0 }.to_string(),
            "[Mcmf] node count must be positive, but got (0)."
        );
        assert_eq!(
            McmfError::NodeOutOfRange { node: 5, n: 3 }.to_string(),
            "[Mcmf] node (5) is out of range [0, 3)."
        );
        assert_eq!(
            McmfError::SourceEqualsSink { node: 7 }.to_string(),
            "[Mcmf] source and sink must be different, but both are (7)."
        );
        assert_eq!(
            McmfError::NegativeCapacity { cap: -1 }.to_string(),
            "[Mcmf] edge capacity must be non-negative, but got (-1)."
        );
        assert_eq!(
            McmfError::NotInitialized.to_string(),
            "[Mcmf] algorithm is not initialized."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = McmfError::NotInitialized;
        assert!(StdError::source(&err).is_none());
    }
}
