use collection::{Collection, Disposable};

use crate::error::QueueError;
use crate::traits::{CircularQueueLike, DequeLike, QueueLike};

#[derive(Debug)]
pub struct CircularQueue<T> {
    elements: Vec<Option<T>>,
    capacity: usize,
    size: usize,
    start: usize,
    end: usize,
    disposed: bool,
}

impl<T> CircularQueue<T> {
    pub fn new(capacity: usize) -> Result<Self, QueueError> {
        if capacity == 0 {
            return Err(QueueError::InvalidCapacity { capacity });
        }

        Ok(Self {
            elements: empty_buffer(capacity),
            capacity,
            size: 0,
            start: 0,
            end: 0,
            disposed: false,
        })
    }

    fn do_enqueue(&mut self, element: T) {
        if self.size == 0 {
            self.elements[0] = Some(element);
            self.size = 1;
            self.start = 0;
            self.end = 0;
            return;
        }

        self.end = self.next_index(self.end);
        self.elements[self.end] = Some(element);

        if self.size < self.capacity {
            self.size += 1;
        } else {
            self.start = self.next_index(self.start);
        }
    }

    fn do_dequeue(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }

        let removed = self.elements[self.start].take();
        debug_assert!(removed.is_some());

        if self.size == 1 {
            self.size = 0;
            self.start = 0;
            self.end = 0;
            return removed;
        }

        self.size -= 1;
        self.start = self.next_index(self.start);
        removed
    }

    fn do_enqueue_front(&mut self, element: T) {
        if self.size == 0 {
            self.elements[0] = Some(element);
            self.size = 1;
            self.start = 0;
            self.end = 0;
            return;
        }

        self.start = self.prev_index(self.start);
        self.elements[self.start] = Some(element);

        if self.size < self.capacity {
            self.size += 1;
        } else {
            self.end = self.prev_index(self.end);
        }
    }

    fn do_dequeue_back(&mut self) -> Option<T> {
        if self.size == 0 {
            return None;
        }

        let removed = self.elements[self.end].take();
        debug_assert!(removed.is_some());

        if self.size == 1 {
            self.size = 0;
            self.start = 0;
            self.end = 0;
            return removed;
        }

        self.size -= 1;
        self.end = self.prev_index(self.end);
        removed
    }

    fn do_replace_front(&mut self, new_back: T) -> Option<T> {
        if self.size == 0 {
            self.elements[0] = Some(new_back);
            self.size = 1;
            self.start = 0;
            self.end = 0;
            return None;
        }

        let removed = self.elements[self.start].take();
        debug_assert!(removed.is_some());

        if self.size == 1 {
            self.elements[0] = Some(new_back);
            self.start = 0;
            self.end = 0;
            return removed;
        }

        self.start = self.next_index(self.start);
        self.end = self.next_index(self.end);
        self.elements[self.end] = Some(new_back);
        removed
    }

    fn do_replace_back(&mut self, new_front: T) -> Option<T> {
        if self.size == 0 {
            self.elements[0] = Some(new_front);
            self.size = 1;
            self.start = 0;
            self.end = 0;
            return None;
        }

        let removed = self.elements[self.end].take();
        debug_assert!(removed.is_some());

        if self.size == 1 {
            self.elements[0] = Some(new_front);
            self.start = 0;
            self.end = 0;
            return removed;
        }

        self.end = self.prev_index(self.end);
        self.start = self.prev_index(self.start);
        self.elements[self.start] = Some(new_front);
        removed
    }

    fn clear_internal(&mut self) {
        for offset in 0..self.size {
            let idx = self.physical_index(offset);
            self.elements[idx].take();
        }
        self.size = 0;
        self.start = 0;
        self.end = 0;
    }

    fn rearrange_impl(&mut self) {
        if self.size == 0 {
            self.start = 0;
            self.end = 0;
            return;
        }
        if self.start == 0 {
            return;
        }

        let capacity = self.capacity;
        let size = self.size;
        let start = self.start;
        let end = self.end;

        // Hybrid strategy:
        // - Dense queue: rotate the whole buffer for better cache locality.
        // - Sparse queue: move only active elements to keep O(size) behavior.
        let is_dense = (size as u128) * 4 >= (capacity as u128) * 3;
        if is_dense {
            self.elements.rotate_left(start);
            self.start = 0;
            self.end = size - 1;
            return;
        }

        if start <= end {
            for i in 0..size {
                let src = start + i;
                self.elements[i] = self.elements[src].take();
            }
        } else {
            let first_len = capacity - start;

            let mut tail = Vec::with_capacity(end + 1);
            for i in 0..=end {
                tail.push(self.elements[i].take());
            }

            for i in 0..first_len {
                let src = start + i;
                self.elements[i] = self.elements[src].take();
            }

            for (i, value) in tail.into_iter().enumerate() {
                self.elements[first_len + i] = value;
            }
        }

        self.start = 0;
        self.end = size - 1;
    }

    fn physical_index(&self, offset: usize) -> usize {
        (self.start + offset) % self.capacity
    }

    fn next_index(&self, index: usize) -> usize {
        let next = index + 1;
        if next == self.capacity { 0 } else { next }
    }

    fn prev_index(&self, index: usize) -> usize {
        if index == 0 {
            self.capacity - 1
        } else {
            index - 1
        }
    }
}

