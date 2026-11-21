//! What a lesson shows on its stage. Readings, graphs and answers are the ones modern physics uses.

use serde::Serialize;
pub use soliton_modern::frame::{Answer, Plot, Reading};

/// What stands on the stage, in the lesson's own terms; the playground draws it and decides nothing.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Picture {
    /// A wire hanging from a support with a load on it, stretched by `strain` of its length.
    Wire {
        strain: f64,
        broken: bool,
        load: f64,
    },
    /// A block with its top face pushed sideways through `angle` radians.
    Shear { angle: f64 },
    /// A cube squeezed from all sides, its volume less by `share`.
    Squeeze { share: f64 },
    /// A tank of liquid with a gauge `depth` metres down, of `deepest` it can go.
    Tank { depth: f64, deepest: f64 },
    /// Two pistons joined by liquid, of radii in metres; the small one pushed down `push` and the large one
    /// raised `rise`, metres.
    Lift {
        small: f64,
        large: f64,
        push: f64,
        rise: f64,
    },
    /// A cube of side `side` in a tank `depth` deep, its centre `height` above the surface, all in metres.
    Float { side: f64, height: f64, depth: f64 },
    /// A pipe that narrows: tracers as `[along, across]`, each from 0 to 1 and from −1 to 1; the bore of the
    /// throat as a share of the pipe's; the heights of water in the two gauges, metres.
    Venturi {
        tracers: Vec<[f64; 2]>,
        throat: f64,
        gauges: [f64; 2],
    },
    /// A tank draining through a hole in its side: water `level` above the hole of `full`, the hole `height`
    /// above the ground, and how far out the jet lands, metres.
    Drain {
        level: f64,
        full: f64,
        height: f64,
        range: f64,
    },
    /// A ball `fallen` metres down a column of liquid `tall` metres deep.
    Column { fallen: f64, tall: f64 },
    /// Tubes standing in liquid: each one's radius and how far the liquid rises in it, metres.
    Capillary { tubes: Vec<[f64; 2]>, angle: f64 },
    /// A rod grown by `share` of its length.
    Rod { share: f64 },
    /// A block dropped into water, with the temperature of each, °C.
    Mixing {
        block: f64,
        water: f64,
        hottest: f64,
    },
    /// A beaker being heated: how much of what is in it is ice, water and steam, and its temperature.
    Beaker {
        ice: f64,
        water: f64,
        steam: f64,
        temperature: f64,
    },
    /// A rod between a hot and a cold reservoir: the temperature along it, and where its two metals meet.
    Bar {
        temperatures: Vec<f64>,
        hot: f64,
        cold: f64,
    },
    /// Something cooling in a room.
    Cup {
        temperature: f64,
        room: f64,
        start: f64,
    },
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
