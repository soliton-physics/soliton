//! Closed-form results used by the tests and shown beside measured values.

use soliton_core::Vec2;

/// Position under constant acceleration.
pub fn constant_acceleration(x0: Vec2, v0: Vec2, a: Vec2, t: f64) -> Vec2 {
    x0 + v0 * t + a * (0.5 * t * t)
}

/// Horizontal range of a projectile launched from the ground at `speed` and `angle` under gravity `g`.
pub fn projectile_range(speed: f64, angle: f64, g: f64) -> f64 {
    speed * speed * (2.0 * angle).sin() / g
}

pub fn projectile_max_height(speed: f64, angle: f64, g: f64) -> f64 {
    let vy = speed * angle.sin();
    vy * vy / (2.0 * g)
}

pub fn projectile_flight_time(speed: f64, angle: f64, g: f64) -> f64 {
    2.0 * speed * angle.sin() / g
}

/// Period of a mass on a spring.
pub fn spring_period(mass: f64, stiffness: f64) -> f64 {
    std::f64::consts::TAU * (mass / stiffness).sqrt()
}

/// The period velocity Verlet actually produces for a harmonic oscillator of frequency `omega` at step `dt`.
pub fn verlet_period(omega: f64, dt: f64) -> f64 {
    std::f64::consts::TAU / ((2.0 / dt) * (0.5 * omega * dt).asin())
}

/// Underdamped oscillator `m x'' + b x' + k x = 0` from rest displacement `x0`.
pub fn damped_oscillator(x0: f64, mass: f64, stiffness: f64, damping: f64, t: f64) -> f64 {
    let gamma = damping / (2.0 * mass);
    let omega_d = (stiffness / mass - gamma * gamma).sqrt();
    (-gamma * t).exp() * (x0 * (omega_d * t).cos() + gamma * x0 / omega_d * (omega_d * t).sin())
}
