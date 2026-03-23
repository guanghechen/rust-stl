use collection::Collection;
use std::cmp::Reverse;
use std::collections::VecDeque;

use queue::{PriorityQueue, QueueLike};

use crate::error::McmfError;
use crate::traits::McmfLike;

pub const DEFAULT_INF: i64 = i64::MAX / 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McmfShortestPathStrategy {
    Spfa,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McmfOptions {
    pub inf: i64,
    pub shortest_path_strategy: McmfShortestPathStrategy,
}

impl Default for McmfOptions {
    fn default() -> Self {
        Self {
            inf: DEFAULT_INF,
            shortest_path_strategy: McmfShortestPathStrategy::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McmfResult {
    pub mincost: i64,
    pub maxflow: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McmfEdge {
    pub from: usize,
    pub to: usize,
    pub cap: i64,
    pub flow: i64,
    pub cost: i64,
}

#[derive(Debug, Clone)]
pub struct Mcmf {
    inf: i64,
    shortest_path_strategy: McmfShortestPathStrategy,
    inq: Vec<bool>,
    dist: Vec<i64>,
    potential: Vec<i64>,
    path: Vec<usize>,
    edges: Vec<McmfEdge>,
    graph: Vec<Vec<usize>>,
    queue: VecDeque<usize>,
    heap: PriorityQueue<(i64, Reverse<usize>)>,
    n: usize,
    source: usize,
    sink: usize,
    maxflow_cache: i64,
    mincost_cache: i64,
    modified_timestamp: u64,
    resolved_timestamp: i64,
}

impl Default for Mcmf {
    fn default() -> Self {
        Self::new()
    }
}

impl Mcmf {
    pub fn new() -> Self {
        Self::with_options(McmfOptions::default()).expect("[Mcmf] invalid default options")
    }

    pub fn with_options(options: McmfOptions) -> Result<Self, McmfError> {
        if options.inf <= 0 {
            return Err(McmfError::InvalidInf { inf: options.inf });
        }

        Ok(Self {
            inf: options.inf,
            shortest_path_strategy: options.shortest_path_strategy,
            inq: Vec::new(),
            dist: Vec::new(),
            potential: Vec::new(),
            path: Vec::new(),
            edges: Vec::new(),
            graph: Vec::new(),
            queue: VecDeque::new(),
            heap: PriorityQueue::new(),
            n: 0,
            source: 0,
            sink: 0,
            maxflow_cache: 0,
            mincost_cache: 0,
            modified_timestamp: 0,
            resolved_timestamp: -1,
        })
    }

    pub fn init(&mut self, source: usize, sink: usize, n: usize) {
        self.try_init(source, sink, n)
            .expect("[Mcmf] init failed due to invalid arguments");
    }

    pub fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), McmfError> {
        if n == 0 {
            return Err(McmfError::InvalidNodeCount { n });
        }
        if source >= n {
            return Err(McmfError::NodeOutOfRange { node: source, n });
        }
        if sink >= n {
            return Err(McmfError::NodeOutOfRange { node: sink, n });
        }
        if source == sink {
            return Err(McmfError::SourceEqualsSink { node: source });
        }

        self.n = n;
        self.source = source;
        self.sink = sink;
        self.maxflow_cache = 0;
        self.mincost_cache = 0;
        self.modified_timestamp = 0;
        self.resolved_timestamp = -1;

        self.inq.clear();
        self.inq.resize(n, false);
        self.dist.clear();
        self.dist.resize(n, self.inf);
        self.potential.clear();
        self.potential.resize(n, 0);
        self.path.clear();
        self.path.resize(n, 0);
        self.edges.clear();
        self.graph.clear();
        self.graph.resize_with(n, Vec::new);
        self.queue = VecDeque::with_capacity(n + 1);
        self.heap = PriorityQueue::new();

        Ok(())
    }

    pub fn add_edge(&mut self, from: usize, to: usize, cap: i64, cost: i64) {
        self.try_add_edge(from, to, cap, cost)
            .expect("[Mcmf] add_edge failed due to invalid arguments");
    }

    pub fn try_add_edge(
        &mut self,
        from: usize,
        to: usize,
        cap: i64,
        cost: i64,
    ) -> Result<(), McmfError> {
        if !self.is_initialized() {
            return Err(McmfError::NotInitialized);
        }
        if from >= self.n {
            return Err(McmfError::NodeOutOfRange {
                node: from,
                n: self.n,
            });
        }
        if to >= self.n {
            return Err(McmfError::NodeOutOfRange {
                node: to,
                n: self.n,
            });
        }
        if cap < 0 {
            return Err(McmfError::NegativeCapacity { cap });
        }

        let edge_id = self.edges.len();
        self.graph[from].push(edge_id);
        self.graph[to].push(edge_id + 1);
        self.edges.push(McmfEdge {
            from,
            to,
            cap,
            flow: 0,
            cost,
        });
        self.edges.push(McmfEdge {
            from: to,
            to: from,
            cap: 0,
            flow: 0,
            cost: -cost,
        });

        self.modified_timestamp = self.modified_timestamp.saturating_add(1);
        Ok(())
    }

    pub fn min_cost_max_flow(&mut self) -> McmfResult {
        if !self.is_initialized() {
            return McmfResult {
                mincost: 0,
                maxflow: 0,
            };
        }

        if self.resolved_timestamp < self.modified_timestamp as i64 {
            let mut maxflow = self.maxflow_cache;
            let mut mincost = self.mincost_cache;

            match self.shortest_path_strategy {
                McmfShortestPathStrategy::Spfa => {
                    while self.bellman_ford() {
                        let (delta_flow, delta_cost) = self.augment_by_path();
                        maxflow += delta_flow;
                        mincost += delta_flow * delta_cost;
                    }
                }
                McmfShortestPathStrategy::Auto => {
                    self.initialize_potential_with_spfa();
                    while self.dijkstra_with_potential() {
                        let (delta_flow, delta_cost) = self.augment_by_path();
                        maxflow += delta_flow;
                        mincost += delta_flow * delta_cost;
                        self.update_potential_from_dist();
                    }
                }
            }

            self.maxflow_cache = maxflow;
            self.mincost_cache = mincost;
            self.resolved_timestamp = self.modified_timestamp as i64;
        }

        McmfResult {
            mincost: self.mincost_cache,
            maxflow: self.maxflow_cache,
        }
    }

    pub fn mincut(&mut self) -> Vec<McmfEdge> {
        self.min_cost_max_flow();

        if !self.is_initialized() {
            return Vec::new();
        }

        self.edges
            .iter()
            .filter(|e| self.dist[e.from] != self.inf && self.dist[e.to] == self.inf && e.cap > 0)
            .cloned()
            .collect()
    }

    fn is_initialized(&self) -> bool {
        self.n > 0 && self.graph.len() == self.n
    }

    fn augment_by_path(&mut self) -> (i64, i64) {
        let mut mif = self.inf;
        let mut o = self.sink;

        while o != self.source {
            let edge_idx = self.path[o];
            let e = &self.edges[edge_idx];
            let remain = e.cap - e.flow;
            if mif > remain {
                mif = remain;
            }
            o = e.from;
        }

        let mut path_cost = 0i64;
        let mut o = self.sink;
        while o != self.source {
            let edge_idx = self.path[o];
            let from = self.edges[edge_idx].from;
            let cost = self.edges[edge_idx].cost;

            self.edges[edge_idx].flow += mif;
            self.edges[edge_idx ^ 1].flow -= mif;
            path_cost += cost;
            o = from;
        }

        (mif, path_cost)
    }

    fn initialize_potential_with_spfa(&mut self) {
        self.potential.fill(0);
        self.bellman_ford();
        for v in 0..self.n {
            if self.dist[v] != self.inf {
                self.potential[v] = self.dist[v];
            }
        }
    }

    fn dijkstra_with_potential(&mut self) -> bool {
        if !self.is_initialized() {
            return false;
        }

        self.dist.fill(self.inf);
        self.heap.clear();
        self.dist[self.source] = 0;
        self.heap.enqueue((0, Reverse(self.source)));

        while let Some((du, Reverse(u))) = self.heap.dequeue() {
            if du != self.dist[u] {
                continue;
            }

            for &edge_idx in &self.graph[u] {
                let e = &self.edges[edge_idx];
                if e.cap == e.flow {
                    continue;
                }

                let reduced_cost =
                    e.cost as i128 + self.potential[u] as i128 - self.potential[e.to] as i128;
                debug_assert!(
                    reduced_cost >= 0,
                    "[Mcmf] reduced cost should be non-negative in Auto mode"
                );
                let reduced_cost = reduced_cost.max(0);

                let candidate = du as i128 + reduced_cost;
                if candidate > i64::MAX as i128 {
                    continue;
                }

                let cand = candidate as i64;
                if self.dist[e.to] > cand {
                    self.dist[e.to] = cand;
                    self.path[e.to] = edge_idx;
                    self.heap.enqueue((cand, Reverse(e.to)));
                }
            }
        }

        self.dist[self.sink] != self.inf
    }

    fn update_potential_from_dist(&mut self) {
        for v in 0..self.n {
            if self.dist[v] == self.inf {
                continue;
            }

            let updated = self.potential[v] as i128 + self.dist[v] as i128;
            self.potential[v] = updated.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        }
    }

    fn bellman_ford(&mut self) -> bool {
        if !self.is_initialized() {
            return false;
        }

        self.dist.fill(self.inf);
        self.inq.fill(false);
        self.queue.clear();

        self.queue.push_back(self.source);
        self.dist[self.source] = 0;
        self.inq[self.source] = true;

        while let Some(u) = self.queue.pop_front() {
            self.inq[u] = false;
            let du = self.dist[u];

            for &edge_idx in &self.graph[u] {
                let e = &self.edges[edge_idx];
                if e.cap == e.flow {
                    continue;
                }

                let candidate_dist = du + e.cost;
                if self.dist[e.to] > candidate_dist {
                    self.dist[e.to] = candidate_dist;
                    self.path[e.to] = edge_idx;

                    if !self.inq[e.to] {
                        self.inq[e.to] = true;
                        self.queue.push_back(e.to);
                    }
                }
            }
        }

        self.dist[self.sink] != self.inf
    }
}

impl McmfLike for Mcmf {
    fn init(&mut self, source: usize, sink: usize, n: usize) {
        Mcmf::init(self, source, sink, n);
    }

    fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), McmfError> {
        Mcmf::try_init(self, source, sink, n)
    }

    fn add_edge(&mut self, from: usize, to: usize, cap: i64, cost: i64) {
        Mcmf::add_edge(self, from, to, cap, cost);
    }

    fn try_add_edge(
        &mut self,
        from: usize,
        to: usize,
        cap: i64,
        cost: i64,
    ) -> Result<(), McmfError> {
        Mcmf::try_add_edge(self, from, to, cap, cost)
    }

    fn min_cost_max_flow(&mut self) -> McmfResult {
        Mcmf::min_cost_max_flow(self)
    }

    fn mincut(&mut self) -> Vec<McmfEdge> {
        Mcmf::mincut(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::McmfLike;

    use super::{Mcmf, McmfEdge, McmfOptions, McmfResult, McmfShortestPathStrategy};
    use crate::McmfError;

    fn normalized_edges(edges: &[McmfEdge]) -> Vec<(usize, usize, i64, i64, i64)> {
        let mut values = edges
            .iter()
            .map(|e| (e.from, e.to, e.cap, e.flow, e.cost))
            .collect::<Vec<_>>();
        values.sort_unstable();
        values
    }

    #[derive(Clone)]
    struct RefEdge {
        to: usize,
        rev: usize,
        cap: i64,
        cost: i64,
    }

    fn reference_mcmf(
        n: usize,
        source: usize,
        sink: usize,
        edges: &[(usize, usize, i64, i64)],
        inf: i64,
    ) -> McmfResult {
        let mut graph = vec![Vec::<RefEdge>::new(); n];

        for &(u, v, cap, cost) in edges {
            let rev_u = graph[v].len();
            let rev_v = graph[u].len();
            graph[u].push(RefEdge {
                to: v,
                rev: rev_u,
                cap,
                cost,
            });
            graph[v].push(RefEdge {
                to: u,
                rev: rev_v,
                cap: 0,
                cost: -cost,
            });
        }

        let mut maxflow = 0i64;
        let mut mincost = 0i64;

        loop {
            let mut dist = vec![inf; n];
            let mut parent_node = vec![usize::MAX; n];
            let mut parent_edge = vec![usize::MAX; n];
            dist[source] = 0;

            for _ in 0..n.saturating_sub(1) {
                let mut updated = false;
                for u in 0..n {
                    if dist[u] == inf {
                        continue;
                    }

                    for (idx, e) in graph[u].iter().enumerate() {
                        if e.cap <= 0 {
                            continue;
                        }

                        let cand = dist[u] + e.cost;
                        if dist[e.to] > cand {
                            dist[e.to] = cand;
                            parent_node[e.to] = u;
                            parent_edge[e.to] = idx;
                            updated = true;
                        }
                    }
                }

                if !updated {
                    break;
                }
            }

            if dist[sink] == inf {
                break;
            }

            let mut mif = inf;
            let mut v = sink;
            while v != source {
                let u = parent_node[v];
                let idx = parent_edge[v];
                mif = mif.min(graph[u][idx].cap);
                v = u;
            }

            let mut v = sink;
            while v != source {
                let u = parent_node[v];
                let idx = parent_edge[v];
                let rev = graph[u][idx].rev;
                graph[u][idx].cap -= mif;
                graph[v][rev].cap += mif;
                v = u;
            }

            maxflow += mif;
            mincost += mif * dist[sink];
        }

        McmfResult { mincost, maxflow }
    }

    #[test]
    fn simple_case_should_match_ts_cost_and_flow() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 1, 4);
        mcmf.add_edge(0, 2, 1, 10);
        mcmf.add_edge(0, 3, 2, 2);
        mcmf.add_edge(2, 1, 1, 9);
        mcmf.add_edge(3, 1, 1, 1);

        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 22,
                maxflow: 2,
            }
        );
        assert_eq!(
            normalized_edges(&mcmf.mincut()),
            vec![(0, 2, 1, 1, 10), (3, 1, 1, 1, 1)]
        );
    }

    #[test]
    fn mincut_should_follow_strict_reachable_partition_definition() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 3, 4);
        mcmf.add_edge(0, 1, 10, 0);
        mcmf.add_edge(1, 3, 1, 5);
        mcmf.add_edge(1, 2, 9, 1);
        mcmf.add_edge(2, 3, 9, 2);

        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 32,
                maxflow: 10,
            }
        );
        assert_eq!(
            mcmf.mincut(),
            vec![McmfEdge {
                from: 0,
                to: 1,
                cap: 10,
                flow: 10,
                cost: 0,
            }]
        );
    }

    #[test]
    fn mincut_should_include_all_cross_edges_of_reachable_partition() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 3, 4);
        mcmf.add_edge(0, 1, 3, 0);
        mcmf.add_edge(0, 2, 2, 0);
        mcmf.add_edge(1, 2, 7, 0);
        mcmf.add_edge(1, 3, 3, 0);
        mcmf.add_edge(2, 3, 2, 0);

        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 0,
                maxflow: 5,
            }
        );
        assert_eq!(
            normalized_edges(&mcmf.mincut()),
            vec![(0, 1, 3, 3, 0), (0, 2, 2, 2, 0)]
        );
    }

    #[test]
    fn mincut_should_ignore_zero_capacity_cross_edges() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 1, 2);
        mcmf.add_edge(0, 1, 0, 7);

        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 0,
                maxflow: 0,
            }
        );
        assert!(mcmf.mincut().is_empty());
    }

    #[test]
    fn min_cost_max_flow_should_be_cached_until_graph_changes() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 3, 4);
        mcmf.add_edge(0, 1, 2, 1);
        mcmf.add_edge(1, 3, 2, 1);

        let first = mcmf.min_cost_max_flow();
        let second = mcmf.min_cost_max_flow();
        assert_eq!(
            first,
            McmfResult {
                mincost: 4,
                maxflow: 2,
            }
        );
        assert_eq!(second, first);

        mcmf.add_edge(0, 2, 3, 2);
        mcmf.add_edge(2, 3, 3, 2);

        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 16,
                maxflow: 5,
            }
        );
    }

    #[test]
    fn min_cost_max_flow_before_init_should_be_zero() {
        let mut mcmf = Mcmf::new();
        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 0,
                maxflow: 0,
            }
        );
        assert!(mcmf.mincut().is_empty());
    }

    #[test]
    fn init_should_reset_graph_and_cache_state() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 2, 3);
        mcmf.add_edge(0, 1, 5, 2);
        mcmf.add_edge(1, 2, 5, 3);
        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 25,
                maxflow: 5,
            }
        );

        mcmf.init(0, 1, 2);
        mcmf.add_edge(0, 1, 3, 4);
        assert_eq!(
            mcmf.min_cost_max_flow(),
            McmfResult {
                mincost: 12,
                maxflow: 3,
            }
        );
    }

    #[test]
    fn checked_api_should_validate_arguments() {
        let mut mcmf = Mcmf::new();

        assert_eq!(
            mcmf.try_init(0, 0, 0),
            Err(McmfError::InvalidNodeCount { n: 0 })
        );
        assert_eq!(
            mcmf.try_init(3, 0, 3),
            Err(McmfError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            mcmf.try_init(0, 3, 3),
            Err(McmfError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            mcmf.try_init(1, 1, 3),
            Err(McmfError::SourceEqualsSink { node: 1 })
        );

        assert_eq!(
            mcmf.try_add_edge(0, 1, 1, 1),
            Err(McmfError::NotInitialized)
        );

        mcmf.try_init(0, 2, 3).expect("init should work");
        assert_eq!(
            mcmf.try_add_edge(3, 1, 1, 1),
            Err(McmfError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            mcmf.try_add_edge(0, 1, -1, 1),
            Err(McmfError::NegativeCapacity { cap: -1 })
        );
    }

    #[test]
    fn with_options_should_validate_inf() {
        assert!(matches!(
            Mcmf::with_options(McmfOptions {
                inf: 0,
                ..McmfOptions::default()
            }),
            Err(McmfError::InvalidInf { inf: 0 })
        ));
        assert!(matches!(
            Mcmf::with_options(McmfOptions {
                inf: -7,
                ..McmfOptions::default()
            }),
            Err(McmfError::InvalidInf { inf: -7 })
        ));
        assert!(
            Mcmf::with_options(McmfOptions {
                inf: 123,
                ..McmfOptions::default()
            })
            .is_ok()
        );
    }

    #[test]
    #[should_panic(expected = "[Mcmf] init failed due to invalid arguments")]
    fn init_should_panic_on_invalid_arguments() {
        let mut mcmf = Mcmf::new();
        mcmf.init(0, 0, 0);
    }

    #[test]
    #[should_panic(expected = "[Mcmf] add_edge failed due to invalid arguments")]
    fn add_edge_should_panic_when_not_initialized() {
        let mut mcmf = Mcmf::new();
        mcmf.add_edge(0, 1, 1, 0);
    }

    #[test]
    fn min_cost_max_flow_should_match_reference_on_small_graphs() {
        let mut seed: u64 = 0x1234_5678_9abc_def0;

        fn next_u32(seed: &mut u64) -> u32 {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }

        let inf = 1_000_000_000i64;

        for n in 2..=8usize {
            let source = 0usize;
            let sink = n - 1;

            for _case_idx in 0..80 {
                let mut edges = Vec::<(usize, usize, i64, i64)>::new();
                let mut mcmf = Mcmf::with_options(McmfOptions {
                    inf,
                    ..McmfOptions::default()
                })
                .expect("valid inf");
                mcmf.init(source, sink, n);

                for u in 0..n {
                    for v in 0..n {
                        if u == v {
                            continue;
                        }
                        if next_u32(&mut seed) % 100 < 35 {
                            let cap = (next_u32(&mut seed) % 16) as i64;
                            let cost = (next_u32(&mut seed) % 9) as i64;
                            mcmf.add_edge(u, v, cap, cost);
                            edges.push((u, v, cap, cost));
                        }
                    }
                }

                let got = mcmf.min_cost_max_flow();
                let expected = reference_mcmf(n, source, sink, &edges, inf);
                assert_eq!(
                    got, expected,
                    "mcmf mismatch on n={n}, expected={expected:?}, got={got:?}"
                );
            }
        }
    }

    #[test]
    fn mincut_capacity_should_equal_maxflow_on_small_graphs() {
        let mut seed: u64 = 0x0ddc_0ffe_e123_4567;

        fn next_u32(seed: &mut u64) -> u32 {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }

        for n in 2..=8usize {
            let source = 0usize;
            let sink = n - 1;

            for _case_idx in 0..80 {
                let mut mcmf = Mcmf::new();
                mcmf.init(source, sink, n);

                for u in 0..n {
                    for v in 0..n {
                        if u == v {
                            continue;
                        }
                        if next_u32(&mut seed) % 100 < 35 {
                            let cap = (next_u32(&mut seed) % 16) as i64;
                            let cost = (next_u32(&mut seed) % 9) as i64;
                            mcmf.add_edge(u, v, cap, cost);
                        }
                    }
                }

                let result = mcmf.min_cost_max_flow();
                let cut_cap: i64 = mcmf.mincut().iter().map(|e| e.cap).sum();
                assert_eq!(
                    cut_cap, result.maxflow,
                    "cut capacity mismatch on n={n}, flow={}, cut={cut_cap}",
                    result.maxflow
                );
            }
        }
    }

    #[test]
    fn trait_object_call_should_work() {
        let mut mcmf = Mcmf::new();
        let d: &mut dyn McmfLike = &mut mcmf;

        d.init(0, 2, 3);
        d.add_edge(0, 1, 2, 1);
        d.add_edge(1, 2, 2, 1);

        assert_eq!(
            d.min_cost_max_flow(),
            McmfResult {
                mincost: 4,
                maxflow: 2,
            }
        );
    }

    #[test]
    fn options_default_should_use_auto_strategy() {
        assert_eq!(
            McmfOptions::default().shortest_path_strategy,
            McmfShortestPathStrategy::Auto
        );
    }

    #[test]
    fn auto_and_spfa_should_match_on_small_graphs() {
        let mut seed: u64 = 0x2ddc_0ffe_e123_4567;

        fn next_u32(seed: &mut u64) -> u32 {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }

        for n in 2..=7usize {
            let source = 0usize;
            let sink = n - 1;

            for _case_idx in 0..40 {
                let mut edges = Vec::<(usize, usize, i64, i64)>::new();
                for u in 0..n {
                    for v in 0..n {
                        if u == v {
                            continue;
                        }
                        if next_u32(&mut seed) % 100 < 35 {
                            let cap = (next_u32(&mut seed) % 16) as i64;
                            let cost = (next_u32(&mut seed) % 9) as i64;
                            edges.push((u, v, cap, cost));
                        }
                    }
                }

                let mut auto = Mcmf::with_options(McmfOptions {
                    shortest_path_strategy: McmfShortestPathStrategy::Auto,
                    ..McmfOptions::default()
                })
                .expect("valid options");
                auto.init(source, sink, n);

                let mut spfa = Mcmf::with_options(McmfOptions {
                    shortest_path_strategy: McmfShortestPathStrategy::Spfa,
                    ..McmfOptions::default()
                })
                .expect("valid options");
                spfa.init(source, sink, n);

                for &(u, v, cap, cost) in &edges {
                    auto.add_edge(u, v, cap, cost);
                    spfa.add_edge(u, v, cap, cost);
                }

                let auto_result = auto.min_cost_max_flow();
                let spfa_result = spfa.min_cost_max_flow();
                assert_eq!(
                    auto_result, spfa_result,
                    "strategy mismatch on n={n}, auto={auto_result:?}, spfa={spfa_result:?}"
                );
            }
        }
    }
}
