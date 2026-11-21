//! The Biot–Savart law, summed along the wire: `dB = (μ₀/4π) I dl × r / r³`.

use crate::MU0;
use std::f64::consts::PI;

type Vec3 = [f64; 3];

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The field at `at` of one ampere flowing along a path of straight pieces, T.
pub fn field_of_path(path: impl IntoIterator<Item = Vec3>, at: Vec3) -> Vec3 {
    let mut total = [0.0; 3];
    let mut points = path.into_iter();
    let Some(mut from) = points.next() else {
        return total;
    };
    for to in points {
        let piece = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let r = [
            at[0] - 0.5 * (from[0] + to[0]),
            at[1] - 0.5 * (from[1] + to[1]),
            at[2] - 0.5 * (from[2] + to[2]),
        ];
        let distance = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        let part = cross(piece, r);
        for (sum, p) in total.iter_mut().zip(part) {
            *sum += MU0 / (4.0 * PI) * p / distance.powi(3);
        }
        from = to;
    }
    total
}

/// A straight wire along z through the origin, long enough to stand for an endless one: the field per ampere a
/// distance `away` from it, T.
pub fn beside_long_wire(away: f64) -> f64 {
    // 200 distances either way leaves out about one part in 10⁵ of an endless wire's field.
    const REACH: f64 = 200.0;
    const PIECES: usize = 40_000;
    let path = (0..=PIECES).map(|i| {
        let share = i as f64 / PIECES as f64;
        [0.0, 0.0, away * REACH * (2.0 * share - 1.0)]
    });
    field_of_path(path, [away, 0.0, 0.0])[1]
}

/// A circular turn of `radius` whose axis is x, carrying one ampere: the field at a point `along` the axis
/// from its plane and `out` from the axis, as its parts along and away from the axis, T.
pub fn of_turn(radius: f64, along: f64, out: f64, pieces: usize) -> [f64; 2] {
    let path = (0..=pieces).map(|i| {
        let angle = 2.0 * PI * i as f64 / pieces as f64;
        [0.0, radius * angle.cos(), radius * angle.sin()]
    });
    let b = field_of_path(path, [along, out, 0.0]);
    [b[0], b[1]]
}
