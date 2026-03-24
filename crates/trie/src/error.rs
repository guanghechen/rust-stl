use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrieError {
    InvalidSigmaSize {
        sigma_size: usize,
    },
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    IndexOutOfRange {
        index: usize,
        sigma_size: usize,
    },
    NodeOverflow {
        max_nodes: usize,
    },
    CapacityOverflow {
        requested_nodes: usize,
        sigma_size: usize,
    },
}

impl fmt::Display for TrieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSigmaSize { sigma_size } => {
                write!(
                    f,
                    "[Trie] sigma_size is expected to be a positive integer, but got ({sigma_size})."
                )
            }
            Self::InvalidRange { start, end, len } => {
                write!(
                    f,
                    "[Trie] invalid range [{start}, {end}) for sequence length ({len})."
                )
            }
            Self::IndexOutOfRange { index, sigma_size } => {
                write!(
                    f,
                    "[Trie] mapped index ({index}) is out of range [0, {sigma_size})."
                )
            }
            Self::NodeOverflow { max_nodes } => {
                write!(
                    f,
                    "[Trie] node index overflow, maximum supported nodes is ({max_nodes})."
                )
            }
            Self::CapacityOverflow {
                requested_nodes,
                sigma_size,
            } => {
                write!(
                    f,
                    "[Trie] capacity overflow for nodes ({requested_nodes}) with sigma_size ({sigma_size})."
                )
            }
        }
    }
}

impl std::error::Error for TrieError {}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::TrieError;

    #[test]
    fn display_should_format_invalid_sigma_size() {
        let err = TrieError::InvalidSigmaSize { sigma_size: 0 };

        assert_eq!(
            err.to_string(),
            "[Trie] sigma_size is expected to be a positive integer, but got (0)."
        );
    }

    #[test]
    fn display_should_format_invalid_range() {
        let err = TrieError::InvalidRange {
            start: 3,
            end: 8,
            len: 4,
        };

        assert_eq!(
            err.to_string(),
            "[Trie] invalid range [3, 8) for sequence length (4)."
        );
    }

    #[test]
    fn display_should_format_index_out_of_range() {
        let err = TrieError::IndexOutOfRange {
            index: 9,
            sigma_size: 8,
        };

        assert_eq!(
            err.to_string(),
            "[Trie] mapped index (9) is out of range [0, 8)."
        );
    }

    #[test]
    fn display_should_format_node_overflow() {
        let err = TrieError::NodeOverflow { max_nodes: 123 };

        assert_eq!(
            err.to_string(),
            "[Trie] node index overflow, maximum supported nodes is (123)."
        );
    }

    #[test]
    fn display_should_format_capacity_overflow() {
        let err = TrieError::CapacityOverflow {
            requested_nodes: 456,
            sigma_size: 62,
        };

        assert_eq!(
            err.to_string(),
            "[Trie] capacity overflow for nodes (456) with sigma_size (62)."
        );
    }

    #[test]
    fn error_source_should_be_none() {
        let err = TrieError::InvalidSigmaSize { sigma_size: 1 };

        assert!(StdError::source(&err).is_none());
    }
}
