use std::{mem::ManuallyDrop, ptr};

use collection::{Collection, Disposable};

use crate::traits::QueueLike;

pub use crate::traits::PriorityQueueLike;

#[derive(Debug, Clone)]
pub struct PriorityQueue<T>
where
    T: Ord,
{
    disposed: bool,
    elements: Vec<T>,
}

struct RestoreOnDrop<T> {
    ptr: *mut T,
    pos: usize,
    item: ManuallyDrop<T>,
}

impl<T> Drop for RestoreOnDrop<T> {
    fn drop(&mut self) {
        // SAFETY: `self.pos` is always in-bounds and marks the current hole.
        // Writing `item` back restores full initialization if unwinding occurs.
        unsafe {
            let dst = self.ptr.add(self.pos);
            ptr::write(dst, ManuallyDrop::take(&mut self.item));
        }
    }
}

impl<T> PriorityQueue<T>
where
    T: Ord,
{
    const UP_HOLE_THRESHOLD: usize = 64;
    const DOWN_HOLE_THRESHOLD: usize = 512;

    pub fn new() -> Self {
        Self {
            disposed: false,
            elements: Vec::new(),
        }
    }

    #[inline(always)]
    fn up(&mut self, index: usize) {
        let len = self.elements.len();
        if index == 0 || index >= len {
            return;
        }

        if len < Self::UP_HOLE_THRESHOLD {
            self.up_swap(index);
        } else {
            self.up_hole(index);
        }
    }

    #[inline(always)]
    fn up_swap(&mut self, index: usize) {
        let mut q = index;
        while q > 0 {
            let p = (q - 1) >> 1;
            if self.elements[p] <= self.elements[q] {
                break;
            }
            self.elements.swap(p, q);
            q = p;
        }
    }

    #[inline(always)]
    fn up_hole(&mut self, index: usize) {
        let len = self.elements.len();
        debug_assert!(index < len);

        unsafe {
            let ptr = self.elements.as_mut_ptr();
            let mut pos = index;

            let item = ManuallyDrop::new(ptr::read(ptr.add(index)));
            let mut restore = RestoreOnDrop { ptr, pos, item };
            let item_ptr = (&restore.item as *const ManuallyDrop<T>).cast::<T>();

            while pos > 0 {
                let parent = (pos - 1) >> 1;
                let parent_ref: &T = &*ptr.add(parent);
                let item_ref: &T = &*item_ptr;
                if parent_ref <= item_ref {
                    break;
                }

                ptr::copy_nonoverlapping(ptr.add(parent), ptr.add(pos), 1);
                pos = parent;
                restore.pos = pos;
            }
        }
    }

    #[inline(always)]
    fn down(&mut self, index: usize) {
        let n = self.elements.len();
        if index >= n {
            return;
        }

        if n < Self::DOWN_HOLE_THRESHOLD {
            self.down_swap(index);
        } else {
            self.down_hole(index);
        }
    }

    #[inline(always)]
    fn down_swap(&mut self, index: usize) {
        let mut p = index;
        let n = self.elements.len();

        while p < n {
            let left = (p << 1) + 1;
            if left >= n {
                break;
            }

            let right = left + 1;
            let mut q = left;
            if right < n && self.elements[right] < self.elements[left] {
                q = right;
            }

            if self.elements[p] <= self.elements[q] {
                break;
            }

            self.elements.swap(p, q);
            p = q;
        }
    }

    #[inline(always)]
    fn down_hole(&mut self, index: usize) {
        let n = self.elements.len();

        unsafe {
            let ptr = self.elements.as_mut_ptr();
            let mut pos = index;

            let item = ManuallyDrop::new(ptr::read(ptr.add(index)));
            let mut restore = RestoreOnDrop { ptr, pos, item };
            let item_ptr = (&restore.item as *const ManuallyDrop<T>).cast::<T>();

            loop {
                let left = (pos << 1) + 1;
                if left >= n {
                    break;
                }

                let right = left + 1;
                let mut child = left;
                if right < n {
                    let right_ref: &T = &*ptr.add(right);
                    let left_ref: &T = &*ptr.add(left);
                    if right_ref < left_ref {
                        child = right;
                    }
                }

                let item_ref: &T = &*item_ptr;
                let child_ref: &T = &*ptr.add(child);
                if item_ref <= child_ref {
                    break;
                }

                ptr::copy_nonoverlapping(ptr.add(child), ptr.add(pos), 1);
                pos = child;
                restore.pos = pos;
            }
        }
    }

    fn fast_build(&mut self) {
        let n = self.elements.len();
        if n <= 1 {
            return;
        }

        let last_parent = (n >> 1) - 1;
        for p in (0..=last_parent).rev() {
            self.down(p);
        }
    }
}

