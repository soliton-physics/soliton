//! Soliton Physics mechanics pillar.
//!
//! Planar rigid bodies under explicit forces and exact constraints, integrated with velocity Verlet and impulses.
//! The engine reports every force it applies, so a free-body diagram is an output rather than a drawing.
//! Designs: `design/001-mechanics-core.md`, `design/002-constraints-and-ropes.md`, `design/003-contact-and-friction.md`.

mod body;
mod constraint;
mod contact;
mod energy;
mod event;
pub mod exact;
mod finish;
mod force;
mod integrator;
mod record;
mod rope;
pub mod scenario;
pub mod scene;
mod shape;
mod simulation;
mod solver;
mod world;

pub use body::{Bodies, Body, BodyId, BodyKind, Material};
pub use constraint::{
    Constraint, Entry, FixedAngle, PinJoint, Rod, Row, RowKind, Rows, Sketch, Slot,
};
pub use contact::Contact;
pub use energy::EnergyLedger;
pub use event::{Event, EventKind};
pub use finish::{Finish, Finished, Trigger};
pub use force::{
    Anchor, Applied, Driving, Force, ForceLabel, ForceSample, Grab, Gravitation, LinearDrag, Loads,
    QuadraticDrag, Spring, Torque,
};
pub use integrator::{Integrator, Verlet};
pub use record::{BodyRecord, Recorder};
pub use rope::{Rope, RopeNode, Wrap};
pub use shape::{Circle, HalfPlane, Polygon, Shape};
pub use simulation::Simulation;
pub use soliton_core::{Invariant, Simulate, Vec2};
pub use world::World;
