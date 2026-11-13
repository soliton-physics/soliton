//! Bar magnets and matter in their field (design 012). A magnet is a pair of poles, north and south; outside
//! the magnet that gives the same field as the currents that really make it, and far away it is the field of a
//! dipole. The fields here lie in the plane of the page.

use crate::body::{Bodies, BodyId};
use crate::electromagnetism::away;
use crate::force::{Force, ForceLabel, Instant, Loads};
use soliton_core::Vec2;

/// `μ₀/4π`, T·m/A.
pub const MAGNETIC: f64 = 1e-7;
/// `μ₀`, T·m/A.
pub const PERMEABILITY: f64 = 4.0 * std::f64::consts::PI * MAGNETIC;

/// What makes a body a bar magnet: its moment, A·m², pointing along the body's x axis from the south pole to the
/// north, and the distance between the poles, m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Magnet {
    pub moment: f64,
    pub length: f64,
}

impl Magnet {
    pub fn new(moment: f64, length: f64) -> Self {
        assert!(length > 0.0, "the poles of a magnet must be apart");
        Self { moment, length }
    }

    /// Strength of each pole, A·m: the moment is this times the distance between them.
    pub fn pole_strength(&self) -> f64 {
        self.moment / self.length
    }
}

/// The moment a small sphere of a linear material takes on per tesla of applied field, A·m²/T. Inside a sphere
/// the material's own magnetisation works against the field that causes it, which is where `3/(3 + χ)` comes
/// from: it hardly matters for a weak material and caps an iron ball at `3V/μ₀`.
pub fn sphere_polarisability(susceptibility: f64, volume: f64) -> f64 {
    3.0 * susceptibility / (3.0 + susceptibility) * volume / PERMEABILITY
}

/// One pole of a magnet in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pole {
    pub body: BodyId,
    pub at: Vec2,
    /// Positive for a north pole.
    pub strength: f64,
}

pub fn poles(bodies: &Bodies) -> Vec<Pole> {
    let mut out = Vec::new();
    for body in bodies.ids() {
        if let Some(magnet) = bodies.magnet(body) {
            let half = Vec2::new(0.5 * magnet.length, 0.0);
            let strength = magnet.pole_strength();
            out.push(Pole {
                body,
                at: bodies.world_point(body, half),
                strength,
            });
            out.push(Pole {
                body,
                at: bodies.world_point(body, -half),
                strength: -strength,
            });
        }
    }
    out
}

/// Magnets acting on one another, pole on pole by an inverse-square law, in a uniform `background` field such as
/// the earth's. A static magnet is a fixed one, felt but not moved. A body with a polarisability is magnetised
/// by the field it finds itself in and pulled towards where that field is stronger, or pushed away if the
/// polarisability is negative. The force reports as `magnetic` and has a potential energy.
#[derive(Clone, Copy, Debug)]
pub struct Magnetostatics {
    constant: f64,
    background: Vec2,
}

impl Magnetostatics {
    pub fn new(constant: f64) -> Self {
        Self {
            constant,
            background: Vec2::ZERO,
        }
    }

    /// Adds a uniform field in the plane, T.
    pub fn background(mut self, field: Vec2) -> Self {
        self.background = field;
        self
    }

    /// The field at `point` of the background and of every pole not on the body `skip`.
    fn field(&self, poles: &[Pole], point: Vec2, skip: Option<BodyId>) -> Vec2 {
        let mut field = Vec2::ZERO;
        for pole in poles.iter().filter(|p| Some(p.body) != skip) {
            field += away(point, pole.at) * pole.strength;
        }
        self.background + field * self.constant
    }
}

impl Default for Magnetostatics {
    fn default() -> Self {
        Self::new(MAGNETIC)
    }
}

impl Force for Magnetostatics {
    fn label(&self) -> ForceLabel {
        ForceLabel::Magnetostatic
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        let poles = poles(bodies);
        for pole in poles.iter().filter(|p| bodies.is_dynamic(p.body)) {
            let field = self.field(&poles, pole.at, Some(pole.body));
            loads.apply(
                bodies,
                pole.body,
                self.label(),
                field * pole.strength,
                pole.at,
            );
        }
        for id in bodies.ids().filter(|&id| bodies.polarisability(id) != 0.0) {
            let at = bodies.position(id);
            let moment = self.field(&poles, at, Some(id)) * bodies.polarisability(id);
            let mut total = Vec2::ZERO;
            for pole in poles.iter().filter(|p| p.body != id) {
                let apart = at - pole.at;
                let distance = apart.length();
                if distance == 0.0 {
                    continue;
                }
                // (m·∇) of the pole's field: the force on a small magnet of moment m sitting in it.
                let force = (moment / distance.powi(3)
                    - apart * (3.0 * moment.dot(apart) / distance.powi(5)))
                    * (self.constant * pole.strength);
                total += force;
                if bodies.is_dynamic(pole.body) {
                    loads.apply(bodies, pole.body, self.label(), -force, pole.at);
                }
            }
            if bodies.is_dynamic(id) {
                loads.apply(bodies, id, self.label(), total, at);
            }
        }
    }

    fn potential_energy(&self, bodies: &Bodies) -> f64 {
        let poles = poles(bodies);
        let mut energy = 0.0;
        for (k, a) in poles.iter().enumerate() {
            if bodies.is_dynamic(a.body) {
                energy -= a.strength * self.background.dot(a.at);
            }
            for b in &poles[k + 1..] {
                let moves = bodies.is_dynamic(a.body) || bodies.is_dynamic(b.body);
                let distance = (a.at - b.at).length();
                if a.body != b.body && moves && distance > 0.0 {
                    energy += self.constant * a.strength * b.strength / distance;
                }
            }
        }
        for id in bodies.ids().filter(|&id| bodies.polarisability(id) != 0.0) {
            let field = self.field(&poles, bodies.position(id), Some(id));
            energy -= 0.5 * bodies.polarisability(id) * field.length_squared();
        }
        energy
    }

    fn magnetic_field_in_plane(&self, bodies: &Bodies, point: Vec2, skip: Option<BodyId>) -> Vec2 {
        self.field(&poles(bodies), point, skip)
    }
}
