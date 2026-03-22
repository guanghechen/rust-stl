use collection::{Collection, Disposable};

use crate::error::StackError;
use crate::traits::{CircularStackLike, StackLike};

#[derive(Debug)]
pub struct CircularStack<T> {
    elements: Vec<Option<T>>,
    capacity: usize,
    size: usize,
    start: usize,
    end: usize,
    disposed: bool,
}

impl<T> CircularStack<T> {
    pub fn new(capacity: usize) -> Result<Self, StackError> {
        if capacity == 0 {
            return Err(StackError::InvalidCapacity { capacity });
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

    pub fn fork(&self) -> Self
    where
        T: Clone,
    {
        self.clone()
    }

    fn fork_snapshot(&self) -> Self
    where
        T: Clone,
    {
        let mut elements = empty_buffer(self.capacity);
        for offset in 0..self.size {
            let src = self.physical_index(offset);
            let item = self.elements[src]
                .as_ref()
                .expect("stack item must exist")
                .clone();
            elements[offset] = Some(item);
        }

        Self {
            elements,
            capacity: self.capacity,
            size: self.size,
            start: 0,
            end: if self.size == 0 { 0 } else { self.size - 1 },
            disposed: self.disposed,
        }
    }

    fn do_push(&mut self, element: T) {
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

    fn do_pop(&mut self) -> Option<T> {
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

    fn do_replace_top(&mut self, new_top: T) -> Option<T> {
        if self.size == 0 {
            self.elements[0] = Some(new_top);
            self.size = 1;
            self.start = 0;
            self.end = 0;
            return None;
        }

        let removed = self.elements[self.end].replace(new_top);
        debug_assert!(removed.is_some());
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

    fn shrink_keep_latest(&mut self, new_capacity: usize) {
        if self.size <= new_capacity {
            return;
        }

        let previous_size = self.size;
        let drop_count = previous_size - new_capacity;

        for i in 0..new_capacity {
            self.elements[i] = self.elements[drop_count + i].take();
        }
        for i in new_capacity..previous_size {
            self.elements[i] = None;
        }

        self.size = new_capacity;
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
        // - Dense stack: rotate the whole buffer for better cache locality.
        // - Sparse stack: move only active elements to keep O(size) behavior.
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

    fn physical_index_from_top(&self, offset: usize) -> usize {
        let off = offset % self.capacity;
        if self.end >= off {
            self.end - off
        } else {
            self.capacity + self.end - off
        }
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

impl<T> Clone for CircularStack<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        self.fork_snapshot()
    }
}

impl<T> StackLike<T> for CircularStack<T> {
    fn top(&self) -> Option<&T> {
        if self.size == 0 {
            return None;
        }
        self.elements[self.end].as_ref()
    }

    fn push(&mut self, element: T) {
        self.do_push(element);
    }

    fn pop(&mut self) -> Option<T> {
        self.do_pop()
    }

    fn pushes<I>(&mut self, elements: I)
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

    fn replace_top(&mut self, new_top: T) -> Option<T> {
        self.do_replace_top(new_top)
    }
}

impl<T> CircularStackLike<T> for CircularStack<T> {
    type Error = StackError;

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

    fn update(&mut self, index: isize, element: T) -> bool {
        if index < 0 || index as usize >= self.size {
            return false;
        }

        let idx = self.physical_index(index as usize);
        self.elements[idx] = Some(element);
        true
    }

    fn resize(&mut self, new_capacity: usize) -> Result<(), Self::Error> {
        if new_capacity == 0 {
            return Err(StackError::InvalidCapacity {
                capacity: new_capacity,
            });
        }

        if new_capacity == self.capacity {
            return Ok(());
        }

        if new_capacity > self.capacity {
            let is_wrapped = self.size > 0 && self.start > self.end;
            if is_wrapped {
                self.rearrange_impl();
                self.start = 0;
                self.end = if self.size == 0 { 0 } else { self.size - 1 };
            }

            self.elements.resize_with(new_capacity, || None);
            self.capacity = new_capacity;
            return Ok(());
        }

        if self.start != 0 {
            self.rearrange_impl();
        }
        self.shrink_keep_latest(new_capacity);

        self.elements.truncate(new_capacity);

        self.capacity = new_capacity;
        self.start = 0;
        self.end = if self.size == 0 { 0 } else { self.size - 1 };
        Ok(())
    }

    fn rearrange(&mut self) {
        self.rearrange_impl();
    }
}

impl<T> Collection for CircularStack<T> {
    type Item = T;
    type Iter<'a>
        = Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        Iter {
            stack: self,
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
                .expect("stack item must exist");
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

impl<T> Disposable for CircularStack<T> {
    fn dispose(&mut self) {
        self.disposed = true;
        self.clear_internal();
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

pub struct Iter<'a, T> {
    stack: &'a CircularStack<T>,
    offset: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.stack.size {
            return None;
        }

        let idx = self.stack.physical_index_from_top(self.offset);
        self.offset += 1;
        self.stack.elements[idx].as_ref()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remain = self.stack.size - self.offset;
        (remain, Some(remain))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}

impl<'a, T> IntoIterator for &'a CircularStack<T> {
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

    use crate::traits::{CircularStackLike, StackLike};

    use super::CircularStack;
    use crate::error::StackError;

    fn as_vec_top<T: Clone>(q: &CircularStack<T>) -> Vec<T> {
        Collection::collect(q)
    }

    #[test]
    fn constructor_should_validate_capacity() {
        assert!(matches!(
            CircularStack::<i32>::new(0),
            Err(StackError::InvalidCapacity { capacity: 0 })
        ));
        assert!(CircularStack::<i32>::new(1).is_ok());
    }

    #[test]
    fn stack_like_ops_should_work() {
        let mut s = CircularStack::new(4).expect("new should work");

        assert_eq!(s.top(), None);
        assert_eq!(s.pop(), None);

        s.pushes([1, 2, 3, 4]);
        assert_eq!(as_vec_top(&s), vec![4, 3, 2, 1]);
        assert_eq!(s.top(), Some(&4));

        s.push(5);
        assert_eq!(as_vec_top(&s), vec![5, 4, 3, 2]);
        assert_eq!(s.at(0), Some(&2));
        assert_eq!(s.at(3), Some(&5));

        assert_eq!(s.replace_top(8), Some(5));
        assert_eq!(as_vec_top(&s), vec![8, 4, 3, 2]);

        assert_eq!(s.pop(), Some(8));
        assert_eq!(as_vec_top(&s), vec![4, 3, 2]);
    }

    #[test]
    fn push_pop_and_replace_top_should_handle_single_and_empty_states() {
        let mut s = CircularStack::new(3).expect("new should work");

        assert_eq!(s.replace_top(10), None);
        assert_eq!(as_vec_top(&s), vec![10]);

        assert_eq!(s.pop(), Some(10));
        assert!(s.is_empty());

        s.push(20);
        assert_eq!(s.top(), Some(&20));
        assert_eq!(as_vec_top(&s), vec![20]);
    }

    #[test]
    fn pushes_should_support_empty_input_without_side_effects() {
        let mut s = CircularStack::new(4).expect("new should work");

        s.pushes(std::iter::empty());
        assert!(s.is_empty());

        s.pushes([1, 2]);
        s.pushes(std::iter::empty());
        assert_eq!(as_vec_top(&s), vec![2, 1]);
    }

    #[test]
    fn at_and_update_should_work() {
        let mut s = CircularStack::new(4).expect("new should work");

        s.pushes([1, 2, 3, 4, 5]);
        assert_eq!(as_vec_top(&s), vec![5, 4, 3, 2]);
        assert_eq!(s.at(-1), None);
        assert_eq!(s.at(0), Some(&2));
        assert_eq!(s.at(1), Some(&3));
        assert_eq!(s.at(2), Some(&4));
        assert_eq!(s.at(3), Some(&5));
        assert_eq!(s.at(4), None);

        assert!(s.update(1, -3));
        assert_eq!(s.at(1), Some(&-3));
        assert_eq!(as_vec_top(&s), vec![5, 4, -3, 2]);

        assert!(!s.update(-1, 9));
        assert!(!s.update(4, 9));
        assert_eq!(as_vec_top(&s), vec![5, 4, -3, 2]);
    }

    #[test]
    fn resize_should_keep_latest_on_shrink_and_support_growth() {
        let mut s = CircularStack::new(4).expect("new should work");

        s.pushes([1, 2, 3, 4]);
        s.resize(3).expect("resize should work");
        assert_eq!(s.capacity(), 3);
        assert_eq!(as_vec_top(&s), vec![4, 3, 2]);

        s.push(5);
        assert_eq!(as_vec_top(&s), vec![5, 4, 3]);

        s.resize(5).expect("resize should work");
        s.push(6);
        assert_eq!(s.capacity(), 5);
        assert_eq!(as_vec_top(&s), vec![6, 5, 4, 3]);

        s.resize(2).expect("resize should work");
        assert_eq!(s.capacity(), 2);
        assert_eq!(as_vec_top(&s), vec![6, 5]);
    }

    #[test]
    fn resize_grow_should_preserve_order_for_shifted_non_wrapped_state() {
        let mut s = CircularStack::new(5).expect("new should work");
        s.pushes([1, 2, 3, 4, 5, 6, 7]);

        assert_eq!(s.pop(), Some(7));
        assert_eq!(s.pop(), Some(6));
        assert_eq!(as_vec_top(&s), vec![5, 4, 3]);

        s.resize(8).expect("resize should work");
        assert_eq!(s.capacity(), 8);
        assert_eq!(as_vec_top(&s), vec![5, 4, 3]);
        assert_eq!(s.at(0), Some(&3));
        assert_eq!(s.at(2), Some(&5));

        s.push(8);
        assert_eq!(as_vec_top(&s), vec![8, 5, 4, 3]);

        s.resize(8).expect("resize should work");
        assert_eq!(as_vec_top(&s), vec![8, 5, 4, 3]);
    }

    #[test]
    fn resize_should_reject_zero_capacity() {
        let mut s = CircularStack::<i32>::new(5).expect("new should work");

        assert_eq!(
            s.resize(0),
            Err(StackError::InvalidCapacity { capacity: 0 })
        );
    }

    #[test]
    fn rearrange_should_preserve_order_for_dense_wrapped_state() {
        let mut s = CircularStack::new(8).expect("new should work");
        s.pushes(1..=10);

        let before = as_vec_top(&s);
        s.rearrange();
        assert_eq!(as_vec_top(&s), before);

        s.push(11);
        assert_eq!(as_vec_top(&s), vec![11, 10, 9, 8, 7, 6, 5, 4]);
    }

    #[test]
    fn rearrange_should_handle_empty_and_sparse_shifted_non_wrapped_state() {
        let mut s = CircularStack::new(10).expect("new should work");

        s.rearrange();
        assert!(s.is_empty());

        s.pushes(1..=12);
        for _ in 0..3 {
            assert!(s.pop().is_some());
        }

        let before = as_vec_top(&s);
        s.rearrange();
        assert_eq!(as_vec_top(&s), before);

        s.push(13);
        assert_eq!(as_vec_top(&s), vec![13, 9, 8, 7, 6, 5, 4, 3]);
    }

    #[test]
    fn rearrange_should_preserve_order_for_sparse_wrapped_state() {
        let mut s = CircularStack::new(16).expect("new should work");
        s.pushes(1..=30);
        for _ in 0..12 {
            assert!(s.pop().is_some());
        }

        let before = as_vec_top(&s);
        s.rearrange();
        assert_eq!(as_vec_top(&s), before);

        s.pushes([31, 32]);
        assert_eq!(as_vec_top(&s), vec![32, 31, 18, 17, 16, 15]);
    }

    #[test]
    fn retain_should_filter_and_preserve_order() {
        let mut s = CircularStack::new(6).expect("new should work");
        s.pushes([1, 2, 3, 4, 5, 6, 7, 8]);

        let removed = s.retain(|x| *x % 2 == 0);
        assert_eq!(removed, 3);
        assert_eq!(as_vec_top(&s), vec![8, 6, 4]);

        let removed_none = s.retain(|_| true);
        assert_eq!(removed_none, 0);
        assert_eq!(as_vec_top(&s), vec![8, 6, 4]);

        let removed_all = s.retain(|_| false);
        assert_eq!(removed_all, 3);
        assert!(s.is_empty());
    }

    #[test]
    fn retain_should_return_zero_on_empty_stack() {
        let mut s = CircularStack::<i32>::new(4).expect("new should work");

        let removed = s.retain(|_| true);
        assert_eq!(removed, 0);
        assert!(s.is_empty());
    }

    #[test]
    fn iter_and_into_iter_should_report_exact_size_hint() {
        let mut s = CircularStack::new(4).expect("new should work");
        s.pushes([1, 2, 3]);

        let mut it = s.iter();
        assert_eq!(it.size_hint(), (3, Some(3)));
        assert_eq!(it.next(), Some(&3));
        assert_eq!(it.size_hint(), (2, Some(2)));
        assert_eq!(it.next(), Some(&2));
        assert_eq!(it.next(), Some(&1));
        assert_eq!(it.size_hint(), (0, Some(0)));
        assert_eq!(it.next(), None);

        let from_into_iter: Vec<i32> = (&s).into_iter().copied().collect();
        assert_eq!(from_into_iter, vec![3, 2, 1]);
    }

    #[test]
    fn collection_and_dispose_contract_should_work() {
        let mut s = CircularStack::new(6).expect("new should work");
        s.pushes([1, 2, 3, 4, 5, 6]);

        assert_eq!(Collection::size(&s), 6);
        assert_eq!(Collection::count(&s, |x| *x % 2 == 0), 3);
        assert_eq!(Collection::collect(&s), vec![6, 5, 4, 3, 2, 1]);

        let removed = Collection::retain(&mut s, |x| *x % 2 == 1);
        assert_eq!(removed, 3);
        assert_eq!(Collection::collect(&s), vec![5, 3, 1]);

        Collection::clear(&mut s);
        assert!(Collection::is_empty(&s));

        assert!(!s.is_disposed());
        s.pushes([7, 8]);
        s.dispose();
        assert!(s.is_disposed());
        assert!(s.is_empty());
    }

    #[test]
    fn circular_stack_like_ops_should_work() {
        let mut s = CircularStack::new(5).expect("new should work");

        s.pushes([1, 2, 3, 4, 5, 6]);
        assert_eq!(s.capacity(), 5);
        assert_eq!(as_vec_top(&s), vec![6, 5, 4, 3, 2]);

        assert_eq!(s.at(0), Some(&2));
        assert_eq!(s.at(4), Some(&6));
        assert_eq!(s.at(5), None);

        assert!(s.update(2, 40));
        assert_eq!(as_vec_top(&s), vec![6, 5, 40, 3, 2]);

        s.rearrange();
        assert_eq!(as_vec_top(&s), vec![6, 5, 40, 3, 2]);

        s.resize(3).expect("resize should work");
        assert_eq!(as_vec_top(&s), vec![6, 5, 40]);
    }

    #[test]
    fn fork_should_create_independent_snapshot() {
        let mut s = CircularStack::new(5).expect("new should work");
        s.pushes([1, 2, 3, 4, 5, 6, 7]);

        let mut forked = StackLike::fork(&s);
        assert_eq!(as_vec_top(&s), as_vec_top(&forked));
        assert_eq!(s.capacity(), forked.capacity());

        s.push(8);
        assert_eq!(as_vec_top(&s), vec![8, 7, 6, 5, 4]);
        assert_eq!(as_vec_top(&forked), vec![7, 6, 5, 4, 3]);

        forked.push(9);
        assert_eq!(as_vec_top(&forked), vec![9, 7, 6, 5, 4]);
        assert_eq!(as_vec_top(&s), vec![8, 7, 6, 5, 4]);
    }

    #[test]
    fn fork_should_preserve_wrapped_layout_observable_behavior() {
        let mut s = CircularStack::new(8).expect("new should work");
        s.pushes(1..=12);
        for _ in 0..3 {
            assert!(s.pop().is_some());
        }
        s.pushes([13, 14]);

        let forked = s.fork();
        assert_eq!(as_vec_top(&forked), as_vec_top(&s));
        assert_eq!(forked.capacity(), s.capacity());

        for i in 0..s.size() {
            assert_eq!(forked.at(i as isize), s.at(i as isize));
        }
    }
}
