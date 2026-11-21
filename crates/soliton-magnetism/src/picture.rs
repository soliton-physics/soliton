//! What a lesson shows on its stage. Readings, graphs and answers are the ones modern physics uses.

use serde::Serialize;
pub use soliton_modern::frame::{Answer, Plot, Reading};

/// What stands on the stage, in the lesson's own terms; the playground draws it and decides nothing.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Picture {
    /// Wires cutting the page, and the field round them as compass needles. Lengths in metres; the stage shows
    /// `half` either way. A wire is `[x, y, sense]`: 1 for current out of the page, −1 for into it. A needle is
    /// `[x, y, angle, strength]`, strength from 0 to 1. A push is `[x, y, direction]` along x.
    Field {
        wires: Vec<[f64; 3]>,
        needles: Vec<[f64; 4]>,
        half: [f64; 2],
        probe: Option<[f64; 2]>,
        pushes: Vec<[f64; 3]>,
    },
    /// A wire carrying current at `angle` radians to a field that lies in the page; the force is out of the
    /// page or into it, and `share` of the most it could be.
    Conductor { angle: f64, out: bool, share: f64 },
    /// A moving-coil meter with its pointer turned through `deflection` radians, of a scale `full` either way.
    Meter { deflection: f64, full: f64 },
    /// A magnet on the axis of a coil, `magnet` coil radii from its plane and moving to the right; the emf as a
    /// share of the most it reaches, signed.
    Coil { magnet: f64, reach: f64, emf: f64 },
    /// A rod sliding on rails that are joined through a resistance: how far it has gone, in rail gaps, and its
    /// speed as a share of its terminal speed.
    Rails { travelled: f64, speed: f64 },
    /// A square loop falling through a band of field. Heights are measured downwards in loop sides: the top of
    /// the loop, and the band's top and bottom. `current` is signed, a share of the most it could be.
    Frame {
        top: f64,
        band: [f64; 2],
        floor: f64,
        current: f64,
    },
    /// A coil turning between two poles, its normal at `angle` radians to the field; the emf as a signed share
    /// of its peak.
    Generator { angle: f64, emf: f64 },
    /// A solenoid, perhaps with a second coil wound over it: the current as a share of its peak, and the emf
    /// induced as a signed share of its peak.
    Solenoid {
        current: f64,
        emf: f64,
        paired: bool,
    },
    /// An iron core with two windings.
    Transformer { primary: f64, secondary: f64 },
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
    /// True for a lesson with nothing to play.
    fn still(&self) -> bool {
        false
    }
    fn clock(&self) -> String {
        String::new()
    }
    fn picture(&self) -> Picture;
    fn readings(&self) -> Vec<Reading>;
    fn plots(&self) -> Vec<Plot>;
    fn answers(&self) -> Vec<Answer>;
}
