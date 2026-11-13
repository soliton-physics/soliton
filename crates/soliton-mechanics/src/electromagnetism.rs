//! Charges and fields (design 011): Coulomb's law between charged bodies, fixed spreads of charge whose field
//! Gauss's law gives, and uniform electric and magnetic fields, optionally confined to a rectangle. The magnetic
//! field here is perpendicular to the plane of the motion, which is the one arrangement in which a charge stays
//! in that plane.

use crate::body::{Bodies, BodyId};
use crate::force::{Force, ForceLabel, Instant, Loads};
use soliton_core::Vec2;

/// Coulomb's constant `1/4πε₀`, N·m²/C².
pub const COULOMB: f64 = 8.987_551_792_3e9;

/// An axis-aligned rectangle a field is confined to: the space between a pair of plates, or the pole of a magnet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub min: Vec2,
    pub max: Vec2,
}

impl Region {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }
}

fn inside(region: &Option<Region>, point: Vec2) -> bool {
    region.is_none_or(|r| r.contains(point))
}

/// A uniform field as the front ends draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldInfo {
    pub kind: FieldKind,
    /// `None` fills the whole plane.
    pub region: Option<Region>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldKind {
    /// Electric field at this instant, N/C.
    Electric(Vec2),
    /// Magnetic field along the axis out of the page, T: positive towards the viewer.
    Magnetic(f64),
}

fn charged(bodies: &Bodies) -> impl Iterator<Item = BodyId> + '_ {
    bodies.ids().filter(|&id| bodies.charge(id) != 0.0)
}

/// The field at `at` of a unit charge at `from`, per unit of Coulomb's constant: `r̂ / r²`, pointing away.
/// A point on top of the charge feels nothing rather than infinity.
pub(crate) fn away(at: Vec2, from: Vec2) -> Vec2 {
    let separation = at - from;
    let distance = separation.length();
    if distance == 0.0 {
        Vec2::ZERO
    } else {
        separation / (distance * distance * distance)
    }
}

/// A fixed spread of charge with enough symmetry that Gauss's law gives its field in one line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Distribution {
    /// A long straight wire through the page at `at`, carrying `density` C/m: `E = λ / 2πε₀r`.
    Wire { at: Vec2, density: f64 },
    /// A large flat sheet seen edge on, through `point` with unit `normal`, carrying `density` C/m²:
    /// `E = σ / 2ε₀` on both sides, whatever the distance.
    Sheet {
        point: Vec2,
        normal: Vec2,
        density: f64,
    },
    /// A thin spherical shell with `charge` C spread evenly over it: nothing inside, a point charge outside.
    Shell {
        centre: Vec2,
        radius: f64,
        charge: f64,
    },
}

impl Distribution {
    /// The field per unit of Coulomb's constant at `point` in the plane and `height` above it, as its part in
    /// the plane and its part out of the page.
    fn field(&self, point: Vec2, height: f64) -> (Vec2, f64) {
        match *self {
            Self::Wire { at, density } => {
                let away = point - at;
                let squared = away.length_squared();
                if squared == 0.0 {
                    (Vec2::ZERO, 0.0)
                } else {
                    (away * (2.0 * density / squared), 0.0)
                }
            }
            Self::Sheet {
                point: on,
                normal,
                density,
            } => {
                let side = (point - on).dot(normal);
                let size = std::f64::consts::TAU * density;
                if side == 0.0 {
                    (Vec2::ZERO, 0.0)
                } else {
                    (normal * (size * side.signum()), 0.0)
                }
            }
            Self::Shell {
                centre,
                radius,
                charge,
            } => {
                let away = point - centre;
                let distance = distance_in_space(away, height);
                if distance <= radius {
                    (Vec2::ZERO, 0.0)
                } else {
                    let scale = charge / (distance * distance * distance);
                    (away * scale, height * scale)
                }
            }
        }
    }

    /// The potential per unit of Coulomb's constant. A wire and a sheet reach to infinity, so their zero cannot
    /// be put there: it is at one metre from the wire, and on the sheet.
    fn potential(&self, point: Vec2) -> f64 {
        match *self {
            Self::Wire { at, density } => {
                let distance = (point - at).length();
                if distance == 0.0 {
                    0.0
                } else {
                    -2.0 * density * distance.ln()
                }
            }
            Self::Sheet {
                point: on,
                normal,
                density,
            } => -std::f64::consts::TAU * density * (point - on).dot(normal).abs(),
            Self::Shell {
                centre,
                radius,
                charge,
            } => charge / (point - centre).length().max(radius),
        }
    }
}

