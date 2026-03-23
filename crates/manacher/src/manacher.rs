/// Computes Manacher radii for a sequence in linear time.
///
/// Returns a vector `radius` of length `2n - 1` for input length `n`.
///
/// Semantics:
/// - `radius[2 * i]` is the radius of the longest palindrome centered at `(i, i)`.
/// - `radius[2 * i + 1]` is the radius of the longest palindrome centered at `(i, i + 1)`.
///
/// A radius is measured in original element count on one side plus center rules:
/// - odd center `(i, i)` has minimum radius `1`.
/// - even center `(i, i + 1)` has minimum radius `0`.
///
/// For empty input, returns an empty vector.
///
/// Complexity:
/// - Time: `O(n)`
/// - Extra space: `O(n)`
pub fn manacher_by<F>(len: usize, mut equals: F) -> Vec<usize>
where
    F: FnMut(usize, usize) -> bool,
{
    if len == 0 {
        return Vec::new();
    }

    // Virtual transformed length: #a#b#...#  => 2n + 1 slots.
    let transformed_len = 2 * len + 1;
    let mut p = vec![0usize; transformed_len];

    // Current right-most palindrome [center - p[center], center + p[center]].
    let mut center = 0usize;
    let mut right = 0usize;

    for i in 0..transformed_len {
        if i <= right {
            let mirrored = center.checked_mul(2).and_then(|v| v.checked_sub(i));
            if let Some(mirror) = mirrored {
                p[i] = (right - i).min(p[mirror]);
            }
        }

        loop {
            let step = p[i] + 1;
            if i < step {
                break;
            }
            let l = i - step;
            let r = i + step;
            if r >= transformed_len {
                break;
            }

            let l_is_sep = (l & 1) == 0;
            let r_is_sep = (r & 1) == 0;
            let matched = if l_is_sep && r_is_sep {
                true
            } else if l_is_sep || r_is_sep {
                false
            } else {
                equals(l / 2, r / 2)
            };

            if !matched {
                break;
            }
            p[i] += 1;
        }

        if i + p[i] > right {
            center = i;
            right = i + p[i];
        }
    }

    let mut radius = vec![0usize; 2 * len - 1];
    for i in 0..len {
        radius[2 * i] = p[2 * i + 1].div_ceil(2);
        if i + 1 < len {
            radius[2 * i + 1] = p[2 * i + 2] / 2;
        }
    }

    radius
}

/// Computes Manacher radii for a sequence in linear time.
///
/// This is a convenience wrapper around [`manacher_by`].
///
/// Returns a vector `radius` of length `2n - 1` for input length `n`.
///
/// Semantics:
/// - `radius[2 * i]` is the radius of the longest palindrome centered at `(i, i)`.
/// - `radius[2 * i + 1]` is the radius of the longest palindrome centered at `(i, i + 1)`.
///
/// A radius is measured in original element count on one side plus center rules:
/// - odd center `(i, i)` has minimum radius `1`.
/// - even center `(i, i + 1)` has minimum radius `0`.
///
/// For empty input, returns an empty vector.
///
/// Complexity:
/// - Time: `O(n)`
/// - Extra space: `O(n)`
pub fn manacher<T>(items: &[T]) -> Vec<usize>
where
    T: Eq,
{
    manacher_by(items.len(), |l, r| items[l] == items[r])
}

/// Computes Manacher radii for `text` using UTF-8 bytes as elements.
///
/// This matches the index model used in the original TypeScript package,
/// where indexing is byte/unit based rather than Unicode grapheme based.
///
/// Complexity:
/// - Time: `O(n)`
/// - Extra space: `O(n)`
pub fn manacher_str(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    manacher_by(bytes.len(), |l, r| bytes[l] == bytes[r])
}

#[cfg(test)]
mod tests {
    use super::{manacher, manacher_by, manacher_str};

    #[derive(Clone)]
    struct XorShift64 {
        state: u64,
    }

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_u64(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }

