use crate::{IsapEdge, IsapError};

pub trait IsapLike {
    fn init(&mut self, source: usize, sink: usize, n: usize);
    fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), IsapError>;

    fn add_edge(&mut self, from: usize, to: usize, cap: i64);
    fn try_add_edge(&mut self, from: usize, to: usize, cap: i64) -> Result<(), IsapError>;

    fn maxflow(&mut self) -> i64;
    fn mincut(&mut self) -> Vec<IsapEdge>;
}
