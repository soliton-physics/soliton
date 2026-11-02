use crate::body::{Bodies, BodyId};
use crate::constraint::{Constraint, Row, RowKind, Rows};
use crate::contact::{Contact, ACTIVATION};
use crate::event::{Event, EventKind};
use crate::force::{ForceLabel, Loads};

const VELOCITY_SWEEPS: usize = 200;
const VELOCITY_TOLERANCE: f64 = 1e-12;
const POSITION_SWEEPS: usize = 50;
const POSITION_TOLERANCE: f64 = 1e-13;
const CONTACT_SWEEPS: usize = 10;
const FRICTION_WARM_START: f64 = 0.5;

/// What is remembered about a contact between steps: the whole-step impulses it delivered, whether it was an
/// impact, and its friction state.
#[derive(Clone, Copy, Debug)]
struct ContactMemory {
    a: BodyId,
    b: BodyId,
    feature: u32,
    normal_impulse: f64,
    tangent_impulse: f64,
    impulsive: bool,
    sliding: bool,
}

/// RATTLE for bilateral constraints and ropes (design 002) plus Moreau–Jean style contact impulses with Coulomb
/// friction and restitution (design 003), all solved by one projected Gauss–Seidel sweep.
#[derive(Default)]
pub(crate) struct ConstraintSolver {
    /// Constraint rows at the start of the step; their directions steer the SHAKE projection.
    start: Rows,
    /// Rows at the current positions: constraints first, then two per contact (normal, friction).
    current: Rows,
    impulses: Vec<f64>,
    position_impulses: Vec<f64>,
    taut: Vec<bool>,
    memory: Vec<ContactMemory>,
    /// How long before the end of the step each current contact's impact happened; zero for persistent contacts.
    impact_delays: Vec<f64>,
    /// Contact rows of the previous step, in `memory` order, for the predictor kick.
    previous_contact_rows: Rows,
    touching_pairs: Vec<(BodyId, BodyId)>,
    sliding_pairs: Vec<(BodyId, BodyId)>,
}

impl ConstraintSolver {
    fn rebuild(rows: &mut Rows, constraints: &[Box<dyn Constraint>], bodies: &Bodies) {
        rows.clear();
        for (index, constraint) in constraints.iter().enumerate() {
            constraint.rows(bodies, index, rows);
        }
    }

    fn contact_rows(rows: &mut Rows, contacts: &[Contact], bodies: &Bodies) {
        for (index, c) in contacts.iter().enumerate() {
            let (ma, mb) = (bodies.material(c.a), bodies.material(c.b));
            let active = c.depth >= -ACTIVATION;
            let normal_row = rows.len();
            rows.push(
                index,
                ForceLabel::Normal,
                RowKind::Normal,
                -c.depth,
                0.0,
                active,
            );
            rows.at_point(bodies, c.a, -c.normal, c.point);
            rows.at_point(bodies, c.b, c.normal, c.point);
            let kind = RowKind::Friction {
                normal: normal_row,
                static_friction: ma.static_friction.max(mb.static_friction),
                kinetic_friction: ma.kinetic_friction.max(mb.kinetic_friction),
            };
            let tangent = c.normal.perp();
            rows.push(index, ForceLabel::Friction, kind, 0.0, 0.0, active);
            rows.at_point(bodies, c.a, -tangent, c.point);
            rows.at_point(bodies, c.b, tangent, c.point);
        }
    }

    fn effective_mass(rows: &Rows, bodies: &Bodies, row: &Row) -> f64 {
        rows.entries_of(row)
            .iter()
            .map(|e| {
                let i = e.body.index();
                bodies.inverse_masses[i] * e.linear.length_squared()
                    + bodies.inverse_inertias[i] * e.angular * e.angular
            })
            .sum()
    }

    fn relative_velocity(rows: &Rows, bodies: &Bodies, row: &Row) -> f64 {
        rows.entries_of(row)
            .iter()
            .map(|e| {
                e.linear.dot(bodies.velocity(e.body)) + e.angular * bodies.angular_velocity(e.body)
            })
            .sum()
    }

