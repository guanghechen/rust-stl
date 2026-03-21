use crate::dispose::Disposable;

pub trait Collection: Disposable {
    type Item;
    type Iter<'a>: Iterator<Item = &'a Self::Item>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_>;
    fn size(&self) -> usize;
    fn clear(&mut self);
    fn retain<F>(&mut self, f: F) -> usize
    where
        F: FnMut(&Self::Item) -> bool;

    fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(&Self::Item) -> bool,
    {
        let mut count = 0usize;
        for item in self.iter() {
            if filter(item) {
                count += 1;
            }
        }
        count
    }

    fn collect_into<C>(&self, out: &mut C)
    where
        C: Extend<Self::Item>,
        Self::Item: Clone,
    {
        out.extend(self.iter().cloned());
    }

    fn collect(&self) -> Vec<Self::Item>
    where
        Self::Item: Clone,
    {
        let mut out = Vec::with_capacity(self.size());
        self.collect_into(&mut out);
        out
    }

    fn is_empty(&self) -> bool {
        self.size() == 0
    }
}
