use core::ops::Range;

use collection::{Collection, Disposable};

use crate::error::TrieError;

/// Options for creating a [`Trie`].
pub struct TrieOptions<Idx, Merge> {
    /// Maximum number of children each node can have.
    pub sigma_size: usize,
    /// Maps an element to an index in `[0, sigma_size)`.
    pub idx: Idx,
    /// Merges values when the same key is inserted multiple times.
    pub merge_node_value: Merge,
}

/// Match item returned by [`Trie::try_matches`] and [`Trie::try_matches_range`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrieNodeData<T> {
    /// The exclusive end position in the original input sequence.
    pub end: usize,
    /// Value stored at this matched node.
    pub val: T,
}

/// Generic trie that stores values on terminal nodes.
///
/// This trie is sequence-oriented and supports any element type `E`,
/// including keystroke tokens, enums, chars, and custom structs.
pub struct Trie<E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    sigma_size: usize,
    vals: Vec<Option<V>>,
    // Flattened adjacency table: edge(node, idx) -> next node.
    next: Vec<u32>,
    map_fn: Idx,
    merge_fn: Merge,
    nodes: usize,
    words: usize,
    disposed: bool,
    marker: core::marker::PhantomData<E>,
}

impl<E, V, Idx, Merge> Trie<E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    const MAX_NODE_INDEX: usize = u32::MAX as usize;
    const MAX_NODE_COUNT: usize = Self::MAX_NODE_INDEX + 1;

    /// Create a trie.
    pub fn new(options: TrieOptions<Idx, Merge>) -> Result<Self, TrieError> {
        let TrieOptions {
            sigma_size,
            idx: map_fn,
            merge_node_value: merge_fn,
        } = options;

        if sigma_size == 0 {
            return Err(TrieError::InvalidSigmaSize { sigma_size });
        }

        Ok(Self {
            sigma_size,
            vals: vec![None],
            next: vec![0_u32; sigma_size],
            map_fn,
            merge_fn,
            nodes: 1,
            words: 0,
            disposed: false,
            marker: core::marker::PhantomData,
        })
    }

    /// Number of keys currently stored.
    pub fn size(&self) -> usize {
        self.words
    }

    /// Reset trie to empty state while keeping allocated buffers for reuse.
    pub fn init(&mut self) {
        self.nodes = 1;
        self.words = 0;
        self.vals[0] = None;
        self.next[..self.sigma_size].fill(0_u32);
    }

    /// Reserve capacity for additional nodes.
    pub fn reserve_nodes(&mut self, additional_nodes: usize) -> Result<(), TrieError> {
        let target_nodes = self.nodes.checked_add(additional_nodes).ok_or(TrieError::NodeOverflow {
            max_nodes: Self::MAX_NODE_COUNT,
        })?;

        if target_nodes > Self::MAX_NODE_COUNT {
            return Err(TrieError::NodeOverflow {
                max_nodes: Self::MAX_NODE_COUNT,
            });
        }

        let required_edges =
            target_nodes
                .checked_mul(self.sigma_size)
                .ok_or(TrieError::CapacityOverflow {
                    requested_nodes: target_nodes,
                    sigma_size: self.sigma_size,
                })?;

        if self.vals.len() < target_nodes {
            self.vals.reserve(target_nodes - self.vals.len());
        }
        if self.next.len() < required_edges {
            self.next.reserve(required_edges - self.next.len());
        }

        Ok(())
    }

    /// Insert a full key.
    pub fn try_insert(&mut self, key: &[E], value: V) -> Result<&mut Self, TrieError> {
        self.try_insert_range(key, 0..key.len(), value)
    }

    /// Insert key range `elements[range]`.
    pub fn try_insert_range(
        &mut self,
        elements: &[E],
        range: Range<usize>,
        value: V,
    ) -> Result<&mut Self, TrieError> {
        let range = Self::check_range(elements.len(), range)?;

        let mut node = 0usize;
        for element in &elements[range] {
            let idx = self.to_idx(element)?;
            let next = self.next_of(node, idx);
            if next == 0 {
                let new = self.new_node()?;
                self.set_next(node, idx, new);
                node = new;
            } else {
                node = next;
            }
        }

        if let Some(prev) = self.vals[node].take() {
            self.vals[node] = Some((self.merge_fn)(prev, value));
        } else {
            self.vals[node] = Some(value);
            self.words += 1;
        }

        Ok(self)
    }

    /// Remove a full key.
    pub fn try_remove(&mut self, key: &[E]) -> Result<bool, TrieError> {
        self.try_remove_range(key, 0..key.len())
    }

    /// Remove key range `elements[range]`.
    pub fn try_remove_range(
        &mut self,
        elements: &[E],
        range: Range<usize>,
    ) -> Result<bool, TrieError> {
        let range = Self::check_range(elements.len(), range)?;

        let mut node = 0usize;
        for element in &elements[range] {
            let idx = self.to_idx(element)?;
            let next = self.next_of(node, idx);
            if next == 0 {
                return Ok(false);
            }
            node = next;
        }

        if self.vals[node].is_none() {
            return Ok(false);
        }

        self.vals[node] = None;
        self.words -= 1;
        Ok(true)
    }

    /// Get value of a full key.
    pub fn try_get(&self, key: &[E]) -> Result<Option<&V>, TrieError> {
        self.try_get_range(key, 0..key.len())
    }

    /// Get value of key range `elements[range]`.
    pub fn try_get_range(
        &self,
        elements: &[E],
        range: Range<usize>,
    ) -> Result<Option<&V>, TrieError> {
        let range = Self::check_range(elements.len(), range)?;

        let mut node = 0usize;
        for element in &elements[range] {
            let idx = self.to_idx(element)?;
            let next = self.next_of(node, idx);
            if next == 0 {
                return Ok(None);
            }
            node = next;
        }

        Ok(self.vals[node].as_ref())
    }

    /// Check full key existence.
    pub fn try_contains(&self, key: &[E]) -> Result<bool, TrieError> {
        Ok(self.try_get(key)?.is_some())
    }

    /// Check range key existence.
    pub fn try_contains_range(
        &self,
        elements: &[E],
        range: Range<usize>,
    ) -> Result<bool, TrieError> {
        Ok(self.try_get_range(elements, range)?.is_some())
    }

    /// Check whether there exists any key that has this prefix.
    pub fn try_contains_prefix(&self, prefix: &[E]) -> Result<bool, TrieError> {
        self.try_contains_prefix_range(prefix, 0..prefix.len())
    }

    /// Check whether there exists any key that has this range prefix.
    pub fn try_contains_prefix_range(
        &self,
        elements: &[E],
        range: Range<usize>,
    ) -> Result<bool, TrieError> {
        let range = Self::check_range(elements.len(), range)?;

        if range.start == range.end {
            return Ok(self.words > 0);
        }

        let mut node = 0usize;
        for element in &elements[range] {
            let idx = self.to_idx(element)?;
            let next = self.next_of(node, idx);
            if next == 0 {
                return Ok(false);
            }
            node = next;
        }

        Ok(true)
    }

    /// Collect all matched prefixes in a full sequence.
    pub fn try_matches<'a>(
        &'a self,
        elements: &'a [E],
    ) -> Result<Vec<TrieNodeData<&'a V>>, TrieError> {
        self.try_matches_range(elements, 0..elements.len())
    }

    /// Collect all matched prefixes in `elements[range]`.
    pub fn try_matches_range<'a>(
        &'a self,
        elements: &'a [E],
        range: Range<usize>,
    ) -> Result<Vec<TrieNodeData<&'a V>>, TrieError> {
        let range = Self::check_range(elements.len(), range)?;

        let mut out = Vec::<TrieNodeData<&V>>::new();
        if range.start == range.end {
            if let Some(val) = self.vals[0].as_ref() {
                out.push(TrieNodeData {
                    end: range.start,
                    val,
                });
            }
            return Ok(out);
        }

        let mut node = 0usize;
        for (offset, element) in elements[range.clone()].iter().enumerate() {
            let idx = self.to_idx(element)?;
            let next = self.next_of(node, idx);
            if next == 0 {
                break;
            }

            node = next;
            if let Some(val) = self.vals[node].as_ref() {
                out.push(TrieNodeData {
                    end: range.start + offset + 1,
                    val,
                });
            }
        }

        Ok(out)
    }

    #[inline]
    fn to_idx(&self, element: &E) -> Result<usize, TrieError> {
        let idx = (self.map_fn)(element);
        if idx < self.sigma_size {
            return Ok(idx);
        }

        Err(TrieError::IndexOutOfRange {
            index: idx,
            sigma_size: self.sigma_size,
        })
    }

    fn check_range(len: usize, range: Range<usize>) -> Result<Range<usize>, TrieError> {
        if range.start <= range.end && range.end <= len {
            return Ok(range);
        }

        Err(TrieError::InvalidRange {
            start: range.start,
            end: range.end,
            len,
        })
    }

    #[inline]
    fn edge_pos(&self, node: usize, idx: usize) -> usize {
        node * self.sigma_size + idx
    }

    #[inline]
    fn next_of(&self, node: usize, idx: usize) -> usize {
        self.next[self.edge_pos(node, idx)] as usize
    }

    #[inline]
    fn set_next(&mut self, node: usize, idx: usize, to: usize) {
        debug_assert!(to <= Self::MAX_NODE_INDEX);
        let pos = self.edge_pos(node, idx);
        self.next[pos] = to as u32;
    }

    #[inline]
    fn new_node(&mut self) -> Result<usize, TrieError> {
        let node = self.nodes;
        if node > Self::MAX_NODE_INDEX {
            return Err(TrieError::NodeOverflow {
                max_nodes: Self::MAX_NODE_COUNT,
            });
        }

        if self.vals.len() <= node {
            self.vals.push(None);
        } else {
            self.vals[node] = None;
        }

        let start = self.edge_pos(node, 0);
        let end = start + self.sigma_size;
        if self.next.len() < end {
            self.next.resize(end, 0_u32);
        } else {
            self.next[start..end].fill(0_u32);
        }

        self.nodes += 1;
        Ok(node)
    }
}

