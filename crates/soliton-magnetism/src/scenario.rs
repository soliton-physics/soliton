//! The catalogue: class 12 currents and magnetism, chapter by chapter (design 019).

use crate::currents::{Conductor, Galvanometer, Loop, Parallel, Solenoid, StraightWire};
use crate::induction::{
    Faraday, Frame, Generator, MutualInduction, Rails, SelfInduction, Transformer,
};
use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Param {
    pub name: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

const fn param(
    name: &'static str,
    label: &'static str,
    unit: &'static str,
    range: (f64, f64),
    step: f64,
    default: f64,
) -> Param {
    Param {
        name,
        label,
        unit,
        min: range.0,
        max: range.1,
        step,
        default,
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Syllabus {
    pub class: u8,
    pub chapter: &'static str,
    pub topic: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct ScenarioSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub syllabus: Syllabus,
    pub params: &'static [Param],
}

/// Chosen parameter values; anything not set takes the spec's default.
#[derive(Clone, Debug, Default)]
pub struct Params {
    values: Vec<(String, f64)>,
}

impl Params {
    pub fn set(mut self, name: &str, value: f64) -> Self {
        self.values.retain(|(n, _)| n != name);
        self.values.push((name.to_string(), value));
        self
    }

    fn get(&self, spec: &ScenarioSpec, name: &str) -> f64 {
        let param = spec
            .params
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("scenario `{}` has no parameter `{name}`", spec.id));
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map_or(param.default, |(_, v)| {
                // A typed value may go past the slider, as far as the quantity allows.
                let (low, high) =
                    soliton_core::typed_limits(param.name, param.unit, param.min, param.max);
                v.clamp(low, high)
            })
    }
}

pub struct Scenario {
    pub spec: ScenarioSpec,
    build: fn(&ScenarioSpec, &Params) -> Box<dyn Lesson>,
}

/// A lesson from the catalogue, running.
pub struct Run {
    pub spec: ScenarioSpec,
    lesson: Box<dyn Lesson>,
}

impl Scenario {
    pub fn start(&self, params: &Params) -> Run {
        Run {
            spec: self.spec,
            lesson: (self.build)(&self.spec, params),
        }
    }
}

impl Run {
    pub fn advance(&mut self, seconds: f64) {
        self.lesson.advance(seconds);
    }
    pub fn finished(&self) -> bool {
        self.lesson.finished()
    }
    pub fn still(&self) -> bool {
        self.lesson.still()
    }
    pub fn clock(&self) -> String {
        self.lesson.clock()
    }
    pub fn picture(&self) -> Picture {
        self.lesson.picture()
    }
    pub fn readings(&self) -> Vec<Reading> {
        self.lesson.readings()
    }
    pub fn plots(&self) -> Vec<Plot> {
        self.lesson.plots()
    }
    pub fn answers(&self) -> Vec<Answer> {
        self.lesson.answers()
    }
}

pub fn catalogue() -> &'static [Scenario] {
    CATALOGUE
}

pub fn find(id: &str) -> Option<&'static Scenario> {
    CATALOGUE.iter().find(|s| s.spec.id == id)
}

const CURRENTS: &str = "Moving charges and magnetism";
const INDUCTION: &str = "Electromagnetic induction";
const ALTERNATING: &str = "Alternating current";

const fn syllabus(chapter: &'static str, topic: &'static str) -> Syllabus {
    Syllabus {
        class: 12,
        chapter,
        topic,
    }
}

static WIRE: [Param; 2] = [
    param("current", "Current in the wire", "A", (0.5, 30.0), 0.5, 5.0),
    param(
        "distance",
        "Distance from the wire",
        "cm",
        (1.0, 20.0),
        0.5,
        5.0,
    ),
];

static LOOP: [Param; 4] = [
    param("current", "Current in the coil", "A", (0.5, 20.0), 0.5, 5.0),
    param("turns", "Turns", "", (1.0, 200.0), 1.0, 1.0),
    param("radius", "Radius of the coil", "cm", (2.0, 20.0), 0.5, 10.0),
    param(
        "distance",
        "Distance along the axis",
        "cm",
        (0.0, 40.0),
        0.5,
        10.0,
    ),
];

static SOLENOID: [Param; 5] = [
    param("current", "Current", "A", (0.1, 10.0), 0.1, 2.0),
    param("turns", "Turns", "", (200.0, 2000.0), 10.0, 400.0),
    param("length", "Length", "cm", (10.0, 60.0), 1.0, 20.0),
    param("radius", "Radius", "cm", (1.0, 5.0), 0.1, 3.0),
    param(
        "position",
        "Point on the axis, from the centre to the end",
        "%",
        (0.0, 150.0),
        1.0,
        0.0,
    ),
];

