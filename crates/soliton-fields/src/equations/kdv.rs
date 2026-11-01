use crate::equation::{Invariant, SemiLinear};
use crate::spectral::SpectralSpace;
use num_complex::Complex64;

/// Korteweg–de Vries equation `u_t + 6 u u_x + u_xxx = 0`.
///
/// Written as `u_t = L u + N(u)` with `L(k) = i k³` and `N(u) = -3 (u²)_x`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Kdv;

impl SemiLinear for Kdv {
    fn linear_symbol(&self, k: f64) -> Complex64 {
        Complex64::new(0.0, k * k * k)
    }

    fn nonlinear(&self, space: &mut SpectralSpace, spectrum: &[Complex64], out: &mut [Complex64]) {
        space.physical_into(spectrum, out);
        out.iter_mut()
            .for_each(|u| *u = Complex64::new(u.re * u.re, 0.0));
        space.to_spectral(out);
        for (z, &k) in out.iter_mut().zip(space.wavenumbers()) {
            *z *= Complex64::new(0.0, -3.0 * k);
        }
    }

    fn invariants(&self, space: &mut SpectralSpace, spectrum: &[Complex64]) -> Vec<Invariant> {
        let mut u = spectrum.to_vec();
        let mut ux = spectrum.to_vec();
        space.differentiate(&mut ux, 1);
        space.to_physical(&mut u);
        space.to_physical(&mut ux);

        let mass = space.integrate(u.iter().map(|z| z.re));
        let momentum = space.integrate(u.iter().map(|z| z.re * z.re));
        let energy = space.integrate(
            u.iter()
                .zip(&ux)
                .map(|(u, ux)| u.re.powi(3) - 0.5 * ux.re * ux.re),
        );
        vec![
            Invariant {
                name: "mass",
                value: mass,
            },
            Invariant {
                name: "momentum",
                value: momentum,
            },
            Invariant {
                name: "energy",
                value: energy,
            },
        ]
    }
}