        fn next_usize(&mut self, upper_exclusive: usize) -> usize {
            if upper_exclusive == 0 {
                return 0;
            }
            (self.next_u64() as usize) % upper_exclusive
        }
    }

    fn min_cut_like_ts(text: &str) -> usize {
        let n = text.len();
        let radius = manacher_str(text);
        let mut dp = vec![0usize; n.max(1)];

        for i in 1..n {
            let mut answer = if i < radius[i] * 2 { 0 } else { dp[i - 1] + 1 };
            if answer > 0 {
                for k in 1..i {
                    if i - k < radius[i + k] * 2 {
                        answer = answer.min(dp[k - 1] + 1);
                    }
                }
            }
            dp[i] = answer;
        }

        dp[n.saturating_sub(1)]
    }

    fn naive_radius_bytes(text: &str) -> Vec<usize> {
        let bytes = text.as_bytes();
        let n = bytes.len();
        if n == 0 {
            return Vec::new();
        }

        let mut radius = vec![0usize; 2 * n - 1];
        for i in 0..n {
            let mut l = i;
            let mut r = i;
            let mut odd = 0usize;
            loop {
                if bytes[l] != bytes[r] {
                    break;
                }
                odd += 1;
                if l == 0 || r + 1 >= n {
                    break;
                }
                l -= 1;
                r += 1;
            }
            radius[2 * i] = odd;

            if i + 1 < n {
                let mut l = i;
                let mut r = i + 1;
                let mut even = 0usize;
                loop {
                    if bytes[l] != bytes[r] {
                        break;
                    }
                    even += 1;
                    if l == 0 || r + 1 >= n {
                        break;
                    }
                    l -= 1;
                    r += 1;
                }
                radius[2 * i + 1] = even;
            }
        }

        radius
    }

    #[test]
    fn manacher_empty_should_return_empty() {
        assert!(manacher_str("").is_empty());
    }

    #[test]
    fn manacher_generic_slice_should_work() {
        let data = [1, 2, 3, 2, 1];
        // centers: 0..4 and gaps 0..3 => len 9
        assert_eq!(manacher(&data), vec![1, 0, 1, 0, 3, 0, 1, 0, 1]);
    }

    #[test]
    fn manacher_snapshot_should_match_expected() {
        assert_eq!(manacher_str("abbab"), vec![1, 0, 1, 2, 1, 0, 2, 0, 1]);
    }

    #[test]
    fn manacher_should_match_ts_min_cut_examples() {
        let data = [
            ("aab", 1usize),
            ("a", 0usize),
            ("ab", 1usize),
            ("abaacaada", 2usize),
        ];

        for (text, answer) in data {
            assert_eq!(min_cut_like_ts(text), answer);
        }
    }

    #[test]
    fn manacher_random_should_match_naive_oracle() {
        let mut rng = XorShift64::new(0xA55A_1234_5678_9ABC);

        for n in 1..=128 {
            for _ in 0..20 {
                let mut bytes = Vec::with_capacity(n);
                for _ in 0..n {
                    let c = b'a' + rng.next_usize(4) as u8;
                    bytes.push(c);
                }
                let s = String::from_utf8(bytes).expect("ascii only");

                let expected = naive_radius_bytes(&s);
                let actual = manacher_str(&s);
                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn manacher_long_case_should_work() {
        let s = "a".repeat(2048);
        let radius = manacher_str(&s);

        assert_eq!(radius.len(), 2 * 2048 - 1);
        assert_eq!(radius[2046], 1024);
        assert_eq!(radius[2047], 1024);
    }

    #[test]
    fn manacher_by_should_match_slice_wrapper() {
        let data = [1, 2, 1, 3, 3, 1, 2, 1];
        let by_slice = manacher(&data);
        let by_index = manacher_by(data.len(), |l, r| data[l] == data[r]);
        assert_eq!(by_index, by_slice);
    }

    #[test]
    fn manacher_by_should_support_char_level_usage() {
        let chars = "abccbaXabccba".chars().collect::<Vec<_>>();
        let by_slice = manacher(&chars);
        let by_index = manacher_by(chars.len(), |l, r| chars[l] == chars[r]);
        assert_eq!(by_index, by_slice);
    }
}