/// How far a point `height` above the plane is from one in it, given their separation along the plane.
fn distance_in_space(along: Vec2, height: f64) -> f64 {
    (along.length_squared() + height * height).sqrt()
}

/// Coulomb's law, `F = k q₁ q₂ / r²` along the line joining two charges. Every charged body acts on every other;
/// a charged static body is a fixed charge, felt but not moved. Charges are points at the centres of their
/// bodies. Fixed wires, sheets and shells of charge ([`Distribution`]) act on the bodies too. The force reports
/// as `electric` and has a potential energy, `k q₁ q₂ / r` for a pair.
#[derive(Clone, Debug)]
pub struct Electrostatics {
    constant: f64,
    fixed: Vec<Distribution>,
}

impl Electrostatics {
    pub fn new(constant: f64) -> Self {
        Self {
            constant,
            fixed: Vec::new(),
        }
    }

    /// Adds a fixed spread of charge.
    pub fn with(mut self, distribution: Distribution) -> Self {
        self.fixed.push(distribution);
        self
    }

    /// The field at a point in the plane and `height` above it, per unit of Coulomb's constant.
    fn field_in_space(&self, bodies: &Bodies, point: Vec2, height: f64) -> (Vec2, f64) {
        let mut plane = Vec2::ZERO;
        let mut out = 0.0;
        for id in charged(bodies) {
            let away = point - bodies.position(id);
            let distance = distance_in_space(away, height);
            if distance > 0.0 {
                let scale = bodies.charge(id) / (distance * distance * distance);
                plane += away * scale;
                out += height * scale;
            }
        }
        for distribution in &self.fixed {
            let (p, z) = distribution.field(point, height);
            plane += p;
            out += z;
        }
        (plane, out)
    }
}

impl Default for Electrostatics {
    fn default() -> Self {
        Self::new(COULOMB)
    }
}

/// Rings the sphere is cut into from equator to pole, and points round each ring.
const FLUX_RINGS: usize = 160;
const FLUX_POINTS: usize = 256;

impl Force for Electrostatics {
    fn label(&self) -> ForceLabel {
        ForceLabel::Electric
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        for id in charged(bodies).filter(|&id| bodies.is_dynamic(id)) {
            let at = bodies.position(id);
            let mut field = Vec2::ZERO;
            for other in charged(bodies).filter(|&other| other != id) {
                field += away(at, bodies.position(other)) * bodies.charge(other);
            }
            for distribution in &self.fixed {
                field += distribution.field(at, 0.0).0;
            }
            let force = field * (self.constant * bodies.charge(id));
            loads.apply(bodies, id, self.label(), force, at);
        }
    }

    fn potential_energy(&self, bodies: &Bodies) -> f64 {
        let all: Vec<BodyId> = charged(bodies).collect();
        let mut energy = 0.0;
        for (k, &a) in all.iter().enumerate() {
            for &b in &all[k + 1..] {
                // Two fixed charges hold a constant energy that no motion can draw on.
                if !bodies.is_dynamic(a) && !bodies.is_dynamic(b) {
                    continue;
                }
                let distance = (bodies.position(a) - bodies.position(b)).length();
                if distance > 0.0 {
                    energy += bodies.charge(a) * bodies.charge(b) / distance;
                }
            }
            if bodies.is_dynamic(a) {
                for distribution in &self.fixed {
                    energy += bodies.charge(a) * distribution.potential(bodies.position(a));
                }
            }
        }
        self.constant * energy
    }

    fn electric_field(&self, bodies: &Bodies, point: Vec2, _time: f64) -> Vec2 {
        self.field_in_space(bodies, point, 0.0).0 * self.constant
    }

    fn electric_potential(&self, bodies: &Bodies, point: Vec2) -> Option<f64> {
        let mut potential = 0.0;
        for id in charged(bodies) {
            let distance = (point - bodies.position(id)).length();
            if distance > 0.0 {
                potential += bodies.charge(id) / distance;
            }
        }
        for distribution in &self.fixed {
            potential += distribution.potential(point);
        }
        Some(self.constant * potential)
    }

