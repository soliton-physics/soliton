//! Soliton Physics pillar for currents and magnetism (design 019).
//!
//! NCERT class 12: the magnetic field of a current, the force on a current in a field, and electromagnetic
//! induction. Fields are summed from the Biot–Savart law over the wire itself, and every emf is found from how
//! the flux actually changed from one moment to the next, so each textbook formula has something to agree with.

pub mod biot_savart;
pub mod currents;
pub mod induction;
pub mod picture;
pub mod scenario;

pub use picture::{Lesson, Picture};
pub use scenario::{catalogue, find, Params, Run};

/// The permeability of free space, `μ₀`, T·m/A.
pub const MU0: f64 = 4.0e-7 * std::f64::consts::PI;
/// The acceleration of free fall, m/s².
pub const G: f64 = 9.8;
