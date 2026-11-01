use crate::body::Bodies;
use crate::constraint::{Constraint, Row, Rows};
use crate::event::{Event, EventKind};
use crate::force::Loads;

const VELOCITY_SWEEPS: usize = 200;
const VELOCITY_TOLERANCE: f64 = 1e-12;
const POSITION_SWEEPS: usize = 50;
const POSITION_TOLERANCE: f64 = 1e-13;

/// RATTLE for the constraints of design 002: a SHAKE position projection along the step-start gradients, then a
/// velocity projection along the step-end gradients, both by projected Gauss–Seidel with warm starting.
#[derive(Default)]
pub(crate) struct ConstraintSolver {
    /// Rows at the start of the step; their directions steer the position projection.
    start: Rows,
    /// Rows at the current positions; their errors drive the position projection and their directions the
    /// velocity projection.
    current: Rows,
    impulses: Vec<f64>,
    position_impulses: Vec<f64>,
    taut: Vec<bool>,
}

impl ConstraintSolver {
    fn rebuild(rows: &mut Rows, constraints: &[Box<dyn Constraint>], bodies: &Bodies) {
        rows.clear();
        for (index, constraint) in constraints.iter().enumerate() {
            constraint.rows(bodies, index, rows);
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

    /// Captures the constraint gradients at the start of a step.
    pub(crate) fn prepare(&mut self, constraints: &[Box<dyn Constraint>], bodies: &Bodies) {
        Self::rebuild(&mut self.start, constraints, bodies);
        let n = self.start.rows.len();
        self.impulses.resize(n, 0.0);
        self.position_impulses.resize(n, 0.0);
        self.taut.resize(n, false);
        self.position_impulses.iter_mut().for_each(|p| *p = 0.0);
    }

    /// SHAKE: moves bodies along the step-start `M⁻¹ Jᵀ` until the constraints hold at the new positions, and
    /// gives the half-step velocities the matching correction. The shifts are kept as impulses for the reaction report.
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
        for _ in 0..POSITION_SWEEPS {
            Self::rebuild(&mut self.current, constraints, bodies);
            let mut worst: f64 = 0.0;
            for (r, now) in self.current.rows.iter().enumerate() {
                let violated = if now.unilateral {
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
                for e in self.start.entries_of(&self.start.rows[r]) {
                    let i = e.body.index();
                    bodies.positions[i] += e.linear * (bodies.inverse_masses[i] * shift);
                    bodies.angles[i] += e.angular * bodies.inverse_inertias[i] * shift;
                    bodies.velocities[i] += e.linear * (bodies.inverse_masses[i] * shift / dt);
                    bodies.angular_velocities[i] +=
                        e.angular * bodies.inverse_inertias[i] * shift / dt;
                }
            }
            if worst < POSITION_TOLERANCE {
                return;
            }
        }
    }

    /// Finds impulses so every active row has zero relative velocity (non-negative for ropes), applies them,
    /// reports the whole step's reactions into `loads`, and emits rope events.
    pub(crate) fn solve_velocities(
        &mut self,
        constraints: &[Box<dyn Constraint>],
        bodies: &mut Bodies,
        loads: &mut Loads,
        dt: f64,
        time: f64,
        events: &mut Vec<Event>,
    ) {
        Self::rebuild(&mut self.current, constraints, bodies);
        let rows = self.current.rows.clone();

        for (r, row) in rows.iter().enumerate() {
            if row.active {
                Self::apply_impulse(&self.current, bodies, row, self.impulses[r]);
            } else {
                self.impulses[r] = 0.0;
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
                let target = self.impulses[r]
                    - Self::relative_velocity(&self.current, bodies, row) / masses[r];
                let clamped = if row.unilateral {
                    target.max(0.0)
                } else {
                    target
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

        for (r, row) in rows.iter().enumerate() {
            let force = (self.impulses[r] + self.position_impulses[r]) / dt;
            for e in self.current.entries_of(row) {
                loads.react(
                    bodies,
                    e.body,
                    row.label,
                    e.linear * force,
                    e.angular * force,
                    e.point,
                );
            }
            if row.unilateral {
                let taut = row.active && self.impulses[r] > 0.0;
                if taut != self.taut[r] {
                    let kind = if taut {
                        EventKind::RopeTaut {
                            rope: row.constraint,
                        }
                    } else {
                        EventKind::RopeSlack {
                            rope: row.constraint,
                        }
                    };
                    events.push(Event { time, kind });
                }
                self.taut[r] = taut;
            }
        }
    }
}
