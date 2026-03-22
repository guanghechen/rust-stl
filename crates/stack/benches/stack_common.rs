use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_stack::{CircularStack, CircularStackLike, StackLike};

const N_PUSH: usize = 1024;
const N_POP: usize = 1024;
const N_MIX: usize = 10_000;
const RESIZE_BASE_CAPACITY: usize = 1024;
const FORK_CAPACITY: usize = 131_072;
const FORK_SIZE: usize = 1024;

fn make_shifted_non_wrapped_stack() -> CircularStack<usize> {
    let mut s = CircularStack::new(RESIZE_BASE_CAPACITY).expect("valid capacity");

    for i in 0..(RESIZE_BASE_CAPACITY + 128) {
        s.push(i);
    }
    for _ in 0..128 {
        black_box(s.pop());
    }

    s
}

fn make_wrapped_stack() -> CircularStack<usize> {
    let mut s = CircularStack::new(RESIZE_BASE_CAPACITY).expect("valid capacity");

    for i in 0..(RESIZE_BASE_CAPACITY + 128) {
        s.push(i);
    }

    s
}

fn make_shrink_stack() -> CircularStack<usize> {
    let mut s = CircularStack::new(RESIZE_BASE_CAPACITY).expect("valid capacity");

    for i in 0..(RESIZE_BASE_CAPACITY + 512) {
        s.push(i);
    }

    s
}

fn make_fork_sparse_stack() -> CircularStack<usize> {
    let mut s = CircularStack::new(FORK_CAPACITY).expect("valid capacity");

    for i in 0..FORK_SIZE {
        s.push(i);
    }

    s
}

fn bench_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack/common_push_1k");

    group.bench_function("circular_stack", |b| {
        b.iter_batched(
            || CircularStack::new(N_PUSH + 1).expect("valid capacity"),
            |mut s| {
                for i in 0..N_PUSH {
                    s.push(black_box(i));
                }
                black_box(s.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack/common_pop_1k");

    group.bench_function("circular_stack", |b| {
        b.iter_batched(
            || {
                let mut s = CircularStack::new(N_POP + 1).expect("valid capacity");
                for i in 0..N_POP {
                    s.push(i);
                }
                s
            },
            |mut s| {
                for _ in 0..N_POP {
                    black_box(s.pop());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_mix(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack/common_mix_10k");

    group.bench_function("circular_stack", |b| {
        b.iter_batched(
            || CircularStack::new(16).expect("valid capacity"),
            |mut s| {
                for i in 0..N_MIX {
                    s.push(black_box(i));
                    black_box(s.pop());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack/common_resize");

    group.bench_function("grow_non_wrapped_fast_path", |b| {
        b.iter_batched(
            make_shifted_non_wrapped_stack,
            |mut s| {
                s.resize(RESIZE_BASE_CAPACITY * 2)
                    .expect("resize should succeed");
                black_box(s.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("grow_wrapped_fallback", |b| {
        b.iter_batched(
            make_wrapped_stack,
            |mut s| {
                s.resize(RESIZE_BASE_CAPACITY * 2)
                    .expect("resize should succeed");
                black_box(s.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("shrink_keep_latest", |b| {
        b.iter_batched(
            make_shrink_stack,
            |mut s| {
                s.resize(RESIZE_BASE_CAPACITY / 2)
                    .expect("resize should succeed");
                black_box(s.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_fork(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack/common_fork");

    group.bench_function("clone_sparse_large_capacity", |b| {
        b.iter_batched(
            make_fork_sparse_stack,
            |s| {
                let cloned = s.clone();
                black_box(cloned.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("fork_sparse_large_capacity", |b| {
        b.iter_batched(
            make_fork_sparse_stack,
            |s| {
                let forked = s.fork();
                black_box(forked.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("trait_fork_sparse_large_capacity", |b| {
        b.iter_batched(
            make_fork_sparse_stack,
            |s| {
                let forked = StackLike::fork(&s);
                black_box(forked.top());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    stack_common,
    bench_push,
    bench_pop,
    bench_mix,
    bench_resize,
    bench_fork
);
criterion_main!(stack_common);
