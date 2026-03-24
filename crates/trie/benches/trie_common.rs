use std::collections::HashMap;

use criterion::{BatchSize, BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rstl_trie::{Trie, TrieOptions, alpha_numeric_idx};

const ALNUM_SIGMA: usize = 62;
const TOKEN_SIGMA: usize = 32;
const N_WORDS: usize = 2048;
const N_QUERIES: usize = 4096;
const N_TOKEN_KEYS: usize = 4096;
const N_TOKEN_QUERIES: usize = 4096;
const N_FIND_RANGES: usize = 512;

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

    fn next_usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            return 0;
        }
        (self.next_u64() % upper as u64) as usize
    }
}

fn alnum_char(idx: usize) -> char {
    if idx < 10 {
        return (b'0' + idx as u8) as char;
    }
    if idx < 36 {
        return (b'A' + (idx - 10) as u8) as char;
    }
    (b'a' + (idx - 36) as u8) as char
}

fn build_words(seed: u64, count: usize, min_len: usize, max_len: usize) -> Vec<Vec<char>> {
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = min_len + rng.next_usize(max_len - min_len + 1);
        let mut word = Vec::with_capacity(len);
        for _ in 0..len {
            word.push(alnum_char(rng.next_usize(ALNUM_SIGMA)));
        }
        out.push(word);
    }
    out
}

fn build_queries(seed: u64, words: &[Vec<char>], n: usize) -> Vec<Vec<char>> {
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let base = &words[rng.next_usize(words.len())];
        if i % 2 == 0 {
            out.push(base.clone());
            continue;
        }

        let mut q = base.clone();
        if q.is_empty() {
            q.push('0');
        } else {
            let pos = rng.next_usize(q.len());
            let src = alpha_numeric_idx(&q[pos]);
            let dst = (src + 1 + rng.next_usize(ALNUM_SIGMA - 1)) % ALNUM_SIGMA;
            q[pos] = alnum_char(dst);
        }
        out.push(q);
    }

    out
}

fn build_token_keys(seed: u64, count: usize) -> Vec<Vec<u8>> {
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = 2 + rng.next_usize(5);
        let mut key = Vec::with_capacity(len);
        for _ in 0..len {
            key.push(rng.next_usize(TOKEN_SIGMA) as u8);
        }
        out.push(key);
    }
    out
}

fn build_token_queries(seed: u64, keys: &[Vec<u8>], n: usize) -> Vec<Vec<u8>> {
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let base = &keys[rng.next_usize(keys.len())];
        if i % 2 == 0 {
            out.push(base.clone());
            continue;
        }

        let mut q = base.clone();
        if q.is_empty() {
            q.push(0);
        } else {
            let pos = rng.next_usize(q.len());
            q[pos] = ((q[pos] as usize + 1 + rng.next_usize(TOKEN_SIGMA - 1)) % TOKEN_SIGMA) as u8;
        }
        out.push(q);
    }

    out
}

fn map_checksum(map: &HashMap<Vec<char>, u64>, queries: &[Vec<char>]) -> u64 {
    queries
        .iter()
        .filter_map(|q| map.get(q))
        .fold(0_u64, |acc, v| acc.wrapping_add(*v))
}

fn token_map_checksum(map: &HashMap<Vec<u8>, u64>, queries: &[Vec<u8>]) -> u64 {
    queries
        .iter()
        .filter_map(|q| map.get(q))
        .fold(0_u64, |acc, v| acc.wrapping_add(*v))
}

fn build_ranges(seed: u64, len: usize, n: usize) -> Vec<(usize, usize)> {
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let start = rng.next_usize(len + 1);
        let span = 1 + rng.next_usize(24);
        let end = (start + span).min(len);
        out.push((start, end));
    }
    out
}

fn matches_checksum_oracle(
    map: &HashMap<Vec<char>, u64>,
    query: &[char],
    ranges: &[(usize, usize)],
) -> u64 {
    let mut checksum = 0_u64;
    for &(start, end) in ranges {
        if start >= end {
            continue;
        }

        for i in (start + 1)..=end {
            let key = query[start..i].to_vec();
            if let Some(value) = map.get(&key) {
                checksum = checksum.wrapping_add(i as u64 ^ value.wrapping_mul(1_315_423_911));
            }
        }
    }
    checksum
}

