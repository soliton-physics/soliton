use crate::body::{Bodies, Body, BodyId};
use crate::constraint::{Constraint, Sketch};
use crate::contact::{detect, Contact};
use crate::event::Event;
use crate::force::{Force, ForceSample, Gravity, Instant, Loads, WorkClass};
use crate::solver::ConstraintSolver;
use soliton_core::Vec2;

/// Bodies, the forces and constraints acting on them, and the loads produced at the current state.
pub struct World {
    pub(crate) bodies: Bodies,
    forces: Vec<Box<dyn Force>>,
    constraints: Vec<Box<dyn Constraint>>,
    solver: ConstraintSolver,
    contacts: Vec<Contact>,
    ignored_pairs: Vec<(BodyId, BodyId)>,
    pub(crate) events: Vec<Event>,
    pub(crate) impact_loss: f64,
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
            constraints: Vec::new(),
            solver: ConstraintSolver::default(),
            contacts: Vec::new(),
            ignored_pairs: Vec::new(),
            events: Vec::new(),
            impact_loss: 0.0,
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

    /// Adds a constraint and returns its index, which events and reactions refer to.
    /// Bodies joined by a pin or rod stop colliding with each other.
    pub fn add_constraint(&mut self, constraint: impl Constraint + 'static) -> usize {
        if let Some((a, b)) = constraint.joined() {
            self.ignore_collisions(a, b);
        }
        self.constraints.push(Box::new(constraint));
        self.constraints.len() - 1
    }

    /// Prevents contact between two bodies, for example a mass that rides on a plank it is pinned to.
    pub fn ignore_collisions(&mut self, a: BodyId, b: BodyId) {
        if !self.ignored_pairs.contains(&(a, b)) && !self.ignored_pairs.contains(&(b, a)) {
            self.ignored_pairs.push((a, b));
        }
    }

    pub fn gravity(&self) -> Vec2 {
        self.gravity
    }

    pub fn bodies(&self) -> &Bodies {
        &self.bodies
    }

    pub fn constraint(&self, index: usize) -> &dyn Constraint {
        self.constraints[index].as_ref()
    }

    /// Contacts found by the last detection pass.
    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// Drawing hints for everything that is not a body: ropes, pulleys, rods, springs, pivots.
    pub fn sketches(&self) -> Vec<Sketch> {
        let mut out = Vec::new();
        for c in &self.constraints {
            out.extend(c.sketches(&self.bodies));
        }
        for f in &self.forces {
            out.extend(f.sketches(&self.bodies));
        }
        out
    }

    pub(crate) fn has_velocity_forces(&self) -> bool {
        self.has_velocity_forces
    }

    pub(crate) fn has_windowed_forces(&self) -> bool {
        self.has_windowed_forces
    }

    /// Recomputes every explicit force at the current state for the given half-step.
    pub(crate) fn evaluate_loads(&mut self, instant: Instant) {
        self.loads.clear();
        for force in &self.forces {
            force.apply(&self.bodies, instant, &mut self.loads);
        }
    }

    pub(crate) fn prepare_constraints(&mut self) {
        self.solver.prepare(&self.constraints, &self.bodies);
        self.solver.kick_predictor(&mut self.bodies);
    }

    pub(crate) fn project_positions(&mut self, dt: f64) {
        if !self.constraints.is_empty() {
            self.solver
                .project_positions(&self.constraints, &mut self.bodies, dt);
        }
    }

    pub(crate) fn detect_contacts(&mut self) {
        detect(&self.bodies, &self.ignored_pairs, &mut self.contacts);
    }

    pub(crate) fn solve_velocities(&mut self, dt: f64, time: f64) {
        let threshold = 4.0 * self.gravity.length() * dt + 1e-6;
        self.impact_loss += self.solver.solve_velocities(
            &self.constraints,
            &self.contacts,
            &mut self.bodies,
            &mut self.loads,
            dt,
            time,
            threshold,
            &mut self.events,
        );
    }

    /// Computes the reactions that hold at the initial state by solving one hypothetical step and discarding its
    /// motion. This seeds the contact predictor and the energy ledger, and gives the first recorded sample real
    /// tensions and normal forces.
    pub(crate) fn initialize_reactions(&mut self, dt: f64) {
        self.solver.settle(&self.constraints, &mut self.bodies);
        let velocities = self.bodies.velocities.clone();
        let spins = self.bodies.angular_velocities.clone();
        self.prepare_constraints();
        self.detect_contacts();
        for i in 0..self.bodies.len() {
            self.bodies.velocities[i] +=
                self.loads.forces[i] * (self.bodies.inverse_masses[i] * dt);
            self.bodies.angular_velocities[i] +=
                self.loads.torques[i] * self.bodies.inverse_inertias[i] * dt;
        }
        self.solve_velocities(dt, 0.0);
        self.bodies.velocities = velocities;
        self.bodies.angular_velocities = spins;
        self.impact_loss = 0.0;
    }

    pub(crate) fn correct_positions(&mut self) {
        if !self.contacts.is_empty() {
            self.solver
                .correct_positions(&self.constraints, &self.contacts, &mut self.bodies);
        }
    }

    /// Net acceleration including constraint and contact reactions.
    pub fn acceleration(&self, id: BodyId) -> Vec2 {
        let i = id.index();
        (self.loads.forces[i] + self.loads.reactions[i]) * self.bodies.inverse_masses[i]
    }

    pub fn angular_acceleration(&self, id: BodyId) -> f64 {
        let i = id.index();
        (self.loads.torques[i] + self.loads.reaction_torques[i]) * self.bodies.inverse_inertias[i]
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
    /// Impact impulses are excluded; their energy is accounted at the impact.
    pub(crate) fn power(&self) -> Power {
        let mut power = Power::default();
        for sample in self.loads.samples.iter().filter(|s| !s.impulsive) {
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
