use crate::body::{Bodies, Body, BodyId};
use crate::constraint::{Constraint, Sketch};
use crate::contact::{detect, Contact};
use crate::electromagnetism::{field_lines, Distribution, FieldInfo, Seed};
use crate::event::{Event, EventKind};
use crate::force::{
    Force, ForceSample, Grab, Gravity, Instant, Loads, WorkClass, GRAB_FREQUENCY, GRAB_STEP_LIMIT,
};
use crate::magnetism::poles;
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
    grab: Option<Grab>,
    grab_frequency: f64,
    /// The hand changed since the loads were last evaluated, so they must not be reused for the next opening kick.
    grab_changed: bool,
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
            grab: None,
            grab_frequency: GRAB_FREQUENCY,
            grab_changed: false,
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

    /// Takes hold of a dynamic body at a world point and pulls that point towards `target` (design 008).
    /// Returns false, holding nothing, for a static body.
    pub fn grab(&mut self, body: BodyId, world_point: Vec2, target: Vec2) -> bool {
        if !self.bodies.is_dynamic(body) {
            return false;
        }
        let local = (world_point - self.bodies.position(body)).rotated(-self.bodies.angle(body));
        self.grab = Some(Grab {
            body,
            local,
            target,
        });
        self.grab_changed = true;
        true
    }

    /// Moves the hand; does nothing when no body is held.
    pub fn move_grab(&mut self, target: Vec2) {
        if let Some(grab) = &mut self.grab {
            grab.target = target;
            self.grab_changed = true;
        }
    }

    pub fn release(&mut self) {
        if self.grab.take().is_some() {
            self.grab_changed = true;
        }
    }

    /// Tells the world its time step, which bounds how stiff the hand's spring may be.
    pub(crate) fn set_time_step(&mut self, dt: f64) {
        self.grab_frequency = GRAB_FREQUENCY.min(GRAB_STEP_LIMIT / dt);
    }

    pub fn grabbed(&self) -> Option<Grab> {
        self.grab
    }

    /// The dynamic body under a world point, the last added winning; failing that, the nearest one whose centre is
    /// within `slop` metres, so a small particle is still easy to pick.
    pub fn body_at(&self, point: Vec2, slop: f64) -> Option<BodyId> {
        let b = &self.bodies;
        let dynamic = || b.ids().filter(|&id| b.is_dynamic(id));
        let inside = |id: BodyId| {
            let local = (point - b.position(id)).rotated(-b.angle(id));
            match b.shape(id) {
                crate::shape::Shape::Circle(c) => local.length() <= c.radius,
                crate::shape::Shape::Polygon(p) => {
                    let v = p.vertices();
                    (0..v.len()).all(|i| (v[(i + 1) % v.len()] - v[i]).cross(local - v[i]) >= 0.0)
                }
                crate::shape::Shape::HalfPlane(_) => false,
            }
        };
        dynamic().filter(|&id| inside(id)).last().or_else(|| {
            dynamic()
                .map(|id| (id, (point - b.position(id)).length()))
                .filter(|&(_, distance)| distance <= slop)
                .min_by(|x, y| x.1.total_cmp(&y.1))
                .map(|(id, _)| id)
        })
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

    /// The electric field at a point, N/C: of every charge and every uniform field there.
    pub fn electric_field(&self, point: Vec2, time: f64) -> Vec2 {
        self.forces
            .iter()
            .map(|f| f.electric_field(&self.bodies, point, time))
            .fold(Vec2::ZERO, |sum, e| sum + e)
    }

    /// The electric potential at a point, V, with zero at infinity for charges and at the origin for a uniform
    /// field. A field held between plates or alternating has no potential and adds nothing here.
    pub fn electric_potential(&self, point: Vec2) -> f64 {
        self.forces
            .iter()
            .filter_map(|f| f.electric_potential(&self.bodies, point))
            .sum()
    }

    /// The magnetic field out of the page at a point, T.
    pub fn magnetic_field(&self, point: Vec2) -> f64 {
        self.forces.iter().map(|f| f.magnetic_field(point)).sum()
    }

    /// The uniform fields in the world, for drawing.
    pub fn fields(&self, time: f64) -> Vec<FieldInfo> {
        self.forces.iter().filter_map(|f| f.field(time)).collect()
    }

    /// The magnetic field in the plane of the page at a point, T: of every bar magnet and of the background.
    pub fn magnetic_field_in_plane(&self, point: Vec2) -> Vec2 {
        self.plane_field(point, None)
    }

    /// The magnetic field in the plane at the centre of a body, T, leaving out the body's own: what a compass
    /// there is reading.
    pub fn magnetic_field_on(&self, id: BodyId) -> Vec2 {
        self.plane_field(self.bodies.position(id), Some(id))
    }

    fn plane_field(&self, point: Vec2, skip: Option<BodyId>) -> Vec2 {
        self.forces
            .iter()
            .map(|f| f.magnetic_field_in_plane(&self.bodies, point, skip))
            .fold(Vec2::ZERO, |sum, b| sum + b)
    }

    /// The electric flux out of a sphere centred in the plane, N·m²/C, summed over its surface. Gauss's law
    /// says it is the charge inside over ε₀; this is the other side of that equation. Uniform fields pass
    /// straight through a closed surface and add nothing.
    pub fn electric_flux(&self, centre: Vec2, radius: f64) -> f64 {
        self.forces
            .iter()
            .map(|f| f.flux_through_sphere(&self.bodies, centre, radius))
            .sum()
    }

    /// The fixed wires, sheets and shells of charge in the world, for drawing.
    pub fn distributions(&self) -> Vec<Distribution> {
        self.forces
            .iter()
            .flat_map(|f| f.distributions().iter().copied())
            .collect()
    }

    /// Field lines of the charges in the world (see [`field_lines`]), `per_charge` from the largest. Where there
    /// is a fixed wire, sheet or shell of charge the lines are those of the fixed charge alone: the field a
    /// bead moving through it feels, without the bead's own drawn over it.
    pub fn field_lines(
        &self,
        per_charge: usize,
        centre: Vec2,
        reach: f64,
        time: f64,
    ) -> Vec<Vec<Vec2>> {
        let fixed = self.distributions();
        if fixed.is_empty() {
            let b = &self.bodies;
            let seeds: Vec<Seed> = b
                .ids()
                .filter(|&id| b.charge(id) != 0.0)
                .map(|id| Seed::point(b.position(id), b.charge(id)))
                .collect();
            return field_lines(
                |p| self.electric_field(p, time),
                &seeds,
                per_charge,
                centre,
                reach,
            );
        }
        let seeds: Vec<Seed> = fixed
            .into_iter()
            .filter_map(|distribution| match distribution {
                Distribution::Wire { at, density } => Some(Seed::point(at, density)),
                Distribution::Shell {
                    centre,
                    radius,
                    charge,
                } => Some(Seed {
                    at: centre,
                    strength: charge,
                    radius,
                }),
                // The lines of a sheet are parallel and start all along it; front ends draw them as arrows.
                Distribution::Sheet { .. } => None,
            })
            .collect();
        let nobody = Bodies::default();
        let field = |p| {
            self.forces
                .iter()
                .map(|f| f.electric_field(&nobody, p, time))
                .fold(Vec2::ZERO, |sum, e| sum + e)
        };
        field_lines(field, &seeds, per_charge, centre, reach)
    }

    /// The magnetic field in the plane, T, of the fixed magnets and the background alone: the field a compass
    /// is put into, without the compass's own.
    pub fn fixed_magnetic_field(&self, point: Vec2) -> Vec2 {
        let fixed = self.fixed_magnets();
        self.forces
            .iter()
            .map(|f| f.magnetic_field_in_plane(&fixed, point, None))
            .fold(Vec2::ZERO, |sum, b| sum + b)
    }

    /// The bodies with every magnet that can move taken off them.
    fn fixed_magnets(&self) -> Bodies {
        let mut fixed = self.bodies.clone();
        for id in self.bodies.ids().filter(|&id| self.bodies.is_dynamic(id)) {
            fixed.magnets[id.index()] = None;
        }
        fixed
    }

    /// Lines of the magnetic field of the fixed magnets in the plane, from the north poles round to the south,
    /// `per_pole` from the strongest. Inside a magnet they are not the true field, which runs the other way
    /// there.
    pub fn magnetic_field_lines(
        &self,
        per_pole: usize,
        centre: Vec2,
        reach: f64,
    ) -> Vec<Vec<Vec2>> {
        let fixed = self.fixed_magnets();
        let seeds: Vec<Seed> = poles(&fixed)
            .into_iter()
            .map(|p| Seed::point(p.at, p.strength))
            .collect();
        let field = |p| {
            self.forces
                .iter()
                .map(|f| f.magnetic_field_in_plane(&fixed, p, None))
                .fold(Vec2::ZERO, |sum, b| sum + b)
        };
        field_lines(field, &seeds, per_pole, centre, reach)
    }

    pub(crate) fn has_velocity_forces(&self) -> bool {
        self.has_velocity_forces || self.grab.is_some()
    }

    /// Whether the loads left by the previous step cannot open the next one: a force switches at a given time, or
    /// the hand moved in between.
    pub(crate) fn needs_fresh_loads(&mut self) -> bool {
        std::mem::take(&mut self.grab_changed) || self.has_windowed_forces
    }

    /// Recomputes every explicit force at the current state for the given half-step.
    pub(crate) fn evaluate_loads(&mut self, instant: Instant) {
        self.loads.clear();
        for force in &self.forces {
            force.apply(&self.bodies, instant, &mut self.loads);
        }
        if let Some(grab) = &self.grab {
            grab.apply(&self.bodies, &mut self.loads, self.grab_frequency);
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
        let explicit =
            |id: BodyId| self.loads.forces[id.index()] * self.bodies.inverse_masses[id.index()];
        let accelerations: Vec<f64> = self
            .contacts
            .iter()
            .map(|c| c.normal.dot(explicit(c.b) - explicit(c.a)))
            .collect();
        self.impact_loss += self.solver.solve_velocities(
            &self.constraints,
            &self.contacts,
            &mut self.bodies,
            &mut self.loads,
            dt,
            time,
            threshold,
            &accelerations,
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
        // Contacts and taut ropes present at the start are remembered, not announced; only a slip is news.
        self.events
            .retain(|e| matches!(e.kind, EventKind::FrictionSlip { .. }));
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
