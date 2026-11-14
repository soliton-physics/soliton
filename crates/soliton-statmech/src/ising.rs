use soliton_thermal::Rng;

/// The temperature at which the square-lattice Ising model loses its magnetisation, in units of `J/k`:
/// Onsager's `2 / ln(1 + √2)`.
pub const CRITICAL_TEMPERATURE: f64 = 2.269_185_314_213_022;

/// The magnetisation per spin of an infinite lattice at temperature `t` and no field, by Onsager and Yang:
/// `(1 − sinh⁻⁴(2/T))^{1/8}` below the critical temperature and nothing above it.
pub fn onsager_magnetisation(t: f64) -> f64 {
    if t <= 0.0 {
        return 1.0;
    }
    if t >= CRITICAL_TEMPERATURE {
        return 0.0;
    }
    (1.0 - (2.0 / t).sinh().powi(-4)).max(0.0).powf(0.125)
}

/// The complete elliptic integral of the first kind, `K(k)`, by the arithmetic-geometric mean.
fn elliptic_k(k: f64) -> f64 {
    let (mut a, mut b) = (1.0, (1.0 - k * k).max(0.0).sqrt());
    for _ in 0..40 {
        if (a - b).abs() < 1e-15 * a {
            break;
        }
        (a, b) = (0.5 * (a + b), (a * b).sqrt());
    }
    std::f64::consts::FRAC_PI_2 / a
}

/// The energy per spin of an infinite lattice at temperature `t` and no field, in units of `J`, by Onsager.
pub fn onsager_energy(t: f64) -> f64 {
    if t <= 0.0 {
        return -2.0;
    }
    let beta = 1.0 / t;
    // Written with tanh and 1/cosh, which stay finite however cold it gets.
    let (lean_on, thin) = ((2.0 * beta).tanh(), 1.0 / (2.0 * beta).cosh());
    let kappa = 2.0 * lean_on * thin;
    let lean = 2.0 * lean_on * lean_on - 1.0;
    // At the critical point `K` is infinite and its coefficient zero; the product is zero.
    let term = if lean.abs() < 1e-12 {
        0.0
    } else {
        std::f64::consts::FRAC_2_PI * lean * elliptic_k(kappa.min(1.0 - 1e-15))
    };
    -(1.0 + term) / lean_on
}

/// A square lattice of spins, each up or down, each coupled to its four neighbours with energy `−J` when they
/// agree and `+J` when they do not, in a field `h`. The lattice wraps round at its edges. Temperatures are in
/// units of `J/k` and the field in units of `J`.
#[derive(Clone, Debug)]
pub struct Ising {
    pub size: usize,
    pub spins: Vec<i8>,
    pub temperature: f64,
    pub field: f64,
    /// Sweeps made so far: one sweep is as many attempted flips as there are spins.
    pub sweeps: u64,
    rng: Rng,
}

impl Ising {
    /// A lattice `size` on a side, all spins up if `ordered` and at random if not.
    pub fn new(size: usize, temperature: f64, field: f64, ordered: bool, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let spins = (0..size * size)
            .map(|_| {
                if ordered || rng.uniform() < 0.5 {
                    1
                } else {
                    -1
                }
            })
            .collect();
        Self {
            size,
            spins,
            temperature,
            field,
            sweeps: 0,
            rng,
        }
    }

    /// The sum of the four neighbours of the spin at `(row, column)`.
    fn neighbours(&self, row: usize, column: usize) -> i32 {
        let n = self.size;
        let at = |r: usize, c: usize| i32::from(self.spins[r * n + c]);
        at((row + 1) % n, column)
            + at((row + n - 1) % n, column)
            + at(row, (column + 1) % n)
            + at(row, (column + n - 1) % n)
    }

    /// One sweep of the Metropolis rule: pick a spin at random and propose to flip it. If that lowers the
    /// energy, flip it. If it raises the energy by `ΔE`, flip it only with probability `e^{−ΔE/kT}`. Repeated
    /// often enough this visits states in proportion to their Boltzmann weights, which is all that an average
    /// at a temperature needs.
    pub fn sweep(&mut self) {
        let n = self.size;
        let beta = if self.temperature > 0.0 {
            1.0 / self.temperature
        } else {
            f64::INFINITY
        };
        for _ in 0..n * n {
            let site = ((self.rng.uniform() * (n * n) as f64) as usize).min(n * n - 1);
            let (row, column) = (site / n, site % n);
            let spin = f64::from(self.spins[site]);
            let cost = 2.0 * spin * (f64::from(self.neighbours(row, column)) + self.field);
            if cost <= 0.0 || self.rng.uniform() < (-beta * cost).exp() {
                self.spins[site] = -self.spins[site];
            }
        }
        self.sweeps += 1;
    }

    /// Mean spin: `+1` with every spin up, `−1` with every one down, zero when they are evenly mixed.
    pub fn magnetisation(&self) -> f64 {
        self.spins.iter().map(|&s| f64::from(s)).sum::<f64>() / self.spins.len() as f64
    }

    /// Energy per spin in units of `J`: `−2` with all spins aligned and no field.
    pub fn energy(&self) -> f64 {
        let n = self.size;
        let mut bonds = 0.0;
        let mut up = 0.0;
        for row in 0..n {
            for column in 0..n {
                let spin = f64::from(self.spins[row * n + column]);
                // Each bond counted once: the neighbour to the right and the one below.
                let right = f64::from(self.spins[row * n + (column + 1) % n]);
                let below = f64::from(self.spins[((row + 1) % n) * n + column]);
                bonds -= spin * (right + below);
                up += spin;
            }
        }
        (bonds - self.field * up) / (n * n) as f64
    }
}
