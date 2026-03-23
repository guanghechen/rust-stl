use std::collections::VecDeque;

use crate::error::IsapError;
use crate::traits::IsapLike;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsapEdge {
    pub from: usize,
    pub to: usize,
    pub cap: i64,
    pub flow: i64,
}

#[derive(Debug, Default, Clone)]
pub struct Isap {
    cur: Vec<usize>,
    cnt: Vec<usize>,
    dist: Vec<usize>,
    path: Vec<usize>,
    edges: Vec<IsapEdge>,
    graph: Vec<Vec<usize>>,
    queue: VecDeque<usize>,
    n: usize,
    source: usize,
    sink: usize,
    maxflow_cache: i64,
    modified_timestamp: u64,
    resolved_timestamp: i64,
}

impl Isap {
    pub fn new() -> Self {
        Self {
            cur: Vec::new(),
            cnt: Vec::new(),
            dist: Vec::new(),
            path: Vec::new(),
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
            .expect("[Isap] init failed due to invalid arguments");
    }

    pub fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), IsapError> {
        if n == 0 {
            return Err(IsapError::InvalidNodeCount { n });
        }
        if source >= n {
            return Err(IsapError::NodeOutOfRange { node: source, n });
        }
        if sink >= n {
            return Err(IsapError::NodeOutOfRange { node: sink, n });
        }
        if source == sink {
            return Err(IsapError::SourceEqualsSink { node: source });
        }

        self.n = n;
        self.source = source;
        self.sink = sink;
        self.maxflow_cache = 0;
        self.modified_timestamp = 0;
        self.resolved_timestamp = -1;

        self.cur.clear();
        self.cur.resize(n, 0);
        self.cnt.clear();
        self.cnt.resize(n + 1, 0);
        self.dist.clear();
        self.dist.resize(n, n);
        self.path.clear();
        self.path.resize(n, 0);
        self.edges.clear();
        self.graph.clear();
        self.graph.resize_with(n, Vec::new);
        self.queue = VecDeque::with_capacity(n + 1);

        Ok(())
    }

