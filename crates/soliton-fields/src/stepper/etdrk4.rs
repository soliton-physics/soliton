use super::Stepper;
use crate::equation::SemiLinear;
use crate::spectral::SpectralSpace;
use num_complex::Complex64;
use std::f64::consts::TAU;

/// Fourth-order exponential time differencing Runge–Kutta (Cox & Matthews 2002),
/// with the coefficient functions evaluated by a full-circle contour mean (after Kassam & Trefethen 2005),
/// which stays exact for complex linear symbols.
pub struct Etdrk4 {
    dt: f64,
    exp_full: Vec<Complex64>,
    exp_half: Vec<Complex64>,
    q: Vec<Complex64>,
    f1: Vec<Complex64>,
    f2: Vec<Complex64>,
    f3: Vec<Complex64>,
    stages: [Vec<Complex64>; 6],
}

const CONTOUR_POINTS: usize = 32;

impl Etdrk4 {
    pub fn new<E: SemiLinear>(equation: &E, space: &SpectralSpace, dt: f64) -> Self {
        assert!(dt > 0.0, "time step must be positive");
        let n = space.points();
        let mut this = Self {
            dt,
            exp_full: Vec::with_capacity(n),
            exp_half: Vec::with_capacity(n),
            q: Vec::with_capacity(n),
            f1: Vec::with_capacity(n),
            f2: Vec::with_capacity(n),
            f3: Vec::with_capacity(n),
            stages: Default::default(),
        };
        for &k in space.wavenumbers() {
            this.push_coefficients(dt * equation.linear_symbol(k));
        }
        this.stages = std::array::from_fn(|_| vec![Complex64::default(); n]);
        this
    }

    fn push_coefficients(&mut self, z: Complex64) {
        self.exp_full.push(z.exp());
        self.exp_half.push((0.5 * z).exp());

        let (mut q, mut f1, mut f2, mut f3) = (
            Complex64::default(),
            Complex64::default(),
            Complex64::default(),
            Complex64::default(),
        );
        for j in 0..CONTOUR_POINTS {
            let angle = TAU * (j as f64 + 0.5) / CONTOUR_POINTS as f64;
            let w = z + Complex64::from_polar(1.0, angle);
            let ew = w.exp();
            let w3 = w * w * w;
            q += ((0.5 * w).exp() - 1.0) / w;
            f1 += (-4.0 - w + ew * (4.0 - 3.0 * w + w * w)) / w3;
            f2 += (2.0 + w + ew * (w - 2.0)) / w3;
            f3 += (-4.0 - 3.0 * w - w * w + ew * (4.0 - w)) / w3;
        }
        let scale = self.dt / CONTOUR_POINTS as f64;
        self.q.push(q * scale);
        self.f1.push(f1 * scale);
        self.f2.push(f2 * scale);
        self.f3.push(f3 * scale);
    }
}

impl Stepper for Etdrk4 {
    fn dt(&self) -> f64 {
        self.dt
    }

    fn step<E: SemiLinear>(
        &mut self,
        equation: &E,
        space: &mut SpectralSpace,
        v: &mut [Complex64],
    ) {
        let [nv, a, na, b, nb, c] = &mut self.stages;

        equation.nonlinear(space, v, nv);
        combine(a, v, &self.exp_half, nv, &self.q);

        equation.nonlinear(space, a, na);
        combine(b, v, &self.exp_half, na, &self.q);

        equation.nonlinear(space, b, nb);
        for i in 0..v.len() {
            c[i] = self.exp_half[i] * a[i] + self.q[i] * (2.0 * nb[i] - nv[i]);
        }

        equation.nonlinear(space, c, b);
        let nc = b;
        for i in 0..v.len() {
            v[i] = self.exp_full[i] * v[i]
                + self.f1[i] * nv[i]
                + 2.0 * self.f2[i] * (na[i] + nb[i])
                + self.f3[i] * nc[i];
        }
    }
}

#[inline]
fn combine(
    out: &mut [Complex64],
    v: &[Complex64],
    e: &[Complex64],
    n: &[Complex64],
    q: &[Complex64],
) {
    for i in 0..out.len() {
        out[i] = e[i] * v[i] + q[i] * n[i];
    }
}
