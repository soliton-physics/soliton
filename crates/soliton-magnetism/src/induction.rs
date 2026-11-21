//! Electromagnetic induction. Every emf here is found the way Faraday's law says, from how much the flux
//! through the circuit changed between one moment and the next; the textbook's formula is the thing compared.

use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use crate::{G, MU0};
use soliton_modern::frame::{answer, curve, measured, plot, reading, said, Style};
use std::f64::consts::PI;

/// A graph of something that goes both ways need not start from zero.
trait Signed {
    fn signed(self) -> Self;
}

impl Signed for Plot {
    fn signed(mut self) -> Self {
        self.from_zero = false;
        self
    }
}

fn series(
    name: &str,
    history: &[[f64; 3]],
    column: usize,
    style: Style,
    tone: u8,
) -> soliton_modern::frame::Curve {
    curve(name, history.iter().map(|p| (p[0], p[column])), style, tone)
}

/// A small magnet carried at a steady speed along the axis of a coil and out the other side.
pub struct Faraday {
    /// A·m², m/s, m, Ω.
    pub moment: f64,
    pub speed: f64,
    pub turns: f64,
    pub radius: f64,
    pub resistance: f64,
    place: f64,
    time: f64,
    emf: f64,
    peak: f64,
    charge: f64,
    /// ms, mWb, mV.
    history: Vec<[f64; 3]>,
}

/// The magnet starts and ends this many coil radii away.
const REACH: f64 = 6.0;

impl Faraday {
    pub fn new(moment: f64, speed: f64, turns: f64, radius: f64, resistance: f64) -> Self {
        let mut out = Self {
            moment,
            speed,
            turns,
            radius,
            resistance,
            place: -REACH * radius,
            time: 0.0,
            emf: 0.0,
            peak: 0.0,
            charge: 0.0,
            history: Vec::new(),
        };
        out.history.push([0.0, out.linkage(out.place) * 1e3, 0.0]);
        out
    }

    /// The flux of a small magnet through all the turns when it is `z` along the axis from their plane, Wb:
    /// `N μ₀mR²/2(R² + z²)^{3/2}`.
    pub fn linkage(&self, z: f64) -> f64 {
        let r2 = self.radius * self.radius;
        self.turns * MU0 * self.moment * r2 / (2.0 * (r2 + z * z).powf(1.5))
    }

    /// The largest emf, reached when the magnet is half a radius from the coil's plane.
    pub fn peak(&self) -> f64 {
        24.0 * self.turns * MU0 * self.moment * self.speed
            / (25.0 * 5f64.sqrt() * self.radius * self.radius)
    }

    /// The charge sent round while the flux grows from nothing to its most: `NΦ/R`.
    pub fn charge(&self) -> f64 {
        self.linkage(0.0) / self.resistance
    }

    fn transit(&self) -> f64 {
        2.0 * REACH * self.radius / self.speed
    }
}

