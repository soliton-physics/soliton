//! Mechanical properties of solids: stretching, shearing and squeezing.

use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use crate::{ATMOSPHERE, G};
use soliton_modern::frame::{answer, curve, plot, reading, said, Style};
use std::f64::consts::PI;

/// A wire with a load hung on it.
pub struct Wire {
    /// Pa, m, m, kg, Pa.
    pub youngs: f64,
    pub length: f64,
    pub diameter: f64,
    pub load: f64,
    pub elastic_limit: f64,
}

/// Past the elastic limit a metal gives this many times more easily, and it breaks at this multiple of the
/// limit. Round figures for a ductile metal, to show the shape of the curve.
const YIELDING: f64 = 40.0;
const BREAKING: f64 = 1.6;

impl Wire {
    pub fn area(&self) -> f64 {
        PI * self.diameter * self.diameter / 4.0
    }

    pub fn stress(&self) -> f64 {
        self.load * G / self.area()
    }

    /// The strain a stress produces: Hooke's law up to the elastic limit, and a far gentler slope beyond.
    pub fn strain_at(&self, stress: f64) -> f64 {
        let elastic = stress.min(self.elastic_limit) / self.youngs;
        elastic + (stress - self.elastic_limit).max(0.0) * YIELDING / self.youngs
    }

    pub fn broken(&self) -> bool {
        self.stress() > BREAKING * self.elastic_limit
    }

    pub fn elastic(&self) -> bool {
        self.stress() <= self.elastic_limit
    }

    pub fn extension(&self) -> f64 {
        self.strain_at(self.stress()) * self.length
    }
}

impl Lesson for Wire {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Wire {
            strain: self.strain_at(self.stress()),
            broken: self.broken(),
            load: self.load,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let state = if self.broken() {
            "it has broken"
        } else if self.elastic() {
            "elastic: it springs back when unloaded"
        } else {
            "past the elastic limit: stretched for good"
        };
        let mut out = vec![
            reading("Force on the wire, Mg", self.load * G, "N"),
            reading("Area of its cross-section", self.area() * 1e6, "mm²"),
            reading("Stress, F/A", self.stress() / 1e6, "MPa"),
            said("The wire is", state),
        ];
        if !self.broken() {
            out.push(reading(
                "Strain, ΔL/L",
                100.0 * self.strain_at(self.stress()),
                "%",
            ));
            out.push(reading("Extension", self.extension() * 1e3, "mm"));
        }
        if self.elastic() {
            out.push(reading(
                "Energy stored, ½ F ΔL",
                0.5 * self.load * G * self.extension(),
                "J",
            ));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let top = BREAKING * self.elastic_limit;
        let points = (0..=80).map(|i| top * f64::from(i) / 80.0);
        let mut out = plot(
            "Stress and strain",
            "strain (%)",
            "stress (MPa)",
            vec![curve(
                "the wire",
                points.map(|s| (100.0 * self.strain_at(s), s / 1e6)),
                Style::Solid,
                0,
            )],
        );
        let stress = self.stress().min(top);
        out.marker = Some([100.0 * self.strain_at(stress), stress / 1e6]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let mut out = vec![answer("Stress, Mg/A (MPa)", self.stress() / 1e6)];
        if self.elastic() {
            out.push(answer("Strain, stress/Y", self.stress() / self.youngs));
            out.push(answer(
                "Extension, MgL/AY (mm)",
                self.load * G * self.length / (self.area() * self.youngs) * 1e3,
            ));
        }
        out.push(answer(
            "Heaviest load it bears and still springs back (kg)",
            self.elastic_limit * self.area() / G,
        ));
        out
    }
}

/// A block with a force along its top face.
pub struct Sheared {
    /// Pa, N, m², m.
    pub modulus: f64,
    pub force: f64,
    pub area: f64,
    pub height: f64,
}

impl Sheared {
    pub fn strain(&self) -> f64 {
        self.force / self.area / self.modulus
    }
}

impl Lesson for Sheared {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Shear {
            angle: self.strain(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Shearing stress, F/A", self.force / self.area / 1e6, "MPa"),
            reading("Shearing strain, Δx/h", self.strain() * 1e6, "× 10⁻⁶"),
            reading(
                "The top face moves across by",
                self.strain() * self.height * 1e6,
                "µm",
            ),
            reading(
                "Angle the sides lean through",
                self.strain().to_degrees() * 3600.0,
                "seconds of arc",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        Vec::new()
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer("Shearing strain, F/AG (× 10⁻⁶)", self.strain() * 1e6),
            answer(
                "Displacement of the top face, Fh/AG (µm)",
                self.strain() * self.height * 1e6,
            ),
        ]
    }
}

/// Something taken down into the sea, squeezed from every side by the water above it.
pub struct Squeezed {
    /// Pa, m.
    pub modulus: f64,
    pub depth: f64,
}

/// Density of sea water, kg/m³.
const SEA: f64 = 1030.0;

impl Squeezed {
    pub fn pressure(&self) -> f64 {
        SEA * G * self.depth
    }

    pub fn share(&self) -> f64 {
        self.pressure() / self.modulus
    }
}

impl Lesson for Squeezed {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Squeeze {
            share: self.share(),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading(
                "Pressure of the water above, ρgh",
                self.pressure() / 1e6,
                "MPa",
            ),
            reading(
                "The same in atmospheres",
                self.pressure() / ATMOSPHERE,
                "atm",
            ),
            reading("Volume lost, ΔV/V", 100.0 * self.share(), "%"),
            reading(
                "Density gained",
                100.0 * self.share() / (1.0 - self.share()),
                "%",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let depths = (0..=110).map(|i| 100.0 * f64::from(i));
        let mut out = plot(
            "Squeezing",
            "depth (m)",
            "volume lost (%)",
            vec![curve(
                "ΔV/V = ρgh/B",
                depths.map(|d| (d, 100.0 * SEA * G * d / self.modulus)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.depth, 100.0 * self.share()]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer("Pressure at that depth, ρgh (MPa)", self.pressure() / 1e6),
            answer("Fraction of its volume lost, p/B (%)", 100.0 * self.share()),
        ]
    }
}
