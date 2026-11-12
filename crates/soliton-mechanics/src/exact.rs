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

/// Acceleration of the lighter side of an Atwood machine, taken positive when `m2` descends.
pub fn atwood_acceleration(m1: f64, m2: f64, g: f64) -> f64 {
    (m2 - m1) / (m1 + m2) * g
}

pub fn atwood_tension(m1: f64, m2: f64, g: f64) -> f64 {
    2.0 * m1 * m2 / (m1 + m2) * g
}

/// Block of mass `m1` on a smooth table pulled by a hanging mass `m2` over a massless pulley.
pub fn table_and_hanging_mass(m1: f64, m2: f64, g: f64) -> (f64, f64) {
    let a = m2 / (m1 + m2) * g;
    (a, m1 * a)
}

/// Small-amplitude period of a simple pendulum.
pub fn pendulum_period_small(length: f64, g: f64) -> f64 {
    std::f64::consts::TAU * (length / g).sqrt()
}

/// Exact period of a simple pendulum released from rest at `amplitude`, via the complete elliptic integral.
pub fn pendulum_period(length: f64, g: f64, amplitude: f64) -> f64 {
    let k = (0.5 * amplitude).sin();
    4.0 * (length / g).sqrt() * complete_elliptic_k(k)
}

/// `K(k)` by the arithmetic–geometric mean.
fn complete_elliptic_k(k: f64) -> f64 {
    let (mut a, mut b) = (1.0, (1.0 - k * k).sqrt());
    while (a - b).abs() > 1e-15 {
        (a, b) = (0.5 * (a + b), (a * b).sqrt());
    }
    std::f64::consts::PI / (2.0 * a)
}

/// Minimum speed at the bottom for a bob on a string to complete a vertical circle.
pub fn vertical_circle_min_speed(length: f64, g: f64) -> f64 {
    (5.0 * g * length).sqrt()
}

/// Angular acceleration (counter-clockwise positive) of a seesaw: a pivoted rod of inertia `rod_inertia` with
/// `m1` at distance `d1` on the left and `m2` at `d2` on the right.
pub fn seesaw_angular_acceleration(
    m1: f64,
    d1: f64,
    m2: f64,
    d2: f64,
    rod_inertia: f64,
    g: f64,
) -> f64 {
    (m1 * d1 - m2 * d2) * g / (rod_inertia + m1 * d1 * d1 + m2 * d2 * d2)
}

/// Acceleration of a block sliding down an incline of angle `theta` with kinetic friction `mu`.
pub fn incline_sliding_acceleration(theta: f64, mu: f64, g: f64) -> f64 {
    g * (theta.sin() - mu * theta.cos())
}

/// Acceleration of a round body rolling without slipping down an incline; `inertia_ratio` is `I / (m R²)`.
pub fn rolling_acceleration(theta: f64, inertia_ratio: f64, g: f64) -> f64 {
    g * theta.sin() / (1.0 + inertia_ratio)
}

/// Least static friction coefficient for a round body to roll without slipping; `inertia_ratio` is `I / (m R²)`.
pub fn rolling_min_friction(theta: f64, inertia_ratio: f64) -> f64 {
    theta.tan() * inertia_ratio / (1.0 + inertia_ratio)
}

/// Final velocities of a head-on collision with restitution `e`.
pub fn collision_1d(m1: f64, u1: f64, m2: f64, u2: f64, e: f64) -> (f64, f64) {
    let total = m1 + m2;
    let v1 = (m1 * u1 + m2 * u2 - m2 * e * (u1 - u2)) / total;
    let v2 = (m1 * u1 + m2 * u2 + m1 * e * (u1 - u2)) / total;
    (v1, v2)
}

/// Distance a block sliding at `u` travels on a floor with kinetic friction `mu` before stopping.
pub fn stopping_distance(u: f64, mu: f64, g: f64) -> f64 {
    u * u / (2.0 * mu * g)
}

/// Least floor friction for a ladder leaning on a smooth wall at angle `theta` from the floor to stay put.
pub fn ladder_min_friction(theta: f64) -> f64 {
    0.5 / theta.tan()
}

/// Acceleration of an Atwood machine whose pulley is a uniform disc of mass `pulley_mass`, positive when `m2` descends.
pub fn atwood_massive_acceleration(m1: f64, m2: f64, pulley_mass: f64, g: f64) -> f64 {
    (m2 - m1) * g / (m1 + m2 + 0.5 * pulley_mass)
}

/// Speed of a circular orbit of radius `r` about a mass with gravitational parameter `mu = G M`.
pub fn circular_orbit_speed(mu: f64, r: f64) -> f64 {
    (mu / r).sqrt()
}

/// Kepler's third law: the period of an orbit with semi-major axis `a`.
pub fn orbit_period(mu: f64, a: f64) -> f64 {
    std::f64::consts::TAU * (a * a * a / mu).sqrt()
}

/// Speed needed at distance `r` to leave for good.
pub fn escape_speed(mu: f64, r: f64) -> f64 {
    (2.0 * mu / r).sqrt()
}

/// The ellipse a body follows when it passes its nearest point, at distance `r_p`, with speed `v_p` at right
/// angles to the radius: semi-major axis and eccentricity. Bound only while `v_p` is below the escape speed.
pub fn orbit_from_perigee(mu: f64, r_p: f64, v_p: f64) -> (f64, f64) {
    let energy = 0.5 * v_p * v_p - mu / r_p;
    (-mu / (2.0 * energy), r_p * v_p * v_p / mu - 1.0)
}

/// How high above its launch radius `r` a body thrown straight up at `speed` rises, when it does not escape.
pub fn vertical_launch_height(mu: f64, r: f64, speed: f64) -> f64 {
    let ratio = 2.0 * mu / (r * speed * speed);
    r / (ratio - 1.0)
}

/// Time for a body thrown straight up from radius `r` to reach its highest point at radius `top`.
pub fn vertical_launch_rise_time(mu: f64, r: f64, top: f64) -> f64 {
    // A radial orbit is an ellipse of eccentricity one with semi-major axis `top / 2`: r = a (1 − cos E).
    let a = 0.5 * top;
    let anomaly = (1.0 - r / a).acos();
    (a * a * a / mu).sqrt() * (std::f64::consts::PI - anomaly + anomaly.sin())
}

/// Steady-state amplitude and phase lag of `m x'' + b x' + k x = F₀ cos(ω t)`.
pub fn driven_response(
    mass: f64,
    stiffness: f64,
    damping: f64,
    force: f64,
    omega: f64,
) -> (f64, f64) {
    let (reactive, resistive) = (stiffness - mass * omega * omega, damping * omega);
    (
        force / (reactive * reactive + resistive * resistive).sqrt(),
        resistive.atan2(reactive),
    )
}
