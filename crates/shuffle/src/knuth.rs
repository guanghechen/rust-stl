use crate::random_int;

/// Shuffles all elements in place with the default RNG.
///
/// Complexity:
/// - Time: `O(n)`
/// - Extra space: `O(1)`
pub fn knuth_shuffle<T>(elements: &mut [T]) {
    knuth_shuffle_range(elements, 0, elements.len());
}

/// Shuffles all elements in place with a caller-provided RNG callback.
///
/// The callback receives an exclusive upper bound `n` and should return a
/// value in `[0, n)`.
///
/// Complexity:
/// - Time: `O(n)`
/// - Extra space: `O(1)`
pub fn knuth_shuffle_with<T, F>(elements: &mut [T], random_int_fn: F)
where
    F: FnMut(usize) -> usize,
{
    knuth_shuffle_range_with(elements, 0, elements.len(), random_int_fn);
}

/// Shuffles elements in `elements[start..end]` with the default RNG.
///
/// Bounds are clamped to the slice length. If the clamped range has fewer than
/// 2 elements, this function is a no-op.
///
/// Complexity:
/// - Time: `O(k)` where `k = end - start` after clamping
/// - Extra space: `O(1)`
pub fn knuth_shuffle_range<T>(elements: &mut [T], start: usize, end: usize) {
    knuth_shuffle_range_with(elements, start, end, random_int);
}

/// Shuffles elements in `elements[start..end]` with a caller-provided RNG callback.
///
/// Bounds are clamped to the slice length. If the clamped range has fewer than
/// 2 elements, this function is a no-op.
///
/// The callback receives an exclusive upper bound `n` and should return a
/// value in `[0, n)`. Returning an out-of-range value will panic.
///
/// Complexity:
/// - Time: `O(k)` where `k = end - start` after clamping
/// - Extra space: `O(1)`
pub fn knuth_shuffle_range_with<T, F>(
    elements: &mut [T],
    start: usize,
    end: usize,
    mut random_int_fn: F,
) where
    F: FnMut(usize) -> usize,
{
    let len = elements.len();
    let start = start.min(len);
    let end = end.min(len);

    if end.saturating_sub(start) < 2 {
        return;
    }

    // Fisher-Yates from right to left: pick i in [start, j] and swap(i, j).
    for j in (start + 1..end).rev() {
        let span = j - start + 1;
        let offset = random_int_fn(span);
        assert!(
            offset < span,
            "knuth_shuffle_range_with: rng returned {offset} but expected < {span}"
        );
        let i = start + offset;
        elements.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::{knuth_shuffle, knuth_shuffle_range, knuth_shuffle_range_with, knuth_shuffle_with};

    #[derive(Clone)]
    struct Lcg {
        state: u64,
    }

    impl Lcg {
        fn new(seed: u64) -> Self {
            Self { state: seed }
        }

        fn next_usize(&mut self, upper_exclusive: usize) -> usize {
            if upper_exclusive == 0 {
                return 0;
            }

            self.state = self
                .state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.state >> 32) as usize) % upper_exclusive
        }
    }

    fn assert_same_multiset(a: &[usize], b: &[usize]) {
        let mut x = a.to_vec();
        let mut y = b.to_vec();
        x.sort_unstable();
        y.sort_unstable();
        assert_eq!(x, y);
    }

    #[test]
    fn knuth_shuffle_with_and_range_with_should_match_on_full_range() {
        let mut a: Vec<usize> = (0..64).collect();
        let mut b = a.clone();

        let mut rng_a = Lcg::new(0x1234_5678_9ABC_DEF0);
        knuth_shuffle_with(&mut a, |n| rng_a.next_usize(n));

        let mut rng_b = Lcg::new(0x1234_5678_9ABC_DEF0);
        let b_len = b.len();
        knuth_shuffle_range_with(&mut b, 0, b_len, |n| rng_b.next_usize(n));

        assert_eq!(a, b);
    }

    #[test]
    fn knuth_shuffle_range_with_should_only_modify_target_subrange() {
        let mut values: Vec<usize> = (0..20).collect();
        let origin = values.clone();

        knuth_shuffle_range_with(&mut values, 4, 16, |_| 0);

        assert_eq!(&values[..4], &origin[..4]);
        assert_eq!(&values[16..], &origin[16..]);
        assert_same_multiset(&values[4..16], &origin[4..16]);
        assert_ne!(&values[4..16], &origin[4..16]);
    }

    #[test]
    fn knuth_shuffle_range_with_should_clamp_bounds() {
        let mut values: Vec<usize> = (0..10).collect();
        let origin = values.clone();

        knuth_shuffle_range_with(&mut values, 8, 99, |_| 0);
        assert_eq!(&values[..8], &origin[..8]);
        assert_same_multiset(&values[8..], &origin[8..]);

        let values_after = values.clone();
        knuth_shuffle_range_with(&mut values, 20, 50, |_| 0);
        assert_eq!(values, values_after);
    }

    #[test]
    fn knuth_shuffle_range_with_should_pass_expected_bounds_to_rng() {
        let mut values: Vec<usize> = (0..8).collect();
        let mut bounds = Vec::new();

        knuth_shuffle_range_with(&mut values, 2, 7, |n| {
            bounds.push(n);
            0
        });

        assert_eq!(bounds, vec![5, 4, 3, 2]);
    }

    #[test]
    fn knuth_shuffle_range_should_keep_noop_for_short_or_reversed_ranges() {
        let mut values: Vec<usize> = (0..10).collect();
        let origin = values.clone();

        knuth_shuffle_range(&mut values, 6, 6);
        assert_eq!(values, origin);

        knuth_shuffle_range(&mut values, 9, 2);
        assert_eq!(values, origin);

        knuth_shuffle_range(&mut values, 9, 10);
        assert_eq!(values, origin);
    }

    #[test]
    #[should_panic(expected = "knuth_shuffle_range_with: rng returned")]
    fn knuth_shuffle_range_with_should_panic_on_invalid_rng_output() {
        let mut values: Vec<usize> = (0..8).collect();
        let len = values.len();
        knuth_shuffle_range_with(&mut values, 0, len, |n| n);
    }

    #[test]
    fn knuth_shuffle_default_should_preserve_multiset() {
        let mut values: Vec<usize> = (0..128).collect();
        let origin = values.clone();

        knuth_shuffle(&mut values);

        assert_same_multiset(&values, &origin);
    }
}
