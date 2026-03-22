use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_history::History;

const APPEND_N: usize = 1024;
const NAV_N: usize = 10_000;
const BRANCH_CAPACITY: usize = 4096;
const REARRANGE_CAPACITY: usize = 4096;
const FORK_CAPACITY: usize = 16_384;
const FORK_SIZE: usize = 4096;

fn make_history(capacity: usize, size: usize) -> History<usize> {
    let mut history = History::new("bench", capacity).expect("valid capacity");
    for i in 0..size {
        history.push(i);
    }
    history
}

fn make_branch_history_for_truncate() -> History<usize> {
    let mut history = make_history(BRANCH_CAPACITY, BRANCH_CAPACITY);
    let _ = history.go(0);
    history
}

fn make_branch_history_for_reuse() -> History<usize> {
    let mut history = make_history(BRANCH_CAPACITY, BRANCH_CAPACITY);
    let _ = history.go(0);
    history
}

fn make_rearrange_history() -> History<usize> {
    make_history(REARRANGE_CAPACITY, REARRANGE_CAPACITY)
}

fn make_fork_history() -> History<usize> {
    make_history(FORK_CAPACITY, FORK_SIZE)
}

fn bench_push_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("history/common_push_append_1k");

    group.bench_function("history", |b| {
        b.iter_batched(
            || History::new("bench", APPEND_N + 1).expect("valid capacity"),
            |mut history| {
                for i in 0..APPEND_N {
                    history.push(black_box(i));
                }
                black_box(history.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("history/common_navigation_10k");

    group.bench_function("backward_forward_ping_pong", |b| {
        b.iter_batched(
            || {
                let mut history = make_history(2048, 1024);
                let _ = history.go(512);
                history
            },
            |mut history| {
                for i in 0..NAV_N {
                    if i % 2 == 0 {
                        black_box(history.backward());
                    } else {
                        black_box(history.forward());
                    }
                }
                black_box(history.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_push_branch(c: &mut Criterion) {
    let mut group = c.benchmark_group("history/common_push_branch");

    group.bench_function("truncate_future_from_bottom", |b| {
        b.iter_batched(
            make_branch_history_for_truncate,
            |mut history| {
                history.push(black_box(BRANCH_CAPACITY + 1));
                black_box(history.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("reuse_future_next_item", |b| {
        b.iter_batched(
            make_branch_history_for_reuse,
            |mut history| {
                history.push(black_box(1));
                black_box(history.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_rearrange(c: &mut Criterion) {
    let mut group = c.benchmark_group("history/common_rearrange");

    group.bench_function("keep_half_odd", |b| {
        b.iter_batched(
            make_rearrange_history,
            |mut history| {
                history.rearrange(|x, _| x % 2 == 1);
                black_box(history.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("keep_all", |b| {
        b.iter_batched(
            make_rearrange_history,
            |mut history| {
                history.rearrange(|_, _| true);
                black_box(history.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_fork(c: &mut Criterion) {
    let mut group = c.benchmark_group("history/common_fork");

    group.bench_function("clone", |b| {
        b.iter_batched(
            make_fork_history,
            |history| {
                let cloned = history.clone();
                black_box(cloned.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("fork_rename", |b| {
        b.iter_batched(
            make_fork_history,
            |history| {
                let forked = history.fork("forked");
                black_box(forked.present());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    history_common,
    bench_push_append,
    bench_navigation,
    bench_push_branch,
    bench_rearrange,
    bench_fork
);
criterion_main!(history_common);