impl<T> Default for PriorityQueue<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> QueueLike<T> for PriorityQueue<T>
where
    T: Ord,
{
    fn front(&self) -> Option<&T> {
        self.elements.first()
    }

    fn enqueue(&mut self, element: T) {
        self.elements.push(element);
        let index = self.elements.len() - 1;
        self.up(index);
    }

    fn dequeue(&mut self) -> Option<T> {
        if self.elements.is_empty() {
            return None;
        }
        if self.elements.len() == 1 {
            return self.elements.pop();
        }

        let removed = self.elements.swap_remove(0);
        self.down(0);
        Some(removed)
    }

    fn enqueues<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = T>,
    {
        let size = self.elements.len();
        self.elements.extend(elements);

        let next_size = self.elements.len();
        if next_size == size {
            return;
        }

        let new_added = next_size - size;
        let next_size_f64 = next_size as f64;
        if (new_added as f64) * next_size_f64.log2() > next_size_f64 {
            self.fast_build();
        } else {
            for i in size..next_size {
                self.up(i);
            }
        }
    }

    fn replace_front(&mut self, new_back: T) -> Option<T> {
        if self.elements.is_empty() {
            self.elements.push(new_back);
            return None;
        }

        let removed = std::mem::replace(&mut self.elements[0], new_back);
        self.down(0);
        Some(removed)
    }
}

impl<T> PriorityQueueLike<T> for PriorityQueue<T> where T: Ord {}

impl<T> Collection for PriorityQueue<T>
where
    T: Ord,
{
    type Item = T;
    type Iter<'a>
        = std::slice::Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.elements.iter()
    }

    fn size(&self) -> usize {
        self.elements.len()
    }

    fn clear(&mut self) {
        self.elements.clear();
    }

    fn retain<F>(&mut self, mut f: F) -> usize
    where
        F: FnMut(&Self::Item) -> bool,
    {
        let before = self.elements.len();
        if before == 0 {
            return 0;
        }

        self.elements.retain(|item| f(item));
        let removed = before - self.elements.len();
        if removed > 0 {
            self.fast_build();
        }
        removed
    }
}

