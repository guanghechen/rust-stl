use std::sync::Arc;

use collection::Collection;
use stack::{CircularStack, CircularStackLike, StackLike};

use crate::error::HistoryError;
use crate::traits::HistoryLike;

pub type EqualsFn<T> = Arc<dyn Fn(&T, &T) -> bool + 'static>;

#[derive(Clone)]
pub struct History<T> {
    name: String,
    equals: EqualsFn<T>,
    stack: CircularStack<T>,
    present: isize,
}

impl<T: PartialEq> History<T> {
    pub fn new(name: impl Into<String>, capacity: usize) -> Result<Self, HistoryError> {
        Self::with_equals(name, capacity, |x, y| x == y)
    }
}

impl<T> History<T> {
    pub fn with_equals<F>(
        name: impl Into<String>,
        capacity: usize,
        equals: F,
    ) -> Result<Self, HistoryError>
    where
        F: Fn(&T, &T) -> bool + 'static,
    {
        if capacity == 0 {
            return Err(HistoryError::InvalidCapacity { capacity });
        }

        let stack =
            CircularStack::new(capacity).map_err(|_| HistoryError::InvalidCapacity { capacity })?;

        Ok(Self {
            name: name.into(),
            equals: Arc::new(equals),
            stack,
            present: -1,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn capacity(&self) -> usize {
        self.stack.capacity()
    }

    pub fn size(&self) -> usize {
        self.stack.size()
    }

    pub fn equals(&self, a: &T, b: &T) -> bool {
        (self.equals)(a, b)
    }

    pub fn top(&self) -> Option<&T> {
        self.stack.top()
    }

    pub fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(&T) -> bool,
    {
        self.stack.count(|x| filter(x))
    }

    pub fn is_bot(&self) -> bool {
        self.stack.is_empty() || self.present == 0
    }

    pub fn is_top(&self) -> bool {
        let size = self.stack.size();
        size == 0 || self.present + 1 == size as isize
    }

    pub fn present(&self) -> (Option<&T>, isize) {
        if self.stack.is_empty() {
            return (None, -1);
        }

        (self.stack.at(self.present), self.present)
    }

    pub fn backward(&mut self) -> (Option<&T>, bool) {
        self.backward_by(1)
    }

    pub fn backward_by(&mut self, step: isize) -> (Option<&T>, bool) {
        let size = self.stack.size();
        if size == 0 {
            return (None, true);
        }

        let high = (size - 1) as isize;
        let next = clamp_i128(self.present as i128 - step as i128, 0, high as i128);
        self.present = next as isize;

        (self.stack.at(self.present), self.present < 1)
    }

    pub fn forward(&mut self) -> (Option<&T>, bool) {
        self.forward_by(1)
    }

    pub fn forward_by(&mut self, step: isize) -> (Option<&T>, bool) {
        let size = self.stack.size();
        if size == 0 {
            return (None, true);
        }

        let high = (size - 1) as isize;
        let next = clamp_i128(self.present as i128 + step as i128, 0, high as i128);
        self.present = next as isize;

        (
            self.stack.at(self.present),
            self.present + 1 == self.stack.size() as isize,
        )
    }

    pub fn go(&mut self, index: isize) -> Option<&T> {
        let size = self.stack.size();
        if size == 0 {
            return None;
        }

        self.present = clamp_i128(index as i128, 0, (size - 1) as i128) as isize;
        self.stack.at(self.present)
    }

    pub fn push(&mut self, element: T) -> &mut Self {
        let present = self.present;

        if let Some(current) = self.stack.at(present)
            && self.equals(&element, current)
        {
            return self;
        }

        if present + 1 < self.stack.size() as isize
            && let Some(next) = self.stack.at(present + 1)
            && self.equals(&element, next)
        {
            self.present += 1;
            return self;
        }

        while present + 1 < self.stack.size() as isize {
            self.stack.pop();
        }

        self.stack.push(element);
        self.present = self.stack.size() as isize - 1;
        self
    }

    pub fn clear(&mut self) {
        let top = self.stack.pop();
        self.stack.clear();

        if let Some(element) = top {
            self.stack.push(element);
            self.present = 0;
        } else {
            self.present = -1;
        }
    }

    pub fn fork(&self, name: impl Into<String>) -> Self
    where
        T: Clone,
    {
        Self {
            name: name.into(),
            equals: Arc::clone(&self.equals),
            stack: self.stack.fork(),
            present: self.present,
        }
    }

    pub fn rearrange<F>(&mut self, mut filter: F) -> &mut Self
    where
        F: FnMut(&T, usize) -> bool,
    {
        let original_present = self.present;
        let mut from_top = Vec::with_capacity(self.stack.size());
        while let Some(element) = self.stack.pop() {
            from_top.push(Some(element));
        }

        self.present = -1;

        let len = from_top.len();
        let filter_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut keep_flags = Vec::with_capacity(len);
            let mut next_present = None;
            let mut kept_count = 0usize;

            for old_idx in 0..len {
                let idx_from_top = len - 1 - old_idx;
                let element = from_top[idx_from_top]
                    .as_ref()
                    .expect("history item must exist");
                let keep = filter(element, old_idx);

                keep_flags.push(keep);
                if keep {
                    if original_present >= 0 && old_idx <= original_present as usize {
                        next_present = Some(kept_count);
                    }
                    kept_count += 1;
                }
            }

            (keep_flags, next_present)
        }));

        let (keep_flags, next_present) = match filter_result {
            Ok(result) => result,
            Err(payload) => {
                for old_idx in 0..len {
                    let idx_from_top = len - 1 - old_idx;
                    if let Some(element) = from_top[idx_from_top].take() {
                        self.stack.push(element);
                    }
                }
                self.present = original_present;
                std::panic::resume_unwind(payload);
            }
        };

        for old_idx in 0..len {
            if !keep_flags[old_idx] {
                continue;
            }

            let idx_from_top = len - 1 - old_idx;
            if let Some(element) = from_top[idx_from_top].take() {
                self.stack.push(element);
            }
        }

        if self.stack.is_empty() {
            self.present = -1;
        } else {
            self.present = next_present.unwrap_or(0) as isize;
        }

        self
    }

