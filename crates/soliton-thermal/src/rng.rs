/// A small, fast, seedable random number generator (xoshiro256++), so that a run can be repeated exactly.
#[derive(Clone, Debug)]
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 spreads any seed, zero included, over the whole state.
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            x ^ (x >> 31)
        };
        Self {
            state: [next(), next(), next(), next()],
        }
    }

    fn next(&mut self) -> u64 {
        let s = &mut self.state;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform in `(0, 1]`: never zero, so its logarithm is safe.
    pub fn uniform(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 1.0) / (1u64 << 53) as f64
    }

    /// Normal with mean 0 and variance 1.
    pub fn normal(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (std::f64::consts::TAU * self.uniform()).cos()
    }

    /// A unit vector in a direction chosen evenly over the sphere.
    pub fn direction(&mut self) -> [f64; 3] {
        let z = 2.0 * self.uniform() - 1.0;
        let around = std::f64::consts::TAU * self.uniform();
        let across = (1.0 - z * z).max(0.0).sqrt();
        [across * around.cos(), across * around.sin(), z]
    }
}
