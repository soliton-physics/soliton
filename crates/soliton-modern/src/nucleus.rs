//! Nuclei: radioactive decay as a game of chance, and the binding energy that holds a nucleus together.

use crate::frame::{
    answer, curve, measured, plot, reading, Answer, Lesson, Picture, Plot, Reading, Style,
};
use soliton_thermal::Rng;

/// A sample of identical unstable nuclei. Each has the same chance of decaying in the next instant however
/// long it has lasted, and that is all the law of radioactive decay is.
pub struct Sample {
    /// s.
    pub half_life: f64,
    pub decayed: Vec<bool>,
    rng: Rng,
    pub time: f64,
    /// Time and number left, a few times a second.
    history: Vec<[f64; 2]>,
    /// The sum of the ages at which nuclei decayed.
    ages: f64,
}

/// The run stops after this many half-lives.
const WATCHED: f64 = 7.0;

impl Sample {
    pub fn new(count: usize, half_life: f64, seed: u64) -> Self {
        Self {
            half_life,
            decayed: vec![false; count],
            rng: Rng::new(seed),
            time: 0.0,
            history: vec![[0.0, count as f64]],
            ages: 0.0,
        }
    }

    pub fn decay_constant(&self) -> f64 {
        std::f64::consts::LN_2 / self.half_life
    }

    pub fn left(&self) -> usize {
        self.decayed.iter().filter(|d| !**d).count()
    }

    /// How many were left at a time already passed, from the record.
    pub fn left_at(&self, time: f64) -> Option<f64> {
        let after = self.history.iter().position(|h| h[0] >= time)?;
        let b = self.history[after];
        let a = self.history[after.saturating_sub(1)];
        Some(if b[0] > a[0] {
            a[1] + (b[1] - a[1]) * (time - a[0]) / (b[0] - a[0])
        } else {
            b[1]
        })
    }

    /// The mean age at decay of the nuclei that have gone, once nearly all have.
    pub fn mean_life(&self) -> Option<f64> {
        let gone = self.decayed.len() - self.left();
        (self.finished() && gone > 0).then(|| self.ages / gone as f64)
    }
}

impl Lesson for Sample {
    fn advance(&mut self, dt: f64) {
        let end = (self.time + dt).min(WATCHED * self.half_life);
        while self.time < end {
            let step = (end - self.time).min(self.half_life / 400.0);
            let chance = 1.0 - (-self.decay_constant() * step).exp();
            for nucleus in self.decayed.iter_mut().filter(|d| !**d) {
                if self.rng.uniform() < chance {
                    *nucleus = true;
                    self.ages += self.time + 0.5 * step;
                }
            }
            self.time += step;
            if self.time - self.history.last().map_or(0.0, |h| h[0]) >= self.half_life / 40.0 {
                self.history.push([self.time, self.left() as f64]);
            }
        }
    }

    fn finished(&self) -> bool {
        self.time >= WATCHED * self.half_life - 1e-9
    }

