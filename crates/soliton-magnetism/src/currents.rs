//! Moving charges and magnetism: the field a current makes, and the force a field puts on a current.

use crate::biot_savart::{beside_long_wire, of_turn};
use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use crate::MU0;
use soliton_modern::frame::{answer, curve, measured, plot, reading, said, Style};
use std::f64::consts::PI;

/// Compass needles on a grid over the stage, left out where one would sit on a wire. The faintest still shows.
fn needles(
    half: [f64; 2],
    grid: [usize; 2],
    wires: &[[f64; 3]],
    field: impl Fn(f64, f64) -> [f64; 2],
) -> Vec<[f64; 4]> {
    let pitch = [
        2.0 * half[0] / grid[0] as f64,
        2.0 * half[1] / grid[1] as f64,
    ];
    let clear = 0.6 * pitch[0].min(pitch[1]);
    let mut found = Vec::new();
    for row in 0..grid[1] {
        for column in 0..grid[0] {
            let x = -half[0] + (column as f64 + 0.5) * pitch[0];
            let y = -half[1] + (row as f64 + 0.5) * pitch[1];
            if wires.iter().any(|w| (w[0] - x).hypot(w[1] - y) < clear) {
                continue;
            }
            let [bx, by] = field(x, y);
            found.push([x, y, by.atan2(bx), bx.hypot(by)]);
        }
    }
    let strongest = found.iter().map(|n| n[3]).fold(0.0, f64::max);
    for needle in &mut found {
        needle[3] = if strongest > 0.0 {
            (needle[3] / strongest).powf(0.35).max(0.25)
        } else {
            0.0
        };
    }
    found
}

/// The field of a long straight wire cutting the page at `(x0, 0)`, at a point of the page, T.
fn round_wire(current: f64, x0: f64, x: f64, y: f64) -> [f64; 2] {
    let (dx, dy) = (x - x0, y);
    let scale = MU0 * current / (2.0 * PI * (dx * dx + dy * dy));
    [-dy * scale, dx * scale]
}

/// A long straight wire and a point beside it.
pub struct StraightWire {
    /// A, m.
    pub current: f64,
    pub distance: f64,
    summed: f64,
}

impl StraightWire {
    pub fn new(current: f64, distance: f64) -> Self {
        Self {
            current,
            distance,
            summed: current * beside_long_wire(distance),
        }
    }

    /// `μ₀I/2πr`.
    pub fn field_at(&self, distance: f64) -> f64 {
        MU0 * self.current / (2.0 * PI * distance)
    }

    /// The same, from the Biot–Savart law summed along the wire.
    pub fn summed(&self) -> f64 {
        self.summed
    }
}

