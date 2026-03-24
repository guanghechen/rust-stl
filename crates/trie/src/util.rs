/// Convert a digit character (`0..=9`) to its trie index in `[0, 10)`.
///
/// # Panics
/// Panics when `c` is not an ASCII digit.
pub fn digit_idx(c: &char) -> usize {
    if c.is_ascii_digit() {
        (*c as usize) - ('0' as usize)
    } else {
        panic!("[Trie] digit_idx expects an ASCII digit, but got ({c}).");
    }
}

/// Convert an uppercase letter (`A..=Z`) to its trie index in `[0, 26)`.
///
/// # Panics
/// Panics when `c` is not an ASCII uppercase letter.
pub fn uppercase_idx(c: &char) -> usize {
    if c.is_ascii_uppercase() {
        (*c as usize) - ('A' as usize)
    } else {
        panic!("[Trie] uppercase_idx expects an ASCII uppercase letter, but got ({c}).");
    }
}

/// Convert a lowercase letter (`a..=z`) to its trie index in `[0, 26)`.
///
/// # Panics
/// Panics when `c` is not an ASCII lowercase letter.
pub fn lowercase_idx(c: &char) -> usize {
    if c.is_ascii_lowercase() {
        (*c as usize) - ('a' as usize)
    } else {
        panic!("[Trie] lowercase_idx expects an ASCII lowercase letter, but got ({c}).");
    }
}

/// Convert an alphanumeric character to its trie index in `[0, 62)`:
///
/// - `0..=9`    => `[0, 10)`
/// - `A..=Z`    => `[10, 36)`
/// - `a..=z`    => `[36, 62)`
///
/// # Panics
/// Panics when `c` is not an ASCII alphanumeric character.
pub fn alpha_numeric_idx(c: &char) -> usize {
    if c.is_ascii_digit() {
        return (*c as usize) - ('0' as usize);
    }
    if c.is_ascii_uppercase() {
        return (*c as usize) - ('A' as usize) + 10;
    }
    if c.is_ascii_lowercase() {
        return (*c as usize) - ('a' as usize) + 36;
    }
    panic!("[Trie] alpha_numeric_idx expects an ASCII alphanumeric character, but got ({c}).");
}

#[cfg(test)]
mod tests {
    use super::{alpha_numeric_idx, digit_idx, lowercase_idx, uppercase_idx};

    #[test]
    fn digit_idx_should_map_digits() {
        let letters: Vec<char> = "0123456789".chars().collect();
        for (i, c) in letters.into_iter().enumerate() {
            assert_eq!(digit_idx(&c), i);
        }
    }

    #[test]
    fn uppercase_idx_should_map_uppercase_letters() {
        let letters: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
        for (i, c) in letters.into_iter().enumerate() {
            assert_eq!(uppercase_idx(&c), i);
        }
    }

    #[test]
    fn lowercase_idx_should_map_lowercase_letters() {
        let letters: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
        for (i, c) in letters.into_iter().enumerate() {
            assert_eq!(lowercase_idx(&c), i);
        }
    }

    #[test]
    fn alpha_numeric_idx_should_map_all_supported_chars() {
        let digits: Vec<char> = "0123456789".chars().collect();
        for (i, c) in digits.into_iter().enumerate() {
            assert_eq!(alpha_numeric_idx(&c), i);
        }

        let upper_letters: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
        for (i, c) in upper_letters.into_iter().enumerate() {
            assert_eq!(alpha_numeric_idx(&c), i + 10);
        }

        let lower_letters: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
        for (i, c) in lower_letters.into_iter().enumerate() {
            assert_eq!(alpha_numeric_idx(&c), i + 36);
        }
    }
}
