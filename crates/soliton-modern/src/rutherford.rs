//! Rutherford scattering: alpha particles turned aside by the charge of a nucleus.

use crate::constants::{ALPHA_MASS, COULOMB, FM_OVER_C};
use crate::frame::{
    curve, measured, plot, reading, Answer, Lesson, Picture, Plot, Reading, Style, Track,
};

/// How far away an alpha starts and is followed to, fm: far enough that the nucleus no longer matters.
const FAR: f64 = 40_000.0;
/// The stage shows this far either side of the nucleus, fm.
pub const HALF: f64 = 420.0;
/// Seconds of watching for an undeflected alpha to cross the stage.
const CROSSING: f64 = 5.0;

/// The nearest a head-on alpha of `energy` MeV gets to a nucleus of charge `z`, fm: where all its kinetic
/// energy has become potential energy.
pub fn closest_approach(energy: f64, z: f64) -> f64 {
    2.0 * z * COULOMB / energy
}

/// The angle an alpha is turned through, degrees, for an impact parameter in fm.
pub fn scattering_angle(energy: f64, z: f64, impact: f64) -> f64 {
    if impact == 0.0 {
        180.0
    } else {
        2.0 * (closest_approach(energy, z) / (2.0 * impact.abs()))
            .atan()
            .to_degrees()
    }
}

/// One alpha followed from far away to far away.
pub struct Flight {
    pub impact: f64,
    /// Where it was at each moment while on the stage: time in fm/c, then x and y in fm.
    pub path: Vec<[f64; 3]>,
    /// The nearest it came to the nucleus, fm, and the angle it left at, degrees from the way it came.
    pub nearest: f64,
    pub turned: f64,
    /// When it left the stage.
    pub left: f64,
}

/// Integrates Newton's second law for an alpha in the Coulomb field of a fixed nucleus.
pub fn fly(energy: f64, z: f64, impact: f64) -> Flight {
    // Lengths in fm, times in fm/c, energies in MeV; the force constant over the mass is then an acceleration.
    let strength = 2.0 * z * COULOMB / ALPHA_MASS;
    let speed_far = (2.0 * energy / ALPHA_MASS).sqrt();
    let mut r = [-FAR, impact];
    // Starting at a finite distance it has already climbed a little of the hill.
    let start = r[0].hypot(r[1]);
    let mut v = [(speed_far * speed_far - 2.0 * strength / start).sqrt(), 0.0];
    let acceleration = |r: [f64; 2]| {
        let d = r[0].hypot(r[1]);
        [strength * r[0] / d.powi(3), strength * r[1] / d.powi(3)]
    };
    // Time is counted from when an alpha that felt nothing would come on stage.
    let mut time = -(FAR - HALF) / speed_far;
    let mut flight = Flight {
        impact,
        path: Vec::new(),
        nearest: start,
        turned: 0.0,
        left: f64::INFINITY,
    };
    let mut a = acceleration(r);
    for _ in 0..2_000_000 {
        let d = r[0].hypot(r[1]);
        // Short steps near the nucleus, where the path bends; long ones in the emptiness outside.
        let dt = 0.004 * d / v[0].hypot(v[1]).max(0.2 * speed_far);
        for i in 0..2 {
            v[i] += 0.5 * a[i] * dt;
            r[i] += v[i] * dt;
        }
        a = acceleration(r);
        for i in 0..2 {
            v[i] += 0.5 * a[i] * dt;
        }
        time += dt;
        let d = r[0].hypot(r[1]);
        flight.nearest = flight.nearest.min(d);
        let on_stage = r[0].abs() <= HALF && r[1].abs() <= HALF;
        if on_stage {
            flight.path.push([time, r[0], r[1]]);
        } else if !flight.path.is_empty() && flight.left.is_infinite() {
            flight.left = time;
        }
        if d > FAR && r[0] * v[0] + r[1] * v[1] > 0.0 {
            break;
        }
    }
    flight.turned = v[1].atan2(v[0]).to_degrees().abs();
    flight
}

