//! The photoelectric effect: light knocking electrons out of a metal, one photon to one electron.

use crate::constants::{C, HC, H_EV};
use crate::frame::{
    answer, curve, measured, plot, reading, Answer, Lesson, Picture, Plot, Reading, Style,
};
use soliton_thermal::Rng;

/// Photons a second at full intensity, as drawn. A real lamp sends some 10¹⁵; the laws do not depend on it.
const PHOTON_RATE: f64 = 40.0;
/// How fast a photon crosses the picture, and how hard a volt pushes an electron, in the picture's units.
const PHOTON_SPEED: f64 = 1.6;
const PUSH: f64 = 0.5;

/// Einstein's equation: the most energy an electron can leave with, eV. Negative below the threshold.
pub fn maximum_energy(wavelength: f64, work_function: f64) -> f64 {
    HC / wavelength - work_function
}

struct Photon {
    position: [f64; 2],
    /// Where on the plate it will land.
    target: f64,
}

struct Electron {
    position: [f64; 2],
    speed: f64,
}

/// A phototube: light of one wavelength on a metal plate, and a collector held at a voltage.
pub struct Tube {
    /// nm, percent, volts on the collector, eV.
    pub wavelength: f64,
    pub intensity: f64,
    pub voltage: f64,
    pub work_function: f64,
    photons: Vec<Photon>,
    electrons: Vec<Electron>,
    rng: Rng,
    time: f64,
    owed: f64,
    /// Electrons that reached the collector, counted once the first to leave have had time to arrive.
    collected: u64,
    counting_since: f64,
}

/// Time allowed for the tube to fill before electrons are counted, s.
const SETTLE: f64 = 3.0;

impl Tube {
    pub fn new(
        wavelength: f64,
        intensity: f64,
        voltage: f64,
        work_function: f64,
        seed: u64,
    ) -> Self {
        Self {
            wavelength,
            intensity,
            voltage,
            work_function,
            photons: Vec::new(),
            electrons: Vec::new(),
            rng: Rng::new(seed),
            time: 0.0,
            owed: 0.0,
            collected: 0,
            counting_since: SETTLE,
        }
    }

    pub fn maximum_energy(&self) -> f64 {
        maximum_energy(self.wavelength, self.work_function)
    }

    fn rate(&self) -> f64 {
        PHOTON_RATE * self.intensity / 100.0
    }

    /// The share of the electrons leaving the plate that reach a collector at `voltage`. They leave with every
    /// energy up to the maximum, equally often, so against a retarding voltage the share falls in a straight
    /// line to nothing at the stopping potential.
    fn share_collected(&self, voltage: f64) -> f64 {
        let most = self.maximum_energy();
        if most <= 0.0 {
            0.0
        } else {
            (1.0 + voltage / most).clamp(0.0, 1.0)
        }
    }

    /// Electrons a second expected at the collector.
    pub fn expected_current(&self) -> f64 {
        self.rate() * self.share_collected(self.voltage)
    }

    /// Electrons a second counted at the collector so far.
    pub fn measured_current(&self) -> Option<f64> {
        let elapsed = self.time - self.counting_since;
        (elapsed > 2.0).then(|| self.collected as f64 / elapsed)
    }
}

impl Lesson for Tube {
    fn advance(&mut self, dt: f64) {
        self.time += dt;
        self.owed += self.rate() * dt;
        while self.owed >= 1.0 {
            self.owed -= 1.0;
            // Spaced at random as real photons are, by starting each a random way along its path.
            let target = 0.12 + 0.76 * self.rng.uniform();
            let back = 1.0 + 0.2 * self.rng.uniform();
            self.photons.push(Photon {
                position: [0.8 * back, target + 0.6 * back],
                target,
            });
        }
        let most = self.maximum_energy();
        let mut landed = Vec::new();
        self.photons.retain_mut(|p| {
            p.position[0] -= 0.8 * PHOTON_SPEED * dt;
            p.position[1] -= 0.6 * PHOTON_SPEED * dt;
            if p.position[0] <= 0.0 {
                landed.push(p.target);
                false
            } else {
                true
            }
        });
        for y in landed {
            // One photon, one electron, and only if the photon has the energy to free it.
            if most > 0.0 {
                let energy = most * self.rng.uniform();
                self.electrons.push(Electron {
                    position: [0.0, y],
                    speed: (2.0 * PUSH * energy).sqrt(),
                });
            }
        }
        let (push, counting) = (PUSH * self.voltage, self.time >= self.counting_since);
        let mut arrived = 0;
        self.electrons.retain_mut(|e| {
            e.speed += push * dt;
            e.position[0] += e.speed * dt;
            if e.position[0] >= 1.0 {
                arrived += 1;
                false
            } else {
                // Turned back to the plate it came from.
                e.position[0] > 0.0 || e.speed > 0.0
            }
        });
        if counting {
            self.collected += arrived;
        }
    }

