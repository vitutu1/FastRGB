//! Lightweight deterministic xorshift64 PRNG for procedural effects.

/// Fast, dependency-free PRNG suitable for real-time particle & noise effects.
#[derive(Debug, Clone)]
pub struct FastRng {
    state: u64,
}

impl Default for FastRng {
    fn default() -> Self {
        Self::seeded(0x5EED_CAFE_BABE_1337)
    }
}

impl FastRng {
    /// Creates a new generator from a non-zero seed.
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Generates the next `u64`.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns a uniform float in `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        let top = (self.next_u64() >> 40) as u32;
        (top as f32) / ((1u32 << 24) as f32)
    }

    /// Returns a uniform float in `[min, max)`.
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Returns a uniform integer in `0..upper` (`0` if `upper == 0`).
    pub fn gen_range_usize(&mut self, upper: usize) -> usize {
        if upper == 0 {
            return 0;
        }
        (self.next_u64() as usize) % upper
    }
}
