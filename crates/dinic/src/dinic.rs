use std::collections::VecDeque;

use crate::error::DinicError;
use crate::traits::DinicLike;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DinicEdge {
    pub from: usize,
    pub to: usize,
    pub cap: i64,
    pub flow: i64,
}

#[derive(Debug, Default, Clone)]
pub struct Dinic {
    cur: Vec<usize>,
    dist: Vec<isize>,
    edges: Vec<DinicEdge>,
    graph: Vec<Vec<usize>>,
    queue: VecDeque<usize>,
    n: usize,
    source: usize,
    sink: usize,
    maxflow_cache: i64,
    modified_timestamp: u64,
    resolved_timestamp: i64,
}

impl Dinic {
    pub fn new() -> Self {
        Self {
            cur: Vec::new(),
            dist: Vec::new(),
            edges: Vec::new(),
            graph: Vec::new(),
            queue: VecDeque::new(),
            n: 0,
            source: 0,
            sink: 0,
            maxflow_cache: 0,
            modified_timestamp: 0,
            resolved_timestamp: -1,
        }
    }

    pub fn init(&mut self, source: usize, sink: usize, n: usize) {
        self.try_init(source, sink, n)
            .expect("[Dinic] init failed due to invalid arguments");
    }

    pub fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), DinicError> {
        if n == 0 {
            return Err(DinicError::InvalidNodeCount { n });
        }
        if source >= n {
            return Err(DinicError::NodeOutOfRange { node: source, n });
        }
        if sink >= n {
            return Err(DinicError::NodeOutOfRange { node: sink, n });
        }
        if source == sink {
            return Err(DinicError::SourceEqualsSink { node: source });
        }

        self.n = n;
        self.source = source;
        self.sink = sink;
        self.maxflow_cache = 0;
        self.modified_timestamp = 0;
        self.resolved_timestamp = -1;

        self.cur.clear();
        self.cur.resize(n, 0);
        self.dist.clear();
        self.dist.resize(n, -1);
        self.edges.clear();
        self.graph.clear();
        self.graph.resize_with(n, Vec::new);
        self.queue = VecDeque::with_capacity(n + 1);

        Ok(())
    }

    pub fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        self.try_add_edge(from, to, cap)
            .expect("[Dinic] add_edge failed due to invalid arguments");
    }

    pub fn try_add_edge(&mut self, from: usize, to: usize, cap: i64) -> Result<(), DinicError> {
        if !self.is_initialized() {
            return Err(DinicError::NotInitialized);
        }
        if from >= self.n {
            return Err(DinicError::NodeOutOfRange {
                node: from,
                n: self.n,
            });
        }
        if to >= self.n {
            return Err(DinicError::NodeOutOfRange {
                node: to,
                n: self.n,
            });
        }
        if cap < 0 {
            return Err(DinicError::NegativeCapacity { cap });
        }

        let edge_id = self.edges.len();
        self.graph[from].push(edge_id);
        self.graph[to].push(edge_id + 1);
        self.edges.push(DinicEdge {
            from,
            to,
            cap,
            flow: 0,
        });
        self.edges.push(DinicEdge {
            from: to,
            to: from,
            cap: 0,
            flow: 0,
        });

        self.modified_timestamp = self.modified_timestamp.saturating_add(1);
        Ok(())
    }

    pub fn maxflow(&mut self) -> i64 {
        if !self.is_initialized() {
            return 0;
        }

        if self.resolved_timestamp < self.modified_timestamp as i64 {
            let mut maxflow = self.maxflow_cache;
            while self.bfs() {
                self.cur.fill(0);
                maxflow += self.dfs(self.source, i64::MAX / 4);
            }
            self.maxflow_cache = maxflow;
            self.resolved_timestamp = self.modified_timestamp as i64;
        }

        self.maxflow_cache
    }

    pub fn mincut(&mut self) -> Vec<DinicEdge> {
        self.maxflow();

        if !self.is_initialized() {
            return Vec::new();
        }

        self.edges
            .iter()
            .filter(|e| self.dist[e.from] != -1 && self.dist[e.to] == -1 && e.cap > 0)
            .cloned()
            .collect()
    }

    fn is_initialized(&self) -> bool {
        self.n > 0 && self.graph.len() == self.n
    }

    fn bfs(&mut self) -> bool {
        self.dist.fill(-1);
        self.queue.clear();

        self.queue.push_back(self.source);
        self.dist[self.source] = 0;

        while let Some(u) = self.queue.pop_front() {
            for &edge_idx in &self.graph[u] {
                let e = &self.edges[edge_idx];
                if self.dist[e.to] == -1 && e.cap > e.flow {
                    self.dist[e.to] = self.dist[u] + 1;
                    self.queue.push_back(e.to);
                }
            }
        }

        self.dist[self.sink] != -1
    }

    fn dfs(&mut self, u: usize, mut inflow: i64) -> i64 {
        if u == self.sink || inflow == 0 {
            return inflow;
        }

        let mut pushed = 0i64;
        while self.cur[u] < self.graph[u].len() {
            let edge_idx = self.graph[u][self.cur[u]];
            let (to, remain, level_ok) = {
                let e = &self.edges[edge_idx];
                (
                    e.to,
                    e.cap - e.flow,
                    self.dist[e.to] == self.dist[u] + 1 && e.cap > e.flow,
                )
            };

            if level_ok {
                let f = self.dfs(to, inflow.min(remain));
                if f > 0 {
                    self.edges[edge_idx].flow += f;
                    self.edges[edge_idx ^ 1].flow -= f;
                    pushed += f;
                    inflow -= f;
                    if inflow == 0 {
                        break;
                    }
                    continue;
                }
            }

            self.cur[u] += 1;
        }

        pushed
    }
}

