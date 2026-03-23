/// Rearranges `elements` into the next lexicographically greater permutation.
///
/// Returns `true` if such permutation exists. Otherwise, reverses `elements`
/// into ascending order and returns `false`.
///
/// Complexity:
/// - Time: `O(n)`
/// - Space: `O(1)`
pub fn next_permutation<T>(elements: &mut [T]) -> bool
where
    T: Ord,
{
    let n = elements.len();
    if n < 2 {
        return false;
    }

    let Some(pivot) = (1..n).rfind(|&i| elements[i - 1] < elements[i]) else {
        elements.reverse();
        return false;
    };
    let pivot = pivot - 1;

    let successor = (pivot + 1..n)
        .rfind(|&i| elements[pivot] < elements[i])
        .expect("pivot guarantees at least one successor");

    elements.swap(pivot, successor);
    elements[pivot + 1..].reverse();
    true
}

/// Rearranges `elements` into the previous lexicographically smaller permutation.
///
/// Returns `true` if such permutation exists. Otherwise, reverses `elements`
/// into descending order and returns `false`.
///
/// Complexity:
/// - Time: `O(n)`
/// - Space: `O(1)`
pub fn prev_permutation<T>(elements: &mut [T]) -> bool
where
    T: Ord,
{
    let n = elements.len();
    if n < 2 {
        return false;
    }

    let Some(pivot) = (1..n).rfind(|&i| elements[i - 1] > elements[i]) else {
        elements.reverse();
        return false;
    };
    let pivot = pivot - 1;

    let predecessor = (pivot + 1..n)
        .rfind(|&i| elements[pivot] > elements[i])
        .expect("pivot guarantees at least one predecessor");

    elements.swap(pivot, predecessor);
    elements[pivot + 1..].reverse();
    true
}

/// Reusable-buffer permutation cursor for the integer range `[start, start + n)`.
///
/// Permutations are generated in lexicographic ascending order.
///
/// Notes:
/// - If `n == 0`, the cursor is exhausted.
/// - If `start + n` overflows `usize`, the cursor is exhausted.
/// - Call [`Self::advance`] to move to the next permutation.
/// - Read the current permutation by [`Self::current`].
///
/// Complexity:
/// - Per item generation: `O(n)`
/// - Per item allocation: `O(1)` (reuses internal buffer)
/// - Internal state memory: `O(n)`
#[derive(Debug, Clone)]
pub struct PermutationIndices {
    current: Vec<usize>,
    started: bool,
    exhausted: bool,
}

impl PermutationIndices {
    /// Creates a new permutation cursor for `[start, start + n)`.
    pub fn new(n: usize, start: usize) -> Self {
        if n == 0 {
            return Self {
                current: Vec::new(),
                started: false,
                exhausted: true,
            };
        }

        let Some(end) = start.checked_add(n) else {
            return Self {
                current: Vec::new(),
                started: false,
                exhausted: true,
            };
        };

        Self {
            current: (start..end).collect(),
            started: false,
            exhausted: false,
        }
    }

    /// Advances to the next permutation.
    ///
    /// Returns `true` if a new current permutation is available.
    /// Returns `false` if the cursor is exhausted.
    pub fn advance(&mut self) -> bool {
        if self.exhausted {
            return false;
        }

        if !self.started {
            self.started = true;
            return true;
        }

        if next_permutation(&mut self.current) {
            true
        } else {
            self.exhausted = true;
            false
        }
    }

    /// Returns the current permutation view.
    ///
    /// Returns `None` before the first successful [`Self::advance`] call,
    /// or after the cursor is exhausted.
    pub fn current(&self) -> Option<&[usize]> {
        if !self.started || self.exhausted {
            None
        } else {
            Some(&self.current)
        }
    }
}

/// Builds a permutation cursor for `[start, start + n)`.
///
/// This is equivalent to `PermutationIndices::new(n, start)`.
pub fn permutation_indices(n: usize, start: usize) -> PermutationIndices {
    PermutationIndices::new(n, start)
}