static PARALLEL: [Param; 3] = [
    param(
        "first",
        "Current in the first wire",
        "A",
        (0.5, 50.0),
        0.5,
        10.0,
    ),
    param(
        "second",
        "Current in the second, negative for the other way",
        "A",
        (-50.0, 50.0),
        0.5,
        10.0,
    ),
    param(
        "separation",
        "Distance between them",
        "cm",
        (1.0, 20.0),
        0.5,
        5.0,
    ),
];

static CONDUCTOR: [Param; 4] = [
    param("current", "Current", "A", (0.0, 20.0), 0.5, 5.0),
    param(
        "length",
        "Length of wire in the field",
        "cm",
        (1.0, 100.0),
        1.0,
        20.0,
    ),
    param("field", "Field", "T", (0.01, 2.0), 0.01, 0.5),
    param(
        "angle",
        "Angle between the current and the field",
        "°",
        (0.0, 180.0),
        1.0,
        90.0,
    ),
];

static METER: [Param; 5] = [
    param(
        "current",
        "Current through the coil",
        "µA",
        (0.0, 100.0),
        1.0,
        50.0,
    ),
    param("turns", "Turns", "", (20.0, 200.0), 1.0, 100.0),
    param("area", "Area of the coil", "cm²", (1.0, 10.0), 0.1, 4.0),
    param("field", "Field of the magnet", "T", (0.05, 0.5), 0.01, 0.2),
    param(
        "stiffness",
        "Stiffness of the spring",
        "µN·m/rad",
        (0.2, 5.0),
        0.05,
        0.8,
    ),
];

static FARADAY: [Param; 5] = [
    param("speed", "Speed of the magnet", "m/s", (0.1, 2.0), 0.05, 0.5),
    param(
        "moment",
        "Moment of the magnet",
        "A·m²",
        (0.1, 5.0),
        0.1,
        1.0,
    ),
    param(
        "turns",
        "Turns of the coil",
        "",
        (10.0, 1000.0),
        10.0,
        200.0,
    ),
    param("radius", "Radius of the coil", "cm", (1.0, 10.0), 0.5, 3.0),
    param(
        "resistance",
        "Resistance of the circuit",
        "Ω",
        (1.0, 100.0),
        1.0,
        10.0,
    ),
];

static RAILS: [Param; 5] = [
    param("pull", "Pull on the rod", "N", (0.01, 0.5), 0.01, 0.05),
    param("field", "Field", "T", (0.1, 1.5), 0.05, 0.5),
    param(
        "gap",
        "Distance between the rails",
        "cm",
        (5.0, 50.0),
        1.0,
        20.0,
    ),
    param("resistance", "Resistance", "Ω", (0.1, 5.0), 0.1, 0.5),
    param("mass", "Mass of the rod", "g", (10.0, 500.0), 10.0, 50.0),
];

static FRAME: [Param; 4] = [
    param("field", "Field in the band", "T", (0.0, 1.0), 0.05, 0.5),
    param("side", "Side of the loop", "cm", (5.0, 20.0), 1.0, 10.0),
    param("mass", "Mass of the loop", "g", (5.0, 100.0), 1.0, 20.0),
    param(
        "resistance",
        "Resistance of the loop",
        "mΩ",
        (2.0, 100.0),
        1.0,
        20.0,
    ),
];

static GENERATOR: [Param; 4] = [
    param(
        "frequency",
        "Turns each second",
        "Hz",
        (5.0, 100.0),
        1.0,
        50.0,
    ),
    param("turns", "Turns of the coil", "", (10.0, 500.0), 10.0, 100.0),
    param(
        "area",
        "Area of the coil",
        "cm²",
        (10.0, 500.0),
        10.0,
        100.0,
    ),
    param("field", "Field", "T", (0.01, 1.0), 0.01, 0.2),
];

static INDUCTOR: [Param; 5] = [
    param(
        "current",
        "Current it is raised to",
        "A",
        (0.1, 10.0),
        0.1,
        2.0,
    ),
    param(
        "ramp",
        "Time taken to raise it",
        "ms",
        (1.0, 100.0),
        1.0,
        10.0,
    ),
    param("turns", "Turns", "", (100.0, 2000.0), 10.0, 500.0),
    param("length", "Length", "cm", (5.0, 100.0), 1.0, 30.0),
    param("radius", "Radius", "cm", (0.5, 5.0), 0.1, 2.0),
];

