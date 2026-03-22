pub trait HistoryLike<T> {
    fn name(&self) -> &str;
    fn capacity(&self) -> usize;
    fn size(&self) -> usize;

    fn count<F>(&self, filter: F) -> usize
    where
        F: FnMut(&T) -> bool;

    fn top(&self) -> Option<&T>;
    fn present(&self) -> (Option<&T>, isize);

    fn is_bot(&self) -> bool;
    fn is_top(&self) -> bool;

    fn backward(&mut self) -> (Option<&T>, bool);
    fn backward_by(&mut self, step: isize) -> (Option<&T>, bool);
    fn forward(&mut self) -> (Option<&T>, bool);
    fn forward_by(&mut self, step: isize) -> (Option<&T>, bool);
    fn go(&mut self, index: isize) -> Option<&T>;

    fn push(&mut self, element: T) -> &mut Self;
    fn clear(&mut self);

    fn rearrange<F>(&mut self, filter: F) -> &mut Self
    where
        F: FnMut(&T, usize) -> bool;

    fn update_top(&mut self, element: T);

    fn fork(&self, name: impl Into<String>) -> Self
    where
        Self: Sized,
        T: Clone;
}
