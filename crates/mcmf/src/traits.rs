use crate::{McmfEdge, McmfError, McmfResult};

pub trait McmfLike {
    fn init(&mut self, source: usize, sink: usize, n: usize);
    fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), McmfError>;

    fn add_edge(&mut self, from: usize, to: usize, cap: i64, cost: i64);
    fn try_add_edge(
        &mut self,
        from: usize,
        to: usize,
        cap: i64,
        cost: i64,
    ) -> Result<(), McmfError>;

    fn min_cost_max_flow(&mut self) -> McmfResult;
    fn mincut(&mut self) -> Vec<McmfEdge>;
}
