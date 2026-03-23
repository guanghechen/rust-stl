use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rstl_permutation::{
    for_each_permutation, next_permutation, permutation_indices, prev_permutation,
};

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

fn build_pattern(pattern: InputPattern, n: usize) -> Vec<usize> {
    match pattern {
        InputPattern::Ascending => (0..n).collect(),
        InputPattern::Descending => (0..n).rev().collect(),
        InputPattern::Random => {
            let mut rng = XorShift64::new(0x1234_5678_9ABC_DEF0);
            (0..n)
                .map(|_| (rng.next_u64() % (n as u64 * 4 + 1)) as usize)
                .collect()
        }
        InputPattern::Duplicates => {
            let mut rng = XorShift64::new(0x0F0E_0D0C_0B0A_0908);
            (0..n).map(|_| (rng.next_u64() % 8) as usize).collect()
        }
    }
}

fn bench_next_prev_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation/next_prev_step");

    let sizes = [16usize, 64, 256];
    let patterns = [
        InputPattern::Ascending,
        InputPattern::Descending,
        InputPattern::Random,
        InputPattern::Duplicates,
    ];

    for n in sizes {
        for pattern in patterns {
            let input = build_pattern(pattern, n);
            let next_id = BenchmarkId::new("next", format!("{}_n{}", pattern.label(), n));
            group.bench_with_input(next_id, &input, |b, input| {
                b.iter_batched(
                    || input.clone(),
                    |mut values| {
                        black_box(next_permutation(&mut values));
                        black_box(values);
                    },
                    BatchSize::SmallInput,
                )
            });

            let prev_id = BenchmarkId::new("prev", format!("{}_n{}", pattern.label(), n));
            group.bench_with_input(prev_id, &input, |b, input| {
                b.iter_batched(
                    || input.clone(),
                    |mut values| {
                        black_box(prev_permutation(&mut values));
                        black_box(values);
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }

    group.finish();
}

fn bench_full_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation/full_traversal");

    for &n in &[7usize, 8] {
        let seed = (0..n).rev().collect::<Vec<_>>();

        let for_each_id = BenchmarkId::new("for_each_permutation", format!("n{}", n));
        group.bench_with_input(for_each_id, &seed, |b, seed| {
            b.iter_batched(
                || seed.clone(),
                |mut values| {
                    let mut checksum = 0usize;
                    for_each_permutation(&mut values, |slice| {
                        checksum = checksum.wrapping_add(slice[0]);
                    });
                    black_box(checksum);
                    black_box(values);
                },
                BatchSize::SmallInput,
            )
        });

        let indices_id = BenchmarkId::new("permutation_indices", format!("n{}", n));
        group.bench_function(indices_id, |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                let mut cursor = permutation_indices(n, 0);
                while cursor.advance() {
                    let p = cursor.current().expect("advanced cursor has current");
                    checksum = checksum.wrapping_add(p[0]);
                    black_box(p);
                }
                black_box(checksum);
            })
        });

        let indices_clone_id = BenchmarkId::new("permutation_indices_clone", format!("n{}", n));
        group.bench_function(indices_clone_id, |b| {
            b.iter(|| {
                let mut checksum = 0usize;
                let mut cursor = permutation_indices(n, 0);
                while cursor.advance() {
                    let p = cursor
                        .current()
                        .expect("advanced cursor has current")
                        .to_vec();
                    checksum = checksum.wrapping_add(p[0]);
                    black_box(p);
                }
                black_box(checksum);
            })
        });
    }

    group.finish();
}

criterion_group!(
    permutation_common,
    bench_next_prev_step,
    bench_full_traversal
);
criterion_main!(permutation_common);