    pub fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        self.try_add_edge(from, to, cap)
            .expect("[Isap] add_edge failed due to invalid arguments");
    }

    pub fn try_add_edge(&mut self, from: usize, to: usize, cap: i64) -> Result<(), IsapError> {
        if !self.is_initialized() {
            return Err(IsapError::NotInitialized);
        }
        if from >= self.n {
            return Err(IsapError::NodeOutOfRange {
                node: from,
                n: self.n,
            });
        }
        if to >= self.n {
            return Err(IsapError::NodeOutOfRange {
                node: to,
                n: self.n,
            });
        }
        if cap < 0 {
            return Err(IsapError::NegativeCapacity { cap });
        }

        let edge_id = self.edges.len();
        self.graph[from].push(edge_id);
        self.graph[to].push(edge_id + 1);
        self.edges.push(IsapEdge {
            from,
            to,
            cap,
            flow: 0,
        });
        self.edges.push(IsapEdge {
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

            self.cur.fill(0);
            self.cnt.fill(0);
            self.path.fill(0);

            self.bfs_from_sink();
            for &d in &self.dist {
                if d < self.n {
                    self.cnt[d] += 1;
                }
            }

            let mut u = self.source;
            while self.dist[self.source] < self.n {
                if u == self.sink {
                    maxflow += self.augment();
                    u = self.source;
                    continue;
                }

                let mut advanced = false;
                while self.cur[u] < self.graph[u].len() {
                    let edge_idx = self.graph[u][self.cur[u]];
                    let (to, residual, relabel_ok) = {
                        let e = &self.edges[edge_idx];
                        (
                            e.to,
                            e.cap - e.flow,
                            self.dist[e.to] < self.n && self.dist[u] == self.dist[e.to] + 1,
                        )
                    };

                    if residual > 0 && relabel_ok {
                        advanced = true;
                        self.path[to] = edge_idx;
                        u = to;
                        break;
                    }

                    self.cur[u] += 1;
                }

                if advanced {
                    continue;
                }

                let old_dist = self.dist[u];
                let mut min_dist = self.n.saturating_sub(1);
                for &edge_idx in &self.graph[u] {
                    let e = &self.edges[edge_idx];
                    if e.cap > e.flow {
                        min_dist = min_dist.min(self.dist[e.to]);
                    }
                }

                self.cnt[old_dist] = self.cnt[old_dist].saturating_sub(1);
                if self.cnt[old_dist] == 0 {
                    break;
                }

                self.dist[u] = min_dist.saturating_add(1).min(self.n);
                self.cnt[self.dist[u]] += 1;
                self.cur[u] = 0;

                if u != self.source {
                    u = self.edges[self.path[u]].from;
                }
            }

            self.maxflow_cache = maxflow;
            self.resolved_timestamp = self.modified_timestamp as i64;
        }

        self.maxflow_cache
    }

    pub fn mincut(&mut self) -> Vec<IsapEdge> {
        self.maxflow();

        if !self.is_initialized() {
            return Vec::new();
        }

        let mut reachable = vec![false; self.n];
        self.queue.clear();
        self.queue.push_back(self.source);
        reachable[self.source] = true;

        while let Some(u) = self.queue.pop_front() {
            for &edge_idx in &self.graph[u] {
                let e = &self.edges[edge_idx];
                if !reachable[e.to] && e.cap > e.flow {
                    reachable[e.to] = true;
                    self.queue.push_back(e.to);
                }
            }
        }

        self.edges
            .iter()
            .filter(|e| reachable[e.from] && !reachable[e.to] && e.cap > 0)
            .cloned()
            .collect()
    }

    fn is_initialized(&self) -> bool {
        self.n > 0 && self.graph.len() == self.n
    }

    fn bfs_from_sink(&mut self) {
        self.dist.fill(self.n);
        self.queue.clear();

        self.dist[self.sink] = 0;
        self.queue.push_back(self.sink);

        while let Some(u) = self.queue.pop_front() {
            for &edge_idx in &self.graph[u] {
                let v = self.edges[edge_idx].to;
                let rev = edge_idx ^ 1;
                if self.dist[v] == self.n && self.edges[rev].cap > self.edges[rev].flow {
                    self.dist[v] = self.dist[u] + 1;
                    self.queue.push_back(v);
                }
            }
        }
    }

    fn augment(&mut self) -> i64 {
        let mut delta = i64::MAX / 4;
        let mut v = self.sink;
        while v != self.source {
            let edge_idx = self.path[v];
            let e = &self.edges[edge_idx];
            delta = delta.min(e.cap - e.flow);
            v = e.from;
        }

        let mut v = self.sink;
        while v != self.source {
            let edge_idx = self.path[v];
            self.edges[edge_idx].flow += delta;
            self.edges[edge_idx ^ 1].flow -= delta;
            v = self.edges[edge_idx].from;
        }

        delta
    }
}

impl IsapLike for Isap {
    fn init(&mut self, source: usize, sink: usize, n: usize) {
        Isap::init(self, source, sink, n);
    }

    fn try_init(&mut self, source: usize, sink: usize, n: usize) -> Result<(), IsapError> {
        Isap::try_init(self, source, sink, n)
    }

    fn add_edge(&mut self, from: usize, to: usize, cap: i64) {
        Isap::add_edge(self, from, to, cap);
    }

    fn try_add_edge(&mut self, from: usize, to: usize, cap: i64) -> Result<(), IsapError> {
        Isap::try_add_edge(self, from, to, cap)
    }

    fn maxflow(&mut self) -> i64 {
        Isap::maxflow(self)
    }