impl<T> QueueLike<T> for CircularQueue<T> {
    fn front(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        self.elements[self.start].as_ref()
    }

    fn enqueue(&mut self, element: T) {
        self.do_enqueue(element);
    }

    fn dequeue(&mut self) -> Option<T> {
        self.do_dequeue()
    }

    fn enqueues<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = T>,
    {
        let capacity = self.capacity;
        let mut size = self.size;
        let mut start = self.start;
        let mut end = if size == 0 { capacity - 1 } else { self.end };

        let mut inserted = 0usize;
        for element in elements {
            inserted += 1;
            size += 1;
            end = if end + 1 == capacity { 0 } else { end + 1 };
            self.elements[end] = Some(element);
        }

        if inserted == 0 {
            return;
        }

        if size > capacity {
            let shift = size - capacity;
            size = capacity;
            start = (start + shift) % capacity;
        }

        self.size = size;
        self.start = start;
        self.end = end;
    }

    fn replace_front(&mut self, new_back: T) -> Option<T> {
        self.do_replace_front(new_back)
    }
}

impl<T> DequeLike<T> for CircularQueue<T> {
    fn back(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        self.elements[self.end].as_ref()
    }

    fn enqueue_front(&mut self, element: T) {
        self.do_enqueue_front(element);
    }

    fn dequeue_back(&mut self) -> Option<T> {
        self.do_dequeue_back()
    }

    fn enqueues_front<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = T>,
    {
        let capacity = self.capacity;
        let mut size = self.size;
        let mut start = self.start;
        let mut end = self.end;

        for element in elements {
            if size == 0 {
                self.elements[0] = Some(element);
                size = 1;
                start = 0;
                end = 0;
                continue;
            }

            start = if start == 0 { capacity - 1 } else { start - 1 };
            self.elements[start] = Some(element);
            if size < capacity {
                size += 1;
            } else {
                end = if end == 0 { capacity - 1 } else { end - 1 };
            }
        }

        self.size = size;
        self.start = start;
        self.end = end;
    }

    fn replace_back(&mut self, new_front: T) -> Option<T> {
        self.do_replace_back(new_front)
    }
}

impl<T> CircularQueueLike<T> for CircularQueue<T> {
    type Error = QueueError;

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn at(&self, index: isize) -> Option<&T> {
        if index < 0 || index as usize >= self.size {
            return None;
        }

        let idx = self.physical_index(index as usize);
        self.elements[idx].as_ref()
    }

    fn resize(&mut self, new_capacity: usize) -> Result<(), Self::Error> {
        if new_capacity == 0 {
            return Err(QueueError::InvalidCapacity {
                capacity: new_capacity,
            });
        }
        if self.size > new_capacity {
            return Err(QueueError::InsufficientCapacity {
                current_size: self.size,
                requested_capacity: new_capacity,
            });
        }

        self.rearrange_impl();

        if new_capacity > self.capacity {
            self.elements.resize_with(new_capacity, || None);
        } else {
            self.elements.truncate(new_capacity);
        }

        self.capacity = new_capacity;
        self.start = 0;
        self.end = if self.size == 0 { 0 } else { self.size - 1 };
        Ok(())
    }

    fn rearrange(&mut self) {
        self.rearrange_impl();
    }
}

