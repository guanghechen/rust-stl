use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_mcmf::Mcmf;

const LAYER_SIZE: usize = 24;
const LAYER_COUNT: usize = 4;

fn build_transport_graph() -> Mcmf {
    let n = LAYER_SIZE * LAYER_COUNT + 2;
    let source = n - 2;
    let sink = n - 1;

    let mut mcmf = Mcmf::new();
    mcmf.init(source, sink, n);

    for i in 0..LAYER_SIZE {
        mcmf.add_edge(source, i, 3, 0);
    }

    for layer in 0..(LAYER_COUNT - 1) {
        let base_u = layer * LAYER_SIZE;
        let base_v = (layer + 1) * LAYER_SIZE;
        for u in 0..LAYER_SIZE {
            for step in 0..3 {
                let v = (u + step) % LAYER_SIZE;
                mcmf.add_edge(base_u + u, base_v + v, 2, (layer + step + 1) as i64);
            }
        }
    }

    let last_base = (LAYER_COUNT - 1) * LAYER_SIZE;
    for i in 0..LAYER_SIZE {
        mcmf.add_edge(last_base + i, sink, 3, 0);
    }

    mcmf
}

fn bench_min_cost_max_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("mcmf/common_min_cost_max_flow");

    group.bench_function("layered_graph", |b| {
        b.iter_batched(
            build_transport_graph,
            |mut mcmf| {
                black_box(mcmf.min_cost_max_flow());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_cached_min_cost_max_flow(c: &mut Criterion) {
    let mut group = c.benchmark_group("mcmf/common_cached_min_cost_max_flow");

    group.bench_function("repeat_without_modify", |b| {
        b.iter_batched(
            || {
                let mut mcmf = build_transport_graph();
                black_box(mcmf.min_cost_max_flow());
                mcmf
            },
            |mut mcmf| {
                black_box(mcmf.min_cost_max_flow());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    mcmf_common,
    bench_min_cost_max_flow,
    bench_cached_min_cost_max_flow
);
criterion_main!(mcmf_common);
