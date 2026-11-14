//! Soliton Physics thermal pillar (design 014).
//!
//! A gas as what it is: molecules flying about in a box, bouncing off the walls and off one another. Nothing
//! thermodynamic is put in. Temperature is the molecules' mean kinetic energy, pressure is counted from their
//! blows on the walls, work is what a moving piston gives them one collision at a time, and heat is what a wall
//! held at a temperature hands over. The gas laws, the Maxwell distribution and the Carnot efficiency are what
//! comes out.
//!
//! Units are those of molecular dynamics: nanometres, picoseconds, atomic mass units, kelvin. Speeds in nm/ps are
//! km/s, so nitrogen at room temperature moves at about 0.5.

// Positions and velocities are triples indexed by axis, and loops over the axis read better than iterators
// zipped three deep.
#![allow(clippy::needless_range_loop)]

mod gas;
mod rng;
pub mod scenario;
pub mod scene;

pub use gas::{Gas, Molecule, Wall, WALLS};
pub use rng::Rng;

/// Boltzmann's constant in amu·nm²/ps² per kelvin.
pub const BOLTZMANN: f64 = 0.008_314_462_618;
/// One unit of energy, amu·nm²/ps², in joules.
pub const JOULES: f64 = 1.660_539_066_6e-21;
/// One unit of pressure, amu/(nm·ps²), in pascals.
pub const PASCALS: f64 = 1.660_539_066_6e6;
/// One standard atmosphere in pascals.
pub const ATMOSPHERE: f64 = 101_325.0;
/// One nm/ps in m/s.
pub const METRES_PER_SECOND: f64 = 1000.0;
