use crate::force::Instant;
use crate::world::World;
use soliton_core::Vec2;

/// Advances a world by one fixed step. On entry the loads are those of the opening half-step at `time`;
/// on exit they are those of the closing half-step at `time + dt`.
pub trait Integrator {
    fn step(&mut self, world: &mut World, time: f64, dt: f64);
}

/// Velocity Verlet. Symplectic, second order, exact for constant acceleration.
/// Velocity-dependent forces are handled by fixed-point iteration on the final kick (design 001).
#[derive(Clone, Copy, Debug, Default)]
pub struct Verlet;

const KICK_ITERATIONS: usize = 8;
const KICK_TOLERANCE: f64 = 1e-14;

impl Integrator for Verlet {
    fn step(&mut self, world: &mut World, time: f64, dt: f64) {
        let n = world.bodies.len();
        let half = 0.5 * dt;
        world.prepare_constraints();

        for i in 0..n {
            let b = &mut world.bodies;
            let a = world.loads.forces[i] * b.inverse_masses[i];
            let alpha = world.loads.torques[i] * b.inverse_inertias[i];
            b.velocities[i] += a * half;
            b.angular_velocities[i] += alpha * half;
            b.positions[i] += b.velocities[i] * dt;
            b.angles[i] += b.angular_velocities[i] * dt;
        }
        world.project_positions(dt);

        let half_velocities: Vec<Vec2> = world.bodies.velocities.clone();
        let half_spins: Vec<f64> = world.bodies.angular_velocities.clone();
        let iterations = if world.has_velocity_forces() {
            KICK_ITERATIONS
        } else {
            1
        };

        for _ in 0..iterations {
            world.evaluate_loads(Instant::closing(time + dt));
            let mut change: f64 = 0.0;
            for i in 0..n {
                let b = &mut world.bodies;
                let v = half_velocities[i] + world.loads.forces[i] * (b.inverse_masses[i] * half);
                let w = half_spins[i] + world.loads.torques[i] * (b.inverse_inertias[i] * half);
                change = change
                    .max((v - b.velocities[i]).length())
                    .max((w - b.angular_velocities[i]).abs());
                b.velocities[i] = v;
                b.angular_velocities[i] = w;
            }
            if change <= KICK_TOLERANCE {
                break;
            }
        }
        world.solve_velocities(dt, time + dt);
    }
}
