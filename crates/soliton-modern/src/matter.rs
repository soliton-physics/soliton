//! Matter waves: de Broglie's wavelength, and the experiment of Davisson and Germer that found it.

use crate::constants::{C, E, ELECTRON_MASS, H};
use crate::frame::{
    answer, curve, measured, plot, reading, said, Answer, Lesson, Picture, Plot, Reading, Style,
};

/// The wavelength, in nm, of an electron that has fallen from rest through `voltage` volts.
pub fn electron_wavelength(voltage: f64) -> f64 {
    H / (2.0 * ELECTRON_MASS * E * voltage).sqrt() * 1e9
}

/// An electron accelerated through a voltage, and the wave that goes with it.
pub struct MatterWave {
    pub voltage: f64,
    /// Spacing of atoms to draw it against, nm.
    pub spacing: f64,
}

impl Lesson for MatterWave {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Wave {
            wavelength: electron_wavelength(self.voltage),
            spacing: self.spacing,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let momentum = (2.0 * ELECTRON_MASS * E * self.voltage).sqrt();
        let speed = momentum / ELECTRON_MASS;
        vec![
            reading(
                "Wavelength of the electron, h/p",
                electron_wavelength(self.voltage),
                "nm",
            ),
            reading("Its momentum, √(2meV)", momentum / 1e-24, "× 10⁻²⁴ kg m/s"),
            reading("Its speed", speed / 1e6, "× 10⁶ m/s"),
            reading("Its speed, as a share of light's", 100.0 * speed / C, "%"),
            reading("Spacing of atoms in a crystal, about", self.spacing, "nm"),
            // 150 g at 30 m/s.
            reading(
                "Wavelength of a cricket ball at 30 m/s",
                H / (0.15 * 30.0) / 1e-34,
                "× 10⁻³⁴ m",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let volts = (1..=200).map(|i| 5.0 * i as f64);
        let mut out = plot(
            "Wavelength",
            "accelerating voltage (V)",
            "wavelength (nm)",
            vec![curve(
                "1.227/√V",
                volts.map(|v| (v, electron_wavelength(v))),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.voltage, electron_wavelength(self.voltage)]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer(
                "Wavelength, h/√(2meV) (nm)",
                electron_wavelength(self.voltage),
            ),
            answer("The same, from 1.227/√V (nm)", 1.227 / self.voltage.sqrt()),
        ]
    }
}

/// Electrons of one energy scattered from the face of a crystal, whose rows of atoms act as a grating.
pub struct Diffraction {
    pub voltage: f64,
    /// Distance between the rows of atoms, nm: 0.215 for the nickel of Davisson and Germer.
    pub spacing: f64,
}

/// Rows of atoms that scatter together. Few, because slow electrons do not go deep.
const ROWS: f64 = 6.0;

impl Diffraction {
    /// How strongly electrons come off at an angle, in degrees from the incoming beam: the rows of atoms as a
    /// grating of a few lines, on a background that fades towards grazing angles.
    pub fn strength(&self, degrees: f64) -> f64 {
        let theta = degrees.to_radians();
        let half =
            std::f64::consts::PI * self.spacing * theta.sin() / electron_wavelength(self.voltage);
        let grating = if half.sin().abs() < 1e-9 {
            1.0
        } else {
            ((ROWS * half).sin() / (ROWS * half.sin())).powi(2)
        };
        theta.cos() * (0.25 + 0.75 * grating)
    }

    /// Where the rows of atoms reinforce: `d sin θ = λ`. `None` if the wavelength is longer than the spacing.
    pub fn peak(&self) -> Option<f64> {
        let sine = electron_wavelength(self.voltage) / self.spacing;
        (sine < 1.0).then(|| sine.asin().to_degrees())
    }

    /// The strongest angle away from straight back, read off the pattern as a detector would.
    pub fn found_peak(&self) -> Option<f64> {
        let expected = self.peak()?;
        let near = |i: i32| expected + 0.05 * i as f64;
        (-300..=300)
            .map(near)
            .filter(|a| (5.0..88.0).contains(a))
            .max_by(|a, b| {
                // The background slopes, so the hump is found on what stands above it.
                let above = |d: f64| self.strength(d) / d.to_radians().cos();
                above(*a).total_cmp(&above(*b))
            })
    }
}

impl Lesson for Diffraction {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Crystal {
            lobe: (0..=180)
                .map(|i| [0.5 * i as f64, self.strength(0.5 * i as f64)])
                .collect(),
            peak: self.peak(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let mut out = vec![
            reading(
                "Wavelength of the electrons, 1.227/√V",
                electron_wavelength(self.voltage),
                "nm",
            ),
            reading("Distance between rows of atoms", self.spacing, "nm"),
        ];
        match self.peak() {
            Some(angle) => out.push(reading("Angle where the rows reinforce", angle, "°")),
            None => out.push(said(
                "Angle where the rows reinforce",
                "none: the wavelength is longer than the spacing",
            )),
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let angles = (0..=180).map(|i| 0.5 * i as f64);
        let mut out = plot(
            "Scattering",
            "angle from the incoming beam (°)",
            "electrons detected",
            vec![curve(
                "pattern",
                angles.map(|a| (a, self.strength(a))),
                Style::Solid,
                0,
            )],
        );
        out.marker = self.peak().map(|a| [a, self.strength(a)]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let mut out = vec![answer(
            "Wavelength of the electrons (nm)",
            electron_wavelength(self.voltage),
        )];
        if let Some(angle) = self.peak() {
            out.push(measured(
                "Angle of strongest scattering, sin θ = λ/d (°)",
                angle,
                self.found_peak(),
                1.5,
            ));
        }
        out
    }
}