/// Traverses all permutations of `elements` in lexicographic ascending order.
///
/// The traversal is zero-allocation at algorithm level: it mutates `elements`
/// in place and passes shared slices to `f`.
///
/// Notes:
/// - If `elements` is empty, `f` is not called.
/// - `elements` is sorted in ascending order before traversal.
/// - After traversal, `elements` remains in ascending order.
///
/// Complexity:
/// - Time: `O(k * n)` where `k` is the number of permutations generated
/// - Extra space: `O(1)`
pub fn for_each_permutation<T, F>(elements: &mut [T], mut f: F)
where
    T: Ord,
    F: FnMut(&[T]),
{
    if elements.is_empty() {
        return;
    }

    elements.sort_unstable();
    loop {
        f(elements);
        if !next_permutation(elements) {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{for_each_permutation, next_permutation, permutation_indices, prev_permutation};

    fn factorial(n: usize) -> usize {
        (1..=n).product::<usize>().max(1)
    }

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

    fn shuffle(values: &mut [usize], rng: &mut XorShift64) {
        for i in (1..values.len()).rev() {
            let j = rng.next_usize(i + 1);
            values.swap(i, j);
        }
    }

    fn is_strictly_lex_increasing(values: &[Vec<usize>]) -> bool {
        values.windows(2).all(|pair| pair[0] < pair[1])
    }

    fn multiset_permutations_sorted(values: &[usize]) -> Vec<Vec<usize>> {
        fn dfs(
            target_len: usize,
            counts: &mut [(usize, usize)],
            current: &mut Vec<usize>,
            out: &mut Vec<Vec<usize>>,
        ) {
            if current.len() == target_len {
                out.push(current.clone());
                return;
            }

            for i in 0..counts.len() {
                if counts[i].1 == 0 {
                    continue;
                }

                counts[i].1 -= 1;
                current.push(counts[i].0);
                dfs(target_len, counts, current, out);
                current.pop();
                counts[i].1 += 1;
            }
        }

        let mut map: BTreeMap<usize, usize> = BTreeMap::new();
        for &value in values {
            *map.entry(value).or_insert(0) += 1;
        }

        let mut counts = map.into_iter().collect::<Vec<_>>();
        let mut current = Vec::with_capacity(values.len());
        let mut out = Vec::new();
        dfs(values.len(), &mut counts, &mut current, &mut out);
        out
    }

    fn collect_permutation_indices(n: usize, start: usize) -> Vec<Vec<usize>> {
        let mut cursor = permutation_indices(n, start);
        let mut result = Vec::new();

        while cursor.advance() {
            result.push(
                cursor
                    .current()
                    .expect("advanced cursor has current")
                    .to_vec(),
            );
        }

        result
    }

    #[test]
    fn next_permutation_sequence_should_match_snapshot() {
        let mut values = [0, 1, 2];
        let mut actual = vec![values.to_vec()];

        while next_permutation(&mut values) {
            actual.push(values.to_vec());
        }

        assert_eq!(
            actual,
            vec![
                vec![0, 1, 2],
                vec![0, 2, 1],
                vec![1, 0, 2],
                vec![1, 2, 0],
                vec![2, 0, 1],
                vec![2, 1, 0],
            ]
        );

        // The terminal false call rewinds to ascending order.
        assert_eq!(values, [0, 1, 2]);
    }

    #[test]
    fn prev_permutation_sequence_should_match_snapshot() {
        let mut values = [2, 1, 0];
        let mut actual = vec![values.to_vec()];

        while prev_permutation(&mut values) {
            actual.push(values.to_vec());
        }

        assert_eq!(
            actual,
            vec![
                vec![2, 1, 0],
                vec![2, 0, 1],
                vec![1, 2, 0],
                vec![1, 0, 2],
                vec![0, 2, 1],
                vec![0, 1, 2],
            ]
        );

        // The terminal false call rewinds to descending order.
        assert_eq!(values, [2, 1, 0]);
    }

    #[test]
    fn next_prev_boundary_should_rewind_and_report_false() {
        let mut asc = [0, 1, 2];
        assert!(next_permutation(&mut asc));

        let mut desc = [2, 1, 0];
        assert!(!next_permutation(&mut desc));
        assert_eq!(desc, [0, 1, 2]);

        let mut asc2 = [0, 1, 2];
        assert!(!prev_permutation(&mut asc2));
        assert_eq!(asc2, [2, 1, 0]);

        let mut single = [1];
        assert!(!next_permutation(&mut single));
        assert!(!prev_permutation(&mut single));
    }

    #[test]
    fn duplicates_should_be_handled_correctly() {
        let mut values = [1, 1, 2];
        let mut actual = vec![values.to_vec()];

        while next_permutation(&mut values) {
            actual.push(values.to_vec());
        }

        assert_eq!(actual, vec![vec![1, 1, 2], vec![1, 2, 1], vec![2, 1, 1]]);
    }

    #[test]
    fn permutation_indices_should_match_snapshot() {
        assert_eq!(
            collect_permutation_indices(3, 0),
            vec![
                vec![0, 1, 2],
                vec![0, 2, 1],
                vec![1, 0, 2],
                vec![1, 2, 0],
                vec![2, 0, 1],
                vec![2, 1, 0],
            ]
        );

        assert_eq!(
            collect_permutation_indices(3, 1),
            vec![
                vec![1, 2, 3],
                vec![1, 3, 2],
                vec![2, 1, 3],
                vec![2, 3, 1],
                vec![3, 1, 2],
                vec![3, 2, 1],
            ]
        );
    }

    #[test]
    fn permutation_indices_edge_cases_should_work() {
        assert_eq!(collect_permutation_indices(0, 0).len(), 0);
        assert_eq!(collect_permutation_indices(2, usize::MAX).len(), 0);
        assert_eq!(collect_permutation_indices(6, 0).len(), factorial(6));
    }

    #[test]
    fn permutation_indices_current_state_should_work() {
        let mut cursor = permutation_indices(3, 0);
        assert_eq!(cursor.current(), None);

        assert!(cursor.advance());
        assert_eq!(cursor.current(), Some(&[0, 1, 2][..]));

        while cursor.advance() {}
        assert_eq!(cursor.current(), None);
    }

    #[test]
    fn property_permutation_indices_should_be_lexicographic_and_complete_for_small_n() {
        for n in 1..=8 {
            let result = collect_permutation_indices(n, 0);
            assert_eq!(result.len(), factorial(n));
            assert_eq!(result.first(), Some(&(0..n).collect::<Vec<_>>()));
            assert_eq!(result.last(), Some(&(0..n).rev().collect::<Vec<_>>()));
            assert!(is_strictly_lex_increasing(&result));
        }
    }

    #[test]
    fn property_next_prev_should_roundtrip_on_random_unique_permutations() {
        let mut rng = XorShift64::new(0xBADC_0FFE_EE00_DD11);

        for n in 2..=9 {
            for _ in 0..200 {
                let mut original = (0..n).collect::<Vec<_>>();
                shuffle(&mut original, &mut rng);

                let mut next_then_prev = original.clone();
                if next_permutation(&mut next_then_prev) {
                    assert!(prev_permutation(&mut next_then_prev));
                    assert_eq!(next_then_prev, original);
                }

                let mut prev_then_next = original.clone();
                if prev_permutation(&mut prev_then_next) {
                    assert!(next_permutation(&mut prev_then_next));
                    assert_eq!(prev_then_next, original);
                }
            }
        }
    }

    #[test]
    fn property_for_each_permutation_should_match_multiset_oracle() {
        let mut rng = XorShift64::new(0xA1B2_C3D4_E5F6_0718);

        for n in 1..=8 {
            for _ in 0..40 {
                let mut values = (0..n).map(|_| rng.next_usize(4)).collect::<Vec<_>>();
                let expected = multiset_permutations_sorted(&values);

                let mut actual = Vec::<Vec<usize>>::new();
                for_each_permutation(&mut values, |slice| actual.push(slice.to_vec()));

                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn for_each_permutation_should_traverse_in_lexicographic_order() {
        let mut values = [2, 1, 3];
        let mut actual: Vec<Vec<i32>> = Vec::new();

        for_each_permutation(&mut values, |slice| actual.push(slice.to_vec()));

        assert_eq!(
            actual,
            vec![
                vec![1, 2, 3],
                vec![1, 3, 2],
                vec![2, 1, 3],
                vec![2, 3, 1],
                vec![3, 1, 2],
                vec![3, 2, 1],
            ]
        );
        assert_eq!(values, [1, 2, 3]);
    }

    #[test]
    fn for_each_permutation_empty_should_skip_callback() {
        let mut values: [usize; 0] = [];
        let mut count = 0;

        for_each_permutation(&mut values, |_| {
            count += 1;
        });

        assert_eq!(count, 0);
    }
}