    /// `∮ E·dA` over a sphere centred in the plane, summed patch by patch. Everything here is the same above
    /// the page as below it, so the upper half is summed and doubled.
    fn flux_through_sphere(&self, bodies: &Bodies, centre: Vec2, radius: f64) -> f64 {
        let patch = radius * radius * std::f64::consts::TAU / (FLUX_RINGS * FLUX_POINTS) as f64;
        let mut flux = 0.0;
        for ring in 0..FLUX_RINGS {
            // Rings of equal height have equal area, so each patch counts the same.
            let up = (ring as f64 + 0.5) / FLUX_RINGS as f64;
            let across = (1.0 - up * up).sqrt();
            for k in 0..FLUX_POINTS {
                let angle = std::f64::consts::TAU * (k as f64 + 0.5) / FLUX_POINTS as f64;
                let outward = Vec2::new(angle.cos(), angle.sin()) * across;
                let (plane, out) =
                    self.field_in_space(bodies, centre + outward * radius, up * radius);
                flux += plane.dot(outward) + out * up;
            }
        }
        2.0 * self.constant * patch * flux
    }

    fn distributions(&self) -> &[Distribution] {
        &self.fixed
    }
}

/// A uniform electric field, `F = qE` on every charged body inside it.
///
/// Filling the plane and steady, it has the potential energy `−q E·r` and reports as `electric`. Confined to a
/// region or alternating, it has none: a field that stops dead at the edge of the plates is not one a charge
/// could be carried round for nothing. What such a field does is booked as work done from outside, by whatever
/// keeps the plates charged, and it reports as `electric` all the same.
#[derive(Clone, Copy, Debug)]
pub struct ElectricField {
    field: Vec2,
    region: Option<Region>,
    angular_frequency: f64,
    phase: f64,
}

impl ElectricField {
    pub fn uniform(field: Vec2) -> Self {
        Self {
            field,
            region: None,
            angular_frequency: 0.0,
            phase: 0.0,
        }
    }

    pub fn within(mut self, region: Region) -> Self {
        self.region = Some(region);
        self
    }

    /// Makes the field alternate as `E cos(ωt + φ)`.
    pub fn alternating(mut self, angular_frequency: f64, phase: f64) -> Self {
        self.angular_frequency = angular_frequency;
        self.phase = phase;
        self
    }

    fn conservative(&self) -> bool {
        self.region.is_none() && self.angular_frequency == 0.0
    }

    fn at_time(&self, time: f64) -> Vec2 {
        if self.angular_frequency == 0.0 {
            self.field
        } else {
            self.field * (self.angular_frequency * time + self.phase).cos()
        }
    }
}

impl Force for ElectricField {
    fn label(&self) -> ForceLabel {
        if self.conservative() {
            ForceLabel::Electric
        } else {
            ForceLabel::ElectricSupply
        }
    }

    fn apply(&self, bodies: &Bodies, instant: Instant, loads: &mut Loads) {
        let field = self.at_time(instant.time);
        for id in charged(bodies).filter(|&id| bodies.is_dynamic(id)) {
            let at = bodies.position(id);
            if inside(&self.region, at) {
                loads.apply(bodies, id, self.label(), field * bodies.charge(id), at);
            }
        }
    }

    fn potential_energy(&self, bodies: &Bodies) -> f64 {
        if !self.conservative() {
            return 0.0;
        }
        charged(bodies)
            .filter(|&id| bodies.is_dynamic(id))
            .map(|id| -bodies.charge(id) * self.field.dot(bodies.position(id)))
            .sum()
    }

    fn electric_field(&self, _bodies: &Bodies, point: Vec2, time: f64) -> Vec2 {
        if inside(&self.region, point) {
            self.at_time(time)
        } else {
            Vec2::ZERO
        }
    }

    fn electric_potential(&self, _bodies: &Bodies, point: Vec2) -> Option<f64> {
        self.conservative().then(|| -self.field.dot(point))
    }

    fn field(&self, time: f64) -> Option<FieldInfo> {
        Some(FieldInfo {
            kind: FieldKind::Electric(self.at_time(time)),
            region: self.region,
        })
    }
}

/// A uniform magnetic field perpendicular to the plane, `F = q v × B`. With `B` out of the page a positive
/// charge is turned clockwise. The force is always at right angles to the velocity, so it does no work and
/// changes no speed; it reports as `magnetic`.
#[derive(Clone, Copy, Debug)]
pub struct MagneticField {
    strength: f64,
    region: Option<Region>,
}

