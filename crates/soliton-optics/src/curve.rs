use soliton_core::Vec2;
use std::f64::consts::TAU;

/// A line a ray can meet: a straight edge, or part of a circle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    Segment {
        a: Vec2,
        b: Vec2,
    },
    /// Part of a circle, starting at the angle `from` and turning through `sweep` radians, anticlockwise when
    /// positive.
    Arc {
        centre: Vec2,
        radius: f64,
        from: f64,
        sweep: f64,
    },
}

/// Where a ray meets a curve: how far along the ray, and a unit normal to the curve there, either way round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub distance: f64,
    pub normal: Vec2,
}

impl Curve {
    /// An arc through `pole`, bulging towards it from a centre `radius` away along `towards_centre`, reaching
    /// `aperture` either side of the axis.
    pub fn cap(pole: Vec2, towards_centre: Vec2, radius: f64, aperture: f64) -> Self {
        let axis = towards_centre.normalized();
        let half = (aperture / radius).clamp(-1.0, 1.0).asin();
        Self::Arc {
            centre: pole + axis * radius,
            radius,
            from: (-axis).angle() - half,
            sweep: 2.0 * half,
        }
    }

    /// Every place the line from `origin` along the unit `direction` meets the curve, ahead or behind.
    pub fn hits(&self, origin: Vec2, direction: Vec2) -> Vec<Hit> {
        match *self {
            Self::Segment { a, b } => {
                let edge = b - a;
                let across = direction.cross(edge);
                if across.abs() < 1e-14 * edge.length() {
                    return Vec::new();
                }
                let distance = (a - origin).cross(edge) / across;
                let along = (a - origin).cross(direction) / across;
                if (0.0..=1.0).contains(&along) {
                    vec![Hit {
                        distance,
                        normal: edge.perp().normalized(),
                    }]
                } else {
                    Vec::new()
                }
            }
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => {
                let offset = origin - centre;
                let half = offset.dot(direction);
                let inside = half * half - offset.length_squared() + radius * radius;
                if inside < 0.0 {
                    return Vec::new();
                }
                let root = inside.sqrt();
                [-half - root, -half + root]
                    .into_iter()
                    .filter_map(|distance| {
                        let normal = (offset + direction * distance) / radius;
                        // How far round from the start of the arc, in the direction it turns.
                        let turned = ((normal.angle() - from) * sweep.signum()).rem_euclid(TAU);
                        (turned <= sweep.abs() + 1e-12).then_some(Hit { distance, normal })
                    })
                    .collect()
            }
        }
    }

    pub fn start(&self) -> Vec2 {
        match *self {
            Self::Segment { a, .. } => a,
            Self::Arc {
                centre,
                radius,
                from,
                ..
            } => centre + Vec2::from_polar(radius, from),
        }
    }

    pub fn end(&self) -> Vec2 {
        match *self {
            Self::Segment { b, .. } => b,
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => centre + Vec2::from_polar(radius, from + sweep),
        }
    }

    /// Points along the curve for drawing: its two ends for a segment, `steps + 1` points for an arc.
    pub fn points(&self, steps: usize) -> Vec<Vec2> {
        match *self {
            Self::Segment { a, b } => vec![a, b],
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => (0..=steps)
                .map(|i| centre + Vec2::from_polar(radius, from + sweep * i as f64 / steps as f64))
                .collect(),
        }
    }
}
