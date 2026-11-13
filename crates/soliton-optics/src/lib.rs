//! Soliton Physics ray optics pillar (design 013).
//!
//! Light as rays in a plane: straight lines that turn only where they meet something. Every turn is worked out
//! from the law of reflection or Snell's law at the point the ray actually strikes, with no small-angle
//! approximation, so a wide beam shows the aberration a real mirror or lens has. Images are found the way the
//! textbook defines them, from rays close to the axis.

mod bench;
mod curve;
pub mod scenario;
pub mod scene;

pub use bench::{crossing, Bench, Element, End, Event, EventKind, Image, Path, Ray};
pub use curve::Curve;
pub use soliton_core::Vec2;
