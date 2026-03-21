use std::collections::LinkedList;

use collection::{Collection, Disposable};

use crate::traits::{DequeLike, QueueLike};

#[derive(Debug, Default)]
pub struct LinkedDeque<T> {
    elements: LinkedList<T>,
    disposed: bool,
}

impl<T> LinkedDeque<T> {
    pub fn new() -> Self {
        Self {
            elements: LinkedList::new(),
            disposed: false,
        }
    }
}

impl<T> QueueLike<T> for LinkedDeque<T> {
    fn front(&self) -> Option<&T> {
        self.elements.front()
    }

    fn enqueue(&mut self, element: T) {
        self.elements.push_back(element);
    }

    fn dequeue(&mut self) -> Option<T> {
        self.elements.pop_front()
    }

    fn replace_front(&mut self, new_back: T) -> Option<T> {
        let removed = self.elements.pop_front();
        self.elements.push_back(new_back);
        removed
    }
}

impl<T> DequeLike<T> for LinkedDeque<T> {
    fn back(&self) -> Option<&T> {
        self.elements.back()
    }

    fn enqueue_front(&mut self, element: T) {
        self.elements.push_front(element);
    }

    fn dequeue_back(&mut self) -> Option<T> {
        self.elements.pop_back()
    }

    fn replace_back(&mut self, new_front: T) -> Option<T> {
        let removed = self.elements.pop_back();
        self.elements.push_front(new_front);
        removed
    }
}

impl<T> Collection for LinkedDeque<T> {
    type Item = T;
    type Iter<'a>
        = std::collections::linked_list::Iter<'a, T>
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

        let mut kept = LinkedList::new();
        while let Some(element) = self.elements.pop_front() {
            if f(&element) {
                kept.push_back(element);
            }
        }

        let removed = before - kept.len();
        self.elements = kept;
        removed
    }
}

impl<T> Disposable for LinkedDeque<T> {
    fn dispose(&mut self) {
        self.disposed = true;
        self.elements.clear();
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}

impl<'a, T> IntoIterator for &'a LinkedDeque<T> {
    type Item = &'a T;
    type IntoIter = std::collections::linked_list::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

#[cfg(test)]
mod tests {
    use collection::{Collection, Disposable};

    use crate::traits::{DequeLike, QueueLike};

    use super::LinkedDeque;

    fn drain_front(q: &mut LinkedDeque<i32>) -> Vec<i32> {
        let mut out = Vec::new();
        while let Some(x) = q.dequeue() {
            out.push(x);
        }
        out
    }

    #[test]
    fn queue_like_should_work() {
        let mut q = LinkedDeque::new();

        assert_eq!(q.front(), None);
        assert_eq!(q.back(), None);
        assert_eq!(q.dequeue(), None);

        q.enqueues([1, 2, 3]);
        assert_eq!(q.front(), Some(&1));
        assert_eq!(q.back(), Some(&3));

        assert_eq!(q.dequeue(), Some(1));
        assert_eq!(q.dequeue(), Some(2));
        assert_eq!(q.dequeue(), Some(3));
        assert_eq!(q.dequeue(), None);
    }

    #[test]
    fn deque_like_should_work() {
        let mut q = LinkedDeque::new();

        q.enqueue_front(2);
        q.enqueue_front(1);
        q.enqueue(3);
        q.enqueue(4);

        assert_eq!(q.front(), Some(&1));
        assert_eq!(q.back(), Some(&4));
        assert_eq!(q.dequeue_back(), Some(4));
        assert_eq!(q.dequeue_back(), Some(3));
        assert_eq!(q.dequeue_back(), Some(2));
        assert_eq!(q.dequeue_back(), Some(1));
        assert_eq!(q.dequeue_back(), None);
    }

    #[test]
    fn replace_front_should_follow_queue_contract() {
        let mut q = LinkedDeque::new();

        assert_eq!(q.replace_front(10), None);
        assert_eq!(drain_front(&mut q), vec![10]);

        q.enqueues([1, 2, 3]);
        assert_eq!(q.replace_front(4), Some(1));
        assert_eq!(drain_front(&mut q), vec![2, 3, 4]);
    }

    #[test]
    fn replace_back_should_follow_deque_contract() {
        let mut q = LinkedDeque::new();

        assert_eq!(q.replace_back(10), None);
        assert_eq!(drain_front(&mut q), vec![10]);

        q.enqueues([1, 2, 3]);
        assert_eq!(q.replace_back(0), Some(3));
        assert_eq!(drain_front(&mut q), vec![0, 1, 2]);
    }

    #[test]
    fn enqueues_front_default_impl_should_keep_reverse_input_order() {
        let mut q = LinkedDeque::new();
        q.enqueues([3, 4]);
        q.enqueues_front([1, 2]);

        assert_eq!(drain_front(&mut q), vec![2, 1, 3, 4]);
    }

    #[test]
    fn retain_should_filter_and_report_removed_count() {
        let mut q = LinkedDeque::new();
        q.enqueues(1..=8);

        assert_eq!(q.retain(|x| *x % 2 == 0), 4);
        assert_eq!(drain_front(&mut q), vec![2, 4, 6, 8]);

        assert_eq!(q.retain(|_| true), 0);
        assert_eq!(q.retain(|_| false), 0);
    }

    #[test]
    fn collection_contract_should_work() {
        let mut q = LinkedDeque::new();
        q.enqueues([3, 1, 2]);

        assert_eq!(Collection::size(&q), 3);
        assert_eq!(Collection::count(&q, |x| *x >= 2), 2);
        assert_eq!(Collection::collect(&q), vec![3, 1, 2]);

        let mut out = Vec::new();
        Collection::collect_into(&q, &mut out);
        assert_eq!(out, vec![3, 1, 2]);

        Collection::clear(&mut q);
        assert!(Collection::is_empty(&q));
    }

    #[test]
    fn iterable_should_be_complete() {
        let mut q = LinkedDeque::new();
        q.enqueues([7, 8, 9]);

        let a: Vec<i32> = q.iter().copied().collect();
        assert_eq!(a, vec![7, 8, 9]);

        let b: Vec<i32> = (&q).into_iter().copied().collect();
        assert_eq!(b, vec![7, 8, 9]);
    }

    #[test]
    fn dispose_should_clear_and_mark_disposed() {
        let mut q = LinkedDeque::new();
        q.enqueues([1, 2, 3]);

        assert!(!q.is_disposed());
        q.dispose();
        assert!(q.is_disposed());
        assert!(q.is_empty());
    }
}