static PAIR: [Param; 6] = [
    param(
        "current",
        "Peak current in the first coil",
        "A",
        (0.1, 10.0),
        0.1,
        2.0,
    ),
    param("frequency", "Frequency", "Hz", (10.0, 1000.0), 10.0, 50.0),
    param(
        "primary",
        "Turns of the first coil",
        "",
        (100.0, 2000.0),
        10.0,
        500.0,
    ),
    param(
        "secondary",
        "Turns of the second coil",
        "",
        (10.0, 2000.0),
        10.0,
        200.0,
    ),
    param(
        "length",
        "Length of the first coil",
        "cm",
        (5.0, 100.0),
        1.0,
        30.0,
    ),
    param(
        "radius",
        "Radius of the first coil",
        "cm",
        (0.5, 5.0),
        0.1,
        2.0,
    ),
];

static TRANSFORMER: [Param; 5] = [
    param(
        "secondary",
        "Turns of the secondary",
        "",
        (10.0, 5000.0),
        10.0,
        1000.0,
    ),
    param(
        "primary",
        "Turns of the primary",
        "",
        (10.0, 1000.0),
        10.0,
        100.0,
    ),
    param(
        "voltage",
        "Voltage of the generator",
        "V",
        (200.0, 20000.0),
        100.0,
        4000.0,
    ),
    param(
        "power",
        "Power to be sent",
        "kW",
        (10.0, 2000.0),
        10.0,
        800.0,
    ),
    param(
        "line",
        "Resistance of the line",
        "Ω",
        (1.0, 50.0),
        1.0,
        15.0,
    ),
];

