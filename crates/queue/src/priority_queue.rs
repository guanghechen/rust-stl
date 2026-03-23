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

impl<T> PriorityQueue<T>
where
    T: Ord,
{
    pub fn new() -> Self {
        Self {
            disposed: false,
            elements: Vec::new(),
        }
    }

    fn up(&mut self, index: usize) {
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

    fn down(&mut self, index: usize) {
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
    use std::cmp::Reverse;

    use collection::{Collection, Disposable};

    use crate::traits::{PriorityQueueLike, QueueLike};

    use super::PriorityQueue;

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
}
