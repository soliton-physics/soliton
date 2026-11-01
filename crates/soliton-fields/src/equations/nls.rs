use crate::equation::{Invariant, SemiLinear};
use crate::spectral::SpectralSpace;
use num_complex::Complex64;

/// Focusing cubic nonlinear Schrödinger equation `i ψ_t + ½ ψ_xx + |ψ|² ψ = 0`.
///
/// Written as `ψ_t = L ψ + N(ψ)` with `L(k) = -i k²/2` and `N(ψ) = i |ψ|² ψ`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Nls;

impl SemiLinear for Nls {
    fn linear_symbol(&self, k: f64) -> Complex64 {
        Complex64::new(0.0, -0.5 * k * k)
    }

    fn nonlinear(&self, space: &mut SpectralSpace, spectrum: &[Complex64], out: &mut [Complex64]) {
        space.physical_into(spectrum, out);
        out.iter_mut()
            .for_each(|psi| *psi *= Complex64::new(0.0, psi.norm_sqr()));
        space.to_spectral(out);
    }

    fn invariants(&self, space: &mut SpectralSpace, spectrum: &[Complex64]) -> Vec<Invariant> {
        let mut psi = spectrum.to_vec();
        let mut psi_x = spectrum.to_vec();
        space.differentiate(&mut psi_x, 1);
        space.to_physical(&mut psi);
        space.to_physical(&mut psi_x);

        let mass = space.integrate(psi.iter().map(|z| z.norm_sqr()));
        let momentum = space.integrate(psi.iter().zip(&psi_x).map(|(p, px)| (p.conj() * px).im));
        let energy = space.integrate(
            psi.iter()
                .zip(&psi_x)
                .map(|(p, px)| 0.5 * px.norm_sqr() - 0.5 * p.norm_sqr().powi(2)),
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