    fn apply_impulse(rows: &Rows, bodies: &mut Bodies, row: &Row, impulse: f64) {
        for e in rows.entries_of(row) {
            let i = e.body.index();
            bodies.velocities[i] += e.linear * (bodies.inverse_masses[i] * impulse);
            bodies.angular_velocities[i] += e.angular * bodies.inverse_inertias[i] * impulse;
        }
    }

    fn shift_positions(rows: &Rows, bodies: &mut Bodies, row: &Row, shift: f64) {
        for e in rows.entries_of(row) {
            let i = e.body.index();
            bodies.positions[i] += e.linear * (bodies.inverse_masses[i] * shift);
            bodies.angles[i] += e.angular * bodies.inverse_inertias[i] * shift;
        }
    }

    /// Captures the constraint gradients at the start of a step.
    pub(crate) fn prepare(&mut self, constraints: &[Box<dyn Constraint>], bodies: &Bodies) {
        Self::rebuild(&mut self.start, constraints, bodies);
        let n = self.start.len();
        self.position_impulses.resize(n, 0.0);
        self.position_impulses.iter_mut().for_each(|p| *p = 0.0);
    }

    /// SHAKE: moves bodies along the step-start `M⁻¹ Jᵀ` until the constraints hold at the new positions, and
    /// gives the half-step velocities the matching correction. The shifts are kept as impulses for the reaction report.
    /// Each row's error is re-evaluated after the rows before it have moved the bodies (Gauss–Seidel); evaluating
    /// all errors once per sweep (Jacobi) diverges on strongly coupled joints such as a seesaw with pinned masses.
    #[allow(clippy::needless_range_loop)]
    pub(crate) fn project_positions(
        &mut self,
        constraints: &[Box<dyn Constraint>],
        bodies: &mut Bodies,
        dt: f64,
    ) {
        let masses: Vec<f64> = self
            .start
            .rows
            .iter()
            .map(|row| Self::effective_mass(&self.start, bodies, row))
            .collect();
        let n = self.start.len();
        for _ in 0..POSITION_SWEEPS {
            let mut worst: f64 = 0.0;
            for r in 0..n {
                Self::rebuild(&mut self.current, constraints, bodies);
                let now = self.current.rows[r];
                let violated = if now.unilateral() {
                    now.active && now.error < 0.0
                } else {
                    true
                };
                if !violated || masses[r] == 0.0 {
                    continue;
                }
                worst = worst.max(now.error.abs());
                let shift = -now.error / masses[r];
                self.position_impulses[r] += shift / dt;
                let row = self.start.rows[r];
                Self::shift_positions(&self.start, bodies, &row, shift);
                Self::apply_impulse(&self.start, bodies, &row, shift / dt);
            }
            if worst < POSITION_TOLERANCE {
                return;
            }
        }
    }

    fn remember(&self, c: &Contact) -> Option<ContactMemory> {
        self.memory
            .iter()
            .copied()
            .find(|m| m.a == c.a && m.b == c.b && m.feature == c.feature)
    }

    /// Applies half of each persistent contact's previous reaction as the opening half-kick, so resting contact
    /// does not drift and sliding positions are second order (design 003). Impacts are not persistent.
    pub(crate) fn kick_predictor(&self, bodies: &mut Bodies) {
        for (k, m) in self.memory.iter().enumerate() {
            if m.impulsive {
                continue;
            }
            let rows = &self.previous_contact_rows;
            Self::apply_impulse(rows, bodies, &rows.rows[2 * k], 0.5 * m.normal_impulse);
            Self::apply_impulse(rows, bodies, &rows.rows[2 * k + 1], 0.5 * m.tangent_impulse);
        }
    }

