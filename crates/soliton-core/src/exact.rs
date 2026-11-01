//! Closed-form solutions used for initial conditions and validation.

use num_complex::Complex64;

fn sech(x: f64) -> f64 {
    1.0 / x.cosh()
}

/// Single-soliton solution of `u_t + 6 u u_x + u_xxx = 0`
/// travelling at `speed > 0` with its crest at `center` when `t = 0`.
pub fn kdv_soliton(x: f64, t: f64, speed: f64, center: f64) -> f64 {
    let phase = 0.5 * speed.sqrt() * (x - speed * t - center);
    0.5 * speed * sech(phase).powi(2)
}

/// Bright soliton of `i ψ_t + ½ ψ_xx + |ψ|² ψ = 0` with the given `amplitude`,
/// moving at `velocity`, centred at `center` when `t = 0`.
pub fn nls_bright_soliton(x: f64, t: f64, amplitude: f64, velocity: f64, center: f64) -> Complex64 {
    let envelope = amplitude * sech(amplitude * (x - velocity * t - center));
    let phase = velocity * x + 0.5 * (amplitude * amplitude - velocity * velocity) * t;
    Complex64::from_polar(envelope, phase)
}