    pub fn update_top(&mut self, element: T) {
        let size = self.stack.size();
        if size == 0 {
            return;
        }

        let _ = self.stack.update((size - 1) as isize, element);
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            history: self,
            offset: 0,
        }
    }
}

impl<T> HistoryLike<T> for History<T> {
    fn name(&self) -> &str {
        self.name()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }

    fn size(&self) -> usize {
        self.size()
    }

    fn count<F>(&self, filter: F) -> usize
    where
        F: FnMut(&T) -> bool,
    {
        self.count(filter)
    }

    fn top(&self) -> Option<&T> {
        self.top()
    }

    fn present(&self) -> (Option<&T>, isize) {
        self.present()
    }

    fn is_bot(&self) -> bool {
        self.is_bot()
    }

    fn is_top(&self) -> bool {
        self.is_top()
    }

    fn backward(&mut self) -> (Option<&T>, bool) {
        self.backward()
    }

    fn backward_by(&mut self, step: isize) -> (Option<&T>, bool) {
        self.backward_by(step)
    }

    fn forward(&mut self) -> (Option<&T>, bool) {
        self.forward()
    }

    fn forward_by(&mut self, step: isize) -> (Option<&T>, bool) {
        self.forward_by(step)
    }

    fn go(&mut self, index: isize) -> Option<&T> {
        self.go(index)
    }

    fn push(&mut self, element: T) -> &mut Self {
        self.push(element)
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn rearrange<F>(&mut self, filter: F) -> &mut Self
    where
        F: FnMut(&T, usize) -> bool,
    {
        self.rearrange(filter)
    }

    fn update_top(&mut self, element: T) {
        self.update_top(element)
    }

    fn fork(&self, name: impl Into<String>) -> Self
    where
        Self: Sized,
        T: Clone,
    {
        self.fork(name)
    }
}

pub struct Iter<'a, T> {
    history: &'a History<T>,
    offset: usize,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.history.size() {
            return None;
        }

        let index = (self.history.size() - 1 - self.offset) as isize;
        self.offset += 1;
        self.history.stack.at(index)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.history.size() - self.offset;
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}

impl<'a, T> IntoIterator for &'a History<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

