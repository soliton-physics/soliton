//! Soliton Physics statistical mechanics pillar (design 016).
//!
//! Systems that are followed by chance and not by Newton's laws: a magnet as a lattice of spins flipped by the
//! Metropolis rule, an area found by throwing points at it, a crowd of random walkers. What they have in common
//! is that the answer is an average over many random trials, and its error falls as one over the square root of
//! their number.

mod ising;
mod sampling;
pub mod scenario;
pub mod scene;

pub use ising::{onsager_energy, onsager_magnetisation, Ising, CRITICAL_TEMPERATURE};
pub use sampling::{Darts, Walkers};
