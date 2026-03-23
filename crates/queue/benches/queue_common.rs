use std::cmp::Reverse;
use std::collections::BinaryHeap;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use rstl_queue::{CircularQueue, LinkedDeque, PriorityQueue, QueueLike};

const N_ENQUEUE: usize = 1024;
const N_DEQUEUE: usize = 1024;
const N_MIX: usize = 10_000;

fn bench_enqueue(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/common_enqueue_1k");

    group.bench_function("circular_queue", |b| {
        b.iter_batched(
            || CircularQueue::new(N_ENQUEUE + 1).expect("valid capacity"),
            |mut q| {
                for i in 0..N_ENQUEUE {
                    q.enqueue(black_box(i));
                }
                black_box(q.front());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("linked_deque", |b| {
        b.iter_batched(
            LinkedDeque::new,
            |mut q| {
                for i in 0..N_ENQUEUE {
                    q.enqueue(black_box(i));
                }
                black_box(q.front());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("priority_queue", |b| {
        b.iter_batched(
            PriorityQueue::<usize>::new,
            |mut q| {
                for i in 0..N_ENQUEUE {
                    q.enqueue(black_box(i));
                }
                black_box(q.front());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("std_binary_heap", |b| {
        b.iter_batched(
            BinaryHeap::<Reverse<usize>>::new,
            |mut q| {
                for i in 0..N_ENQUEUE {
                    q.push(Reverse(black_box(i)));
                }
                black_box(q.peek());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_dequeue(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/common_dequeue_1k");

    group.bench_function("circular_queue", |b| {
        b.iter_batched(
            || {
                let mut q = CircularQueue::new(N_DEQUEUE + 1).expect("valid capacity");
                for i in 0..N_DEQUEUE {
                    q.enqueue(i);
                }
                q
            },
            |mut q| {
                for _ in 0..N_DEQUEUE {
                    black_box(q.dequeue());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("linked_deque", |b| {
        b.iter_batched(
            || {
                let mut q = LinkedDeque::new();
                for i in 0..N_DEQUEUE {
                    q.enqueue(i);
                }
                q
            },
            |mut q| {
                for _ in 0..N_DEQUEUE {
                    black_box(q.dequeue());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("priority_queue", |b| {
        b.iter_batched(
            || {
                let mut q = PriorityQueue::<usize>::new();
                for i in 0..N_DEQUEUE {
                    q.enqueue(i);
                }
                q
            },
            |mut q| {
                for _ in 0..N_DEQUEUE {
                    black_box(q.dequeue());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("std_binary_heap", |b| {
        b.iter_batched(
            || {
                let mut q = BinaryHeap::<Reverse<usize>>::new();
                for i in 0..N_DEQUEUE {
                    q.push(Reverse(i));
                }
                q
            },
            |mut q| {
                for _ in 0..N_DEQUEUE {
                    black_box(q.pop());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_mix(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/common_mix_10k");

    group.bench_function("circular_queue", |b| {
        b.iter_batched(
            || CircularQueue::new(16).expect("valid capacity"),
            |mut q| {
                for i in 0..N_MIX {
                    q.enqueue(black_box(i));
                    black_box(q.dequeue());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("linked_deque", |b| {
        b.iter_batched(
            LinkedDeque::new,
            |mut q| {
                for i in 0..N_MIX {
                    q.enqueue(black_box(i));
                    black_box(q.dequeue());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("priority_queue", |b| {
        b.iter_batched(
            PriorityQueue::<usize>::new,
            |mut q| {
                for i in 0..N_MIX {
                    q.enqueue(black_box(i));
                    black_box(q.dequeue());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("std_binary_heap", |b| {
        b.iter_batched(
            BinaryHeap::<Reverse<usize>>::new,
            |mut q| {
                for i in 0..N_MIX {
                    q.push(Reverse(black_box(i)));
                    black_box(q.pop());
                }
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(queue_common, bench_enqueue, bench_dequeue, bench_mix);
criterion_main!(queue_common);