impl<T> Collection for CircularQueue<T> {
    type Item = T;
    type Iter<'a>
        = Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        Iter {
            queue: self,
            offset: 0,
        }
    }

    fn size(&self) -> usize {
        self.size
    }

    fn clear(&mut self) {
        self.clear_internal();
    }

    fn retain<F>(&mut self, mut f: F) -> usize
    where
        F: FnMut(&Self::Item) -> bool,
    {
        let original_size = self.size;
        if original_size == 0 {
            return 0;
        }

        self.rearrange_impl();

        let mut kept_size = 0usize;
        for read_idx in 0..original_size {
            let item = self.elements[read_idx]
                .take()
                .expect("queue item must exist");
            if f(&item) {
                self.elements[kept_size] = Some(item);
                kept_size += 1;
            }
        }

        self.size = kept_size;
        self.start = 0;
        self.end = if kept_size == 0 { 0 } else { kept_size - 1 };

        original_size - kept_size
    }
}

impl<T> Disposable for CircularQueue<T> {
    fn dispose(&mut self) {
        self.disposed = true;
        self.clear_internal();
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

pub struct Iter<'a, T> {
    queue: &'a CircularQueue<T>,
    offset: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.queue.size {
            return None;
        }

        let idx = self.queue.physical_index(self.offset);
        self.offset += 1;
        self.queue.elements[idx].as_ref()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remain = self.queue.size - self.offset;
        (remain, Some(remain))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}

impl<'a, T> IntoIterator for &'a CircularQueue<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

fn empty_buffer<T>(capacity: usize) -> Vec<Option<T>> {
    std::iter::repeat_with(|| None).take(capacity).collect()
}

#[cfg(test)]
mod tests {
    use collection::{Collection, Disposable};

    use crate::traits::{CircularQueueLike, DequeLike, QueueLike};

    use super::CircularQueue;
    use crate::error::QueueError;

    fn as_vec<T: Clone>(q: &CircularQueue<T>) -> Vec<T> {
        Collection::collect(q)
    }

    #[test]
    fn constructor_should_validate_capacity() {
        assert!(matches!(
            CircularQueue::<i32>::new(0),
            Err(QueueError::InvalidCapacity { capacity: 0 })
        ));
        assert!(CircularQueue::<i32>::new(1).is_ok());
    }

    #[test]
    fn queue_like_ops_should_work() {
        let mut q = CircularQueue::new(4).expect("new should work");

        assert_eq!(q.dequeue(), None);

        q.enqueue(1);
        q.enqueues([2, 3, 4]);
        assert_eq!(as_vec(&q), vec![1, 2, 3, 4]);

        q.enqueue(5);
        assert_eq!(as_vec(&q), vec![2, 3, 4, 5]);

        assert_eq!(q.front(), Some(&2));
        assert_eq!(q.replace_front(6), Some(2));
        assert_eq!(as_vec(&q), vec![3, 4, 5, 6]);
    }

    #[test]
    fn replace_front_should_handle_empty_and_single_item() {
        let mut q = CircularQueue::new(3).expect("new should work");

        assert_eq!(q.replace_front(10), None);
        assert_eq!(as_vec(&q), vec![10]);

        assert_eq!(q.replace_front(20), Some(10));
        assert_eq!(as_vec(&q), vec![20]);
    }

    #[test]
    fn front_and_back_should_return_none_on_empty_queue() {
        let q = CircularQueue::<i32>::new(3).expect("new should work");

        assert_eq!(q.front(), None);
        assert_eq!(q.back(), None);
    }

    #[test]
    fn dequeue_and_dequeue_back_should_handle_single_and_empty_states() {
        let mut q = CircularQueue::new(3).expect("new should work");

        assert_eq!(q.dequeue(), None);
        assert_eq!(q.dequeue_back(), None);

        q.enqueue(42);
        assert_eq!(q.dequeue(), Some(42));
        assert!(q.is_empty());

        q.enqueue(7);
        assert_eq!(q.dequeue_back(), Some(7));
        assert!(q.is_empty());
    }

    #[test]
    fn enqueues_should_keep_latest_elements_when_over_capacity() {
        let mut q = CircularQueue::new(4).expect("new should work");

        q.enqueues(1..=10);

        assert_eq!(as_vec(&q), vec![7, 8, 9, 10]);
        assert_eq!(q.front(), Some(&7));
        assert_eq!(q.back(), Some(&10));
    }

