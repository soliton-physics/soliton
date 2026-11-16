//! Bohr's model of the hydrogen atom and of ions with one electron, and the spectral lines it accounts for.

use crate::constants::{BOHR_RADIUS, BOHR_SPEED, HC, RYDBERG};
use crate::frame::{
    answer, curve, plot, reading, said, Answer, Lesson, Line, Picture, Plot, Reading, Style,
};

/// Energy of the nth level, eV; radius of its orbit, nm; speed of the electron in it, m/s.
pub fn energy(z: f64, n: u32) -> f64 {
    -RYDBERG * z * z / (n * n) as f64
}

pub fn radius(z: f64, n: u32) -> f64 {
    BOHR_RADIUS * (n * n) as f64 / z
}

pub fn speed(z: f64, n: u32) -> f64 {
    BOHR_SPEED * z / n as f64
}

/// Wavelength of the photon given out in falling from `upper` to `lower`, nm.
pub fn wavelength(z: f64, upper: u32, lower: u32) -> f64 {
    HC / (energy(z, upper) - energy(z, lower))
}

pub fn series_name(lower: u32) -> &'static str {
    ["Lyman", "Balmer", "Paschen", "Brackett", "Pfund"]
        .get(lower as usize - 1)
        .copied()
        .unwrap_or("Humphreys")
}

fn region(nm: f64) -> &'static str {
    if nm < 380.0 {
        "ultraviolet"
    } else if nm <= 750.0 {
        "visible"
    } else {
        "infrared"
    }
}

/// One electron round a nucleus of charge `z`, falling from one allowed orbit to another.
pub struct Atom {
    pub z: f64,
    pub upper: u32,
    pub lower: u32,
    time: f64,
}

/// The cycle that is played over and over: so long in the upper orbit, then the photon leaving.
const BEFORE: f64 = 2.5;
const FLIGHT: f64 = 2.0;
const CYCLE: f64 = 6.5;

impl Atom {
    pub fn new(z: f64, upper: u32, lower: u32) -> Self {
        let lower = lower.min(upper - 1).max(1);
        Self {
            z,
            upper,
            lower,
            time: 0.0,
        }
    }

    fn levels(&self) -> std::ops::RangeInclusive<u32> {
        1..=self.upper.max(4)
    }
}

impl Lesson for Atom {
    fn advance(&mut self, dt: f64) {
        self.time += dt;
    }

    fn clock(&self) -> String {
        let fallen = self.time % CYCLE >= BEFORE;
        format!("n = {}", if fallen { self.lower } else { self.upper })
    }

