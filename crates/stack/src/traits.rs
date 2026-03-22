use collection::Collection;

pub trait StackLike<T>: Collection<Item = T> {
    fn top(&self) -> Option<&T>;
    fn pop(&mut self) -> Option<T>;
    fn push(&mut self, element: T);

    fn fork(&self) -> Self
    where
        Self: Sized + Clone,
    {
        self.clone()
    }

    fn pushes<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = T>,
    {
        for element in elements {
            self.push(element);
        }
    }

    fn replace_top(&mut self, new_top: T) -> Option<T> {
        let removed = self.pop();
        self.push(new_top);
        removed
    }
}

pub trait CircularStackLike<T>: StackLike<T> {
    type Error;

    fn capacity(&self) -> usize;
    fn at(&self, index: isize) -> Option<&T>;
    fn update(&mut self, index: isize, element: T) -> bool;
    fn resize(&mut self, new_capacity: usize) -> Result<(), Self::Error>;
    fn rearrange(&mut self);
}

#[cfg(test)]
mod tests {
    use collection::{Collection, Disposable};

    use super::StackLike;

    #[derive(Default, Clone)]
    struct TestStack {
        disposed: bool,
        values: Vec<i32>,
    }

    impl Disposable for TestStack {
        fn dispose(&mut self) {
            self.disposed = true;
            self.values.clear();
        }

        fn is_disposed(&self) -> bool {
            self.disposed
        }
    }

    impl Collection for TestStack {
        type Item = i32;
        type Iter<'a>
            = std::slice::Iter<'a, i32>
        where
            Self: 'a;

        fn iter(&self) -> Self::Iter<'_> {
            self.values.iter()
        }

        fn size(&self) -> usize {
            self.values.len()
        }

        fn clear(&mut self) {
            self.values.clear();
        }

        fn retain<F>(&mut self, mut f: F) -> usize
        where
            F: FnMut(&Self::Item) -> bool,
        {
            let before = self.values.len();
            self.values.retain(|x| f(x));
            before - self.values.len()
        }
    }

    impl StackLike<i32> for TestStack {
        fn top(&self) -> Option<&i32> {
            self.values.last()
        }

        fn push(&mut self, element: i32) {
            self.values.push(element);
        }

        fn pop(&mut self) -> Option<i32> {
            self.values.pop()
        }
    }

    #[test]
    fn default_pushes_should_push_elements_in_order() {
        let mut s = TestStack::default();

        s.pushes([1, 2, 3]);
        assert_eq!(s.values, vec![1, 2, 3]);
        assert_eq!(s.top(), Some(&3));
    }

    #[test]
    fn default_replace_top_should_work_for_empty_and_non_empty_stack() {
        let mut s = TestStack::default();

        assert_eq!(s.replace_top(10), None);
        assert_eq!(s.values, vec![10]);
        assert_eq!(s.top(), Some(&10));

        assert_eq!(s.replace_top(20), Some(10));
        assert_eq!(s.values, vec![20]);
        assert_eq!(s.top(), Some(&20));
    }

    #[test]
    fn default_fork_should_clone_stack_snapshot() {
        let mut s = TestStack::default();
        s.pushes([1, 2, 3]);

        let forked = StackLike::fork(&s);
        assert_eq!(forked.values, vec![1, 2, 3]);
        assert_eq!(forked.top(), Some(&3));
    }
}