/// A beam of alphas at evenly spaced impact parameters, and one more that is marked and measured.
pub struct Foil {
    /// MeV, protons in the nucleus, fm.
    pub energy: f64,
    pub z: f64,
    pub impact: f64,
    flights: Vec<Flight>,
    /// fm/c.
    time: f64,
    end: f64,
}

impl Foil {
    pub fn new(energy: f64, z: f64, impact: f64) -> Self {
        let mut flights = vec![fly(energy, z, impact)];
        flights.extend((-8..=8).map(|i| fly(energy, z, 45.0 * i as f64)));
        let end = flights.iter().map(|f| f.left).fold(0.0, f64::max);
        Self {
            energy,
            z,
            impact,
            flights,
            time: 0.0,
            end,
        }
    }

    fn pace(&self) -> f64 {
        2.0 * HALF / (2.0 * self.energy / ALPHA_MASS).sqrt() / CROSSING
    }

    pub fn marked(&self) -> &Flight {
        &self.flights[0]
    }

    pub fn head_on(&self) -> &Flight {
        self.flights
            .iter()
            .find(|f| f.impact == 0.0)
            .expect("the beam has a head-on alpha")
    }
}

impl Lesson for Foil {
    fn advance(&mut self, dt: f64) {
        self.time = (self.time + dt * self.pace()).min(self.end);
    }

    fn finished(&self) -> bool {
        self.time >= self.end
    }

    fn clock(&self) -> String {
        format!("{:.1} × 10⁻²¹ s", self.time * FM_OVER_C / 1e-21)
    }

    fn picture(&self) -> Picture {
        Picture::Scattering {
            tracks: self
                .flights
                .iter()
                .enumerate()
                .map(|(i, f)| Track {
                    // Every third point is plenty to draw a smooth curve.
                    points: f
                        .path
                        .iter()
                        .take_while(|p| p[0] <= self.time)
                        .step_by(3)
                        .map(|p| [p[1], p[2]])
                        .collect(),
                    marked: i == 0,
                })
                .collect(),
            half: HALF,
            closest: closest_approach(self.energy, self.z),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let d = closest_approach(self.energy, self.z);
        let marked = self.marked();
        let mut out = vec![
            reading("Energy of each alpha particle", self.energy, "MeV"),
            reading(
                "Its speed, as a share of light's",
                100.0 * (2.0 * self.energy / ALPHA_MASS).sqrt(),
                "%",
            ),
            reading("Nearest a head-on alpha can get", d, "fm"),
            reading("Impact parameter of the marked alpha", self.impact, "fm"),
            reading("Impact parameter that turns one through 90°", d / 2.0, "fm"),
        ];
        if self.time >= marked.left {
            out.push(reading(
                "Marked alpha: nearest it came",
                marked.nearest,
                "fm",
            ));
            out.push(reading("Marked alpha: turned through", marked.turned, "°"));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let theory = (1..=200)
            .map(|i| 2.0 * i as f64)
            .map(|b| (b, scattering_angle(self.energy, self.z, b)));
        let gone = self
            .flights
            .iter()
            .skip(1)
            .filter(|f| f.impact > 0.0 && self.time >= f.left);
        let mut out = plot(
            "Deflection",
            "impact parameter (fm)",
            "angle turned through (°)",
            vec![
                curve("Rutherford's formula", theory, Style::Solid, 2),
                curve(
                    "the alphas",
                    gone.map(|f| (f.impact, f.turned)),
                    Style::Points,
                    0,
                ),
            ],
        );
        let marked = self.marked();
        out.marker = (self.time >= marked.left).then_some([marked.impact.abs(), marked.turned]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let (marked, head_on) = (self.marked(), self.head_on());
        // The head-on alpha has turned back once it has left the stage the way it came.
        vec![
            measured(
                "Nearest approach, head on, 2Ze²/4πε₀E (fm)",
                closest_approach(self.energy, self.z),
                (self.time >= head_on.left).then_some(head_on.nearest),
                0.01 * closest_approach(self.energy, self.z),
            ),
            measured(
                "Angle the marked alpha turns through (°)",
                scattering_angle(self.energy, self.z, self.impact),
                (self.time >= marked.left).then_some(marked.turned),
                0.5,
            ),
        ]
    }
}
