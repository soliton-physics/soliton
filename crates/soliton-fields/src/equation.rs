use crate::spectral::SpectralSpace;
use num_complex::Complex64;

/// A conserved quantity evaluated on the current state.
#[derive(Debug, Clone, PartialEq)]
pub struct Invariant {
    pub name: &'static str,
    pub value: f64,
}

/// An evolution equation `u_t = L u + N(u)` with `L` diagonal in Fourier space.
pub trait SemiLinear {
    /// Fourier symbol `L(k)` of the linear operator.
    fn linear_symbol(&self, wavenumber: f64) -> Complex64;

    /// Writes the spectrum of `N(u)` into `out`, given the spectrum of `u`.
    /// `out` may be used freely as scratch space.
    fn nonlinear(&self, space: &mut SpectralSpace, spectrum: &[Complex64], out: &mut [Complex64]);

    /// Conserved quantities of the continuous equation, for monitoring accuracy.
    fn invariants(&self, space: &mut SpectralSpace, spectrum: &[Complex64]) -> Vec<Invariant>;
}
