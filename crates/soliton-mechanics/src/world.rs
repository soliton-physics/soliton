use crate::body::{Bodies, Body, BodyId};
use crate::force::{Force, ForceSample, Gravity, Instant, Loads, WorkClass};
use soliton_core::Vec2;

/// Bodies, the forces acting on them, and the loads those forces produced at the current state.
pub struct World {
    pub(crate) bodies: Bodies,
    forces: Vec<Box<dyn Force>>,
    pub(crate) loads: Loads,
    gravity: Vec2,
    has_velocity_forces: bool,
    has_windowed_forces: bool,
}

impl World {
    pub fn new(gravity: Vec2) -> Self {
        let mut world = Self {
            bodies: Bodies::default(),
            forces: Vec::new(),
            loads: Loads::default(),
            gravity,
            has_velocity_forces: false,
            has_windowed_forces: false,
        };
        world.add_force(Gravity { g: gravity });
        world
    }

    pub fn add(&mut self, body: Body) -> BodyId {
        let id = self.bodies.push(body);
        self.loads.resize(self.bodies.len());
        id
    }

    pub fn add_force(&mut self, force: impl Force + 'static) {
        self.has_velocity_forces |= force.depends_on_velocity();
        self.has_windowed_forces |= force.has_time_window();
        self.forces.push(Box::new(force));
    }

    pub fn gravity(&self) -> Vec2 {
        self.gravity
    }

    pub fn bodies(&self) -> &Bodies {
        &self.bodies
    }

    pub(crate) fn has_velocity_forces(&self) -> bool {
        self.has_velocity_forces
    }

    pub(crate) fn has_windowed_forces(&self) -> bool {
        self.has_windowed_forces
    }

    /// Recomputes every force at the current state for the given half-step.
    pub(crate) fn evaluate_loads(&mut self, instant: Instant) {
        self.loads.clear();
        for force in &self.forces {
            force.apply(&self.bodies, instant, &mut self.loads);
        }
    }

    pub fn acceleration(&self, id: BodyId) -> Vec2 {
        self.loads.forces[id.index()] * self.bodies.inverse_masses[id.index()]
    }

    pub fn angular_acceleration(&self, id: BodyId) -> f64 {
        self.loads.torques[id.index()] * self.bodies.inverse_inertias[id.index()]
    }

    /// The free-body diagram of `id` at the current state.
    pub fn forces_on(&self, id: BodyId) -> impl Iterator<Item = &ForceSample> {
        self.loads.samples.iter().filter(move |s| s.body == id)
    }

    pub fn force_samples(&self) -> &[ForceSample] {
        self.loads.samples()
    }

    pub fn kinetic_energy(&self) -> f64 {
        let b = &self.bodies;
        b.ids()
            .filter(|&id| b.is_dynamic(id))
            .map(|id| {
                let i = id.index();
                let rotational = if b.inverse_inertias[i] > 0.0 {
                    0.5 * b.inertias[i] * b.angular_velocities[i] * b.angular_velocities[i]
                } else {
                    0.0
                };
                0.5 * b.masses[i] * b.velocities[i].length_squared() + rotational
            })
            .sum()
    }

    pub fn potential_energy(&self) -> f64 {
        self.forces
            .iter()
            .map(|f| f.potential_energy(&self.bodies))
            .sum()
    }

    pub fn momentum(&self) -> Vec2 {
        let b = &self.bodies;
        b.ids()
            .filter(|&id| b.is_dynamic(id))
            .map(|id| b.velocity(id) * b.mass(id))
            .fold(Vec2::ZERO, |acc, p| acc + p)
    }

    /// Angular momentum about the origin.
    pub fn angular_momentum(&self) -> f64 {
        let b = &self.bodies;
        b.ids()
            .filter(|&id| b.is_dynamic(id))
            .map(|id| {
                let i = id.index();
                let spin = if b.inverse_inertias[i] > 0.0 {
                    b.inertias[i] * b.angular_velocities[i]
                } else {
                    0.0
                };
                b.positions[i].cross(b.velocities[i] * b.masses[i]) + spin
            })
            .sum()
    }

    /// Instantaneous power delivered by dissipative and external forces at the current state.
    pub(crate) fn power(&self) -> Power {
        let mut power = Power::default();
        for sample in &self.loads.samples {
            let rate = sample
                .force
                .dot(self.bodies.point_velocity(sample.body, sample.point))
                + sample.torque * self.bodies.angular_velocity(sample.body);
            match sample.label.work_class() {
                WorkClass::Dissipative => power.dissipative -= rate,
                WorkClass::External => power.external += rate,
                WorkClass::Conservative | WorkClass::Reaction => {}
            }
        }
        power
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Power {
    pub dissipative: f64,
    pub external: f64,
}
