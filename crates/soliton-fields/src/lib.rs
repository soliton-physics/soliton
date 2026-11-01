//! Soliton Physics engine core.
//!
//! Periodic pseudo-spectral solvers for semi-linear evolution equations of the
//! form `u_t = L u + N(u)`, where `L` is a linear operator diagonal in Fourier
//! space and `N` is a nonlinearity evaluated in physical space.

pub mod equation;
pub mod equations;
pub mod exact;
pub mod fourier;
pub mod grid;
pub mod simulation;
pub mod spectral;
pub mod stepper;

pub use num_complex::Complex64;

pub use equation::{Invariant, SemiLinear};
pub use equations::{Kdv, Nls};
pub use grid::PeriodicGrid;
pub use simulation::Simulation;
pub use spectral::SpectralSpace;
pub use stepper::{Etdrk4, Stepper};