impl DinicLike for Dinic {
    fn init(&mut self, source: usize, sink: usize, n: usize) {
        Dinic::init(self, source, sink, n);
    }

    fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), DinicError> {
        Dinic::try_init(self, source, sink, n)
    }

    fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        Dinic::add_edge(self, from, to, cap);
    }

    fn try_add_edge(&mut self, from: usize, to: usize, cap: i64) -> Result<(), DinicError> {
        Dinic::try_add_edge(self, from, to, cap)
    }

    fn maxflow(&mut self) -> i64 {
        Dinic::maxflow(self)
    }

    fn mincut(&mut self) -> Vec<DinicEdge> {
        Dinic::mincut(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::DinicLike;

    use super::{Dinic, DinicEdge};
    use crate::DinicError;

    fn normalized_edges(edges: &[DinicEdge]) -> Vec<(usize, usize, i64, i64)> {
        let mut values = edges
            .iter()
            .map(|e| (e.from, e.to, e.cap, e.flow))
            .collect::<Vec<_>>();
        values.sort_unstable();
        values
    }

    #[test]
    fn simple_case_should_match_ts_behavior() {
        let mut dinic = Dinic::new();
        dinic.init(0, 1, 4);
        dinic.add_edge(0, 2, 1);
        dinic.add_edge(0, 3, 2);
        dinic.add_edge(3, 1, 1);

        assert_eq!(dinic.maxflow(), 1);
        assert_eq!(
            dinic.mincut(),
            vec![DinicEdge {
                from: 3,
                to: 1,
                cap: 1,
                flow: 1,
            }]
        );
    }

    #[test]
    fn mincut_should_follow_strict_reachable_partition_definition() {
        // Graph:
        // s=0 -> a=1 (10)
        // a=1 -> t=3 (1)
        // a=1 -> b=2 (9)
        // b=2 -> t=3 (9)
        //
        // After maxflow=10, saturated forward edges are:
        //   (0,1), (1,3), (1,2), (2,3)
        // But strict min-cut edges are only edges crossing S->T where
        // S is reachable-from-source in residual graph. Here S={0}, so mincut={(0,1)}.
        let mut dinic = Dinic::new();
        dinic.init(0, 3, 4);
        dinic.add_edge(0, 1, 10);
        dinic.add_edge(1, 3, 1);
        dinic.add_edge(1, 2, 9);
        dinic.add_edge(2, 3, 9);

        assert_eq!(dinic.maxflow(), 10);
        assert_eq!(
            dinic.mincut(),
            vec![DinicEdge {
                from: 0,
                to: 1,
                cap: 10,
                flow: 10,
            }]
        );
    }

    #[test]
    fn mincut_should_include_all_cross_edges_of_reachable_partition() {
        let mut dinic = Dinic::new();
        dinic.init(0, 3, 4);
        dinic.add_edge(0, 1, 3);
        dinic.add_edge(0, 2, 2);
        dinic.add_edge(1, 2, 7);
        dinic.add_edge(1, 3, 3);
        dinic.add_edge(2, 3, 2);

        assert_eq!(dinic.maxflow(), 5);
        let cut = dinic.mincut();
        assert_eq!(
            normalized_edges(&cut),
            vec![(0, 1, 3, 3), (0, 2, 2, 2)],
            "strict mincut should return all S->T cross edges"
        );
    }

    #[test]
    fn mincut_should_ignore_zero_capacity_cross_edges() {
        let mut dinic = Dinic::new();
        dinic.init(0, 1, 2);
        dinic.add_edge(0, 1, 0);

        assert_eq!(dinic.maxflow(), 0);
        assert!(dinic.mincut().is_empty());
    }

    #[test]
    fn mincut_should_be_stable_across_cached_maxflow_calls() {
        let mut dinic = Dinic::new();
        dinic.init(0, 3, 4);
        dinic.add_edge(0, 1, 3);
        dinic.add_edge(0, 2, 2);
        dinic.add_edge(1, 3, 3);
        dinic.add_edge(2, 3, 2);

        let first = dinic.mincut();
        let second = dinic.mincut();
        assert_eq!(normalized_edges(&first), normalized_edges(&second));
    }

    #[test]
    fn mincut_before_init_should_return_empty() {
        let mut dinic = Dinic::new();
        assert_eq!(dinic.mincut(), Vec::<DinicEdge>::new());
    }

    #[test]
    fn maxflow_should_be_cached_until_graph_changes() {
        let mut dinic = Dinic::new();
        dinic.init(0, 3, 4);
        dinic.add_edge(0, 1, 2);
        dinic.add_edge(1, 3, 2);

        let first = dinic.maxflow();
        let second = dinic.maxflow();
        assert_eq!(first, 2);
        assert_eq!(second, 2);

        dinic.add_edge(0, 2, 3);
        dinic.add_edge(2, 3, 3);

        assert_eq!(dinic.maxflow(), 5);
    }

    #[test]
    fn parallel_edges_and_zero_capacity_should_work() {
        let mut dinic = Dinic::new();
        dinic.init(0, 2, 3);
        dinic.add_edge(0, 1, 2);
        dinic.add_edge(0, 1, 3);
        dinic.add_edge(1, 2, 4);
        dinic.add_edge(1, 2, 0);

        assert_eq!(dinic.maxflow(), 4);
    }

    #[test]
    fn unreachable_sink_should_return_zero_flow_and_empty_cut() {
        let mut dinic = Dinic::new();
        dinic.init(0, 3, 4);
        dinic.add_edge(0, 1, 10);
        dinic.add_edge(1, 2, 5);

        assert_eq!(dinic.maxflow(), 0);
        assert_eq!(dinic.mincut(), Vec::<DinicEdge>::new());
    }

    #[test]
    fn init_should_reset_graph_and_flow_state() {
        let mut dinic = Dinic::new();
        dinic.init(0, 2, 3);
        dinic.add_edge(0, 1, 5);
        dinic.add_edge(1, 2, 5);
        assert_eq!(dinic.maxflow(), 5);

        dinic.init(0, 1, 2);
        dinic.add_edge(0, 1, 3);
        assert_eq!(dinic.maxflow(), 3);
    }

    #[test]
    fn checked_api_should_validate_arguments() {
        let mut dinic = Dinic::new();

        assert_eq!(
            dinic.try_init(0, 0, 0),
            Err(DinicError::InvalidNodeCount { n: 0 })
        );
        assert_eq!(
            dinic.try_init(3, 0, 3),
            Err(DinicError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            dinic.try_init(0, 3, 3),
            Err(DinicError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            dinic.try_init(1, 1, 3),
            Err(DinicError::SourceEqualsSink { node: 1 })
        );

        assert_eq!(dinic.try_add_edge(0, 1, 1), Err(DinicError::NotInitialized));

        dinic.try_init(0, 2, 3).expect("init should work");
        assert_eq!(
            dinic.try_add_edge(3, 1, 1),
            Err(DinicError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            dinic.try_add_edge(0, 1, -1),
            Err(DinicError::NegativeCapacity { cap: -1 })
        );
    }

    #[test]
    #[should_panic(expected = "[Dinic] init failed due to invalid arguments")]
    fn init_should_panic_on_invalid_arguments() {
        let mut dinic = Dinic::new();
        dinic.init(0, 0, 0);
    }

    #[test]
    #[should_panic(expected = "[Dinic] init failed due to invalid arguments")]
    fn init_should_panic_when_source_equals_sink() {
        let mut dinic = Dinic::new();
        dinic.init(1, 1, 3);
    }

    #[test]
    #[should_panic(expected = "[Dinic] add_edge failed due to invalid arguments")]
    fn add_edge_should_panic_when_not_initialized() {
        let mut dinic = Dinic::new();
        dinic.add_edge(0, 1, 1);
    }

    #[test]
    fn maxflow_should_match_edmonds_karp_on_small_graphs() {
        // Deterministic LCG to avoid introducing an extra rand dependency.
        let mut seed: u64 = 0x1234_5678_9abc_def0;

        fn next_u32(seed: &mut u64) -> u32 {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*seed >> 32) as u32
        }

        fn edmonds_karp(cap: &[Vec<i64>], source: usize, sink: usize) -> i64 {
            let n = cap.len();
            let mut residual = cap.to_vec();
            let mut maxflow = 0i64;

            loop {
                let mut parent = vec![usize::MAX; n];
                let mut q = VecDeque::with_capacity(n);
                q.push_back(source);
                parent[source] = source;

                while let Some(u) = q.pop_front() {
                    if u == sink {
                        break;
                    }
                    for (v, &c) in residual[u].iter().enumerate() {
                        if parent[v] == usize::MAX && c > 0 {
                            parent[v] = u;
                            q.push_back(v);
                        }
                    }
                }

                if parent[sink] == usize::MAX {
                    break;
                }

                let mut aug = i64::MAX;
                let mut v = sink;
                while v != source {
                    let u = parent[v];
                    aug = aug.min(residual[u][v]);
                    v = u;
                }

                let mut v = sink;
                while v != source {
                    let u = parent[v];
                    residual[u][v] -= aug;
                    residual[v][u] += aug;
                    v = u;
                }

                maxflow += aug;
            }

            maxflow
        }

        for n in 2..=8usize {
            let source = 0usize;
            let sink = n - 1;
            for _case_idx in 0..80 {
                let mut dinic = Dinic::new();
                dinic.init(source, sink, n);

                let mut cap = vec![vec![0i64; n]; n];
                for (u, row) in cap.iter_mut().enumerate() {
                    for (v, cell) in row.iter_mut().enumerate() {
                        if u == v {
                            continue;
                        }
                        // ~35% chance to create an edge with capacity in [0, 15].
                        let roll = next_u32(&mut seed) % 100;
                        if roll < 35 {
                            let c = (next_u32(&mut seed) % 16) as i64;
                            dinic.add_edge(u, v, c);
                            *cell += c;
                        }
                    }
                }

                let got = dinic.maxflow();
                let expected = edmonds_karp(&cap, source, sink);
                assert_eq!(
                    got, expected,
                    "maxflow mismatch on n={n}, expected={expected}, got={got}"
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
                let mut dinic = Dinic::new();
                dinic.init(source, sink, n);

                for u in 0..n {
                    for v in 0..n {
                        if u == v {
                            continue;
                        }
                        if next_u32(&mut seed) % 100 < 35 {
                            let cap = (next_u32(&mut seed) % 16) as i64;
                            dinic.add_edge(u, v, cap);
                        }
                    }
                }

                let flow = dinic.maxflow();
                let cut_cap: i64 = dinic.mincut().iter().map(|e| e.cap).sum();
                assert_eq!(
                    cut_cap, flow,
                    "cut capacity mismatch on n={n}, expected flow={flow}, got cut={cut_cap}"
                );
            }
        }
    }

    #[test]
    fn trait_object_call_should_work() {
        let mut dinic = Dinic::new();
        let d: &mut dyn DinicLike = &mut dinic;

        d.init(0, 2, 3);
        d.add_edge(0, 1, 2);
        d.add_edge(1, 2, 2);
        assert_eq!(d.maxflow(), 2);
    }
}