impl<T> Disposable for PriorityQueue<T>
where
    T: Ord,
{
    fn dispose(&mut self) {
        self.disposed = true;
        self.elements.clear();
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

impl<'a, T> IntoIterator for &'a PriorityQueue<T>
where
    T: Ord,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp::Reverse, collections::BinaryHeap};

    use collection::{Collection, Disposable};

    use crate::traits::{PriorityQueueLike, QueueLike};

    use super::PriorityQueue;

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

        fn next_i32_in(&mut self, bound: i32) -> i32 {
            debug_assert!(bound > 0);
            (self.next_u64() % bound as u64) as i32
        }
    }

    fn drain_all<T>(q: &mut PriorityQueue<T>) -> Vec<T>
    where
        T: Ord,
    {
        let mut out = Vec::new();
        while let Some(x) = q.dequeue() {
            out.push(x);
        }
        out
    }

    #[test]
    fn queue_like_min_heap_ops_should_work() {
        let mut q = PriorityQueue::new();

        assert_eq!(q.front(), None);
        assert_eq!(q.dequeue(), None);

        q.enqueues([4, 2, 5, 1, 3]);
        assert_eq!(q.front(), Some(&1));
        assert_eq!(q.replace_front(6), Some(1));
        assert_eq!(drain_all(&mut q), vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn enqueue_should_cover_up_break_path() {
        let mut q = PriorityQueue::new();

        q.enqueue(1);
        q.enqueue(2);
        q.enqueue(0);

        assert_eq!(q.front(), Some(&0));
        assert_eq!(drain_all(&mut q), vec![0, 1, 2]);
    }

    #[test]
    fn replace_front_should_handle_empty_and_single_item() {
        let mut q = PriorityQueue::new();

        assert_eq!(q.replace_front(10), None);
        assert_eq!(q.front(), Some(&10));

        assert_eq!(q.replace_front(5), Some(10));
        assert_eq!(q.front(), Some(&5));
        assert_eq!(q.dequeue(), Some(5));
    }

    #[test]
    fn enqueues_should_work_for_small_and_large_batch() {
        let mut q = PriorityQueue::new();

        q.enqueues([5, 4]);
        q.enqueues(0..100);

        assert_eq!(q.size(), 102);
        assert_eq!(q.front(), Some(&0));

        let drained = drain_all(&mut q);
        assert_eq!(drained.len(), 102);
        assert!(drained.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn enqueues_empty_should_be_noop() {
        let mut q = PriorityQueue::new();

        q.enqueues(std::iter::empty());
        assert!(q.is_empty());

        q.enqueue(3);
        q.enqueues(std::iter::empty());
        assert_eq!(q.front(), Some(&3));
        assert_eq!(q.size(), 1);
    }

    #[test]
    fn reverse_ord_should_support_max_heap() {
        let mut q = PriorityQueue::new();

        q.enqueues([
            Reverse(1_i32),
            Reverse(5_i32),
            Reverse(2_i32),
            Reverse(4_i32),
            Reverse(3_i32),
        ]);

        assert_eq!(q.front(), Some(&Reverse(5)));
        assert_eq!(
            drain_all(&mut q),
            vec![Reverse(5), Reverse(4), Reverse(3), Reverse(2), Reverse(1)]
        );
    }

    #[test]
    fn retain_should_rebuild_heap() {
        let mut q = PriorityQueue::new();
        q.enqueues(1..=8);

        let removed = q.retain(|x| *x % 2 == 0);
        assert_eq!(removed, 4);
        assert_eq!(drain_all(&mut q), vec![2, 4, 6, 8]);

        let mut single = PriorityQueue::new();
        single.enqueues([1, 2]);
        let removed_single = single.retain(|x| *x == 2);
        assert_eq!(removed_single, 1);
        assert_eq!(single.front(), Some(&2));
    }

    #[test]
    fn retain_on_empty_should_return_zero() {
        let mut q = PriorityQueue::<i32>::new();

        let removed = q.retain(|_| true);
        assert_eq!(removed, 0);
        assert!(q.is_empty());
    }

    #[test]
    fn iter_should_be_unsorted_but_complete() {
        let mut q = PriorityQueue::new();
        q.enqueues([7, 1, 9, 3, 5]);

        let mut from_iter: Vec<i32> = q.iter().copied().collect();
        from_iter.sort();
        assert_eq!(from_iter, vec![1, 3, 5, 7, 9]);

        let mut from_into_iter: Vec<i32> = (&q).into_iter().copied().collect();
        from_into_iter.sort();
        assert_eq!(from_into_iter, vec![1, 3, 5, 7, 9]);
    }

    #[test]
    fn collection_and_dispose_contract_should_work() {
        let mut q = PriorityQueue::new();
        q.enqueues([3, 1, 2]);

        assert_eq!(Collection::size(&q), 3);
        assert_eq!(Collection::count(&q, |x| *x % 2 == 1), 2);

        let mut all = Collection::collect(&q);
        all.sort();
        assert_eq!(all, vec![1, 2, 3]);

        Collection::clear(&mut q);
        assert!(Collection::is_empty(&q));

        assert!(!Disposable::is_disposed(&q));
        Disposable::dispose(&mut q);
        assert!(Disposable::is_disposed(&q));
        assert!(Collection::is_empty(&q));
    }

    #[test]
    fn priority_queue_like_should_be_implemented() {
        fn assert_priority_queue_like<Q: PriorityQueueLike<i32>>(_q: &Q) {}

        let q = PriorityQueue::new();
        assert_priority_queue_like(&q);
    }

    fn model_front(model: &[i32]) -> Option<i32> {
        model.iter().min().copied()
    }

    fn model_dequeue(model: &mut Vec<i32>) -> Option<i32> {
        let (idx, _) = model.iter().enumerate().min_by_key(|(_, x)| *x)?;
        Some(model.swap_remove(idx))
    }

    fn model_replace_front(model: &mut Vec<i32>, new_back: i32) -> Option<i32> {
        match model_dequeue(model) {
            Some(removed) => {
                model.push(new_back);
                Some(removed)
            }
            None => {
                model.push(new_back);
                None
            }
        }
    }

    fn sorted_dequeue_from_binary_heap(heap: &mut BinaryHeap<Reverse<i32>>) -> Vec<i32> {
        let mut out = Vec::new();
        while let Some(Reverse(x)) = heap.pop() {
            out.push(x);
        }
        out
    }

    #[test]
    fn randomized_ops_should_match_model_and_binary_heap() {
        let seeds = [1_u64, 7, 97, 0x1234_5678, 0xDEAD_BEEF, 0xCAFE_BABE];

        for seed in seeds {
            let mut rng = XorShift64::new(seed);
            let mut q = PriorityQueue::new();
            let mut model: Vec<i32> = Vec::new();
            let mut bh = BinaryHeap::<Reverse<i32>>::new();

            for step in 0..5000 {
                match rng.next_u64() % 6 {
                    0 => {
                        let x = rng.next_i32_in(10_000) - 5000;
                        q.enqueue(x);
                        model.push(x);
                        bh.push(Reverse(x));
                    }
                    1 => {
                        let got = q.dequeue();
                        let expect = model_dequeue(&mut model);
                        let bh_expect = bh.pop().map(|Reverse(x)| x);
                        assert_eq!(got, expect);
                        assert_eq!(got, bh_expect);
                    }
                    2 => {
                        let x = rng.next_i32_in(10_000) - 5000;
                        let got = q.replace_front(x);
                        let expect = model_replace_front(&mut model, x);
                        let bh_expect = match bh.pop() {
                            Some(Reverse(v)) => {
                                bh.push(Reverse(x));
                                Some(v)
                            }
                            None => {
                                bh.push(Reverse(x));
                                None
                            }
                        };
                        assert_eq!(got, expect);
                        assert_eq!(got, bh_expect);
                    }
                    3 => {
                        let batch_size = (rng.next_u64() % 8) as usize;
                        let mut batch = Vec::with_capacity(batch_size);
                        for _ in 0..batch_size {
                            let x = rng.next_i32_in(10_000) - 5000;
                            batch.push(x);
                        }
                        q.enqueues(batch.iter().copied());
                        for &x in &batch {
                            model.push(x);
                            bh.push(Reverse(x));
                        }
                    }
                    4 => {
                        let div = (rng.next_u64() % 5 + 2) as i32;
                        let rem = (rng.next_u64() % div as u64) as i32;
                        let removed = q.retain(|x| x.rem_euclid(div) != rem);

                        let before = model.len();
                        model.retain(|x| x.rem_euclid(div) != rem);
                        let expect_removed = before - model.len();

                        let mut kept = Vec::new();
                        while let Some(Reverse(x)) = bh.pop() {
                            if x.rem_euclid(div) != rem {
                                kept.push(x);
                            }
                        }
                        for x in kept {
                            bh.push(Reverse(x));
                        }

                        assert_eq!(removed, expect_removed);
                    }
                    _ => {
                        q.clear();
                        model.clear();
                        bh.clear();
                    }
                }

                assert_eq!(q.size(), model.len());
                assert_eq!(q.front().copied(), model_front(&model));
                assert_eq!(q.front().copied(), bh.peek().map(|Reverse(x)| *x));

                if step % 257 == 0 {
                    let mut q_clone = q.clone();
                    let mut expected = model.clone();
                    expected.sort_unstable();
                    let actual = drain_all(&mut q_clone);
                    assert_eq!(actual, expected);

                    let mut bh_clone = bh.clone();
                    assert_eq!(actual, sorted_dequeue_from_binary_heap(&mut bh_clone));
                }
            }

            let mut expected = model;
            expected.sort_unstable();
            assert_eq!(drain_all(&mut q), expected);
            assert_eq!(expected, sorted_dequeue_from_binary_heap(&mut bh));
        }
    }

    fn enqueue_with_swap(q: &mut PriorityQueue<i32>, x: i32) {
        q.elements.push(x);
        let index = q.elements.len() - 1;
        q.up_swap(index);
    }

    fn enqueue_with_hole(q: &mut PriorityQueue<i32>, x: i32) {
        q.elements.push(x);
        let index = q.elements.len() - 1;
        q.up_hole(index);
    }

    fn dequeue_with_swap(q: &mut PriorityQueue<i32>) -> Option<i32> {
        if q.elements.is_empty() {
            return None;
        }
        if q.elements.len() == 1 {
            return q.elements.pop();
        }

        let removed = q.elements.swap_remove(0);
        q.down_swap(0);
        Some(removed)
    }

    fn dequeue_with_hole(q: &mut PriorityQueue<i32>) -> Option<i32> {
        if q.elements.is_empty() {
            return None;
        }
        if q.elements.len() == 1 {
            return q.elements.pop();
        }

        let removed = q.elements.swap_remove(0);
        q.down_hole(0);
        Some(removed)
    }

    fn replace_front_with_swap(q: &mut PriorityQueue<i32>, x: i32) -> Option<i32> {
        if q.elements.is_empty() {
            q.elements.push(x);
            return None;
        }

        let removed = std::mem::replace(&mut q.elements[0], x);
        q.down_swap(0);
        Some(removed)
    }

    fn replace_front_with_hole(q: &mut PriorityQueue<i32>, x: i32) -> Option<i32> {
        if q.elements.is_empty() {
            q.elements.push(x);
            return None;
        }

        let removed = std::mem::replace(&mut q.elements[0], x);
        q.down_hole(0);
        Some(removed)
    }

    #[test]
    fn forced_swap_and_hole_paths_should_match() {
        let seeds = [3_u64, 31, 131, 997, 65537, 0xBADC_0FFE];

        for seed in seeds {
            let mut rng = XorShift64::new(seed);
            let mut q_swap = PriorityQueue::<i32>::new();
            let mut q_hole = PriorityQueue::<i32>::new();

            for step in 0..7000 {
                match rng.next_u64() % 4 {
                    0 => {
                        let x = rng.next_i32_in(20_000) - 10_000;
                        enqueue_with_swap(&mut q_swap, x);
                        enqueue_with_hole(&mut q_hole, x);
                    }
                    1 => {
                        let got_swap = dequeue_with_swap(&mut q_swap);
                        let got_hole = dequeue_with_hole(&mut q_hole);
                        assert_eq!(got_swap, got_hole);
                    }
                    2 => {
                        let x = rng.next_i32_in(20_000) - 10_000;
                        let got_swap = replace_front_with_swap(&mut q_swap, x);
                        let got_hole = replace_front_with_hole(&mut q_hole, x);
                        assert_eq!(got_swap, got_hole);
                    }
                    _ => {
                        q_swap.clear();
                        q_hole.clear();
                    }
                }

                assert_eq!(q_swap.size(), q_hole.size());
                assert_eq!(q_swap.front().copied(), q_hole.front().copied());

                if step % 311 == 0 {
                    let mut a = q_swap.clone();
                    let mut b = q_hole.clone();
                    assert_eq!(drain_all(&mut a), drain_all(&mut b));
                }
            }

            assert_eq!(drain_all(&mut q_swap), drain_all(&mut q_hole));
        }
    }
}