    #[test]
    fn deque_like_ops_should_work() {
        let mut q = CircularQueue::new(4).expect("new should work");

        q.enqueues([1, 2]);
        q.enqueue_front(9);
        q.enqueue_front(8);
        assert_eq!(as_vec(&q), vec![8, 9, 1, 2]);

        q.enqueues_front([7, 6]);
        assert_eq!(as_vec(&q), vec![6, 7, 8, 9]);

        assert_eq!(q.back(), Some(&9));
        assert_eq!(q.dequeue_back(), Some(9));
        assert_eq!(q.replace_back(5), Some(8));
        assert_eq!(as_vec(&q), vec![5, 6, 7]);
    }

    #[test]
    fn replace_back_should_handle_empty_and_single_item() {
        let mut q = CircularQueue::new(3).expect("new should work");

        assert_eq!(q.replace_back(10), None);
        assert_eq!(as_vec(&q), vec![10]);

        assert_eq!(q.replace_back(20), Some(10));
        assert_eq!(as_vec(&q), vec![20]);
    }

    #[test]
    fn enqueues_front_should_keep_latest_front_inserted_elements() {
        let mut q = CircularQueue::new(4).expect("new should work");

        q.enqueues_front([1, 2, 3, 4, 5]);

        assert_eq!(as_vec(&q), vec![5, 4, 3, 2]);
        assert_eq!(q.front(), Some(&5));
        assert_eq!(q.back(), Some(&2));
    }

    #[test]
    fn enqueue_front_should_handle_empty_and_full_queue() {
        let mut q = CircularQueue::new(3).expect("new should work");

        q.enqueue_front(1);
        assert_eq!(as_vec(&q), vec![1]);

        q.enqueue(2);
        q.enqueue(3);
        assert_eq!(as_vec(&q), vec![1, 2, 3]);

        q.enqueue_front(9);
        assert_eq!(as_vec(&q), vec![9, 1, 2]);
    }

    #[test]
    fn circular_queue_like_ops_should_work() {
        let mut q = CircularQueue::new(4).expect("new should work");
        q.enqueues([1, 2, 3, 4, 5]);

        assert_eq!(q.capacity(), 4);
        assert_eq!(q.at(-1), None);
        assert_eq!(q.at(0), Some(&2));
        assert_eq!(q.at(3), Some(&5));
        assert_eq!(q.at(4), None);

        assert_eq!(
            q.resize(3),
            Err(QueueError::InsufficientCapacity {
                current_size: 4,
                requested_capacity: 3,
            })
        );

        q.resize(6).expect("resize should work");
        assert_eq!(q.capacity(), 6);

        q.enqueue(7);
        q.rearrange();
        assert_eq!(as_vec(&q), vec![2, 3, 4, 5, 7]);
    }

