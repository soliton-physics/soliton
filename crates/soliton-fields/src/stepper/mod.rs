mod etdrk4;

pub use etdrk4::Etdrk4;

use crate::equation::SemiLinear;
use crate::spectral::SpectralSpace;
use num_complex::Complex64;

/// Advances a spectral state by one fixed time step.
pub trait Stepper {
    fn dt(&self) -> f64;

    fn step<E: SemiLinear>(
        &mut self,
        equation: &E,
        space: &mut SpectralSpace,
        spectrum: &mut [Complex64],
    );
}
