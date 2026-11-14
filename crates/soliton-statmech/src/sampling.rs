use soliton_thermal::Rng;

/// Points thrown at random into a unit square, and how many land within a quarter circle drawn in it. The
/// fraction that do is `π/4`: an area found by counting.
#[derive(Clone, Debug)]
pub struct Darts {
    pub thrown: u64,
    pub inside: u64,
    /// The most recent points, for drawing: `x, y, 1 if inside else 0`.
    pub recent: Vec<[f64; 3]>,
    keep: usize,
    rng: Rng,
}

impl Darts {
    pub fn new(keep: usize, seed: u64) -> Self {
        Self {
            thrown: 0,
            inside: 0,
            recent: Vec::new(),
            keep,
            rng: Rng::new(seed),
        }
    }

    pub fn throw(&mut self, count: usize) {
        for _ in 0..count {
            let (x, y) = (self.rng.uniform(), self.rng.uniform());
            let hit = x * x + y * y <= 1.0;
            self.thrown += 1;
            self.inside += u64::from(hit);
            self.recent.push([x, y, f64::from(u8::from(hit))]);
        }
        if self.recent.len() > self.keep {
            let extra = self.recent.len() - self.keep;
            self.recent.drain(..extra);
        }
    }

    /// Four times the fraction inside: the estimate of `π` so far.
    pub fn estimate(&self) -> f64 {
        if self.thrown == 0 {
            0.0
        } else {
            4.0 * self.inside as f64 / self.thrown as f64
        }
    }

    /// The error to expect in the estimate after this many throws, one standard deviation:
    /// `4 √(p(1 − p)/N)` with `p = π/4`.
    pub fn expected_error(&self) -> f64 {
        let p = std::f64::consts::FRAC_PI_4;
        if self.thrown == 0 {
            f64::INFINITY
        } else {
            4.0 * (p * (1.0 - p) / self.thrown as f64).sqrt()
        }
    }
}

/// Many walkers on a square grid, all starting at the origin, each taking one step at a time in a direction
/// chosen at random from the four.
#[derive(Clone, Debug)]
pub struct Walkers {
    pub positions: Vec<[i32; 2]>,
    pub steps: u64,
    rng: Rng,
}

impl Walkers {
    pub fn new(count: usize, seed: u64) -> Self {
        Self {
            positions: vec![[0, 0]; count],
            steps: 0,
            rng: Rng::new(seed),
        }
    }

    pub fn step(&mut self) {
        for position in &mut self.positions {
            match (self.rng.uniform() * 4.0) as u32 {
                0 => position[0] += 1,
                1 => position[0] -= 1,
                2 => position[1] += 1,
                _ => position[1] -= 1,
            }
        }
        self.steps += 1;
    }

    /// The root of the mean of the squares of the walkers' distances from where they began.
    pub fn rms_distance(&self) -> f64 {
        let sum: f64 = self
            .positions
            .iter()
            .map(|p| f64::from(p[0]).powi(2) + f64::from(p[1]).powi(2))
            .sum();
        (sum / self.positions.len().max(1) as f64).sqrt()
    }

    /// Where the crowd is on average: it stays at the origin, since no direction is favoured.
    pub fn mean_position(&self) -> [f64; 2] {
        let count = self.positions.len().max(1) as f64;
        let sum = self.positions.iter().fold([0.0, 0.0], |acc, p| {
            [acc[0] + f64::from(p[0]), acc[1] + f64::from(p[1])]
        });
        [sum[0] / count, sum[1] / count]
    }
}
