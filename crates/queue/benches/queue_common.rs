use std::cmp::Reverse;
use std::collections::BinaryHeap;

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use rstl_queue::{CircularQueue, LinkedDeque, PriorityQueue, QueueLike};

const N_ENQUEUE: usize = 1024;
const N_DEQUEUE: usize = 1024;
const N_MIX: usize = 10_000;
const N_PATTERN: usize = 4096;
const N_STEADY_OPS: usize = 20_000;

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
}

#[derive(Clone, Copy)]
enum InputPattern {
    Ascending,
    Descending,
    Random,
    Duplicates,
}

impl InputPattern {
    fn label(self) -> &'static str {
        match self {
            Self::Ascending => "ascending",
            Self::Descending => "descending",
            Self::Random => "random",
            Self::Duplicates => "duplicates",
        }
    }
}

fn build_pattern(pattern: InputPattern, n: usize) -> Vec<usize> {
    match pattern {
        InputPattern::Ascending => (0..n).collect(),
        InputPattern::Descending => (0..n).rev().collect(),
        InputPattern::Random => {
            let mut rng = XorShift64::new(0x1234_5678_9ABC_DEF0);
            (0..n)
                .map(|_| (rng.next_u64() % (n as u64 * 8 + 1)) as usize)
                .collect()
        }
        InputPattern::Duplicates => {
            let mut rng = XorShift64::new(0x0F0E_0D0C_0B0A_0908);
            (0..n).map(|_| (rng.next_u64() % 32) as usize).collect()
        }
    }
}

fn prepare_seed_data(n: usize, seed: u64) -> Vec<usize> {
    let mut rng = XorShift64::new(seed);
    (0..n)
        .map(|_| (rng.next_u64() % (n as u64 * 4 + 1)) as usize)
        .collect()
}

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

fn bench_pq_vs_bh_enqueue_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/pq_vs_bh_enqueue_patterns_4k");

    for pattern in [
        InputPattern::Ascending,
        InputPattern::Descending,
        InputPattern::Random,
        InputPattern::Duplicates,
    ] {
        let values = build_pattern(pattern, N_PATTERN);
        let pq_name = format!("priority_queue/{}", pattern.label());
        let bh_name = format!("std_binary_heap/{}", pattern.label());

        group.bench_function(&pq_name, |b| {
            b.iter_batched(
                || values.clone(),
                |values| {
                    let mut q = PriorityQueue::<usize>::new();
                    for value in values {
                        q.enqueue(black_box(value));
                    }
                    black_box(q.front());
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_function(&bh_name, |b| {
            b.iter_batched(
                || values.clone(),
                |values| {
                    let mut q = BinaryHeap::<Reverse<usize>>::new();
                    for value in values {
                        q.push(Reverse(black_box(value)));
                    }
                    black_box(q.peek());
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_pq_vs_bh_dequeue_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/pq_vs_bh_dequeue_sizes");

    for &size in &[256_usize, 4096, 16384] {
        let values = prepare_seed_data(size, 0xA5A5_A5A5_u64 ^ size as u64);

        let pq_name = format!("priority_queue/{}", size);
        group.bench_function(&pq_name, |b| {
            b.iter_batched(
                || {
                    let mut q = PriorityQueue::<usize>::new();
                    for &value in &values {
                        q.enqueue(value);
                    }
                    q
                },
                |mut q| {
                    for _ in 0..size {
                        black_box(q.dequeue());
                    }
                },
                BatchSize::SmallInput,
            )
        });

        let bh_name = format!("std_binary_heap/{}", size);
        group.bench_function(&bh_name, |b| {
            b.iter_batched(
                || {
                    let mut q = BinaryHeap::<Reverse<usize>>::new();
                    for &value in &values {
                        q.push(Reverse(value));
                    }
                    q
                },
                |mut q| {
                    for _ in 0..size {
                        black_box(q.pop());
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_pq_vs_bh_steady_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/pq_vs_bh_steady_state");

    for &size in &[32_usize, 512, 4096] {
        let init_values = prepare_seed_data(size, 0xFACE_B00C_u64 ^ size as u64);
        let stream = prepare_seed_data(N_STEADY_OPS, 0xDEAD_BEEF_u64 ^ size as u64);

        let pq_name = format!("priority_queue/{}", size);
        group.bench_function(&pq_name, |b| {
            b.iter_batched(
                || {
                    let mut q = PriorityQueue::<usize>::new();
                    q.enqueues(init_values.iter().copied());
                    q
                },
                |mut q| {
                    for &value in &stream {
                        q.enqueue(black_box(value));
                        black_box(q.dequeue());
                    }
                },
                BatchSize::SmallInput,
            )
        });

        let bh_name = format!("std_binary_heap/{}", size);
        group.bench_function(&bh_name, |b| {
            b.iter_batched(
                || {
                    let mut q = BinaryHeap::<Reverse<usize>>::new();
                    for &value in &init_values {
                        q.push(Reverse(value));
                    }
                    q
                },
                |mut q| {
                    for &value in &stream {
                        q.push(Reverse(black_box(value)));
                        black_box(q.pop());
                    }
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(
    queue_common,
    bench_enqueue,
    bench_dequeue,
    bench_mix,
    bench_pq_vs_bh_enqueue_patterns,
    bench_pq_vs_bh_dequeue_sizes,
    bench_pq_vs_bh_steady_state
);
criterion_main!(queue_common);
