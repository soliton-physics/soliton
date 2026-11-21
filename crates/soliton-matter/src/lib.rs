//! Soliton Physics properties of matter pillar (design 018).
//!
//! NCERT class 11: mechanical properties of solids, mechanical properties of fluids, and thermal properties of
//! matter. Where something happens in time it is followed: a block bobs until it floats, a tank drains, a ball
//! comes to its terminal speed, heat creeps along a rod. The rest is each law evaluated, with a picture.

pub mod fluids;
pub mod heat;
pub mod picture;
pub mod scenario;
pub mod solids;

pub use picture::{Lesson, Picture};
pub use scenario::{catalogue, find, Params, Run};

/// The acceleration of free fall, m/s².
pub const G: f64 = 9.8;
/// One standard atmosphere, Pa.
pub const ATMOSPHERE: f64 = 1.013e5;