    fn clock(&self) -> String {
        format!("{:.1} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Sample {
            columns: (self.decayed.len() as f64).sqrt().ceil() as usize,
            decayed: self.decayed.iter().map(|d| *d as u8).collect(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let left = self.left() as f64;
        vec![
            reading("Nuclei at the start", self.decayed.len() as f64, ""),
            reading("Nuclei left", left, ""),
            reading("Half-lives gone by", self.time / self.half_life, ""),
            reading(
                "Decay constant, ln 2 / T",
                self.decay_constant(),
                "per second",
            ),
            reading(
                "Activity now, λN",
                self.decay_constant() * left,
                "decays a second",
            ),
            reading("Mean life, T / ln 2", 1.0 / self.decay_constant(), "s"),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let start = self.decayed.len() as f64;
        let law = (0..=140)
            .map(|i| 0.05 * i as f64 * self.half_life)
            .map(|t| (t, start * (-self.decay_constant() * t).exp()));
        let mut out = plot(
            "Nuclei left",
            "time (s)",
            "nuclei left",
            vec![
                curve("N₀ e^(−λt)", law, Style::Dashed, 2),
                curve(
                    "the sample",
                    self.history.iter().map(|h| (h[0], h[1])),
                    Style::Solid,
                    0,
                ),
            ],
        );
        out.marker = Some([self.time, self.left() as f64]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let start = self.decayed.len() as f64;
        // Each nucleus is a coin toss, so a count expected to be Np scatters by √(Np(1 − p)).
        let scatter = |share: f64| 3.0 * (start * share * (1.0 - share)).sqrt() + 1.0;
        vec![
            measured(
                "Left after one half-life, N₀/2",
                start / 2.0,
                self.left_at(self.half_life),
                scatter(0.5),
            ),
            measured(
                "Left after two half-lives, N₀/4",
                start / 4.0,
                self.left_at(2.0 * self.half_life),
                scatter(0.25),
            ),
            measured(
                "Left after three half-lives, N₀/8",
                start / 8.0,
                self.left_at(3.0 * self.half_life),
                scatter(0.125),
            ),
            measured(
                "Mean life, T / ln 2 (s)",
                1.0 / self.decay_constant(),
                self.mean_life(),
                // Those still left at the end are not counted, which pulls the mean down a little.
                4.0 / self.decay_constant() / start.sqrt() + 0.04 / self.decay_constant(),
            ),
        ]
    }
}

/// Binding energy of a nucleus with `a` nucleons of which `z` are protons, MeV, from the liquid-drop formula
/// of Weizsäcker: volume, surface, Coulomb, asymmetry and pairing terms.
pub fn liquid_drop(a: u32, z: u32) -> f64 {
    let (mass, protons) = (a as f64, z as f64);
    let pairing = match (a % 2, z % 2) {
        (0, 0) => 11.18 / mass.sqrt(),
        (0, _) => -11.18 / mass.sqrt(),
        _ => 0.0,
    };
    15.75 * mass
        - 17.8 * mass.powf(2.0 / 3.0)
        - 0.711 * protons * (protons - 1.0) / mass.cbrt()
        - 23.7 * (mass - 2.0 * protons).powi(2) / mass
        + pairing
}

/// The number of protons that binds `a` nucleons most tightly, by the liquid-drop formula.
pub fn most_stable(a: u32) -> u32 {
    (1..a.max(2))
        .max_by(|x, y| liquid_drop(a, *x).total_cmp(&liquid_drop(a, *y)))
        .unwrap_or(1)
}

/// The most that `a` nucleons can be bound by, MeV.
pub fn best_binding(a: u32) -> f64 {
    liquid_drop(a, most_stable(a))
}

/// Measured binding energies per nucleon, MeV, from the atomic mass tables: mass number, protons, value.
pub const MEASURED: [(u32, u32, f64); 30] = [
    (2, 1, 1.112),
    (3, 1, 2.827),
    (3, 2, 2.573),
    (4, 2, 7.074),
    (6, 3, 5.332),
    (7, 3, 5.606),
    (9, 4, 6.463),
    (11, 5, 6.928),
    (12, 6, 7.680),
    (14, 7, 7.476),
    (16, 8, 7.976),
    (19, 9, 7.779),
    (20, 10, 8.032),
    (23, 11, 8.111),
    (24, 12, 8.261),
    (27, 13, 8.332),
    (28, 14, 8.448),
    (32, 16, 8.493),
    (40, 20, 8.551),
    (56, 26, 8.790),
    (62, 28, 8.795),
    (63, 29, 8.752),
    (84, 36, 8.717),
    (120, 50, 8.505),
    (138, 56, 8.393),
    (197, 79, 7.916),
    (208, 82, 7.868),
    (232, 90, 7.615),
    (235, 92, 7.591),
    (238, 92, 7.570),
];

fn table(a: u32, z: u32) -> f64 {
    let found = MEASURED
        .iter()
        .find(|m| m.0 == a && m.1 == z)
        .expect("a tabulated nucleus");
    found.2 * a as f64
}

/// Energy released when deuterium and tritium fuse to helium-4 and a neutron, MeV, from the measured values.
pub fn fusion_energy() -> f64 {
    table(4, 2) - table(2, 1) - table(3, 1)
}

/// The curve of binding energy, with one mass number picked out.
pub struct Nuclide {
    pub a: u32,
}

impl Nuclide {
    /// Energy released if the nucleus split into two equal parts, MeV. Negative if it would have to be supplied.
    pub fn fission(&self) -> f64 {
        let half = self.a / 2;
        best_binding(half) + best_binding(self.a - half) - best_binding(self.a)
    }

    /// Energy released if two such nuclei fused into one, MeV.
    pub fn fusion(&self) -> f64 {
        best_binding(2 * self.a) - 2.0 * best_binding(self.a)
    }
}

impl Lesson for Nuclide {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Chart
    }

    fn readings(&self) -> Vec<Reading> {
        let (a, z) = (self.a, most_stable(self.a));
        let mut out = vec![
            reading("Mass number, A", a as f64, ""),
            reading("Protons in its most stable form, Z", z as f64, ""),
            reading(
                "Binding energy, liquid-drop formula",
                best_binding(a),
                "MeV",
            ),
            reading("Per nucleon", best_binding(a) / a as f64, "MeV"),
        ];
        if let Some(m) = MEASURED.iter().find(|m| m.0 == a) {
            out.push(reading("Per nucleon, measured", m.2, "MeV"));
        }
        out.push(reading(
            "Released if it split into two halves",
            self.fission(),
            "MeV",
        ));
        out.push(reading(
            "Released if two of them fused",
            self.fusion(),
            "MeV",
        ));
        out
    }

    fn plots(&self) -> Vec<Plot> {
        // The formula treats the nucleus as a drop of liquid, which a handful of nucleons is not.
        // Even mass numbers only: odd ones lack the pairing term, and the two together make a sawtooth.
        let drop = (8..=125)
            .map(|i| 2 * i)
            .map(|a| (a as f64, best_binding(a) / a as f64));
        let mut out = plot(
            "Binding energy",
            "mass number, A",
            "binding energy per nucleon (MeV)",
            vec![
                curve("liquid-drop formula", drop, Style::Solid, 2),
                curve(
                    "measured",
                    MEASURED.iter().map(|m| (m.0 as f64, m.2)),
                    Style::Points,
                    0,
                ),
            ],
        );
        let per = MEASURED
            .iter()
            .find(|m| m.0 == self.a)
            .map_or(best_binding(self.a) / self.a as f64, |m| m.2);
        out.marker = Some([self.a as f64, per]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            measured(
                "Iron-56, near the top of the curve (MeV per nucleon)",
                8.790,
                Some(liquid_drop(56, 26) / 56.0),
                0.15,
            ),
            measured(
                "Uranium-238 (MeV per nucleon)",
                7.570,
                Some(liquid_drop(238, 92) / 238.0),
                0.15,
            ),
            answer(
                "Deuterium + tritium → helium-4 + neutron releases (MeV)",
                fusion_energy(),
            ),
        ]
    }
}
