pub trait Disposable {
    fn dispose(&mut self);
    fn is_disposed(&self) -> bool;
}