    #[test]
    fn rearrange_should_preserve_order_after_wraparound() {
        let mut q = CircularQueue::new(5).expect("new should work");

        q.enqueues([1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(as_vec(&q), vec![3, 4, 5, 6, 7]);

        q.rearrange();
        assert_eq!(as_vec(&q), vec![3, 4, 5, 6, 7]);

        q.enqueue(8);
        assert_eq!(as_vec(&q), vec![4, 5, 6, 7, 8]);
    }

    #[test]
    fn rearrange_should_handle_empty_and_non_wrapped_shifted_state() {
        let mut q = CircularQueue::new(6).expect("new should work");

        q.rearrange();
        assert!(q.is_empty());

        q.enqueues([1, 2, 3, 4]);
        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
        assert_eq!(as_vec(&q), vec![3, 4]);

        q.rearrange();
        assert_eq!(as_vec(&q), vec![3, 4]);

        q.enqueue(5);
        assert_eq!(as_vec(&q), vec![3, 4, 5]);
    }

    #[test]
    fn rearrange_should_handle_sparse_wrapped_state() {
        let mut q = CircularQueue::new(10).expect("new should work");

        q.enqueues(1..=10);
        for expected in 1..=6 {
            assert_eq!(q.dequeue(), Some(expected));
        }
        q.enqueues([11, 12]);
        assert_eq!(as_vec(&q), vec![7, 8, 9, 10, 11, 12]);

        q.rearrange();
        assert_eq!(as_vec(&q), vec![7, 8, 9, 10, 11, 12]);

        q.enqueue(13);
        assert_eq!(as_vec(&q), vec![7, 8, 9, 10, 11, 12, 13]);
    }

    #[test]
    fn enqueues_should_support_empty_input_without_side_effects() {
        let mut q = CircularQueue::new(4).expect("new should work");

        q.enqueues(std::iter::empty());
        assert!(q.is_empty());

        q.enqueues([1, 2]);
        q.enqueues(std::iter::empty());
        assert_eq!(as_vec(&q), vec![1, 2]);
    }

    #[test]
    fn resize_should_support_exact_shrink_and_followup_push() {
        let mut q = CircularQueue::new(5).expect("new should work");

        q.enqueues([1, 2, 3]);
        q.resize(3).expect("resize to exact size should work");
        assert_eq!(q.capacity(), 3);
        assert_eq!(as_vec(&q), vec![1, 2, 3]);

        q.enqueue(4);
        assert_eq!(as_vec(&q), vec![2, 3, 4]);
    }

    #[test]
    fn resize_should_reject_zero_capacity() {
        let mut q = CircularQueue::<i32>::new(5).expect("new should work");

        assert_eq!(
            q.resize(0),
            Err(QueueError::InvalidCapacity { capacity: 0 })
        );
    }

    #[test]
    fn retain_should_work_on_wrapped_queue() {
        let mut q = CircularQueue::new(6).expect("new should work");

        q.enqueues([1, 2, 3, 4, 5, 6]);
        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
        q.enqueues([7, 8]);
        assert_eq!(as_vec(&q), vec![3, 4, 5, 6, 7, 8]);

        let removed = q.retain(|x| *x % 2 == 0);
        assert_eq!(removed, 3);
        assert_eq!(as_vec(&q), vec![4, 6, 8]);
    }

    #[test]
    fn retain_should_support_keep_all_and_remove_all() {
        let mut q = CircularQueue::new(4).expect("new should work");

        q.enqueues([1, 2, 3, 4]);
        let removed_none = q.retain(|_| true);
        assert_eq!(removed_none, 0);
        assert_eq!(as_vec(&q), vec![1, 2, 3, 4]);

        let removed_all = q.retain(|_| false);
        assert_eq!(removed_all, 4);
        assert!(q.is_empty());
    }

    #[test]
    fn retain_should_return_zero_on_empty_queue() {
        let mut q = CircularQueue::<i32>::new(4).expect("new should work");

        let removed = q.retain(|_| true);
        assert_eq!(removed, 0);
        assert!(q.is_empty());
    }

    #[test]
    fn iter_and_into_iter_should_report_exact_size_hint() {
        let mut q = CircularQueue::new(4).expect("new should work");
        q.enqueues([1, 2, 3]);

        let mut it = q.iter();
        assert_eq!(it.size_hint(), (3, Some(3)));
        assert_eq!(it.next(), Some(&1));
        assert_eq!(it.size_hint(), (2, Some(2)));
        assert_eq!(it.next(), Some(&2));
        assert_eq!(it.next(), Some(&3));
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.next(), None);

        let from_into_iter: Vec<i32> = (&q).into_iter().copied().collect();
        assert_eq!(from_into_iter, vec![1, 2, 3]);
    }

    #[test]
    fn collection_and_dispose_contract_should_work() {
        let mut q = CircularQueue::new(6).expect("new should work");
        q.enqueues([1, 2, 3, 4, 5, 6]);

        assert_eq!(Collection::size(&q), 6);
        assert_eq!(Collection::count(&q, |x| *x % 2 == 0), 3);
        assert_eq!(Collection::collect(&q), vec![1, 2, 3, 4, 5, 6]);

        let removed = Collection::retain(&mut q, |x| *x % 2 == 1);
        assert_eq!(removed, 3);
        assert_eq!(Collection::collect(&q), vec![1, 3, 5]);

        Collection::clear(&mut q);
        assert!(Collection::is_empty(&q));
        assert_eq!(Collection::size(&q), 0);

        assert!(!Disposable::is_disposed(&q));
        Disposable::dispose(&mut q);
        assert!(Disposable::is_disposed(&q));
        assert!(Collection::is_empty(&q));
    }
}
