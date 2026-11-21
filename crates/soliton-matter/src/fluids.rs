//! Mechanical properties of fluids: pressure, buoyancy, flow, viscosity and surface tension.

use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use crate::{ATMOSPHERE, G};
use soliton_modern::frame::{answer, curve, measured, plot, reading, said, Style};
use std::f64::consts::PI;

/// A gauge lowered into a tank of liquid open to the air.
pub struct Depth {
    /// m, kg/m³.
    pub depth: f64,
    pub density: f64,
}

/// As deep as the tank goes, m.
const DEEPEST: f64 = 50.0;

impl Lesson for Depth {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Tank {
            depth: self.depth,
            deepest: DEEPEST.max(self.depth),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let gauge = self.density * G * self.depth;
        vec![
            reading("Pressure of the liquid above, ρgh", gauge / 1e3, "kPa"),
            reading(
                "With the air on top of that",
                (gauge + ATMOSPHERE) / 1e3,
                "kPa",
            ),
            reading(
                "The same in atmospheres",
                (gauge + ATMOSPHERE) / ATMOSPHERE,
                "atm",
            ),
            reading(
                "Force on a window a metre square",
                (gauge + ATMOSPHERE) / 1e3,
                "kN",
            ),
            reading(
                "Depth that adds one atmosphere",
                ATMOSPHERE / (self.density * G),
                "m",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let depths = (0..=100).map(|i| DEEPEST.max(self.depth) * f64::from(i) / 100.0);
        let mut out = plot(
            "Pressure",
            "depth (m)",
            "pressure (kPa)",
            vec![curve(
                "P₀ + ρgh",
                depths.map(|h| (h, (ATMOSPHERE + self.density * G * h) / 1e3)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([
            self.depth,
            (ATMOSPHERE + self.density * G * self.depth) / 1e3,
        ]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let gauge = self.density * G * self.depth;
        vec![
            answer("Gauge pressure, ρgh (kPa)", gauge / 1e3),
            answer(
                "Absolute pressure, P₀ + ρgh (kPa)",
                (gauge + ATMOSPHERE) / 1e3,
            ),
        ]
    }
}

/// A hydraulic lift: a small piston pushed down, and a large one raised by the liquid between them.
pub struct Lift {
    /// N, m, m.
    pub force: f64,
    pub small: f64,
    pub large: f64,
    /// How far the small piston has been pushed, m.
    push: f64,
}

/// How far the small piston is pushed in all, and the seconds of watching it takes.
const STROKE: f64 = 0.2;
const PUSHING: f64 = 5.0;

impl Lift {
    pub fn new(force: f64, small: f64, large: f64) -> Self {
        Self {
            force,
            small,
            large,
            push: 0.0,
        }
    }

    /// Area of the large piston over that of the small one.
    pub fn gain(&self) -> f64 {
        (self.large / self.small).powi(2)
    }

    /// The liquid cannot be squeezed, so what leaves one cylinder enters the other.
    pub fn rise(&self) -> f64 {
        self.push / self.gain()
    }
}

impl Lesson for Lift {
    fn advance(&mut self, dt: f64) {
        self.push = (self.push + STROKE * dt / PUSHING).min(STROKE);
    }

    fn finished(&self) -> bool {
        self.push >= STROKE
    }

    fn clock(&self) -> String {
        format!("pushed {:.1} cm", 100.0 * self.push)
    }

    fn picture(&self) -> Picture {
        Picture::Lift {
            small: self.small / 2.0,
            large: self.large / 2.0,
            push: self.push,
            rise: self.rise(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let pressure = self.force / (PI * self.small * self.small / 4.0);
        vec![
            reading("Pressure in the liquid, F₁/A₁", pressure / 1e3, "kPa"),
            reading("Force on the large piston", self.force * self.gain(), "N"),
            reading("Mass it can hold up", self.force * self.gain() / G, "kg"),
            reading("Small piston pushed down", 100.0 * self.push, "cm"),
            reading("Large piston raised", 100.0 * self.rise(), "cm"),
            reading("Work put in, F₁d₁", self.force * self.push, "J"),
            reading(
                "Work got out, F₂d₂",
                self.force * self.gain() * self.rise(),
                "J",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        Vec::new()
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer(
                "Force on the large piston, F₁A₂/A₁ (N)",
                self.force * self.gain(),
            ),
            answer("Times the force is multiplied", self.gain()),
            measured(
                "Rise of the large piston for the full push (cm)",
                100.0 * STROKE / self.gain(),
                self.finished().then(|| 100.0 * self.rise()),
                1e-9,
            ),
        ]
    }
}

/// A cube let go just above a tank of liquid.
pub struct Floating {
    /// kg/m³.
    pub block: f64,
    pub fluid: f64,
    /// Height of the cube's centre above the surface, m, and its velocity.
    height: f64,
    velocity: f64,
    time: f64,
    /// The force of the bottom of the tank on it, N.
    support: f64,
}

/// Side of the cube and depth of the tank, m.
const SIDE: f64 = 0.2;
const TANK: f64 = 0.9;

impl Floating {
    pub fn new(block: f64, fluid: f64) -> Self {
        Self {
            block,
            fluid,
            height: 0.75 * SIDE,
            velocity: 0.0,
            time: 0.0,
            support: 0.0,
        }
    }

    /// How much of the cube's height is under the surface, m.
    pub fn under(&self) -> f64 {
        (0.5 * SIDE - self.height).clamp(0.0, SIDE)
    }

    pub fn weight(&self) -> f64 {
        self.block * SIDE.powi(3) * G
    }

    /// Archimedes: the weight of the liquid the cube has pushed aside.
    pub fn buoyancy(&self) -> f64 {
        self.fluid * SIDE * SIDE * self.under() * G
    }

    pub fn floats(&self) -> bool {
        self.block < self.fluid
    }

    fn settled(&self) -> bool {
        self.time > 1.0
            && self.velocity.abs() < 2e-4
            && (self.support > 0.0
                || (self.weight() - self.buoyancy()).abs() < 1e-3 * self.weight())
    }
}

impl Lesson for Floating {
    fn advance(&mut self, dt: f64) {
        let mass = self.block * SIDE.powi(3);
        // The liquid resists the cube's motion, a little under what would stop it bobbing at all.
        let drag = 1.2 * (self.fluid * SIDE * SIDE * G * mass).sqrt();
        let floor = -(TANK - 0.5 * SIDE);
        let steps = (dt / 5e-4).ceil().max(1.0) as usize;
        let h = dt / steps as f64;
        for _ in 0..steps {
            let force = self.buoyancy() - self.weight() - drag * self.velocity;
            self.velocity += force / mass * h;
            self.height += self.velocity * h;
            self.support = 0.0;
            if self.height <= floor {
                self.height = floor;
                self.velocity = 0.0;
                self.support = (self.weight() - self.buoyancy()).max(0.0);
            }
        }
        self.time += dt;
    }

    fn finished(&self) -> bool {
        self.settled()
    }

    fn clock(&self) -> String {
        format!("{:.1} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Float {
            side: SIDE,
            height: self.height,
            depth: TANK,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Weight of the block", self.weight(), "N"),
            reading("Upthrust on it now", self.buoyancy(), "N"),
            reading(
                "Share of it under the surface",
                100.0 * self.under() / SIDE,
                "%",
            ),
            said(
                "In this liquid it",
                if self.floats() { "floats" } else { "sinks" },
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        Vec::new()
    }

    fn answers(&self) -> Vec<Answer> {
        if self.floats() {
            vec![measured(
                "Share under the surface when it floats, ρ_block/ρ_liquid (%)",
                100.0 * self.block / self.fluid,
                self.settled().then(|| 100.0 * self.under() / SIDE),
                0.1,
            )]
        } else {
            let apparent = (self.block - self.fluid) * SIDE.powi(3) * G;
            vec![
                answer(
                    "Upthrust when wholly under, ρ_liquid V g (N)",
                    self.fluid * SIDE.powi(3) * G,
                ),
                measured(
                    "Apparent weight on the bottom (N)",
                    apparent,
                    (self.settled() && self.support > 0.0).then_some(self.support),
                    1e-6 * apparent.max(1.0),
                ),
            ]
        }
    }
}

/// Liquid flowing steadily through a pipe with a narrow throat: a Venturi meter.
pub struct Venturi {
    /// m/s in the wide part; area of the pipe over area of the throat; kg/m³.
    pub speed: f64,
    pub ratio: f64,
    pub density: f64,
    /// Specks carried along, each `[along, across]`.
    tracers: Vec<[f64; 2]>,
    /// When the speck being timed entered the throat, and the speed its crossing gave.
    entered: Option<f64>,
    timed: Option<f64>,
    time: f64,
}

/// The pipe is a metre long; its throat runs between these two places along it, with a taper either side.
const THROAT: (f64, f64) = (0.42, 0.58);
const TAPER: f64 = 0.1;

/// Area of the pipe at a place along it, as a share of the area of the wide part, for a throat `ratio` times
/// narrower.
fn bore(ratio: f64, x: f64) -> f64 {
    let narrow = 1.0 / ratio;
    let blend = |t: f64| 1.0 + (narrow - 1.0) * t.clamp(0.0, 1.0);
    if x < THROAT.0 {
        blend((x - (THROAT.0 - TAPER)) / TAPER)
    } else if x > THROAT.1 {
        blend((THROAT.1 + TAPER - x) / TAPER)
    } else {
        narrow
    }
}

impl Venturi {
    pub fn new(speed: f64, ratio: f64, density: f64) -> Self {
        let tracers = (0..42)
            .map(|i| {
                [
                    f64::from(i % 14) / 14.0 + 0.03 * f64::from(i / 14),
                    f64::from(i / 14) - 1.0,
                ]
            })
            .map(|[x, lane]| [x % 1.0, 0.6 * lane])
            .collect();
        Self {
            speed,
            ratio,
            density,
            tracers,
            entered: None,
            timed: None,
            time: 0.0,
        }
    }

    fn area(&self, x: f64) -> f64 {
        bore(self.ratio, x)
    }

    pub fn throat_speed(&self) -> f64 {
        self.speed * self.ratio
    }

    /// Bernoulli: what the liquid gains in speed it pays for in pressure, Pa.
    pub fn pressure_drop(&self) -> f64 {
        0.5 * self.density * (self.throat_speed().powi(2) - self.speed.powi(2))
    }
}

impl Lesson for Venturi {
    fn advance(&mut self, dt: f64) {
        // Slowed so that the eye can follow a speck through the throat; the timing below is in the pipe's time.
        let slowed = dt * 0.12;
        let steps = (slowed * self.throat_speed() / 2e-3).ceil().max(1.0) as usize;
        let h = slowed / steps as f64;
        for _ in 0..steps {
            let (now, ratio) = (self.time, self.ratio);
            for (i, tracer) in self.tracers.iter_mut().enumerate() {
                let before = tracer[0];
                // Continuity: the same volume passes every section each second, so it goes faster where it is narrow.
                let area = bore(ratio, before);
                tracer[0] = before + self.speed / area * h;
                if i == 0 {
                    if before < THROAT.0 && tracer[0] >= THROAT.0 {
                        self.entered = Some(now);
                    }
                    if before < THROAT.1 && tracer[0] >= THROAT.1 {
                        if let Some(start) = self.entered.take() {
                            self.timed = Some((THROAT.1 - THROAT.0) / (now + h - start));
                        }
                    }
                }
                if tracer[0] >= 1.0 {
                    tracer[0] -= 1.0;
                }
            }
            self.time += h;
        }
    }

    fn clock(&self) -> String {
        format!("{:.2} s", self.time)
    }

    fn picture(&self) -> Picture {
        // Each gauge stands as high as the pressure under it will hold the liquid.
        let drop = self.pressure_drop() / (self.density * G);
        let wide = drop + 0.15;
        Picture::Venturi {
            tracers: self
                .tracers
                .iter()
                .map(|t| [t[0], t[1] * self.area(t[0]).sqrt()])
                .collect(),
            throat: (1.0 / self.ratio).sqrt(),
            gauges: [wide, wide - drop],
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Speed in the wide part", self.speed, "m/s"),
            reading("Speed in the throat, v₁A₁/A₂", self.throat_speed(), "m/s"),
            reading(
                "Pressure lost in the throat",
                self.pressure_drop() / 1e3,
                "kPa",
            ),
            reading(
                "Difference between the two gauges",
                self.pressure_drop() / (self.density * G),
                "m",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let along = (0..=100).map(|i| f64::from(i) / 100.0);
        let pressure = |x: f64| {
            -0.5 * self.density * ((self.speed / self.area(x)).powi(2) - self.speed.powi(2)) / 1e3
        };
        vec![
            plot(
                "Speed",
                "distance along the pipe (m)",
                "speed (m/s)",
                vec![curve(
                    "v",
                    along.clone().map(|x| (x, self.speed / self.area(x))),
                    Style::Solid,
                    0,
                )],
            ),
            {
                let mut p = plot(
                    "Pressure",
                    "distance along the pipe (m)",
                    "pressure, against the wide part (kPa)",
                    vec![curve("p", along.map(|x| (x, pressure(x))), Style::Solid, 1)],
                );
                p.from_zero = false;
                p
            },
        ]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            measured(
                "Speed in the throat, v₁A₁/A₂ (m/s)",
                self.throat_speed(),
                self.timed,
                0.02 * self.throat_speed(),
            ),
            answer(
                "Pressure drop, ½ρ(v₂² − v₁²) (kPa)",
                self.pressure_drop() / 1e3,
            ),
            answer(
                "Difference in the gauges, (v₂² − v₁²)/2g (m)",
                self.pressure_drop() / (self.density * G),
            ),
        ]
    }
}

/// A tank of water emptying through a small hole in its side.
pub struct Draining {
    /// Water above the hole at the start, height of the hole above the ground, and the diameters of the hole
    /// and of the tank, all m.
    pub full: f64,
    pub height: f64,
    pub hole: f64,
    pub tank: f64,
    level: f64,
    time: f64,
    emptied: Option<f64>,
}

/// Seconds of watching for the tank to empty.
const EMPTYING: f64 = 18.0;

impl Draining {
    pub fn new(full: f64, height: f64, hole: f64, tank: f64) -> Self {
        Self {
            full,
            height,
            hole,
            tank,
            level: full,
            time: 0.0,
            emptied: None,
        }
    }

    /// Torricelli: the water leaves as fast as if it had fallen from the surface.
    pub fn efflux(level: f64) -> f64 {
        (2.0 * G * level.max(0.0)).sqrt()
    }

    /// How far out the jet lands from a level `level` above the hole.
    pub fn range(&self, level: f64) -> f64 {
        Self::efflux(level) * (2.0 * self.height / G).sqrt()
    }

    /// The time to empty, from integrating the falling level: `(A/a) √(2h/g)`.
    pub fn time_to_empty(&self) -> f64 {
        (self.tank / self.hole).powi(2) * (2.0 * self.full / G).sqrt()
    }
}

impl Lesson for Draining {
    fn advance(&mut self, dt: f64) {
        if self.emptied.is_some() {
            return;
        }
        let span = dt * self.time_to_empty() / EMPTYING;
        let shrink = (self.hole / self.tank).powi(2);
        let fall = |level: f64| -shrink * Self::efflux(level);
        let steps = 40;
        let h = span / f64::from(steps);
        for _ in 0..steps {
            // The level falls at the rate the hole carries water away; stepped by the midpoint rule.
            // The last drops would take for ever to follow, the level falling ever more slowly. Near the end
            // the level goes as the square of the time left, which says how long that is.
            if self.level < 1e-4 * self.full {
                self.time += 2.0 * self.level / -fall(self.level);
                self.level = 0.0;
                self.emptied = Some(self.time);
                return;
            }
            let half = (self.level + 0.5 * h * fall(self.level)).max(0.0);
            let next = self.level + h * fall(half);
            self.level = next;
            self.time += h;
        }
    }

    fn finished(&self) -> bool {
        self.emptied.is_some()
    }

    fn clock(&self) -> String {
        format!("{:.0} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Drain {
            level: self.level,
            full: self.full,
            height: self.height,
            range: self.range(self.level),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Water above the hole now", self.level, "m"),
            reading(
                "Speed of the jet now, √(2gh)",
                Self::efflux(self.level),
                "m/s",
            ),
            reading("The jet lands this far out", self.range(self.level), "m"),
            reading(
                "Water leaving each second",
                Self::efflux(self.level) * PI * self.hole * self.hole / 4.0 * 1e3,
                "litres",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let total = self.time_to_empty();
        let law = (0..=100)
            .map(|i| f64::from(i) / 100.0)
            .map(|s| (s * total, self.full * (1.0 - s).powi(2)));
        let mut out = plot(
            "Level",
            "time (s)",
            "water above the hole (m)",
            vec![curve("h₀(1 − t/T)²", law, Style::Dashed, 2)],
        );
        out.marker = Some([self.time, self.level]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer(
                "Speed of the jet at the start, √(2gh) (m/s)",
                Self::efflux(self.full),
            ),
            answer(
                "Where it lands at the start, 2√(hH) (m)",
                2.0 * (self.full * self.height).sqrt(),
            ),
            measured(
                "Time to empty, (A/a)√(2h/g) (s)",
                self.time_to_empty(),
                self.emptied,
                0.01 * self.time_to_empty(),
            ),
        ]
    }
}

/// A ball dropped into a tall column of thick liquid.
pub struct Falling {
    /// m, kg/m³, kg/m³, Pa·s.
    pub radius: f64,
    pub ball: f64,
    pub liquid: f64,
    pub viscosity: f64,
    fallen: f64,
    velocity: f64,
    time: f64,
    /// Time and speed, a few hundred times in the first moments.
    history: Vec<[f64; 2]>,
}

/// Depth of the column, m.
const COLUMN: f64 = 0.5;

impl Falling {
    pub fn new(radius: f64, ball: f64, liquid: f64, viscosity: f64) -> Self {
        Self {
            radius,
            ball,
            liquid,
            viscosity,
            fallen: 0.0,
            velocity: 0.0,
            time: 0.0,
            history: vec![[0.0, 0.0]],
        }
    }

    /// Stokes's terminal speed, where the drag and the upthrust together balance the weight.
    pub fn terminal(&self) -> f64 {
        2.0 * self.radius * self.radius * (self.ball - self.liquid) * G / (9.0 * self.viscosity)
    }

    /// The time in which it gets most of the way there: its mass over `6πηr`.
    pub fn time_constant(&self) -> f64 {
        2.0 * self.radius * self.radius * self.ball / (9.0 * self.viscosity)
    }

    /// Below about one, the flow round the ball is smooth and Stokes's law holds.
    pub fn reynolds(&self) -> f64 {
        self.liquid * self.terminal().abs() * 2.0 * self.radius / self.viscosity
    }

    pub fn speed(&self) -> f64 {
        self.velocity
    }
}

impl Lesson for Falling {
    fn advance(&mut self, dt: f64) {
        let tau = self.time_constant();
        let steps = (dt / (0.05 * tau)).ceil().clamp(1.0, 20_000.0) as usize;
        let h = dt / steps as f64;
        // Weight less upthrust, as an acceleration, and Stokes's drag taking speed away at a rate 1/τ.
        let pull = (1.0 - self.liquid / self.ball) * G;
        let acceleration = |v: f64| pull - v / tau;
        for _ in 0..steps {
            if self.fallen >= COLUMN {
                break;
            }
            let k1 = acceleration(self.velocity);
            let k2 = acceleration(self.velocity + 0.5 * h * k1);
            let k3 = acceleration(self.velocity + 0.5 * h * k2);
            let k4 = acceleration(self.velocity + h * k3);
            let next = self.velocity + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
            self.fallen += 0.5 * (self.velocity + next) * h;
            self.velocity = next;
            self.time += h;
            if self.time <= 8.0 * tau
                && self.time - self.history.last().map_or(0.0, |p| p[0]) >= 0.04 * tau
            {
                self.history.push([self.time, self.velocity]);
            }
        }
        self.fallen = self.fallen.clamp(-COLUMN, COLUMN);
    }

    fn finished(&self) -> bool {
        self.fallen.abs() >= COLUMN
    }

    fn clock(&self) -> String {
        format!("{:.2} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Column {
            // A ball lighter than the liquid starts at the bottom and rises.
            fallen: if self.terminal() < 0.0 {
                COLUMN + self.fallen
            } else {
                self.fallen
            },
            tall: COLUMN,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let mut out = vec![
            reading("Speed now", self.velocity * 1e3, "mm/s"),
            reading(
                "Terminal speed, Stokes's law",
                self.terminal() * 1e3,
                "mm/s",
            ),
            reading(
                "Time to get most of the way there",
                self.time_constant() * 1e3,
                "ms",
            ),
            reading("Reynolds number at that speed", self.reynolds(), ""),
        ];
        if self.reynolds() > 1.0 {
            out.push(said(
                "Stokes's law here",
                "is stretched: the flow is no longer smooth",
            ));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let (tau, top) = (self.time_constant(), self.terminal());
        let law = (0..=160)
            .map(|i| 0.05 * f64::from(i) * tau)
            .map(|t| (t * 1e3, top * (1.0 - (-t / tau).exp()) * 1e3));
        vec![plot(
            "Speed",
            "time (ms)",
            "speed (mm/s)",
            vec![
                curve("v_t (1 − e^(−t/τ))", law, Style::Dashed, 2),
                curve(
                    "the ball",
                    self.history.iter().map(|p| (p[0] * 1e3, p[1] * 1e3)),
                    Style::Solid,
                    0,
                ),
            ],
        )]
    }

    fn answers(&self) -> Vec<Answer> {
        let top = self.terminal();
        vec![measured(
            "Terminal speed, 2r²(ρ − σ)g/9η (mm/s)",
            top * 1e3,
            (self.time > 12.0 * self.time_constant()).then_some(self.velocity * 1e3),
            1e-3 * top.abs() * 1e3,
        )]
    }
}

/// Fine tubes standing in a liquid, which climbs them or is pushed down in them.
pub struct Capillary {
    /// m, N/m, degrees, kg/m³.
    pub radius: f64,
    pub tension: f64,
    pub angle: f64,
    pub density: f64,
}

impl Capillary {
    pub fn rise(&self, radius: f64) -> f64 {
        2.0 * self.tension * self.angle.to_radians().cos() / (self.density * G * radius)
    }
}

impl Lesson for Capillary {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Capillary {
            tubes: [1.0, 2.0, 4.0]
                .iter()
                .map(|m| [m * self.radius, self.rise(m * self.radius)])
                .collect(),
            angle: self.angle,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading(
                "Rise in the narrowest tube",
                self.rise(self.radius) * 1e3,
                "mm",
            ),
            reading(
                "In one twice as wide",
                self.rise(2.0 * self.radius) * 1e3,
                "mm",
            ),
            said(
                "The liquid",
                if self.angle < 90.0 {
                    "wets the glass and climbs"
                } else {
                    "does not wet it, and is pushed down"
                },
            ),
            reading(
                "Extra pressure inside a drop this size, 2S/r",
                2.0 * self.tension / self.radius,
                "Pa",
            ),
            reading(
                "Inside a soap bubble this size, 4S/r",
                4.0 * self.tension / self.radius,
                "Pa",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let radii = (1..=80).map(|i| 0.05e-3 * f64::from(i));
        let mut out = plot(
            "Rise",
            "radius of the tube (mm)",
            "rise (mm)",
            vec![curve(
                "2S cos θ / ρgr",
                radii.map(|r| (r * 1e3, self.rise(r) * 1e3)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.radius * 1e3, self.rise(self.radius) * 1e3]);
        out.from_zero = self.angle < 90.0;
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer("Rise, 2S cos θ / ρgr (mm)", self.rise(self.radius) * 1e3),
            answer(
                "Extra pressure inside a drop, 2S/r (Pa)",
                2.0 * self.tension / self.radius,
            ),
        ]
    }
}