impl Lesson for StraightWire {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        let reach = (1.3 * self.distance).max(0.08);
        let wires = vec![[0.0, 0.0, 1.0]];
        Picture::Field {
            needles: needles([reach, reach], [11, 11], &wires, |x, y| {
                round_wire(self.current, 0.0, x, y)
            }),
            wires,
            half: [reach, reach],
            probe: Some([self.distance, 0.0]),
            pushes: Vec::new(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading(
                "Field at the marked point",
                self.field_at(self.distance) * 1e6,
                "µT",
            ),
            reading(
                "At twice the distance",
                self.field_at(2.0 * self.distance) * 1e6,
                "µT",
            ),
            reading("The earth's field, for scale", 40.0, "µT"),
            said(
                "The lines of the field",
                "are circles round the wire: grip it with the right hand, thumb along the current",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let far = (2.5 * self.distance).max(0.1);
        let points = (1..=80).map(|i| far * f64::from(i) / 80.0);
        let mut out = plot(
            "Field against distance",
            "distance from the wire (cm)",
            "field (µT)",
            vec![curve(
                "μ₀I/2πr",
                points
                    .filter(|r| *r >= 0.2 * self.distance)
                    .map(|r| (r * 100.0, self.field_at(r) * 1e6)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.distance * 100.0, self.field_at(self.distance) * 1e6]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let field = self.field_at(self.distance) * 1e6;
        vec![measured(
            "Field beside a long wire, μ₀I/2πr (µT)",
            field,
            Some(self.summed * 1e6),
            1e-3 * field.abs(),
        )]
    }
}

/// A flat circular coil and a point on its axis.
pub struct Loop {
    /// A, m, m.
    pub current: f64,
    pub turns: f64,
    pub radius: f64,
    pub distance: f64,
    summed: f64,
}

impl Loop {
    pub fn new(current: f64, turns: f64, radius: f64, distance: f64) -> Self {
        Self {
            current,
            turns,
            radius,
            distance,
            summed: turns * current * of_turn(radius, distance, 0.0, 720)[0],
        }
    }

    /// `μ₀NIR²/2(R² + x²)^{3/2}`.
    pub fn on_axis(&self, x: f64) -> f64 {
        let r2 = self.radius * self.radius;
        MU0 * self.turns * self.current * r2 / (2.0 * (r2 + x * x).powf(1.5))
    }

    pub fn summed(&self) -> f64 {
        self.summed
    }

    pub fn moment(&self) -> f64 {
        self.turns * self.current * PI * self.radius * self.radius
    }
}

impl Lesson for Loop {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        let reach = (2.2 * self.radius).max(1.2 * self.distance);
        let wires = vec![[0.0, self.radius, 1.0], [0.0, -self.radius, -1.0]];
        Picture::Field {
            needles: needles([reach, reach], [11, 11], &wires, |x, y| {
                of_turn(self.radius, x, y, 48)
            }),
            wires,
            half: [reach, reach],
            probe: Some([self.distance, 0.0]),
            pushes: Vec::new(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading(
                "Field at the centre, μ₀NI/2R",
                self.on_axis(0.0) * 1e6,
                "µT",
            ),
            reading(
                "Field at the marked point",
                self.on_axis(self.distance) * 1e6,
                "µT",
            ),
            reading("Magnetic moment, NIA", self.moment(), "A·m²"),
            said(
                "From far away the coil",
                "is a small bar magnet: its field falls off as the cube of the distance",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let far = (3.0 * self.radius).max(1.2 * self.distance);
        let points = (-60..=60).map(|i| far * f64::from(i) / 60.0);
        let mut out = plot(
            "Field along the axis",
            "distance from the centre (cm)",
            "field (µT)",
            vec![curve(
                "on the axis",
                points.map(|x| (x * 100.0, self.on_axis(x) * 1e6)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.distance * 100.0, self.on_axis(self.distance) * 1e6]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let field = self.on_axis(self.distance) * 1e6;
        vec![
            measured(
                "Field on the axis, μ₀NIR²/2(R² + x²)^(3/2) (µT)",
                field,
                Some(self.summed * 1e6),
                1e-3 * field.abs(),
            ),
            answer("Field at the centre, μ₀NI/2R (µT)", self.on_axis(0.0) * 1e6),
        ]
    }
}

/// A solenoid and a point on its axis.
pub struct Solenoid {
    /// A, m, m, and the point's distance from the centre, m.
    pub current: f64,
    pub turns: f64,
    pub length: f64,
    pub radius: f64,
    pub position: f64,
}

/// How many turns stand for the winding in the picture.
const DRAWN: usize = 24;

impl Solenoid {
    /// `μ₀nI`, the field deep inside a long solenoid.
    pub fn inside(&self) -> f64 {
        MU0 * self.turns / self.length * self.current
    }

    /// The field on the axis of a solenoid of this length, its ends counted: `½μ₀nI(cos θ₁ + cos θ₂)`, the
    /// angles being those the two rims are seen at.
    pub fn on_axis(&self, x: f64) -> f64 {
        let towards = |end: f64| end / end.hypot(self.radius);
        0.5 * self.inside() * (towards(0.5 * self.length + x) + towards(0.5 * self.length - x))
    }

    /// The same, adding up the field of every turn.
    pub fn summed(&self, x: f64) -> f64 {
        let turns = self.turns.round().max(1.0) as usize;
        let r2 = self.radius * self.radius;
        (0..turns)
            .map(|i| {
                let at = self.length * ((i as f64 + 0.5) / turns as f64 - 0.5);
                MU0 * self.current * r2 / (2.0 * (r2 + (x - at).powi(2)).powf(1.5))
            })
            .sum()
    }
}

impl Lesson for Solenoid {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        let half = [
            0.85 * self.length,
            (3.5 * self.radius).max(0.35 * self.length),
        ];
        let places: Vec<f64> = (0..DRAWN)
            .map(|i| self.length * ((i as f64 + 0.5) / DRAWN as f64 - 0.5))
            .collect();
        let wires: Vec<[f64; 3]> = places
            .iter()
            .flat_map(|x| [[*x, self.radius, 1.0], [*x, -self.radius, -1.0]])
            .collect();
        Picture::Field {
            needles: needles(half, [17, 9], &[], |x, y| {
                places.iter().fold([0.0, 0.0], |sum, at| {
                    let b = of_turn(self.radius, x - at, y, 24);
                    [sum[0] + b[0], sum[1] + b[1]]
                })
            }),
            wires,
            half,
            probe: Some([self.position, 0.0]),
            pushes: Vec::new(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Turns in each metre, n", self.turns / self.length, "/m"),
            reading(
                "Field at the marked point",
                self.on_axis(self.position) * 1e3,
                "mT",
            ),
            reading(
                "Field at the very end",
                self.on_axis(0.5 * self.length) * 1e3,
                "mT",
            ),
            said(
                "Inside, the field",
                "is even and runs along the axis; outside it is weak, like a bar magnet's",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let points = (-60..=60).map(|i| self.length * f64::from(i) / 60.0);
        let mut out = plot(
            "Field along the axis",
            "distance from the centre (cm)",
            "field (mT)",
            vec![curve(
                "on the axis",
                points.map(|x| (x * 100.0, self.on_axis(x) * 1e3)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.position * 100.0, self.on_axis(self.position) * 1e3]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let here = self.on_axis(self.position) * 1e3;
        vec![
            answer(
                "Deep inside a long solenoid, μ₀nI (mT)",
                self.inside() * 1e3,
            ),
            measured(
                "At the marked point, the ends counted (mT)",
                here,
                Some(self.summed(self.position) * 1e3),
                5e-3 * here.abs(),
            ),
        ]
    }
}

/// Two long parallel wires.
pub struct Parallel {
    /// A, A, m. A negative second current flows the other way.
    pub first: f64,
    pub second: f64,
    pub separation: f64,
    summed: f64,
}

impl Parallel {
    pub fn new(first: f64, second: f64, separation: f64) -> Self {
        Self {
            first,
            second,
            separation,
            summed: first * beside_long_wire(separation) * second,
        }
    }

    /// The force on each metre of either wire, N/m: positive when they attract.
    pub fn force(&self) -> f64 {
        MU0 * self.first * self.second / (2.0 * PI * self.separation)
    }
}

impl Lesson for Parallel {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        let reach = (1.6 * self.separation).max(0.06);
        let x = 0.5 * self.separation;
        let mut wires = vec![[-x, 0.0, 1.0]];
        let mut pushes = Vec::new();
        if self.second != 0.0 {
            wires.push([x, 0.0, self.second.signum() * self.first.signum()]);
            let towards = self.force().signum();
            pushes = vec![[-x, 0.0, towards], [x, 0.0, -towards]];
        }
        Picture::Field {
            needles: needles([reach, reach], [11, 11], &wires, |px, py| {
                let a = round_wire(self.first.abs(), -x, px, py);
                let b = round_wire(self.second * self.first.signum(), x, px, py);
                [a[0] + b[0], a[1] + b[1]]
            }),
            wires,
            half: [reach, reach],
            probe: None,
            pushes,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let force = self.force();
        vec![
            reading(
                "Field of the first wire at the second",
                MU0 * self.first.abs() / (2.0 * PI * self.separation) * 1e6,
                "µT",
            ),
            reading("Force on each metre", force.abs() * 1e6, "µN"),
            said(
                "The wires",
                if force > 0.0 {
                    "attract: currents the same way pull together"
                } else if force < 0.0 {
                    "repel: currents opposite ways push apart"
                } else {
                    "feel nothing: one of them carries no current"
                },
            ),
            said(
                "The ampere was defined by this",
                "1 A in each of two wires 1 m apart gives 2 × 10⁻⁷ N on every metre",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let far = (2.5 * self.separation).max(0.1);
        let points = (1..=80).map(|i| far * f64::from(i) / 80.0);
        let mut out = plot(
            "Force against separation",
            "separation (cm)",
            "force on each metre (µN)",
            vec![curve(
                "μ₀I₁I₂/2πd",
                points
                    .filter(|d| *d >= 0.2 * self.separation)
                    .map(|d| (d * 100.0, (self.force() * self.separation / d).abs() * 1e6)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.separation * 100.0, self.force().abs() * 1e6]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let force = self.force().abs() * 1e6;
        vec![measured(
            "Force on each metre, μ₀I₁I₂/2πd (µN)",
            force,
            Some(self.summed.abs() * 1e6),
            1e-3 * force,
        )]
    }
}

/// A straight wire in an even field.
pub struct Conductor {
    /// A, m, T, and the angle between the current and the field, radians.
    pub current: f64,
    pub length: f64,
    pub field: f64,
    pub angle: f64,
}

impl Conductor {
    /// `ILB sin θ`.
    pub fn force(&self) -> f64 {
        self.current * self.length * self.field * self.angle.sin()
    }
}

impl Lesson for Conductor {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Conductor {
            angle: self.angle,
            // With the field to the right and the current turned anticlockwise from it, L × B points into
            // the page.
            out: self.force() < 0.0,
            share: if self.current == 0.0 {
                0.0
            } else {
                self.angle.sin().abs()
            },
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Force on the wire", self.force().abs(), "N"),
            reading(
                "The most it could be, ILB",
                (self.current * self.length * self.field).abs(),
                "N",
            ),
            said(
                "Its direction",
                "is at right angles to both the current and the field: Fleming's left-hand rule",
            ),
            said("Along the field", "a current feels no force at all"),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let most = (self.current * self.length * self.field).abs();
        let mut out = plot(
            "Force against angle",
            "angle between current and field (°)",
            "force (N)",
            vec![curve(
                "ILB sin θ",
                (0..=90).map(|i| {
                    let degrees = 2.0 * f64::from(i);
                    (degrees, most * degrees.to_radians().sin())
                }),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.angle.to_degrees(), self.force().abs()]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![answer("Force, ILB sin θ (N)", self.force().abs())]
    }
}

/// A moving-coil galvanometer: a coil on a spring between the poles of a magnet shaped so the field always
/// lies in the coil's plane.
pub struct Galvanometer {
    /// A, m², T, N·m/rad.
    pub current: f64,
    pub turns: f64,
    pub area: f64,
    pub field: f64,
    pub stiffness: f64,
    deflection: f64,
    rate: f64,
    time: f64,
    history: Vec<[f64; 2]>,
}

/// The coil swings with this period, s, and this share of critical damping, so it overshoots once and settles.
const SWING: f64 = 1.2;
const DAMPING: f64 = 0.45;
/// The scale ends this far either side of zero, radians.
const FULL: f64 = 1.2;

impl Galvanometer {
    pub fn new(current: f64, turns: f64, area: f64, field: f64, stiffness: f64) -> Self {
        Self {
            current,
            turns,
            area,
            field,
            stiffness,
            deflection: 0.0,
            rate: 0.0,
            time: 0.0,
            history: vec![[0.0, 0.0]],
        }
    }

    /// `NIAB`, N·m: the same at every angle, because the field is radial.
    pub fn torque(&self) -> f64 {
        self.turns * self.current * self.area * self.field
    }

    /// Where the spring's torque balances it: `NIAB/k`, radians.
    pub fn rest(&self) -> f64 {
        self.torque() / self.stiffness
    }

    pub fn deflection(&self) -> f64 {
        self.deflection
    }
}

impl Lesson for Galvanometer {
    fn advance(&mut self, dt: f64) {
        let omega = 2.0 * PI / SWING;
        let steps = (dt / 1e-3).ceil().max(1.0) as usize;
        let h = dt / steps as f64;
        for _ in 0..steps {
            let pull =
                omega * omega * (self.rest() - self.deflection) - 2.0 * DAMPING * omega * self.rate;
            self.rate += pull * h;
            self.deflection += self.rate * h;
            self.time += h;
            if self.time - self.history.last().map_or(0.0, |p| p[0]) >= 0.02 {
                self.history.push([self.time, self.deflection.to_degrees()]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.time >= 5.0 * SWING
    }

    fn clock(&self) -> String {
        format!("{:.2} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Meter {
            deflection: self.deflection.clamp(-1.08 * FULL, 1.08 * FULL),
            full: FULL,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let mut out = vec![
            reading("Torque on the coil, NIAB", self.torque() * 1e9, "nN·m"),
            reading("Deflection now", self.deflection.to_degrees(), "°"),
            reading(
                "Deflection for each microampere",
                (self.turns * self.area * self.field / self.stiffness).to_degrees() * 1e-6,
                "°",
            ),
        ];
        if self.rest().abs() > FULL {
            out.push(said(
                "The pointer",
                "is off the scale: this meter needs a shunt to read so large a current",
            ));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let swing = plot(
            "The pointer settling",
            "time (s)",
            "deflection (°)",
            vec![
                curve(
                    "pointer",
                    self.history.iter().map(|p| (p[0], p[1])),
                    Style::Solid,
                    0,
                ),
                curve(
                    "NIAB/k",
                    [0.0, 5.0 * SWING].map(|t| (t, self.rest().to_degrees())),
                    Style::Dashed,
                    2,
                ),
            ],
        );
        vec![swing]
    }

    fn answers(&self) -> Vec<Answer> {
        let rest = self.rest().to_degrees();
        vec![measured(
            "Steady deflection, NIAB/k (°)",
            rest,
            self.finished().then(|| self.deflection.to_degrees()),
            0.01 * rest.abs().max(0.1),
        )]
    }
}
