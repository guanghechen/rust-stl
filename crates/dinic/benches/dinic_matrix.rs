use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_dinic::Dinic;

type Case = (&'static str, fn() -> Dinic);

#[derive(Clone)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper_exclusive
    }

    fn next_cap(&mut self, cap_exclusive: i64) -> i64 {
        1 + (self.next_u64() % cap_exclusive as u64) as i64
    }
}

fn build_layered_graph() -> Dinic {
    const LAYER_SIZE: usize = 64;
    const LAYER_COUNT: usize = 5;

    let n = LAYER_SIZE * LAYER_COUNT + 2;
    let source = n - 2;
    let sink = n - 1;

    let mut dinic = Dinic::new();
    dinic.init(source, sink, n);

    for i in 0..LAYER_SIZE {
        dinic.add_edge(source, i, 4);
    }

    for layer in 0..(LAYER_COUNT - 1) {
        let base_u = layer * LAYER_SIZE;
        let base_v = (layer + 1) * LAYER_SIZE;
        for u in 0..LAYER_SIZE {
            for step in 0..4 {
                let v = (u + step) % LAYER_SIZE;
                dinic.add_edge(base_u + u, base_v + v, 2);
            }
        }
    }

    let last_base = (LAYER_COUNT - 1) * LAYER_SIZE;
    for i in 0..LAYER_SIZE {
        dinic.add_edge(last_base + i, sink, 4);
    }

    dinic
}

fn build_sparse_random_graph() -> Dinic {
    let n = 256usize;
    let source = 0usize;
    let sink = n - 1;

    let mut rng = XorShift64::new(0xA0B1_C2D3_E4F5_0617);
    let mut dinic = Dinic::new();
    dinic.init(source, sink, n);

    for u in 0..(n - 1) {
        dinic.add_edge(u, u + 1, rng.next_cap(8));
    }

    for u in 0..n {
        for _ in 0..3 {
            let mut v = rng.next_usize(n);
            if v == u {
                v = (v + 1) % n;
            }
            dinic.add_edge(u, v, rng.next_cap(10));
        }
    }

    dinic
}

fn build_dense_random_graph() -> Dinic {
    let n = 96usize;
    let source = 0usize;
    let sink = n - 1;

    let mut rng = XorShift64::new(0x1020_3040_5060_7080);
    let mut dinic = Dinic::new();
    dinic.init(source, sink, n);

    for u in 0..n {
        for v in 0..n {
            if u == v {
                continue;
            }
            if rng.next_usize(100) < 15 {
                dinic.add_edge(u, v, rng.next_cap(20));
            }
        }
    }

    dinic
}

fn build_bipartite_graph() -> Dinic {
    let left = 96usize;
    let right = 96usize;
    let source = left + right;
    let sink = source + 1;
    let n = sink + 1;

    let mut rng = XorShift64::new(0x9999_AAAA_BBBB_CCCC);
    let mut dinic = Dinic::new();
    dinic.init(source, sink, n);

    for u in 0..left {
        dinic.add_edge(source, u, 8);
    }

    for u in 0..left {
        for _ in 0..8 {
            let v = left + rng.next_usize(right);
            dinic.add_edge(u, v, rng.next_cap(6));
        }
    }

    for v in 0..right {
        dinic.add_edge(left + v, sink, 8);
    }

    dinic
}

fn build_grid_graph() -> Dinic {
    let rows = 28usize;
    let cols = 28usize;
    let grid_n = rows * cols;
    let source = grid_n;
    let sink = grid_n + 1;
    let n = grid_n + 2;

    let mut dinic = Dinic::new();
    dinic.init(source, sink, n);

    for c in 0..cols {
        dinic.add_edge(source, c, 3);
    }

    for r in 0..rows {
        for c in 0..cols {
            let u = r * cols + c;
            if c + 1 < cols {
                dinic.add_edge(u, u + 1, 2);
            }
            if r + 1 < rows {
                dinic.add_edge(u, u + cols, 2);
            }
        }
    }

    let last_row = (rows - 1) * cols;
    for c in 0..cols {
        dinic.add_edge(last_row + c, sink, 3);
    }

    dinic
}

fn bench_maxflow_matrix(c: &mut Criterion) {
    let mut group = c.benchmark_group("dinic/matrix_maxflow");

    let cases: [Case; 5] = [
        ("layered", build_layered_graph),
        ("sparse_random", build_sparse_random_graph),
        ("dense_random", build_dense_random_graph),
        ("bipartite", build_bipartite_graph),
        ("grid", build_grid_graph),
    ];

    for (name, builder) in cases {
        group.bench_function(name, |b| {
            b.iter_batched(
                builder,
                |mut alg| {
                    black_box(alg.maxflow());
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_cached_matrix(c: &mut Criterion) {
    let mut group = c.benchmark_group("dinic/matrix_cached_maxflow");

    let cases: [Case; 5] = [
        ("layered", build_layered_graph),
        ("sparse_random", build_sparse_random_graph),
        ("dense_random", build_dense_random_graph),
        ("bipartite", build_bipartite_graph),
        ("grid", build_grid_graph),
    ];

    for (name, builder) in cases {
        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    let mut alg = builder();
                    black_box(alg.maxflow());
                    alg
                },
                |mut alg| {
                    black_box(alg.maxflow());
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(dinic_matrix, bench_maxflow_matrix, bench_cached_matrix);
criterion_main!(dinic_matrix);