#[rustfmt::skip]
static CATALOGUE: &[Scenario] = &[
    Scenario {
        spec: ScenarioSpec {
            id: "straight-wire-field",
            title: "The field round a straight wire",
            description: "A current makes a magnetic field that circles the wire. The compass needles show its direction; it is twice as weak at twice the distance. The engine adds up the Biot–Savart law along the wire and compares the sum with μ₀I/2πr.",
            syllabus: syllabus(CURRENTS, "The Biot–Savart law and Ampere's law"),
            params: &WIRE,
        },
        build: |s, p| Box::new(StraightWire::new(p.get(s, "current"), p.get(s, "distance") * 1e-2)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "circular-loop-field",
            title: "The field on the axis of a coil",
            description: "A circular coil seen edge on: the current comes out of the page at the top and goes in at the bottom. The field threads through the coil and curls back outside, as it does for a short bar magnet. Move the point along the axis and the field falls away, at last as the cube of the distance.",
            syllabus: syllabus(CURRENTS, "Field on the axis of a circular loop"),
            params: &LOOP,
        },
        build: |s, p| Box::new(Loop::new(p.get(s, "current"), p.get(s, "turns"), p.get(s, "radius") * 1e-2, p.get(s, "distance") * 1e-2)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "solenoid-field",
            title: "The solenoid",
            description: "Many turns side by side. Inside, their fields add to one even field along the axis, μ₀nI, whatever the radius. Outside they nearly cancel. At the very end the field is half what it is deep inside, because half the turns are missing.",
            syllabus: syllabus(CURRENTS, "The solenoid"),
            params: &SOLENOID,
        },
        build: |s, p| {
            let length = p.get(s, "length") * 1e-2;
            Box::new(Solenoid { current: p.get(s, "current"), turns: p.get(s, "turns"), length, radius: p.get(s, "radius") * 1e-2, position: 0.5 * length * p.get(s, "position") / 100.0 })
        },
    },
    Scenario {
        spec: ScenarioSpec {
            id: "parallel-currents",
            title: "Force between two parallel currents",
            description: "Each wire sits in the other's field and is pushed by it. Currents the same way attract; opposite ways repel. Make the second current negative to turn it round, and watch the field between the wires go from nearly cancelling to piling up.",
            syllabus: syllabus(CURRENTS, "Force between two parallel currents; the ampere"),
            params: &PARALLEL,
        },
        build: |s, p| Box::new(Parallel::new(p.get(s, "first"), p.get(s, "second"), p.get(s, "separation") * 1e-2)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "force-on-a-wire",
            title: "Force on a current in a field",
            description: "A wire carrying current across a magnetic field is pushed sideways, at right angles to both. Turn the wire towards the field and the force shrinks; along the field there is none. This is what turns every electric motor.",
            syllabus: syllabus(CURRENTS, "Force on a current-carrying conductor"),
            params: &CONDUCTOR,
        },
        build: |s, p| Box::new(Conductor { current: p.get(s, "current"), length: p.get(s, "length") * 1e-2, field: p.get(s, "field"), angle: p.get(s, "angle").to_radians() }),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "moving-coil-galvanometer",
            title: "The moving-coil galvanometer",
            description: "A coil hangs between the curved poles of a magnet. A current through it makes a torque NIAB, and it turns until a hair spring's torque matches. The deflection is in proportion to the current, which is what makes it a meter. Press play to let the current in.",
            syllabus: syllabus(CURRENTS, "Torque on a current loop; the moving coil galvanometer"),
            params: &METER,
        },
        build: |s, p| Box::new(Galvanometer::new(p.get(s, "current") * 1e-6, p.get(s, "turns"), p.get(s, "area") * 1e-4, p.get(s, "field"), p.get(s, "stiffness") * 1e-6)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "faradays-law",
            title: "Faraday's law: a magnet through a coil",
            description: "A magnet is carried through a coil at a steady speed. An emf appears only while the flux through the coil is changing: one way as the magnet comes in, the other way as it leaves, and none at the middle where the flux is greatest. Twice the speed gives twice the emf for half as long.",
            syllabus: syllabus(INDUCTION, "Faraday's law and Lenz's law"),
            params: &FARADAY,
        },
        build: |s, p| Box::new(Faraday::new(p.get(s, "moment"), p.get(s, "speed"), p.get(s, "turns"), p.get(s, "radius") * 1e-2, p.get(s, "resistance"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "motional-emf",
            title: "Motional emf: a rod on rails",
            description: "A rod is pulled along two rails joined through a resistance, with a field through the circuit. Its motion makes an emf Blv, the emf drives a current, and the field pushes back on that current. The faster it goes the harder it is held back, until it goes no faster. From then on every joule of the pull's work becomes heat.",
            syllabus: syllabus(INDUCTION, "Motional electromotive force"),
            params: &RAILS,
        },
        build: |s, p| Box::new(Rails::new(p.get(s, "field"), p.get(s, "gap") * 1e-2, p.get(s, "resistance"), p.get(s, "pull"), p.get(s, "mass") * 1e-3)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "falling-loop",
            title: "Lenz's law: a loop falling through a field",
            description: "A square loop falls through a band of magnetic field. Going in, the flux through it grows and a current flows to oppose that, which brakes it. Wholly inside, the flux is steady: no current, and it falls freely. Coming out, the current flows the other way and brakes it again. The energy the braking takes turns up as heat, to the last millijoule. Set the field to zero to see it fall freely.",
            syllabus: syllabus(INDUCTION, "Lenz's law, conservation of energy and eddy currents"),
            params: &FRAME,
        },
        build: |s, p| Box::new(Frame::new(p.get(s, "side") * 1e-2, p.get(s, "mass") * 1e-3, p.get(s, "resistance") * 1e-3, p.get(s, "field"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "ac-generator",
            title: "The AC generator",
            description: "A coil turned steadily between the poles of a magnet. The flux through it goes as the cosine of the angle, so the emf goes as the sine: greatest when the coil lies along the field, nothing when it faces it, and reversed every half turn. That is alternating current.",
            syllabus: syllabus(INDUCTION, "The AC generator"),
            params: &GENERATOR,
        },
        build: |s, p| Box::new(Generator::new(p.get(s, "turns"), p.get(s, "area") * 1e-4, p.get(s, "field"), p.get(s, "frequency"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "self-inductance",
            title: "Self-inductance of a solenoid",
            description: "Raise the current in a coil and its own growing flux induces an emf against the rise; bring the current down and the emf tries to keep it going. A steady current induces nothing. The work done against that emf while the current rises is stored in the field, ½LI².",
            syllabus: syllabus(INDUCTION, "Self-inductance and the energy in an inductor"),
            params: &INDUCTOR,
        },
        build: |s, p| Box::new(SelfInduction::new(p.get(s, "turns"), p.get(s, "length") * 1e-2, p.get(s, "radius") * 1e-2, p.get(s, "current"), p.get(s, "ramp") * 1e-3)),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "mutual-inductance",
            title: "Mutual inductance of two coils",
            description: "An alternating current in one coil sends a changing flux through a second coil wound round it, and an emf appears there though no wire joins them. The emf is a quarter of a cycle out of step with the current: greatest when the current is passing through zero.",
            syllabus: syllabus(INDUCTION, "Mutual inductance"),
            params: &PAIR,
        },
        build: |s, p| Box::new(MutualInduction::new(p.get(s, "primary"), p.get(s, "secondary"), p.get(s, "length") * 1e-2, p.get(s, "radius") * 1e-2, p.get(s, "current"), p.get(s, "frequency"))),
    },
    Scenario {
        spec: ScenarioSpec {
            id: "transformer",
            title: "The transformer and the power line",
            description: "Two coils on one iron core share the same changing flux, so their voltages are in the ratio of their turns. Stepping the voltage up brings the current down for the same power, and the heat wasted in a long line goes as the square of the current. That is why power travels at hundreds of kilovolts.",
            syllabus: syllabus(ALTERNATING, "Transformers"),
            params: &TRANSFORMER,
        },
        build: |s, p| Box::new(Transformer { voltage: p.get(s, "voltage"), primary: p.get(s, "primary"), secondary: p.get(s, "secondary"), power: p.get(s, "power") * 1e3, line: p.get(s, "line") }),
    },
];
