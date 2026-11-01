use crate::fourier::Fourier;
use crate::grid::PeriodicGrid;
use num_complex::Complex64;

/// A periodic grid together with its Fourier machinery.
///
/// Spectral coefficients follow the unnormalised forward-FFT convention,
/// `û_k = Σ_j u_j e^{-i k x_j}`, so physical fields are recovered with [`Self::to_physical`].
pub struct SpectralSpace {
    grid: PeriodicGrid,
    wavenumbers: Vec<f64>,
    fourier: Fourier,
}

impl SpectralSpace {
    pub fn new(grid: PeriodicGrid) -> Self {
        Self {
            grid,
            wavenumbers: grid.wavenumbers(),
            fourier: Fourier::new(grid.points()),
        }
    }

    pub fn grid(&self) -> &PeriodicGrid {
        &self.grid
    }

    pub fn points(&self) -> usize {
        self.grid.points()
    }

    pub fn wavenumbers(&self) -> &[f64] {
        &self.wavenumbers
    }

    pub fn to_spectral(&mut self, field: &mut [Complex64]) {
        self.fourier.forward(field);
    }

    pub fn to_physical(&mut self, spectrum: &mut [Complex64]) {
        self.fourier.inverse(spectrum);
    }

    /// Writes the physical field of `spectrum` into `out`.
    pub fn physical_into(&mut self, spectrum: &[Complex64], out: &mut [Complex64]) {
        out.copy_from_slice(spectrum);
        self.to_physical(out);
    }

    /// Spectral derivative of the given order: multiplies by `(ik)^order`.
    pub fn differentiate(&self, spectrum: &mut [Complex64], order: u32) {
        for (z, &k) in spectrum.iter_mut().zip(&self.wavenumbers) {
            *z *= Complex64::new(0.0, k).powu(order);
        }
    }

    /// Trapezoidal (exact for periodic smooth data) integral of a physical field.
    pub fn integrate(&self, field: impl Iterator<Item = f64>) -> f64 {
        self.grid.spacing() * field.sum::<f64>()
    }
}