    fn clock(&self) -> String {
        format!("{:.1} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Tube {
            photons: self.photons.iter().map(|p| p.position).collect(),
            electrons: self.electrons.iter().map(|e| e.position).collect(),
            wavelength: self.wavelength,
            voltage: self.voltage,
            emitting: self.maximum_energy() > 0.0,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let most = self.maximum_energy();
        let mut out = vec![
            reading("Energy of one photon, hc/λ", HC / self.wavelength, "eV"),
            reading(
                "Frequency of the light",
                C / (self.wavelength * 1e-9) / 1e14,
                "× 10¹⁴ Hz",
            ),
            reading("Work function of the metal", self.work_function, "eV"),
            reading("Threshold wavelength, hc/φ", HC / self.work_function, "nm"),
        ];
        if most > 0.0 {
            out.push(reading("Most energy an electron leaves with", most, "eV"));
            out.push(reading(
                "Electrons expected at the collector",
                self.expected_current(),
                "a second",
            ));
            if let Some(found) = self.measured_current() {
                out.push(reading(
                    "Electrons counted at the collector",
                    found,
                    "a second",
                ));
            }
        } else {
            out.push(reading("Energy a photon is short by", -most, "eV"));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let most = self.maximum_energy();
        let volts = (0..=160).map(|i| -4.0 + 0.05 * i as f64);
        let mut current = plot(
            "Current",
            "voltage on the collector (V)",
            "electrons a second",
            vec![curve(
                "Einstein",
                volts.map(|v| (v, self.rate() * self.share_collected(v))),
                Style::Solid,
                0,
            )],
        );
        current.marker = Some([
            self.voltage,
            self.measured_current()
                .unwrap_or_else(|| self.expected_current()),
        ]);
        // Frequencies in units of 10¹⁴ Hz, from the infrared to the far ultraviolet.
        let threshold = self.work_function / H_EV / 1e14;
        let mut energy = plot(
            "Energy",
            "frequency of the light (× 10¹⁴ Hz)",
            "most energy of an electron (eV)",
            vec![
                curve(
                    "below the threshold",
                    [(0.0, 0.0), (threshold, 0.0)],
                    Style::Dashed,
                    2,
                ),
                curve(
                    "hf − φ",
                    [
                        (threshold, 0.0),
                        (22.0, 22.0e14 * H_EV - self.work_function),
                    ],
                    Style::Solid,
                    1,
                ),
            ],
        );
        energy.marker = Some([C / (self.wavelength * 1e-9) / 1e14, most.max(0.0)]);
        vec![current, energy]
    }

    fn answers(&self) -> Vec<Answer> {
        let most = self.maximum_energy();
        let mut out = vec![answer(
            "Threshold wavelength, hc/φ (nm)",
            HC / self.work_function,
        )];
        if most > 0.0 {
            out.push(answer("Most energy of an electron, hf − φ (eV)", most));
            out.push(answer("Stopping potential (V)", most));
            let expected = self.expected_current();
            let elapsed = (self.time - self.counting_since).max(1.0);
            // Arrivals are random, so a count of N is uncertain by about its square root.
            let within = 3.0 * (expected / elapsed).sqrt() + 0.3;
            out.push(measured(
                "Electrons reaching the collector (a second)",
                expected,
                self.measured_current(),
                within,
            ));
        } else {
            out.push(answer("Electrons freed, at any brightness", 0.0));
        }
        out
    }
}
