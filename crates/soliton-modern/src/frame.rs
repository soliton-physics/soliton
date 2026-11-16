//! What a lesson shows: a picture, readings, graphs and answers. The playground draws these and decides nothing.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Reading {
    pub label: String,
    pub value: f64,
    pub unit: &'static str,
    /// Words to show in place of the number, for a reading that is not a quantity.
    pub text: &'static str,
}

pub fn reading(label: &str, value: f64, unit: &'static str) -> Reading {
    Reading {
        label: label.to_string(),
        value,
        unit,
        text: "",
    }
}

pub fn said(label: &str, text: &'static str) -> Reading {
    Reading {
        label: label.to_string(),
        value: 0.0,
        unit: "",
        text,
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Solid,
    Dashed,
    Points,
}

#[derive(Clone, Debug, Serialize)]
pub struct Curve {
    pub name: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub style: Style,
    /// Which of the palette's colours: 0 the main one, 1 the second, 2 muted.
    pub tone: u8,
}

pub fn curve(
    name: &str,
    points: impl IntoIterator<Item = (f64, f64)>,
    style: Style,
    tone: u8,
) -> Curve {
    let (x, y) = points.into_iter().unzip();
    Curve {
        name: name.to_string(),
        x,
        y,
        style,
        tone,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Plot {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub curves: Vec<Curve>,
    /// A point to mark: where things stand now.
    pub marker: Option<[f64; 2]>,
    pub from_zero: bool,
    /// Energy levels: horizontal lines with nothing along the x axis.
    pub ladder: bool,
}

pub fn plot(title: &str, x_label: &str, y_label: &str, curves: Vec<Curve>) -> Plot {
    Plot {
        title: title.to_string(),
        x_label: x_label.to_string(),
        y_label: y_label.to_string(),
        curves,
        marker: None,
        from_zero: true,
        ladder: false,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Answer {
    pub name: String,
    pub value: f64,
    pub measured: Option<f64>,
    /// How close, in the answer's own units, counts as agreeing.
    pub within: f64,
}

pub fn answer(name: &str, value: f64) -> Answer {
    Answer {
        name: name.to_string(),
        value,
        measured: None,
        within: 0.0,
    }
}

pub fn measured(name: &str, value: f64, found: Option<f64>, within: f64) -> Answer {
    Answer {
        name: name.to_string(),
        value,
        measured: found,
        within,
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Line {
    /// nm.
    pub wavelength: f64,
    pub lower: u32,
    pub upper: u32,
}

/// One alpha particle's track so far, in fm from the nucleus.
#[derive(Clone, Debug, Serialize)]
pub struct Track {
    pub points: Vec<[f64; 2]>,
    pub marked: bool,
}

/// What stands on the stage. Lengths are in the units the lesson works in; the tube is in a unit square.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Picture {
    /// A phototube: the plate the light falls on at `x = 0`, the collector at `x = 1`.
    Tube {
        photons: Vec<[f64; 2]>,
        electrons: Vec<[f64; 2]>,
        wavelength: f64,
        voltage: f64,
        emitting: bool,
    },
    /// A matter wave drawn to scale over a row of atoms, nm.
    Wave { wavelength: f64, spacing: f64 },
    /// Electrons scattered from a crystal: strength against angle from the incoming beam, degrees.
    Crystal {
        lobe: Vec<[f64; 2]>,
        peak: Option<f64>,
    },
    /// Alpha particles passing a nucleus at the origin; the stage shows `half` fm either way.
    Scattering {
        tracks: Vec<Track>,
        half: f64,
        closest: f64,
    },
    /// Bohr orbits, radii in nm, with the electron on one of them and perhaps a photon on its way out.
    Orbits {
        radii: Vec<f64>,
        level: u32,
        angle: f64,
        upper: u32,
        lower: u32,
        /// How far the photon has gone, from 0 at the atom to 1 at the edge, and its wavelength in nm.
        photon: Option<[f64; 2]>,
    },
    /// Spectral lines on a wavelength axis.
    Spectrum { lines: Vec<Line>, chosen: [u32; 2] },
    /// A sample of nuclei, row by row: 0 for one that has not decayed yet, 1 for one that has.
    Sample { columns: usize, decayed: Vec<u8> },
    /// Nothing but the first graph, drawn large.
    Chart,
}

/// One lesson in progress.
pub trait Lesson: Send + Sync {
    /// Moves on by `dt` seconds of watching.
    fn advance(&mut self, _dt: f64) {}
    fn finished(&self) -> bool {
        false
    }
    /// True for a lesson with nothing to play: a formula and its graph.
    fn still(&self) -> bool {
        false
    }
    /// The time in the lesson's own terms, for the clock.
    fn clock(&self) -> String {
        String::new()
    }
    fn picture(&self) -> Picture;
    fn readings(&self) -> Vec<Reading>;
    fn plots(&self) -> Vec<Plot>;
    fn answers(&self) -> Vec<Answer>;
}
