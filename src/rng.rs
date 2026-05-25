//! Deterministic RNG wrapper — the ONLY source of randomness in the consensus core.

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Deterministic RNG. The ONLY source of randomness in the consensus core.
pub struct DetRng { inner: ChaCha20Rng }

impl DetRng {
    pub fn from_seed(seed: u64) -> Self {
        // expand u64 seed to the 32-byte ChaCha seed deterministically
        let mut s = [0u8; 32];
        s[..8].copy_from_slice(&seed.to_le_bytes());
        DetRng { inner: ChaCha20Rng::from_seed(s) }
    }
    pub fn next_u64(&mut self) -> u64 { self.inner.next_u64() }
    /// Inclusive range [lo, hi]. Rejection-free modulo is fine here (bias negligible for our small ranges; documented).
    pub fn gen_range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(hi >= lo);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    /// ChaCha stream position (block counter) — recordable in a Frame for re-execution.
    pub fn position(&self) -> u128 { self.inner.get_word_pos() }
    pub fn set_position(&mut self, pos: u128) { self.inner.set_word_pos(pos) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_seed_same_stream() {
        let mut a = DetRng::from_seed(42);
        let mut b = DetRng::from_seed(42);
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(DetRng::from_seed(43).next_u64(), a_first(42));
    }
    fn a_first(s: u64) -> u64 { DetRng::from_seed(s).next_u64() }

    #[test]
    fn range_is_inclusive_and_bounded() {
        let mut r = DetRng::from_seed(1);
        for _ in 0..1000 { let v = r.gen_range_i64(5, 10); assert!((5..=10).contains(&v)); }
    }

    #[test]
    fn position_roundtrips() {
        let mut r = DetRng::from_seed(7);
        let _ = r.next_u64();
        let pos = r.position();
        let next = r.next_u64();
        let mut r2 = DetRng::from_seed(7);
        r2.set_position(pos);
        assert_eq!(r2.next_u64(), next);
    }

    // New: gen_range_i64 where lo == hi must always return lo.
    #[test]
    fn gen_range_degenerate_lo_eq_hi() {
        let mut r = DetRng::from_seed(99);
        for v in [-1_000_000i64, 0, 1, 42, i64::MAX / 2] {
            assert_eq!(r.gen_range_i64(v, v), v, "lo==hi must always return lo");
        }
    }

    // New: a wider range stays entirely in-bounds across many draws.
    #[test]
    fn gen_range_wider_range_stays_in_bounds() {
        let mut r = DetRng::from_seed(17);
        for _ in 0..2000 {
            let v = r.gen_range_i64(-500, 500);
            assert!((-500..=500).contains(&v));
        }
    }

    // New: set_position to an arbitrary earlier position reproduces the exact subsequent stream.
    #[test]
    fn set_position_reproduces_stream() {
        let mut r = DetRng::from_seed(5);
        // consume some values to get to an interesting position
        for _ in 0..50 { let _ = r.next_u64(); }
        let checkpoint = r.position();
        // record 10 values from this point
        let expected: Vec<u64> = (0..10).map(|_| r.next_u64()).collect();
        // consume more values so the stream is well past the checkpoint
        for _ in 0..30 { let _ = r.next_u64(); }
        // restore and verify the stream is identical
        r.set_position(checkpoint);
        let replayed: Vec<u64> = (0..10).map(|_| r.next_u64()).collect();
        assert_eq!(expected, replayed, "set_position must reproduce the exact stream");
    }
}