    fn picture(&self) -> Picture {
        let phase = self.time % CYCLE;
        let level = if phase < BEFORE {
            self.upper
        } else {
            self.lower
        };
        // Kepler's third law holds for these orbits too: the period goes as n³. Slowed to be watched.
        let turn = |n: u32, seconds: f64| 14.0 * seconds / (n * n * n) as f64;
        let angle = if phase < BEFORE {
            turn(self.upper, phase)
        } else {
            turn(self.upper, BEFORE) + turn(self.lower, phase - BEFORE)
        };
        let flown = (phase - BEFORE) / FLIGHT;
        Picture::Orbits {
            radii: self.levels().map(|n| radius(self.z, n)).collect(),
            level,
            angle,
            upper: self.upper,
            lower: self.lower,
            photon: (0.0..=1.0)
                .contains(&flown)
                .then(|| [flown, wavelength(self.z, self.upper, self.lower)]),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let (z, hi, lo) = (self.z, self.upper, self.lower);
        let nm = wavelength(z, hi, lo);
        vec![
            reading("Energy of the upper level", energy(z, hi), "eV"),
            reading("Energy of the lower level", energy(z, lo), "eV"),
            reading("Energy of the photon", energy(z, hi) - energy(z, lo), "eV"),
            reading("Its wavelength", nm, "nm"),
            said("Where in the spectrum", region(nm)),
            reading("Radius of the upper orbit", radius(z, hi), "nm"),
            reading(
                "Speed of the electron in it",
                speed(z, hi) / 1e6,
                "× 10⁶ m/s",
            ),
            reading(
                "Energy to free the electron from n = 1",
                -energy(z, 1),
                "eV",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let mut curves: Vec<_> = self
            .levels()
            .map(|n| {
                curve(
                    &format!("n = {n}"),
                    [(0.0, energy(self.z, n)), (1.0, energy(self.z, n))],
                    Style::Solid,
                    2,
                )
            })
            .collect();
        curves.push(curve("free", [(0.0, 0.0), (1.0, 0.0)], Style::Dashed, 2));
        curves.push(curve(
            "the jump",
            [
                (0.5, energy(self.z, self.upper)),
                (0.5, energy(self.z, self.lower)),
            ],
            Style::Solid,
            0,
        ));
        let mut out = plot("Energy levels", "", "energy (eV)", curves);
        out.marker = Some([0.5, energy(self.z, self.lower)]);
        out.from_zero = false;
        out.ladder = true;
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let (z, hi, lo) = (self.z, self.upper, self.lower);
        vec![
            answer(
                "Energy of the photon, 13.6 Z²(1/n₁² − 1/n₂²) (eV)",
                energy(z, hi) - energy(z, lo),
            ),
            answer(
                "Its wavelength, from Rydberg's formula (nm)",
                wavelength(z, hi, lo),
            ),
            answer("Radius of the upper orbit, 0.0529 n²/Z (nm)", radius(z, hi)),
        ]
    }
}

/// The line spectrum of hydrogen: every series, with one line picked out.
pub struct Series {
    pub lower: u32,
    /// Which line of the series: 1 for the first (alpha), 2 for the next.
    pub line: u32,
}

/// Lines drawn in each series before they crowd together at its limit.
const LINES: u32 = 9;

impl Series {
    fn upper(&self) -> u32 {
        self.lower + self.line
    }

    /// Where the series ends: the wavelength for an electron falling in from rest far away, nm.
    pub fn limit(&self) -> f64 {
        HC / -energy(1.0, self.lower)
    }
}

impl Lesson for Series {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Spectrum {
            lines: (1..=5)
                .flat_map(|lower| {
                    (lower + 1..=lower + LINES).map(move |upper| Line {
                        wavelength: wavelength(1.0, upper, lower),
                        lower,
                        upper,
                    })
                })
                .collect(),
            chosen: [self.lower, self.upper()],
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let nm = wavelength(1.0, self.upper(), self.lower);
        vec![
            said("Series", series_name(self.lower)),
            reading("The electron falls from level", self.upper() as f64, ""),
            reading("to level", self.lower as f64, ""),
            reading("Its wavelength", nm, "nm"),
            reading("Energy of its photon", HC / nm, "eV"),
            said("Where in the spectrum", region(nm)),
            reading(
                "Longest wavelength of the series",
                wavelength(1.0, self.lower + 1, self.lower),
                "nm",
            ),
            reading("Series limit, the shortest", self.limit(), "nm"),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let levels = 1..=(self.upper().max(6));
        let mut curves: Vec<_> = levels
            .map(|n| {
                curve(
                    &format!("n = {n}"),
                    [(0.0, energy(1.0, n)), (1.0, energy(1.0, n))],
                    Style::Solid,
                    2,
                )
            })
            .collect();
        curves.push(curve("free", [(0.0, 0.0), (1.0, 0.0)], Style::Dashed, 2));
        curves.push(curve(
            "the jump",
            [
                (0.5, energy(1.0, self.upper())),
                (0.5, energy(1.0, self.lower)),
            ],
            Style::Solid,
            0,
        ));
        let mut out = plot("Energy levels", "", "energy (eV)", curves);
        out.marker = Some([0.5, energy(1.0, self.lower)]);
        out.from_zero = false;
        out.ladder = true;
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer(
                "Wavelength of the line, 1/λ = R(1/n₁² − 1/n₂²) (nm)",
                wavelength(1.0, self.upper(), self.lower),
            ),
            answer("Series limit, n₁²/R (nm)", self.limit()),
        ]
    }
}