impl<E, V, Idx, Merge> Collection for Trie<E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    type Item = V;
    type Iter<'a>
        = Iter<'a, E, V, Idx, Merge>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        Iter::new(self)
    }

    fn size(&self) -> usize {
        self.words
    }

    fn clear(&mut self) {
        self.init();
    }

    fn retain<F>(&mut self, mut f: F) -> usize
    where
        F: FnMut(&Self::Item) -> bool,
    {
        let mut removed = 0usize;
        for value in self.vals.iter_mut().take(self.nodes) {
            let keep = match value.as_ref() {
                Some(v) => f(v),
                None => true,
            };

            if !keep {
                *value = None;
                removed += 1;
            }
        }

        self.words -= removed;
        removed
    }
}

impl<E, V, Idx, Merge> Disposable for Trie<E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    fn dispose(&mut self) {
        self.disposed = true;
        self.init();
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

pub struct Iter<'a, E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    trie: &'a Trie<E, V, Idx, Merge>,
    node_stack: Vec<usize>,
    idx_stack: Vec<usize>,
    root_wait: bool,
    remaining: usize,
}

impl<'a, E, V, Idx, Merge> Iter<'a, E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    fn new(trie: &'a Trie<E, V, Idx, Merge>) -> Self {
        Self {
            trie,
            node_stack: vec![0],
            idx_stack: vec![0],
            root_wait: true,
            remaining: trie.words,
        }
    }
}