impl Lesson for Faraday {
    fn advance(&mut self, dt: f64) {
        let h = self.transit() / 4000.0;
        let steps = (dt * self.transit() / 8.0 / h).round().max(1.0) as usize;
        for _ in 0..steps {
            if self.finished() {
                break;
            }
            let next = self.place + self.speed * h;
            self.emf = -(self.linkage(next) - self.linkage(self.place)) / h;
            if next <= 0.0 {
                self.charge += self.emf.abs() / self.resistance * h;
            }
            self.peak = self.peak.max(self.emf.abs());
            self.place = next;
            self.time += h;
            if self.time * 1e3 - self.history.last().map_or(0.0, |p| p[0])
                >= self.transit() * 1e3 / 300.0
            {
                self.history.push([
                    self.time * 1e3,
                    self.linkage(self.place) * 1e3,
                    self.emf * 1e3,
                ]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.place >= REACH * self.radius
    }

    fn clock(&self) -> String {
        format!("{:.0} ms", self.time * 1e3)
    }

    fn picture(&self) -> Picture {
        Picture::Coil {
            magnet: self.place / self.radius,
            reach: REACH,
            emf: self.emf / self.peak(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading(
                "Flux through all the turns",
                self.linkage(self.place) * 1e3,
                "mWb",
            ),
            reading("emf now", self.emf.abs() * 1e3, "mV"),
            reading("Current now", self.emf.abs() / self.resistance * 1e3, "mA"),
            said(
                "By Lenz's law the coil",
                if self.emf.abs() < 0.02 * self.peak() {
                    "does nothing: the flux through it is not changing"
                } else if self.place < 0.0 {
                    "pushes the magnet back: its near face is a like pole"
                } else {
                    "pulls the magnet back: the face it is leaving is an unlike pole"
                },
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        vec![
            plot(
                "emf round the coil",
                "time (ms)",
                "emf (mV)",
                vec![series("emf", &self.history, 2, Style::Solid, 0)],
            )
            .signed(),
            plot(
                "Flux through the coil",
                "time (ms)",
                "flux through all turns (mWb)",
                vec![series("flux", &self.history, 1, Style::Solid, 1)],
            ),
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        let (peak, charge) = (self.peak() * 1e3, self.charge() * 1e3);
        vec![
            measured(
                "Largest emf, the greatest rate of change of flux (mV)",
                peak,
                self.finished().then_some(self.peak * 1e3),
                5e-3 * peak,
            ),
            measured(
                "Charge sent round as it comes in, NΦ/R (mC)",
                charge,
                (self.place >= 0.0).then_some(self.charge * 1e3),
                5e-3 * charge,
            ),
        ]
    }
}

/// A rod pulled along two rails joined through a resistance, in a field at right angles to them.
pub struct Rails {
    /// T, m, Ω, N, kg.
    pub field: f64,
    pub gap: f64,
    pub resistance: f64,
    pub pull: f64,
    pub mass: f64,
    speed: f64,
    travelled: f64,
    heat: f64,
    time: f64,
    /// s, m/s, and the heating as a share of the pull's power.
    history: Vec<[f64; 3]>,
}

impl Rails {
    pub fn new(field: f64, gap: f64, resistance: f64, pull: f64, mass: f64) -> Self {
        Self {
            field,
            gap,
            resistance,
            pull,
            mass,
            speed: 0.0,
            travelled: 0.0,
            heat: 0.0,
            time: 0.0,
            history: vec![[0.0, 0.0, 0.0]],
        }
    }

    /// The braking force for each m/s of speed: the emf `Blv` drives `Blv/R`, and the field pushes back on
    /// that current with `B²l²v/R`.
    fn braking(&self) -> f64 {
        (self.field * self.gap).powi(2) / self.resistance
    }

    /// Where the braking has grown to match the pull: `FR/B²l²`.
    pub fn terminal(&self) -> f64 {
        self.pull / self.braking()
    }

    pub fn time_constant(&self) -> f64 {
        self.mass / self.braking()
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    fn span(&self) -> f64 {
        7.0 * self.time_constant()
    }
}

impl Lesson for Rails {
    fn advance(&mut self, dt: f64) {
        let h = self.span() / 4000.0;
        let steps = (dt * self.span() / 8.0 / h).round().max(1.0) as usize;
        let (k, m, pull) = (self.braking(), self.mass, self.pull);
        let rate = |v: f64| (pull - k * v) / m;
        for _ in 0..steps {
            if self.finished() {
                break;
            }
            let v = self.speed;
            let k1 = rate(v);
            let k2 = rate(v + 0.5 * h * k1);
            let k3 = rate(v + 0.5 * h * k2);
            let k4 = rate(v + h * k3);
            let next = v + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
            let middle = 0.5 * (v + next);
            // Simpson's rule for the distance and for the heat, I²R = (Blv)²/R.
            self.travelled += h * (v + 4.0 * middle + next) / 6.0;
            self.heat += h * k * (v * v + 4.0 * middle * middle + next * next) / 6.0;
            self.speed = next;
            self.time += h;
            if self.time - self.history.last().map_or(0.0, |p| p[0]) >= self.span() / 300.0 {
                self.history
                    .push([self.time, next, 100.0 * k * next / pull]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.time >= self.span()
    }

    fn clock(&self) -> String {
        format!("{:.2} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Rails {
            travelled: self.travelled / self.gap,
            speed: self.speed / self.terminal(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let emf = self.field * self.gap * self.speed;
        vec![
            reading("Speed now", self.speed, "m/s"),
            reading("emf across the rod, Blv", emf, "V"),
            reading("Current round the circuit", emf / self.resistance, "A"),
            reading(
                "Force holding the rod back, BIl",
                self.braking() * self.speed,
                "N",
            ),
            reading(
                "Work done by the pull so far",
                self.pull * self.travelled,
                "J",
            ),
            reading("Heat in the resistance so far", self.heat, "J"),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        vec![
            plot(
                "Speed of the rod",
                "time (s)",
                "speed (m/s)",
                vec![
                    series("the rod", &self.history, 1, Style::Solid, 0),
                    curve(
                        "FR/B²l²",
                        [0.0, self.span()].map(|t| (t, self.terminal())),
                        Style::Dashed,
                        2,
                    ),
                ],
            ),
            plot(
                "Where the pull's power goes",
                "time (s)",
                "share turned to heat (%)",
                vec![series("heat", &self.history, 2, Style::Solid, 1)],
            ),
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        let (terminal, work) = (self.terminal(), self.pull * self.travelled);
        vec![
            measured(
                "Terminal speed, FR/B²l² (m/s)",
                terminal,
                self.finished().then_some(self.speed),
                2e-3 * terminal,
            ),
            answer(
                "emf at that speed, Blv (V)",
                self.field * self.gap * terminal,
            ),
            measured(
                "Work by the pull, as heat and kinetic energy (J)",
                work,
                (self.time > 0.0).then(|| self.heat + 0.5 * self.mass * self.speed.powi(2)),
                2e-3 * work,
            ),
        ]
    }
}

/// A square loop let fall through a band of field that lies across its path.
pub struct Frame {
    /// m, kg, Ω, T.
    pub side: f64,
    pub mass: f64,
    pub resistance: f64,
    pub field: f64,
    /// How far the top of the loop has fallen, m, and how fast.
    top: f64,
    speed: f64,
    heat: f64,
    time: f64,
    /// ms, m/s, A.
    history: Vec<[f64; 3]>,
}

/// In loop sides, measured down from where the loop's top starts: the band of field, where the run ends, and
/// the floor of the picture.
const BAND: [f64; 2] = [1.5, 4.5];
const END: f64 = 5.0;
const FLOOR: f64 = 6.2;

impl Frame {
    pub fn new(side: f64, mass: f64, resistance: f64, field: f64) -> Self {
        Self {
            side,
            mass,
            resistance,
            field,
            top: 0.0,
            speed: 0.0,
            heat: 0.0,
            time: 0.0,
            history: vec![[0.0, 0.0, 0.0]],
        }
    }

    fn braking(&self) -> f64 {
        (self.field * self.side).powi(2) / self.resistance
    }

    /// The speed at which the braking on one edge carries the loop's weight: `mgR/B²l²`.
    pub fn terminal(&self) -> f64 {
        self.mass * G / self.braking()
    }

    /// 1 while only the lower edge is in the field, −1 while only the upper edge is, 0 otherwise: the flux
    /// through the loop changes only when one edge cuts the field and the other does not.
    fn cutting(&self, top: f64) -> f64 {
        let within = |y: f64| (BAND[0]..=BAND[1]).contains(&(y / self.side));
        match (within(top + self.side), within(top)) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        }
    }

    pub fn current(&self) -> f64 {
        self.cutting(self.top) * self.field * self.side * self.speed / self.resistance
    }

    pub fn heat(&self) -> f64 {
        self.heat
    }

    /// What the fall has cost in potential energy and not shown up as motion.
    pub fn lost(&self) -> f64 {
        self.mass * G * self.top - 0.5 * self.mass * self.speed * self.speed
    }

    /// About how long the fall takes: the time to fall freely, and for a strongly braked loop the time to
    /// creep in and out of the field at its terminal speed.
    fn span(&self) -> f64 {
        let free = 1.6 * (2.0 * END * self.side / G).sqrt();
        if self.field == 0.0 {
            free
        } else {
            free + 2.0 * self.side / self.terminal()
        }
    }
}

impl Lesson for Frame {
    fn advance(&mut self, dt: f64) {
        let h = self.span() / 40_000.0;
        let steps = (dt * self.span() / 7.0 / h).round().max(1.0) as usize;
        let k = self.braking();
        for _ in 0..steps {
            if self.finished() {
                break;
            }
            let v = self.speed;
            if self.cutting(self.top) == 0.0 || k == 0.0 {
                self.top += v * h + 0.5 * G * h * h;
                self.speed = v + G * h;
            } else {
                // While an edge cuts the field the speed closes on the terminal speed exponentially. Taking
                // that step exactly keeps a strongly braked loop steady however long the step.
                let tau = self.mass / k;
                let terminal = self.terminal();
                let excess = v - terminal;
                let fade = (-h / tau).exp();
                self.top += terminal * h + excess * tau * (1.0 - fade);
                self.heat += k
                    * (terminal * terminal * h
                        + 2.0 * terminal * excess * tau * (1.0 - fade)
                        + 0.5 * excess * excess * tau * (1.0 - fade * fade));
                self.speed = terminal + excess * fade;
            }
            self.time += h;
            if self.time - 1e-3 * self.history.last().map_or(0.0, |p| p[0]) >= self.span() / 400.0 {
                self.history
                    .push([self.time * 1e3, self.speed, self.current()]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.top >= END * self.side
    }

    fn clock(&self) -> String {
        format!("{:.0} ms", self.time * 1e3)
    }

    fn picture(&self) -> Picture {
        let most = self.field * self.side * self.terminal() / self.resistance;
        Picture::Frame {
            top: self.top / self.side,
            band: BAND,
            floor: FLOOR,
            current: if self.field == 0.0 {
                0.0
            } else {
                (self.current() / most).clamp(-1.0, 1.0)
            },
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let state = match (self.cutting(self.top), self.top / self.side) {
            (c, _) if c > 0.0 => {
                "is entering: the flux through it grows, and the current opposes that"
            }
            (c, _) if c < 0.0 => {
                "is leaving: the flux through it falls, and the current runs the other way"
            }
            (_, y) if y + 1.0 < BAND[0] => "has not reached the field: it falls freely",
            (_, y) if y > BAND[1] => "is clear of the field: it falls freely again",
            _ => "is wholly inside: the flux is steady, so no current and no braking",
        };
        let mut out = vec![
            said("The loop", state),
            reading("Speed now", self.speed, "m/s"),
            reading("Current round the loop", self.current().abs(), "A"),
            reading("Heat made in the loop so far", self.heat * 1e3, "mJ"),
        ];
        if self.field != 0.0 {
            out.push(reading(
                "Speed at which the braking carries its weight",
                self.terminal(),
                "m/s",
            ));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let end = self.history.last().map_or(0.0, |p| p[0]);
        vec![
            plot(
                "Speed of the loop",
                "time (ms)",
                "speed (m/s)",
                vec![
                    series("the loop", &self.history, 1, Style::Solid, 0),
                    curve(
                        "falling freely",
                        [0.0, end].map(|t| (t, G * t * 1e-3)),
                        Style::Dashed,
                        2,
                    ),
                ],
            ),
            plot(
                "Current round the loop",
                "time (ms)",
                "current (A)",
                vec![series("current", &self.history, 2, Style::Solid, 1)],
            )
            .signed(),
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        let heat = self.heat * 1e3;
        vec![
            measured(
                "Heat in the loop, against the energy its fall lost (mJ)",
                heat,
                (self.heat > 0.0).then(|| self.lost() * 1e3),
                0.01 * heat + 1e-6,
            ),
            answer(
                "Speed it has at the end (m/s)",
                (2.0 * (G * END * self.side - self.heat / self.mass)).sqrt(),
            ),
        ]
    }
}

/// A coil turned steadily between the poles of a magnet.
pub struct Generator {
    /// m², T, Hz.
    pub turns: f64,
    pub area: f64,
    pub field: f64,
    pub frequency: f64,
    time: f64,
    emf: f64,
    peak: f64,
    /// ms, Wb, V.
    history: Vec<[f64; 3]>,
}

/// How many turns of the coil a run lasts.
const TURNS: f64 = 3.0;

impl Generator {
    pub fn new(turns: f64, area: f64, field: f64, frequency: f64) -> Self {
        let mut out = Self {
            turns,
            area,
            field,
            frequency,
            time: 0.0,
            emf: 0.0,
            peak: 0.0,
            history: Vec::new(),
        };
        out.history.push([0.0, out.linkage(0.0), 0.0]);
        out
    }

    /// `NBA cos ωt`.
    pub fn linkage(&self, time: f64) -> f64 {
        self.turns * self.field * self.area * (2.0 * PI * self.frequency * time).cos()
    }

    /// `NBAω`.
    pub fn peak(&self) -> f64 {
        self.turns * self.field * self.area * 2.0 * PI * self.frequency
    }
}

impl Lesson for Generator {
    fn advance(&mut self, dt: f64) {
        let period = 1.0 / self.frequency;
        let h = period / 2000.0;
        let steps = (dt * period / 3.0 / h).round().max(1.0) as usize;
        for _ in 0..steps {
            if self.finished() {
                break;
            }
            self.emf = -(self.linkage(self.time + h) - self.linkage(self.time)) / h;
            self.peak = self.peak.max(self.emf.abs());
            self.time += h;
            if self.time * 1e3 - self.history.last().map_or(0.0, |p| p[0]) >= period * 1e3 / 120.0 {
                self.history
                    .push([self.time * 1e3, self.linkage(self.time), self.emf]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.time * self.frequency >= TURNS
    }

    fn clock(&self) -> String {
        format!("{:.1} ms", self.time * 1e3)
    }

    fn picture(&self) -> Picture {
        Picture::Generator {
            angle: 2.0 * PI * self.frequency * self.time,
            emf: self.emf / self.peak(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Flux through all the turns", self.linkage(self.time), "Wb"),
            reading("emf now", self.emf, "V"),
            reading("Time for one turn", 1e3 / self.frequency, "ms"),
            said(
                "The emf is greatest",
                "when the coil lies along the field: the flux through it is nothing, but changing fastest",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        vec![
            plot(
                "emf of the generator",
                "time (ms)",
                "emf (V)",
                vec![series("emf", &self.history, 2, Style::Solid, 0)],
            )
            .signed(),
            plot(
                "Flux through the coil",
                "time (ms)",
                "flux through all turns (Wb)",
                vec![series("flux", &self.history, 1, Style::Solid, 1)],
            )
            .signed(),
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        let peak = self.peak();
        vec![
            measured(
                "Peak emf, NBAω (V)",
                peak,
                (self.time * self.frequency >= 1.0).then_some(self.peak),
                2e-3 * peak,
            ),
            answer("Its rms value, peak/√2 (V)", peak / 2f64.sqrt()),
        ]
    }
}

/// A solenoid whose current is raised steadily, held, and brought steadily down again.
pub struct SelfInduction {
    /// m, m, A, s.
    pub turns: f64,
    pub length: f64,
    pub radius: f64,
    pub current: f64,
    pub ramp: f64,
    time: f64,
    emf: f64,
    rising: f64,
    work: f64,
    /// ms, A, mV.
    history: Vec<[f64; 3]>,
}

/// The run in ramp times: up for one, steady for one, down for one, and a pause.
const RUN: f64 = 3.5;

impl SelfInduction {
    pub fn new(turns: f64, length: f64, radius: f64, current: f64, ramp: f64) -> Self {
        Self {
            turns,
            length,
            radius,
            current,
            ramp,
            time: 0.0,
            emf: 0.0,
            rising: 0.0,
            work: 0.0,
            history: vec![[0.0, 0.0, 0.0]],
        }
    }

    /// `μ₀N²A/l`, H.
    pub fn inductance(&self) -> f64 {
        MU0 * self.turns * self.turns * PI * self.radius * self.radius / self.length
    }

    pub fn current_at(&self, time: f64) -> f64 {
        let t = time / self.ramp;
        self.current * t.min(3.0 - t).clamp(0.0, 1.0)
    }

    /// The flux through all the turns: each holds `BA`, with `B = μ₀nI` inside.
    fn linkage(&self, time: f64) -> f64 {
        let inside = MU0 * self.turns / self.length * self.current_at(time);
        self.turns * inside * PI * self.radius * self.radius
    }

    /// The emf while the current changes at its steady rate: `L I/T`.
    pub fn back_emf(&self) -> f64 {
        self.inductance() * self.current / self.ramp
    }

    /// `½LI²`.
    pub fn energy(&self) -> f64 {
        0.5 * self.inductance() * self.current * self.current
    }
}

impl Lesson for SelfInduction {
    fn advance(&mut self, dt: f64) {
        let span = RUN * self.ramp;
        let h = span / 7000.0;
        let steps = (dt * span / 8.0 / h).round().max(1.0) as usize;
        for _ in 0..steps {
            if self.finished() {
                break;
            }
            self.emf = -(self.linkage(self.time + h) - self.linkage(self.time)) / h;
            if self.time + h <= self.ramp * (1.0 + 1e-9) {
                self.rising = self.rising.max(self.emf.abs());
                // The supply pushes the current through against the emf.
                self.work += -self.emf * self.current_at(self.time + 0.5 * h) * h;
            }
            self.time += h;
            if self.time - 1e-3 * self.history.last().map_or(0.0, |p| p[0]) >= span / 350.0 {
                self.history
                    .push([self.time * 1e3, self.current_at(self.time), self.emf * 1e3]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.time >= RUN * self.ramp
    }

    fn clock(&self) -> String {
        format!("{:.1} ms", self.time * 1e3)
    }

    fn picture(&self) -> Picture {
        Picture::Solenoid {
            current: self.current_at(self.time) / self.current,
            emf: self.emf / self.back_emf(),
            paired: false,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let now = self.current_at(self.time);
        vec![
            reading("Inductance, L", self.inductance() * 1e3, "mH"),
            reading("Current now", now, "A"),
            reading("emf across the coil now", self.emf.abs() * 1e3, "mV"),
            said(
                "The emf",
                if self.emf < -1e-12 {
                    "opposes the current: it resists the rise"
                } else if self.emf > 1e-12 {
                    "runs with the current: it resists the fall"
                } else {
                    "is nothing: a steady current induces none"
                },
            ),
            reading(
                "Energy in its field, ½LI²",
                0.5 * self.inductance() * now * now * 1e3,
                "mJ",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        vec![
            plot(
                "emf across the coil",
                "time (ms)",
                "emf (mV)",
                vec![series("emf", &self.history, 2, Style::Solid, 0)],
            )
            .signed(),
            plot(
                "Current through it",
                "time (ms)",
                "current (A)",
                vec![series("current", &self.history, 1, Style::Solid, 1)],
            ),
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        let (emf, energy) = (self.back_emf() * 1e3, self.energy() * 1e3);
        let risen = self.time >= self.ramp;
        vec![
            answer("Inductance, μ₀N²A/l (mH)", self.inductance() * 1e3),
            measured(
                "emf while the current rises, L I/T (mV)",
                emf,
                risen.then_some(self.rising * 1e3),
                2e-3 * emf,
            ),
            measured(
                "Work done against it, ½LI² (mJ)",
                energy,
                risen.then_some(self.work * 1e3),
                2e-3 * energy,
            ),
        ]
    }
}

/// A long solenoid carrying an alternating current, with a second coil wound round its middle.
pub struct MutualInduction {
    /// m, m, A, Hz.
    pub primary: f64,
    pub secondary: f64,
    pub length: f64,
    pub radius: f64,
    pub current: f64,
    pub frequency: f64,
    time: f64,
    emf: f64,
    peak: f64,
    /// ms, A, mV.
    history: Vec<[f64; 3]>,
}

impl MutualInduction {
    pub fn new(
        primary: f64,
        secondary: f64,
        length: f64,
        radius: f64,
        current: f64,
        frequency: f64,
    ) -> Self {
        let mut out = Self {
            primary,
            secondary,
            length,
            radius,
            current,
            frequency,
            time: 0.0,
            emf: 0.0,
            peak: 0.0,
            history: Vec::new(),
        };
        // The current starts by rising at its fastest, so the emf starts at its peak.
        out.history.push([0.0, 0.0, -out.peak() * 1e3]);
        out
    }

    /// `μ₀N₁N₂A/l`, H.
    pub fn mutual(&self) -> f64 {
        MU0 * self.primary * self.secondary * PI * self.radius * self.radius / self.length
    }

    pub fn current_at(&self, time: f64) -> f64 {
        self.current * (2.0 * PI * self.frequency * time).sin()
    }

    /// The flux of the first coil's field through all the turns of the second.
    fn linkage(&self, time: f64) -> f64 {
        let inside = MU0 * self.primary / self.length * self.current_at(time);
        self.secondary * inside * PI * self.radius * self.radius
    }

    /// `MωI₀`.
    pub fn peak(&self) -> f64 {
        self.mutual() * 2.0 * PI * self.frequency * self.current
    }
}

impl Lesson for MutualInduction {
    fn advance(&mut self, dt: f64) {
        let period = 1.0 / self.frequency;
        let h = period / 2000.0;
        let steps = (dt * period / 3.0 / h).round().max(1.0) as usize;
        for _ in 0..steps {
            if self.finished() {
                break;
            }
            self.emf = -(self.linkage(self.time + h) - self.linkage(self.time)) / h;
            self.peak = self.peak.max(self.emf.abs());
            self.time += h;
            if self.time * 1e3 - self.history.last().map_or(0.0, |p| p[0]) >= period * 1e3 / 120.0 {
                self.history
                    .push([self.time * 1e3, self.current_at(self.time), self.emf * 1e3]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.time * self.frequency >= TURNS
    }

    fn clock(&self) -> String {
        format!("{:.1} ms", self.time * 1e3)
    }

    fn picture(&self) -> Picture {
        Picture::Solenoid {
            current: self.current_at(self.time) / self.current,
            emf: self.emf / self.peak(),
            paired: true,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Mutual inductance, M", self.mutual() * 1e3, "mH"),
            reading(
                "Current in the first coil now",
                self.current_at(self.time),
                "A",
            ),
            reading("emf in the second coil now", self.emf * 1e3, "mV"),
            said(
                "The emf is greatest",
                "as the current passes through zero, which is when it is changing fastest",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        vec![
            plot(
                "emf in the second coil",
                "time (ms)",
                "emf (mV)",
                vec![series("emf", &self.history, 2, Style::Solid, 0)],
            )
            .signed(),
            plot(
                "Current in the first coil",
                "time (ms)",
                "current (A)",
                vec![series("current", &self.history, 1, Style::Solid, 1)],
            )
            .signed(),
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        let peak = self.peak() * 1e3;
        vec![
            answer("Mutual inductance, μ₀N₁N₂A/l (mH)", self.mutual() * 1e3),
            measured(
                "Peak emf in the second coil, MωI₀ (mV)",
                peak,
                (self.time * self.frequency >= 1.0).then_some(self.peak * 1e3),
                2e-3 * peak,
            ),
        ]
    }
}

/// An ideal transformer at the head of a power line.
pub struct Transformer {
    /// V, W, Ω.
    pub voltage: f64,
    pub primary: f64,
    pub secondary: f64,
    pub power: f64,
    pub line: f64,
}

impl Transformer {
    /// `V_p N_s/N_p`.
    pub fn stepped(&self) -> f64 {
        self.voltage * self.secondary / self.primary
    }

    /// The power lost heating a line that carries this lesson's power at `voltage`: `(P/V)²R`.
    pub fn loss_at(&self, voltage: f64) -> f64 {
        (self.power / voltage).powi(2) * self.line
    }
}

impl Lesson for Transformer {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Transformer {
            primary: self.primary,
            secondary: self.secondary,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let (before, after) = (self.loss_at(self.voltage), self.loss_at(self.stepped()));
        let share = |loss: f64| 100.0 * loss / self.power;
        let mut out = vec![
            reading("Voltage on the line", self.stepped() / 1e3, "kV"),
            reading("Current in the line", self.power / self.stepped(), "A"),
            reading("Lost in the line", share(after).min(100.0), "%"),
            reading(
                "Current without the transformer",
                self.power / self.voltage,
                "A",
            ),
        ];
        if before >= self.power {
            out.push(said(
                "Without the transformer",
                "the line could not carry this power at all: it would all go in heating the wire",
            ));
        } else {
            out.push(reading("Lost without it", share(before), "%"));
        }
        out.push(said(
            "A transformer",
            "needs a changing flux in its core, so it works only on alternating current",
        ));
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let (low, high) = (
            self.voltage.min(self.stepped()),
            self.voltage.max(self.stepped()),
        );
        let points = (0..=120).map(|i| 0.5 * low + (1.3 * high - 0.5 * low) * f64::from(i) / 120.0);
        let lost = |v: f64| (100.0 * self.loss_at(v) / self.power).min(100.0);
        let mut out = plot(
            "Loss in the line",
            "voltage on the line (kV)",
            "power lost (%)",
            vec![curve(
                "(P/V)²R",
                points.map(|v| (v / 1e3, lost(v))),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.stepped() / 1e3, lost(self.stepped())]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer(
                "Voltage after the transformer, VₚNₛ/Nₚ (kV)",
                self.stepped() / 1e3,
            ),
            answer("Current in the line, P/V (A)", self.power / self.stepped()),
            answer(
                "Power lost in the line, I²R (kW)",
                self.loss_at(self.stepped()) / 1e3,
            ),
        ]
    }
}