fn bench_insert_lookup_chars(c: &mut Criterion) {
    let words = build_words(0x1020_3040_5060_7080, N_WORDS, 3, 12);
    let queries = build_queries(0x8877_6655_4433_2211, &words, N_QUERIES);

    let mut oracle = HashMap::<Vec<char>, u64>::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        *oracle.entry(word.clone()).or_insert(0) += i as u64 + 1;
    }
    let expected = map_checksum(&oracle, &queries);

    let mut group = c.benchmark_group("trie/insert_lookup_chars");

    group.bench_function("trie", |b| {
        b.iter_batched(
            || {
                Trie::new(TrieOptions {
                    sigma_size: ALNUM_SIGMA,
                    idx: alpha_numeric_idx,
                    merge_node_value: |x, y| x + y,
                })
                .expect("valid trie options")
            },
            |mut trie| {
                for (i, word) in words.iter().enumerate() {
                    trie.try_insert(word, i as u64 + 1).expect("insert");
                }

                let mut checksum = 0_u64;
                for query in &queries {
                    if let Some(value) = trie.try_get(query).expect("get") {
                        checksum = checksum.wrapping_add(*value);
                    }
                }

                assert_eq!(checksum, expected);
                black_box(checksum);
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("std_hash_map", |b| {
        b.iter_batched(
            || HashMap::<Vec<char>, u64>::with_capacity(words.len()),
            |mut map| {
                for (i, word) in words.iter().enumerate() {
                    *map.entry(word.clone()).or_insert(0) += i as u64 + 1;
                }

                let mut checksum = 0_u64;
                for query in &queries {
                    if let Some(value) = map.get(query) {
                        checksum = checksum.wrapping_add(*value);
                    }
                }

                assert_eq!(checksum, expected);
                black_box(checksum);
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_matches_range_chars(c: &mut Criterion) {
    let words = build_words(0xA0A1_A2A3_A4A5_A6A7, N_WORDS, 3, 9);
    let query = words.iter().flatten().copied().collect::<Vec<char>>();
    let ranges = build_ranges(0xC0C1_C2C3_C4C5_C6C7, query.len(), N_FIND_RANGES);

    let mut oracle = HashMap::<Vec<char>, u64>::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        *oracle.entry(word.clone()).or_insert(0) += i as u64 + 1;
    }

    let mut group = c.benchmark_group("trie/matches_range_chars");

    for &count in &[64_usize, 256, N_FIND_RANGES] {
        let count = count.min(ranges.len());
        let expected = matches_checksum_oracle(&oracle, &query, &ranges[..count]);

        group.bench_with_input(BenchmarkId::new("trie", count), &count, |b, &n| {
            b.iter_batched(
                || {
                    let mut trie = Trie::new(TrieOptions {
                        sigma_size: ALNUM_SIGMA,
                        idx: alpha_numeric_idx,
                        merge_node_value: |x, y| x + y,
                    })
                    .expect("valid trie options");

                    for (i, word) in words.iter().enumerate() {
                        trie.try_insert(word, i as u64 + 1).expect("insert");
                    }

                    trie
                },
                |trie| {
                    let mut checksum = 0_u64;
                    for &(start, end) in ranges.iter().take(n) {
                        let nodes = trie
                            .try_matches_range(&query, start..end)
                            .expect("matches range");
                        for node in nodes {
                            checksum = checksum.wrapping_add(
                                node.end as u64 ^ (*node.val).wrapping_mul(1_315_423_911),
                            );
                        }
                    }

                    assert_eq!(checksum, expected);
                    black_box(checksum);
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_insert_lookup_tokens(c: &mut Criterion) {
    let keys = build_token_keys(0x55AA_33CC_0F0F_F0F0, N_TOKEN_KEYS);
    let queries = build_token_queries(0x1234_4321_ABCD_DCBA, &keys, N_TOKEN_QUERIES);

    let mut oracle = HashMap::<Vec<u8>, u64>::with_capacity(keys.len());
    for (i, key) in keys.iter().enumerate() {
        *oracle.entry(key.clone()).or_insert(0) += i as u64 + 1;
    }
    let expected = token_map_checksum(&oracle, &queries);

    let mut group = c.benchmark_group("trie/insert_lookup_tokens");

    group.bench_function("trie", |b| {
        b.iter_batched(
            || {
                Trie::new(TrieOptions {
                    sigma_size: TOKEN_SIGMA,
                    idx: |token: &u8| *token as usize,
                    merge_node_value: |x, y| x + y,
                })
                .expect("valid trie options")
            },
            |mut trie| {
                for (i, key) in keys.iter().enumerate() {
                    trie.try_insert(key, i as u64 + 1).expect("insert");
                }

                let mut checksum = 0_u64;
                for query in &queries {
                    if let Some(value) = trie.try_get(query).expect("get") {
                        checksum = checksum.wrapping_add(*value);
                    }
                }

                assert_eq!(checksum, expected);
                black_box(checksum);
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("std_hash_map", |b| {
        b.iter_batched(
            || HashMap::<Vec<u8>, u64>::with_capacity(keys.len()),
            |mut map| {
                for (i, key) in keys.iter().enumerate() {
                    *map.entry(key.clone()).or_insert(0) += i as u64 + 1;
                }

                let mut checksum = 0_u64;
                for query in &queries {
                    if let Some(value) = map.get(query) {
                        checksum = checksum.wrapping_add(*value);
                    }
                }

                assert_eq!(checksum, expected);
                black_box(checksum);
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    trie_common,
    bench_insert_lookup_chars,
    bench_matches_range_chars,
    bench_insert_lookup_tokens
);
criterion_main!(trie_common);