impl<'a, E, V, Idx, Merge> Iterator for Iter<'a, E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        if self.root_wait {
            self.root_wait = false;
            if let Some(value) = self.trie.vals[0].as_ref() {
                self.remaining -= 1;
                return Some(value);
            }
        }

        while let Some(h) = self.node_stack.len().checked_sub(1) {
            let node = self.node_stack[h];
            let mut idx = self.idx_stack[h];

            while idx < self.trie.sigma_size && self.trie.next[self.trie.edge_pos(node, idx)] == 0 {
                idx += 1;
            }

            if idx < self.trie.sigma_size {
                let next = self.trie.next_of(node, idx);
                self.idx_stack[h] = idx + 1;

                self.node_stack.push(next);
                self.idx_stack.push(0);

                if let Some(value) = self.trie.vals[next].as_ref() {
                    self.remaining -= 1;
                    return Some(value);
                }
            } else {
                self.node_stack.pop();
                self.idx_stack.pop();
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<E, V, Idx, Merge> ExactSizeIterator for Iter<'_, E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
}

impl<'a, E, V, Idx, Merge> IntoIterator for &'a Trie<E, V, Idx, Merge>
where
    Idx: Fn(&E) -> usize,
    Merge: Fn(V, V) -> V,
{
    type Item = &'a V;
    type IntoIter = Iter<'a, E, V, Idx, Merge>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use collection::{Collection, Disposable};

    use super::{Trie, TrieNodeData, TrieOptions};
    use crate::{TrieError, alpha_numeric_idx};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Stroke {
        Ctrl,
        Shift,
        Alt,
        KeyA,
        KeyB,
    }

    fn stroke_idx(stroke: &Stroke) -> usize {
        match stroke {
            Stroke::Ctrl => 0,
            Stroke::Shift => 1,
            Stroke::Alt => 2,
            Stroke::KeyA => 3,
            Stroke::KeyB => 4,
        }
    }

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn new_should_validate_sigma_size() {
        let trie = Trie::<char, i32, _, _>::new(TrieOptions {
            sigma_size: 0,
            idx: alpha_numeric_idx,
            merge_node_value: |_x, y| y,
        });

        assert_eq!(
            trie.err(),
            Some(TrieError::InvalidSigmaSize { sigma_size: 0 })
        );
    }

    #[test]
    fn try_insert_try_get_try_remove_should_work() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 62,
            idx: alpha_numeric_idx,
            merge_node_value: |x, y| x + y,
        })
        .unwrap();

        let cat = chars("cat");
        let ca = chars("ca");
        let c = chars("c");

        assert_eq!(trie.try_get(&cat).unwrap(), None);
        trie.try_insert(&c, 1).unwrap();
        trie.try_insert(&ca, 2).unwrap();
        trie.try_insert(&cat, 3).unwrap();
        trie.try_insert(&cat, 20).unwrap();

        assert_eq!(trie.size(), 3);
        assert_eq!(trie.try_get(&c).unwrap(), Some(&1));
        assert_eq!(trie.try_get(&ca).unwrap(), Some(&2));
        assert_eq!(trie.try_get(&cat).unwrap(), Some(&23));

        assert!(trie.try_remove(&cat).unwrap());
        assert_eq!(trie.try_get(&cat).unwrap(), None);
        assert!(!trie.try_remove(&cat).unwrap());
    }

    #[test]
    fn range_ops_should_work() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 62,
            idx: alpha_numeric_idx,
            merge_node_value: |x, y| x + y,
        })
        .unwrap();

        let cat = chars("cat");
        trie.try_insert_range(&cat, 0..1, 1).unwrap();
        trie.try_insert_range(&cat, 0..2, 2).unwrap();
        trie.try_insert_range(&cat, 0..3, 3).unwrap();
        trie.try_insert_range(&cat, 1..2, 4).unwrap();
        trie.try_insert_range(&cat, 1..3, 100).unwrap();
        trie.try_insert_range(&cat, 2..3, 103).unwrap();
        trie.try_insert_range(&cat, 3..3, 120).unwrap();

        assert_eq!(trie.size(), 7);
        assert_eq!(trie.try_get_range(&cat, 3..3).unwrap(), Some(&120));
        assert_eq!(trie.try_get_range(&cat, 0..3).unwrap(), Some(&3));
        assert_eq!(trie.try_get_range(&cat, 1..2).unwrap(), Some(&4));

        assert!(trie.try_remove_range(&cat, 0..3).unwrap());
        assert_eq!(trie.try_get_range(&cat, 0..3).unwrap(), None);

        assert!(trie.try_contains_prefix_range(&cat, 0..2).unwrap());
        assert!(trie.try_contains_prefix_range(&cat, 1..3).unwrap());
    }

    #[test]
    fn try_matches_should_work() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 26,
            idx: |c: &char| (*c as usize) - ('a' as usize),
            merge_node_value: |_x, y| y,
        })
        .unwrap();

        let ban = chars("ban");
        let banana = chars("banana");
        let apple = chars("apple");

        trie.try_insert(&ban, 2).unwrap();
        trie.try_insert(&banana, 1).unwrap();
        trie.try_insert(&apple, 3).unwrap();

        let matched = trie.try_matches(&banana).unwrap();
        assert_eq!(
            matched,
            vec![
                TrieNodeData { end: 3, val: &2 },
                TrieNodeData { end: 6, val: &1 }
            ]
        );

        let matched = trie.try_matches_range(&banana, 0..3).unwrap();
        assert_eq!(matched, vec![TrieNodeData { end: 3, val: &2 }]);

        let empty: Vec<char> = vec![];
        trie.try_insert(&empty, 99).unwrap();
        assert_eq!(
            trie.try_matches(&empty).unwrap(),
            vec![TrieNodeData { end: 0, val: &99 }]
        );
        assert_eq!(
            trie.try_matches_range(&apple, 3..3).unwrap(),
            vec![TrieNodeData { end: 3, val: &99 }]
        );
    }

    #[test]
    fn generic_keystroke_sequences_should_work() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 5,
            idx: stroke_idx,
            merge_node_value: |_x, y| y,
        })
        .unwrap();

        let k1 = [Stroke::Ctrl, Stroke::KeyA];
        let k2 = [Stroke::Ctrl, Stroke::Shift, Stroke::KeyA];
        let k3 = [Stroke::Alt, Stroke::KeyB];
        let p1 = [Stroke::Ctrl];

        trie.try_insert(&k1, 10).unwrap();
        trie.try_insert(&k2, 20).unwrap();
        trie.try_insert(&k3, 30).unwrap();

        assert_eq!(trie.try_get(&k1).unwrap(), Some(&10));
        assert_eq!(trie.try_get(&k2).unwrap(), Some(&20));
        assert_eq!(trie.try_get(&k3).unwrap(), Some(&30));
        assert!(trie.try_contains_prefix(&p1).unwrap());
    }

    #[test]
    fn invalid_range_should_return_error() {
        let trie = Trie::<char, i32, _, _>::new(TrieOptions {
            sigma_size: 62,
            idx: alpha_numeric_idx,
            merge_node_value: |_x, y| y,
        })
        .unwrap();

        let text = chars("cat");
        let err = trie.try_get_range(&text, 2..5).unwrap_err();
        assert_eq!(
            err,
            TrieError::InvalidRange {
                start: 2,
                end: 5,
                len: 3
            }
        );
    }

    #[test]
    fn index_out_of_range_should_return_error() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 2,
            idx: |_c: &u8| 9,
            merge_node_value: |_x, y| y,
        })
        .unwrap();

        let key = [1u8, 2u8];
        let err = match trie.try_insert(&key, 1) {
            Ok(_) => panic!("expected index out of range"),
            Err(err) => err,
        };
        assert_eq!(
            err,
            TrieError::IndexOutOfRange {
                index: 9,
                sigma_size: 2
            }
        );
    }

    #[test]
    fn collection_and_iter_should_work() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 62,
            idx: alpha_numeric_idx,
            merge_node_value: |_x, y| y,
        })
        .unwrap();

        let cat = chars("cat");
        let car = chars("car");
        let dog = chars("dog");
        let empty: Vec<char> = vec![];

        trie.try_insert(&cat, 1).unwrap();
        trie.try_insert(&car, 2).unwrap();
        trie.try_insert(&dog, 3).unwrap();
        trie.try_insert(&empty, 4).unwrap();

        let values = trie.iter().copied().collect::<Vec<i32>>();
        assert_eq!(values.len(), 4);
        assert_eq!(Collection::size(&trie), 4);

        let removed = trie.retain(|v| *v % 2 == 0);
        assert_eq!(removed, 2);
        assert_eq!(Collection::size(&trie), 2);
        assert_eq!(trie.try_get(&cat).unwrap(), None);
        assert_eq!(trie.try_get(&car).unwrap(), Some(&2));
    }

    #[test]
    fn dispose_should_be_idempotent_and_clear_data() {
        let mut trie = Trie::new(TrieOptions {
            sigma_size: 62,
            idx: alpha_numeric_idx,
            merge_node_value: |_x, y| y,
        })
        .unwrap();

        let apple = chars("apple");

        assert!(!Disposable::is_disposed(&trie));
        trie.try_insert(&apple, 2).unwrap();

        Disposable::dispose(&mut trie);
        assert!(Disposable::is_disposed(&trie));
        assert_eq!(trie.size(), 0);
        assert_eq!(trie.try_get(&apple).unwrap(), None);

        trie.try_insert(&apple, 9).unwrap();
        assert_eq!(trie.try_get(&apple).unwrap(), Some(&9));

        Disposable::dispose(&mut trie);
        assert!(Disposable::is_disposed(&trie));
        assert_eq!(trie.size(), 0);
    }
}
