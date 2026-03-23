use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_isap::Isap;

const LAYER_SIZE: usize = 64;
const LAYER_COUNT: usize = 4;

fn build_layered_graph() -> Isap {
    let n = LAYER_SIZE * LAYER_COUNT + 2;
    let source = n - 2;
    let sink = n - 1;

    let mut isap = Isap::new();
    isap.init(source, sink, n);

    for i in 0..LAYER_SIZE {
        isap.add_edge(source, i, 3);
    }

    for layer in 0..(LAYER_COUNT - 1) {
        let base_u = layer * LAYER_SIZE;
        let base_v = (layer + 1) * LAYER_SIZE;
        for u in 0..LAYER_SIZE {
            for step in 0..4 {
                let v = (u + step) % LAYER_SIZE;
                isap.add_edge(base_u + u, base_v + v, 2);
            }
        }
    }

    let last_base = (LAYER_COUNT - 1) * LAYER_SIZE;
    for i in 0..LAYER_SIZE {
        isap.add_edge(last_base + i, sink, 3);
    }

    isap
}

fn bench_maxflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("isap/common_maxflow");

    group.bench_function("layered_graph", |b| {
        b.iter_batched(
            build_layered_graph,
            |mut d| {
                black_box(d.maxflow());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cached_maxflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("isap/common_cached_maxflow");

    group.bench_function("repeat_without_modify", |b| {
        b.iter_batched(
            || {
                let mut d = build_layered_graph();
                black_box(d.maxflow());
                d
            },
            |mut d| {
                black_box(d.maxflow());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(isap_common, bench_maxflow, bench_cached_maxflow);
criterion_main!(isap_common);