impl MagneticField {
    /// `strength` in tesla, positive out of the page towards the viewer.
    pub fn uniform(strength: f64) -> Self {
        Self {
            strength,
            region: None,
        }
    }

    pub fn within(mut self, region: Region) -> Self {
        self.region = Some(region);
        self
    }
}

impl Force for MagneticField {
    fn label(&self) -> ForceLabel {
        ForceLabel::Magnetic
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        for id in charged(bodies).filter(|&id| bodies.is_dynamic(id)) {
            let at = bodies.position(id);
            if inside(&self.region, at) {
                let v = bodies.velocity(id);
                // v × B ẑ, with v in the plane.
                let force = Vec2::new(v.y, -v.x) * (bodies.charge(id) * self.strength);
                loads.apply(bodies, id, self.label(), force, at);
            }
        }
    }

    fn depends_on_velocity(&self) -> bool {
        true
    }

    fn magnetic_field(&self, point: Vec2) -> f64 {
        if inside(&self.region, point) {
            self.strength
        } else {
            0.0
        }
    }

    fn field(&self, _time: f64) -> Option<FieldInfo> {
        Some(FieldInfo {
            kind: FieldKind::Magnetic(self.strength),
            region: self.region,
        })
    }
}

/// Where field lines begin or end: a point charge, a pole of a magnet, or a charged shell of some `radius`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub at: Vec2,
    /// Lines leave a positive seed and arrive at a negative one, in number proportional to its size.
    pub strength: f64,
    pub radius: f64,
}

impl Seed {
    pub fn point(at: Vec2, strength: f64) -> Self {
        Self {
            at,
            strength,
            radius: 0.0,
        }
    }
}

/// Field lines for drawing. Lines leave each positive seed, `per_seed` of them for the strongest and in
/// proportion for the rest, and are followed until they end on a negative seed or run `reach` away from
/// `centre`. Lines are then followed backwards from each negative seed, and those kept that have come in from
/// far away rather than from a positive seed, which has drawn them already. Each line is a list of points in
/// the direction of the field.
pub fn field_lines(
    field: impl Fn(Vec2) -> Vec2,
    seeds: &[Seed],
    per_seed: usize,
    centre: Vec2,
    reach: f64,
) -> Vec<Vec<Vec2>> {
    let largest = seeds.iter().map(|s| s.strength.abs()).fold(0.0, f64::max);
    if largest == 0.0 || per_seed == 0 {
        return Vec::new();
    }
    let step = reach / 150.0;
    let mut lines = Vec::new();
    for seed in seeds {
        let count = (per_seed as f64 * seed.strength.abs() / largest).round() as usize;
        let sign = seed.strength.signum();
        for k in 0..count {
            let angle = std::f64::consts::TAU * (k as f64 + 0.5) / count as f64;
            let outward = Vec2::new(angle.cos(), angle.sin());
            let mut point = seed.at + outward * (seed.radius + 2.0 * step);
            let mut line = vec![seed.at + outward * seed.radius, point];
            let mut duplicate = false;
            for _ in 0..2000 {
                // Midpoint rule on the unit field: accurate enough that a line returns to the charge it aims at.
                let Some(first) = direction(&field, point) else {
                    break;
                };
                let Some(middle) = direction(&field, point + first * (0.5 * step * sign)) else {
                    break;
                };
                point += middle * (step * sign);
                // A line ends on a seed of the other sign; a test charge too small to matter does not stop it.
                let landed = seeds.iter().find(|other| {
                    other.strength * seed.strength < 0.0
                        && other.strength.abs() >= 0.05 * largest
                        && (other.at - point).length() < other.radius + 1.5 * step
                });
                if let Some(end) = landed {
                    let inward = (point - end.at).normalized();
                    line.push(end.at + inward * end.radius);
                    duplicate = sign < 0.0;
                    break;
                }
                line.push(point);
                if (point - centre).length() > reach {
                    break;
                }
            }
            if duplicate {
                continue;
            }
            if sign < 0.0 {
                line.reverse();
            }
            lines.push(line);
        }
    }
    lines
}

fn direction(field: &impl Fn(Vec2) -> Vec2, point: Vec2) -> Option<Vec2> {
    let e = field(point);
    let size = e.length();
    (size > 0.0 && size.is_finite()).then(|| e / size)
}
