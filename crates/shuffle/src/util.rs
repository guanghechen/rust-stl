use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static RNG_STATE: Cell<u64> = Cell::new(initial_seed());
}

fn initial_seed() -> u64 {
    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let seed = now_nanos ^ 0x9E37_79B9_7F4A_7C15;
    if seed == 0 {
        0xA076_1D64_78BD_642F
    } else {
        seed
    }
}

fn next_u64() -> u64 {
    RNG_STATE.with(|state| {
        let mut x = state.get();
        // xorshift64*
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        if x == 0 {
            x = 0xA076_1D64_78BD_642F;
        }
        state.set(x);
        x
    })
}

/// Generates a pseudo-random integer in `[0, n)` using the default RNG.
///
/// Notes:
/// - This RNG is for algorithmic utilities and tests, not cryptographic usage.
/// - If `n == 0`, returns `0`.
pub fn random_int(n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    (next_u64() as usize) % n
}

#[cfg(test)]
mod tests {
    use super::random_int;

    #[test]
    fn random_int_zero_should_return_zero() {
        assert_eq!(random_int(0), 0);
    }

    #[test]
    fn random_int_should_stay_in_range_and_vary() {
        let n = 100;
        let mut values = Vec::new();
        for _ in 0..200 {
            values.push(random_int(n));
        }

        assert!(values.iter().all(|&x| x < n));
        assert!(values.iter().any(|&x| x != values[0]));
    }
}
