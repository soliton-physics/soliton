//! Soliton Physics mechanics pillar.
//!
//! Planar rigid bodies under explicit forces, integrated with velocity Verlet. The engine reports every force it
//! applies, so a free-body diagram is an output rather than a drawing. Design: `design/001-mechanics-core.md`.

mod body;
mod energy;
mod event;
pub mod exact;
mod force;
mod integrator;
mod record;
mod shape;
mod simulation;
mod world;

pub use body::{Bodies, Body, BodyId, BodyKind, Material};
pub use energy::EnergyLedger;
pub use event::{Event, EventKind};
pub use force::{
    Anchor, Applied, Driving, Force, ForceLabel, ForceSample, LinearDrag, Loads, QuadraticDrag,
    Spring, Torque,
};
pub use integrator::{Integrator, Verlet};
pub use record::{BodyRecord, Recorder};
pub use shape::{Circle, HalfPlane, Polygon, Shape};
pub use simulation::Simulation;
pub use soliton_core::{Invariant, Simulate, Vec2};
pub use world::World;
