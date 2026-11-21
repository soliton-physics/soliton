//! Types shared by every pillar of the Soliton Physics engine.

mod invariant;
mod limits;
mod simulate;
mod vec2;

pub use invariant::Invariant;
pub use limits::typed_limits;
pub use simulate::Simulate;
pub use vec2::Vec2;
