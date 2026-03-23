use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rstl_shuffle::{knuth_shuffle, knuth_shuffle_range, knuth_shuffle_with};

#[derive(Clone)]
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            return 0;
        }

        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.state >> 32) as usize) % upper_exclusive
    }
}

fn bench_shuffle_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle/full");

    for &n in &[64usize, 1024, 16_384] {
        let input = (0..n).collect::<Vec<_>>();

        let default_id = BenchmarkId::new("default_rng", n);
        group.bench_with_input(default_id, &input, |b, input| {
            b.iter_batched(
                || input.clone(),
                |mut values| {
                    knuth_shuffle(&mut values);
                    black_box(values);
                },
                BatchSize::SmallInput,
            )
        });

        let custom_id = BenchmarkId::new("custom_lcg", n);
        group.bench_with_input(custom_id, &input, |b, input| {
            b.iter_batched(
                || input.clone(),
                |mut values| {
                    let mut rng = Lcg::new(0x1234_5678_9ABC_DEF0 ^ n as u64);
                    knuth_shuffle_with(&mut values, |bound| rng.next_usize(bound));
                    black_box(values);
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_shuffle_subrange(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle/subrange");

    for &n in &[1024usize, 8192] {
        let start = n / 4;
        let end = n * 3 / 4;
        let input = (0..n).collect::<Vec<_>>();

        let id = BenchmarkId::new("default_rng", n);
        group.bench_with_input(id, &input, |b, input| {
            b.iter_batched(
                || input.clone(),
                |mut values| {
                    knuth_shuffle_range(&mut values, start, end);
                    black_box(values);
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(shuffle_common, bench_shuffle_full, bench_shuffle_subrange);
criterion_main!(shuffle_common);