    /// Solves all rows together, applies the impulses, reports reactions, emits events, and returns the kinetic
    /// energy removed by impacts.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn solve_velocities(
        &mut self,
        constraints: &[Box<dyn Constraint>],
        contacts: &[Contact],
        bodies: &mut Bodies,
        loads: &mut Loads,
        dt: f64,
        time: f64,
        restitution_threshold: f64,
        accelerations: &[f64],
        events: &mut Vec<Event>,
    ) -> f64 {
        Self::rebuild(&mut self.current, constraints, bodies);
        let constraint_rows = self.current.len();
        Self::contact_rows(&mut self.current, contacts, bodies);
        let mut rows = self.current.rows.clone();
        let n = rows.len();
        self.impulses.resize(n, 0.0);
        self.taut.resize(n, false);
        self.position_impulses.resize(n, 0.0);
        let mut sliding = vec![false; contacts.len()];
        let mut impulsive = vec![false; n];
        // Impulse already delivered by the predictor kick, per row.
        let mut predicted = vec![0.0; n];

        // Warm start: constraints from the previous step, contacts from memory by feature. A persistent contact
        // has had half its previous impulse applied by the predictor, so the solve starts from the other half.
        for (r, row) in rows.iter().enumerate().take(constraint_rows) {
            if !row.active {
                self.impulses[r] = 0.0;
            }
        }
        for (k, c) in contacts.iter().enumerate() {
            let (normal_row, friction_row) = (constraint_rows + 2 * k, constraint_rows + 2 * k + 1);
            let memory = self.remember(c);
            let persistent = memory.is_some_and(|m| !m.impulsive);
            let share = if persistent { 0.5 } else { 0.0 };
            predicted[normal_row] = memory.map_or(0.0, |m| share * m.normal_impulse);
            predicted[friction_row] = memory.map_or(0.0, |m| share * m.tangent_impulse);
            self.impulses[normal_row] = predicted[normal_row];
            // Static friction between several contact points of one body is statically indeterminate. Starting the
            // sweep from a reduced guess lets a leftover equal-and-opposite pair decay to zero over a few steps,
            // while a genuinely loaded contact is restored exactly within the sweep.
            self.impulses[friction_row] = FRICTION_WARM_START * predicted[friction_row];
            sliding[k] = memory.is_some_and(|m| m.sliding);
            if !rows[normal_row].active {
                self.impulses[normal_row] = 0.0;
                self.impulses[friction_row] = 0.0;
            }
        }

        // Restitution targets from the approach speed at the moment of impact. The contact was found after the
        // drift, at most `dt` late: the bodies overlap by `depth`, so the impact happened `δ = depth / |v_n|` ago.
        // Over that time the approach speed grew by `a_n δ` and the rebound has already decelerated by the same, so
        // the velocity to aim for at the end of the step is `-e (v_n - a_n δ) + a_n δ`. This makes bounce heights
        // and landing times second order instead of first (design 003, impact timing).
        let pre: Vec<f64> = rows
            .iter()
            .map(|row| Self::relative_velocity(&self.current, bodies, row))
            .collect();
        let mut impact_delay = vec![0.0; contacts.len()];
        for (r, row) in rows.iter_mut().enumerate() {
            if let RowKind::Normal = row.kind {
                let c = &contacts[row.source];
                let restitution = bodies
                    .material(c.a)
                    .restitution
                    .min(bodies.material(c.b).restitution);
                if row.active && pre[r] < -restitution_threshold {
                    let delay = (c.depth.max(0.0) / -pre[r]).min(dt);
                    let approach = pre[r] - accelerations[row.source] * delay;
                    row.bias = -restitution * approach + accelerations[row.source] * delay;
                    impact_delay[row.source] = delay;
                    impulsive[r] = true;
                    impulsive[r + 1] = true;
                }
            }
        }

        for (r, row) in rows.iter().enumerate() {
            if row.active {
                Self::apply_impulse(&self.current, bodies, row, self.impulses[r]);
            }
        }

        let masses: Vec<f64> = rows
            .iter()
            .map(|row| Self::effective_mass(&self.current, bodies, row))
            .collect();
        for _ in 0..VELOCITY_SWEEPS {
            let mut largest_change: f64 = 0.0;
            let mut largest_impulse: f64 = 0.0;
            for (r, row) in rows.iter().enumerate() {
                if !row.active || masses[r] == 0.0 {
                    continue;
                }
                let unconstrained = self.impulses[r]
                    + (row.bias - Self::relative_velocity(&self.current, bodies, row)) / masses[r];
                let clamped = match row.kind {
                    RowKind::Bilateral => unconstrained,
                    RowKind::Rope | RowKind::Normal => unconstrained.max(0.0),
                    RowKind::Friction {
                        normal,
                        static_friction,
                        kinetic_friction,
                    } => {
                        let limit = self.impulses[normal];
                        let contact = row.source;
                        let threshold = if sliding[contact] {
                            kinetic_friction
                        } else {
                            static_friction
                        } * limit;
                        if unconstrained.abs() <= threshold {
                            sliding[contact] = false;
                            unconstrained
                        } else {
                            sliding[contact] = true;
                            unconstrained.clamp(-kinetic_friction * limit, kinetic_friction * limit)
                        }
                    }
                };
                let change = clamped - self.impulses[r];
                self.impulses[r] = clamped;
                Self::apply_impulse(&self.current, bodies, row, change);
                largest_change = largest_change.max(change.abs());
                largest_impulse = largest_impulse.max(clamped.abs());
            }
            if largest_change <= VELOCITY_TOLERANCE * (1.0 + largest_impulse) {
                break;
            }
        }

        // Reactions, impact energy, memory and events.
        let mut impact_loss = 0.0;
        for (r, row) in rows.iter().enumerate() {
            let force = (self.impulses[r] + predicted[r] + self.position_impulses[r]) / dt;
            for e in self.current.entries_of(row) {
                loads.react(
                    bodies,
                    e.body,
                    row.label,
                    e.linear * force,
                    e.angular * force,
                    e.point,
                    impulsive[r],
                );
            }
            if impulsive[r] {
                let post = Self::relative_velocity(&self.current, bodies, row);
                impact_loss -= 0.5 * self.impulses[r] * (pre[r] + post);
            }
            if let RowKind::Rope = row.kind {
                let taut = row.active && self.impulses[r] > 0.0;
                if taut != self.taut[r] {
                    let kind = if taut {
                        EventKind::RopeTaut { rope: row.source }
                    } else {
                        EventKind::RopeSlack { rope: row.source }
                    };
                    events.push(Event { time, kind });
                }
                self.taut[r] = taut;
            }
        }

        self.memory.clear();
        self.previous_contact_rows.clear();
        let contact_rows = self.current.tail(constraint_rows);
        let mut touching = Vec::new();
        let mut sliding_now = Vec::new();
        for (k, c) in contacts.iter().enumerate() {
            let normal_row = constraint_rows + 2 * k;
            if !rows[normal_row].active {
                continue;
            }
            for r in [2 * k, 2 * k + 1] {
                let row = contact_rows.rows[r];
                let start = self.previous_contact_rows.entries.len();
                self.previous_contact_rows
                    .entries
                    .extend_from_slice(contact_rows.entries_of(&row));
                self.previous_contact_rows.rows.push(Row { start, ..row });
            }
            self.memory.push(ContactMemory {
                a: c.a,
                b: c.b,
                feature: c.feature,
                normal_impulse: self.impulses[normal_row] + predicted[normal_row],
                tangent_impulse: self.impulses[normal_row + 1] + predicted[normal_row + 1],
                impulsive: impulsive[normal_row],
                sliding: sliding[k],
            });
            if !touching.contains(&(c.a, c.b)) {
                touching.push((c.a, c.b));
            }
            if sliding[k] && self.impulses[normal_row] > 0.0 && !sliding_now.contains(&(c.a, c.b)) {
                sliding_now.push((c.a, c.b));
            }
        }
        for &(a, b) in &touching {
            if !self.touching_pairs.contains(&(a, b)) {
                let delay = contacts
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.a == a && c.b == b)
                    .map(|(k, _)| impact_delay[k])
                    .fold(0.0, f64::max);
                events.push(Event {
                    time: time - delay,
                    kind: EventKind::ContactBegin { a, b },
                });
            }
        }
        self.impact_delays = impact_delay;
        for &(a, b) in &self.touching_pairs {
            if !touching.contains(&(a, b)) {
                events.push(Event {
                    time,
                    kind: EventKind::ContactEnd { a, b },
                });
            }
        }
        for &(a, b) in &sliding_now {
            if !self.sliding_pairs.contains(&(a, b)) {
                events.push(Event {
                    time,
                    kind: EventKind::FrictionSlip { a, b },
                });
            }
        }
        for &(a, b) in &self.sliding_pairs {
            if !sliding_now.contains(&(a, b)) && touching.contains(&(a, b)) {
                events.push(Event {
                    time,
                    kind: EventKind::FrictionStick { a, b },
                });
            }
        }
        self.touching_pairs = touching;
        self.sliding_pairs = sliding_now;
        impact_loss
    }

    /// Split-impulse position correction: pushes penetrating contacts apart without changing velocities, then
    /// touches up the bilateral constraints the same way if anything moved. Gaps are tracked through the row
    /// Jacobians against a snapshot, so a two-point manifold converges in a few sweeps.
    pub(crate) fn correct_positions(
        &mut self,
        constraints: &[Box<dyn Constraint>],
        contacts: &[Contact],
        bodies: &mut Bodies,
    ) {
        self.current.clear();
        Self::contact_rows(&mut self.current, contacts, bodies);
        let positions = bodies.positions.clone();
        let angles = bodies.angles.clone();
        let mut moved = false;
        for _ in 0..CONTACT_SWEEPS {
            let mut worst: f64 = 0.0;
            for (k, c) in contacts.iter().enumerate() {
                let row = self.current.rows[2 * k];
                if !row.active {
                    continue;
                }
                let displacement: f64 = self
                    .current
                    .entries_of(&row)
                    .iter()
                    .map(|e| {
                        let i = e.body.index();
                        e.linear.dot(bodies.positions[i] - positions[i])
                            + e.angular * (bodies.angles[i] - angles[i])
                    })
                    .sum();
                // An impact found `delay` late should already have rebounded for that long.
                let rebound = self.impact_delays.get(k).copied().unwrap_or(0.0)
                    * Self::relative_velocity(&self.current, bodies, &row).max(0.0);
                let gap = -c.depth + displacement - rebound;
                if gap >= -POSITION_TOLERANCE {
                    continue;
                }
                let mass = Self::effective_mass(&self.current, bodies, &row);
                if mass == 0.0 {
                    continue;
                }
                worst = worst.max(-gap);
                Self::shift_positions(&self.current, bodies, &row, -gap / mass);
                moved = true;
            }
            if worst < POSITION_TOLERANCE {
                break;
            }
        }
        if moved {
            self.settle(constraints, bodies);
        }
    }

    /// Satisfies the constraints at the position level along the current gradients without touching velocities.
    /// Used after contact corrections and once at construction, so a scene may start slightly inconsistent.
    pub(crate) fn settle(&mut self, constraints: &[Box<dyn Constraint>], bodies: &mut Bodies) {
        if constraints.is_empty() {
            return;
        }
        Self::rebuild(&mut self.current, constraints, bodies);
        let n = self.current.len();
        for _ in 0..POSITION_SWEEPS {
            let mut worst: f64 = 0.0;
            for r in 0..n {
                Self::rebuild(&mut self.current, constraints, bodies);
                let row = self.current.rows[r];
                let violated = if row.unilateral() {
                    row.active && row.error < 0.0
                } else {
                    true
                };
                let mass = Self::effective_mass(&self.current, bodies, &row);
                if !violated || mass == 0.0 {
                    continue;
                }
                worst = worst.max(row.error.abs());
                Self::shift_positions(&self.current, bodies, &row, -row.error / mass);
            }
            if worst < POSITION_TOLERANCE {
                break;
            }
        }
    }
}
