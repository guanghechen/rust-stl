use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rstl_manacher::{manacher, manacher_by, manacher_str};

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

    fn next_byte(&mut self, alphabet: u8) -> u8 {
        b'a' + (self.next_u64() % alphabet as u64) as u8
    }
}

fn build_random_ascii(n: usize, alphabet: u8, seed: u64) -> String {
    let mut rng = XorShift64::new(seed ^ n as u64 ^ ((alphabet as u64) << 32));
    let mut bytes = Vec::with_capacity(n);
    for _ in 0..n {
        bytes.push(rng.next_byte(alphabet));
    }
    String::from_utf8(bytes).expect("ascii only")
}

fn bench_manacher(c: &mut Criterion) {
    let mut group = c.benchmark_group("manacher/radius");

    for &(n, alphabet) in &[(64usize, 4u8), (1024, 4), (16_384, 8)] {
        let text = build_random_ascii(n, alphabet, 0x1234_5678_9ABC_DEF0);
        let bytes = text.as_bytes().to_vec();

        let id = BenchmarkId::new("str_bytes", format!("n{n}_a{alphabet}"));
        group.bench_with_input(id, &text, |b, input| {
            b.iter_batched(
                || input.clone(),
                |s| {
                    let r = manacher_str(&s);
                    black_box(r);
                },
                BatchSize::SmallInput,
            )
        });

        let id = BenchmarkId::new("slice_u8", format!("n{n}_a{alphabet}"));
        group.bench_with_input(id, &bytes, |b, input| {
            b.iter_batched(
                || input.clone(),
                |v| {
                    let r = manacher(&v);
                    black_box(r);
                },
                BatchSize::SmallInput,
            )
        });

        let id = BenchmarkId::new("index_by", format!("n{n}_a{alphabet}"));
        group.bench_with_input(id, &bytes, |b, input| {
            b.iter_batched(
                || input.clone(),
                |v| {
                    let r = manacher_by(v.len(), |l, r| v[l] == v[r]);
                    black_box(r);
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(manacher_common, bench_manacher);
criterion_main!(manacher_common);