    fn mincut(&mut self) -> Vec<IsapEdge> {
        Isap::mincut(self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::IsapLike;

    use super::{Isap, IsapEdge};
    use crate::IsapError;

    fn normalized_edges(edges: &[IsapEdge]) -> Vec<(usize, usize, i64, i64)> {
        let mut values = edges
            .iter()
            .map(|e| (e.from, e.to, e.cap, e.flow))
            .collect::<Vec<_>>();
        values.sort_unstable();
        values
    }

    #[test]
    fn simple_case_should_match_ts_behavior() {
        let mut isap = Isap::new();
        isap.init(0, 1, 4);
        isap.add_edge(0, 2, 1);
        isap.add_edge(0, 3, 2);
        isap.add_edge(3, 1, 1);

        assert_eq!(isap.maxflow(), 1);
        assert_eq!(
            isap.mincut(),
            vec![IsapEdge {
                from: 3,
                to: 1,
                cap: 1,
                flow: 1,
            }]
        );
    }

    #[test]
    fn mincut_should_follow_strict_reachable_partition_definition() {
        let mut isap = Isap::new();
        isap.init(0, 3, 4);
        isap.add_edge(0, 1, 10);
        isap.add_edge(1, 3, 1);
        isap.add_edge(1, 2, 9);
        isap.add_edge(2, 3, 9);

        assert_eq!(isap.maxflow(), 10);
        assert_eq!(
            isap.mincut(),
            vec![IsapEdge {
                from: 0,
                to: 1,
                cap: 10,
                flow: 10,
            }]
        );
    }

    #[test]
    fn mincut_should_include_all_cross_edges_of_reachable_partition() {
        let mut isap = Isap::new();
        isap.init(0, 3, 4);
        isap.add_edge(0, 1, 3);
        isap.add_edge(0, 2, 2);
        isap.add_edge(1, 2, 7);
        isap.add_edge(1, 3, 3);
        isap.add_edge(2, 3, 2);

        assert_eq!(isap.maxflow(), 5);
        let cut = isap.mincut();
        assert_eq!(
            normalized_edges(&cut),
            vec![(0, 1, 3, 3), (0, 2, 2, 2)],
            "strict mincut should return all S->T cross edges"
        );
    }

    #[test]
    fn mincut_should_ignore_zero_capacity_cross_edges() {
        let mut isap = Isap::new();
        isap.init(0, 1, 2);
        isap.add_edge(0, 1, 0);

        assert_eq!(isap.maxflow(), 0);
        assert!(isap.mincut().is_empty());
    }

    #[test]
    fn mincut_should_be_stable_across_cached_maxflow_calls() {
        let mut isap = Isap::new();
        isap.init(0, 3, 4);
        isap.add_edge(0, 1, 3);
        isap.add_edge(0, 2, 2);
        isap.add_edge(1, 3, 3);
        isap.add_edge(2, 3, 2);

        let first = isap.mincut();
        let second = isap.mincut();
        assert_eq!(normalized_edges(&first), normalized_edges(&second));
    }

    #[test]
    fn mincut_before_init_should_return_empty() {
        let mut isap = Isap::new();
        assert_eq!(isap.mincut(), Vec::<IsapEdge>::new());
    }

    #[test]
    fn maxflow_before_init_should_return_zero() {
        let mut isap = Isap::new();
        assert_eq!(isap.maxflow(), 0);
    }

    #[test]
    fn maxflow_should_be_cached_until_graph_changes() {
        let mut isap = Isap::new();
        isap.init(0, 3, 4);
        isap.add_edge(0, 1, 2);
        isap.add_edge(1, 3, 2);

        let first = isap.maxflow();
        let second = isap.maxflow();
        assert_eq!(first, 2);
        assert_eq!(second, 2);

        isap.add_edge(0, 2, 3);
        isap.add_edge(2, 3, 3);

        assert_eq!(isap.maxflow(), 5);
    }

    #[test]
    fn parallel_edges_and_zero_capacity_should_work() {
        let mut isap = Isap::new();
        isap.init(0, 2, 3);
        isap.add_edge(0, 1, 2);
        isap.add_edge(0, 1, 3);
        isap.add_edge(1, 2, 4);
        isap.add_edge(1, 2, 0);

        assert_eq!(isap.maxflow(), 4);
    }

    #[test]
    fn unreachable_sink_should_return_zero_flow_and_empty_cut() {
        let mut isap = Isap::new();
        isap.init(0, 3, 4);
        isap.add_edge(0, 1, 10);
        isap.add_edge(1, 2, 5);

        assert_eq!(isap.maxflow(), 0);
        assert_eq!(isap.mincut(), Vec::<IsapEdge>::new());
    }

    #[test]
    fn init_should_reset_graph_and_flow_state() {
        let mut isap = Isap::new();
        isap.init(0, 2, 3);
        isap.add_edge(0, 1, 5);
        isap.add_edge(1, 2, 5);
        assert_eq!(isap.maxflow(), 5);

        isap.init(0, 1, 2);
        isap.add_edge(0, 1, 3);
        assert_eq!(isap.maxflow(), 3);
    }

    #[test]
    fn checked_api_should_validate_arguments() {
        let mut isap = Isap::new();

        assert_eq!(
            isap.try_init(0, 0, 0),
            Err(IsapError::InvalidNodeCount { n: 0 })
        );
        assert_eq!(
            isap.try_init(3, 0, 3),
            Err(IsapError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            isap.try_init(0, 3, 3),
            Err(IsapError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            isap.try_init(1, 1, 3),
            Err(IsapError::SourceEqualsSink { node: 1 })
        );

        assert_eq!(isap.try_add_edge(0, 1, 1), Err(IsapError::NotInitialized));

        isap.try_init(0, 2, 3).expect("init should work");
        assert_eq!(
            isap.try_add_edge(3, 1, 1),
            Err(IsapError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            isap.try_add_edge(0, 3, 1),
            Err(IsapError::NodeOutOfRange { node: 3, n: 3 })
        );
        assert_eq!(
            isap.try_add_edge(0, 1, -1),
            Err(IsapError::NegativeCapacity { cap: -1 })
        );
    }

    #[test]
    #[should_panic(expected = "[Isap] init failed due to invalid arguments")]
    fn init_should_panic_on_invalid_arguments() {
        let mut isap = Isap::new();
        isap.init(0, 0, 0);
    }

    #[test]
    #[should_panic(expected = "[Isap] init failed due to invalid arguments")]
    fn init_should_panic_when_source_equals_sink() {
        let mut isap = Isap::new();
        isap.init(1, 1, 3);
    }

    #[test]
    #[should_panic(expected = "[Isap] add_edge failed due to invalid arguments")]
    fn add_edge_should_panic_when_not_initialized() {
        let mut isap = Isap::new();
        isap.add_edge(0, 1, 1);
    }

    #[test]
    fn maxflow_should_match_edmonds_karp_on_small_graphs() {
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
                let mut isap = Isap::new();
                isap.init(source, sink, n);

                let mut cap = vec![vec![0i64; n]; n];
                for (u, row) in cap.iter_mut().enumerate() {
                    for (v, cell) in row.iter_mut().enumerate() {
                        if u == v {
                            continue;
                        }
                        let roll = next_u32(&mut seed) % 100;
                        if roll < 35 {
                            let c = (next_u32(&mut seed) % 16) as i64;
                            isap.add_edge(u, v, c);
                            *cell += c;
                        }
                    }
                }

                let got = isap.maxflow();
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
                let mut isap = Isap::new();
                isap.init(source, sink, n);

                for u in 0..n {
                    for v in 0..n {
                        if u == v {
                            continue;
                        }
                        if next_u32(&mut seed) % 100 < 35 {
                            let cap = (next_u32(&mut seed) % 16) as i64;
                            isap.add_edge(u, v, cap);
                        }
                    }
                }

                let flow = isap.maxflow();
                let cut_cap: i64 = isap.mincut().iter().map(|e| e.cap).sum();
                assert_eq!(
                    cut_cap, flow,
                    "cut capacity mismatch on n={n}, expected flow={flow}, got cut={cut_cap}"
                );
            }
        }
    }

    #[test]
    fn trait_object_call_should_work() {
        let mut isap = Isap::new();
        let d: &mut dyn IsapLike = &mut isap;

        d.init(0, 2, 3);
        d.add_edge(0, 1, 2);
        d.add_edge(1, 2, 2);
        assert_eq!(d.maxflow(), 2);
    }
}