fn clamp_i128(value: i128, low: i128, high: i128) -> i128 {
    value.max(low).min(high)
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::History;

    fn as_vec_top<T: Clone + PartialEq>(history: &History<T>) -> Vec<T> {
        history.iter().cloned().collect()
    }

    fn assert_state(
        history: &History<i32>,
        expected: &[i32],
        expected_present_element: Option<i32>,
        expected_present_index: isize,
    ) {
        assert_eq!(as_vec_top(history), expected.to_vec());
        assert_eq!(history.present().0.copied(), expected_present_element);
        assert_eq!(history.present().1, expected_present_index);
        assert_eq!(history.top().copied(), expected.first().copied());
    }

    #[test]
    fn constructor_should_validate_capacity() {
        assert!(History::<i32>::new("demo", 1).is_ok());
        assert!(matches!(
            History::<i32>::new("demo", 0),
            Err(crate::HistoryError::InvalidCapacity { capacity: 0 })
        ));
    }

    #[test]
    fn backward_forward_and_go_should_support_clamp_and_negative_step() {
        let mut history = History::new("demo", 4).expect("history should be created");

        assert_eq!(history.backward(), (None, true));
        assert_eq!(history.forward(), (None, true));

        history.push(1).push(2).push(3).push(4);
        assert_state(&history, &[4, 3, 2, 1], Some(4), 3);

        assert_eq!(history.backward(), (Some(&3), false));
        assert_eq!(history.backward_by(2), (Some(&1), true));
        assert_eq!(history.backward(), (Some(&1), true));
        assert_state(&history, &[4, 3, 2, 1], Some(1), 0);

        assert_eq!(history.forward(), (Some(&2), false));
        assert_eq!(history.forward_by(10), (Some(&4), true));
        assert_state(&history, &[4, 3, 2, 1], Some(4), 3);

        assert_eq!(history.forward_by(-2), (Some(&2), false));
        assert_eq!(history.backward_by(-1), (Some(&3), false));
        assert_state(&history, &[4, 3, 2, 1], Some(3), 2);

        assert_eq!(history.go(-10), Some(&1));
        assert_eq!(history.forward_by(-10), (Some(&1), false));
        assert_eq!(history.go(100), Some(&4));
        assert_eq!(history.backward_by(-1), (Some(&4), false));
    }

    #[test]
    fn push_should_reuse_history_and_truncate_future() {
        let mut history = History::new("demo", 4).expect("history should be created");

        history.push(1).push(2).push(3);
        assert_state(&history, &[3, 2, 1], Some(3), 2);

        let _ = history.backward();
        let _ = history.backward();
        assert_state(&history, &[3, 2, 1], Some(1), 0);

        history.push(1);
        assert_state(&history, &[3, 2, 1], Some(1), 0);

        history.push(2);
        assert_state(&history, &[3, 2, 1], Some(2), 1);

        history.push(4);
        assert_state(&history, &[4, 2, 1], Some(4), 2);

        history.push(5).push(6);
        assert_state(&history, &[6, 5, 4, 2], Some(6), 3);
    }

    #[test]
    fn clear_should_keep_top_when_exists() {
        let mut history = History::new("demo", 4).expect("history should be created");

        history.clear();
        assert_state(&history, &[], None, -1);

        history.push(1).push(2).push(3);
        history.clear();
        assert_state(&history, &[3], Some(3), 0);

        history.push(4);
        assert_state(&history, &[4, 3], Some(4), 1);
    }

    #[test]
    fn fork_should_clone_snapshot_and_rename() {
        let mut history = History::new("main", 4).expect("history should be created");
        history.push(1).push(2).push(3);
        let _ = history.backward();

        let mut forked = history.fork("forked");
        assert_eq!(forked.name(), "forked");
        assert_state(&forked, &[3, 2, 1], Some(2), 1);

        forked.push(9);
        assert_state(&forked, &[9, 2, 1], Some(9), 2);
        assert_state(&history, &[3, 2, 1], Some(2), 1);
    }

    #[test]
    fn rearrange_should_keep_present_stable_when_possible() {
        let mut history = History::new("demo", 6).expect("history should be created");
        history.push(1).push(2).push(3).push(4).push(5);

        let _ = history.backward_by(2);
        assert_state(&history, &[5, 4, 3, 2, 1], Some(3), 2);

        history.rearrange(|x, _| x % 2 == 1);
        assert_state(&history, &[5, 3, 1], Some(3), 1);

        history.rearrange(|x, _| *x > 3);
        assert_state(&history, &[5], Some(5), 0);

        history.rearrange(|_, _| false);
        assert_state(&history, &[], None, -1);
    }

    #[test]
    fn rearrange_should_keep_present_when_filter_keeps_all() {
        let mut history = History::new("demo", 5).expect("history should be created");
        history.push(1).push(2).push(3).push(4);
        let _ = history.go(1);

        history.rearrange(|_, _| true);
        assert_state(&history, &[4, 3, 2, 1], Some(2), 1);
    }

    #[test]
    fn rearrange_should_restore_state_when_filter_panics() {
        let mut history = History::new("demo", 6).expect("history should be created");
        history.push(1).push(2).push(3).push(4).push(5);
        let _ = history.go(2);

        let result = catch_unwind(AssertUnwindSafe(|| {
            history.rearrange(|x, _| {
                assert!(*x != 3, "boom");
                true
            });
        }));

        assert!(result.is_err());
        assert_state(&history, &[5, 4, 3, 2, 1], Some(3), 2);
    }

    #[test]
    fn update_top_should_only_change_top() {
        let mut history = History::new("demo", 4).expect("history should be created");

        history.update_top(100);
        assert_state(&history, &[], None, -1);

        history.push(1).push(2).push(3);
        history.update_top(9);
        assert_state(&history, &[9, 2, 1], Some(9), 2);
    }

    #[test]
    fn iter_and_count_should_work() {
        let mut history = History::new("demo", 4).expect("history should be created");
        history.push(1).push(2).push(3);

        let from_ref_iter: Vec<i32> = (&history).into_iter().copied().collect();
        assert_eq!(from_ref_iter, vec![3, 2, 1]);
        assert_eq!(history.count(|x| *x % 2 == 1), 2);
    }

    #[test]
    fn with_equals_should_support_custom_compare() {
        let mut history = History::with_equals("demo", 8, |a: &String, b: &String| {
            a.to_lowercase() == b.to_lowercase()
        })
        .expect("history should be created");

        history.push("Hello".to_string());
        history.push("HELLO".to_string());
        history.push("World".to_string());

        let values: Vec<String> = history.iter().cloned().collect();
        assert_eq!(values, vec!["World".to_string(), "Hello".to_string()]);
    }
}
