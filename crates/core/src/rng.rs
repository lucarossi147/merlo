//! A tiny deterministic PRNG (xorshift64*), so drills are reproducible in tests
//! and the core needs no platform entropy.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        // xorshift must never hold zero.
        Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let (mut a, mut b) = (Rng::new(42), Rng::new(42));
        for _ in 0..1000 {
            let x = a.unit();
            assert_eq!(x.to_bits(), b.unit().to_bits());
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn zero_seed_is_usable() {
        let mut r = Rng::new(0);
        assert_ne!(r.next_u64(), r.next_u64());
    }

    #[test]
    fn roughly_uniform() {
        let mut r = Rng::new(7);
        let mean = (0..10_000).map(|_| r.unit()).sum::<f64>() / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "{mean}");
    }
}
